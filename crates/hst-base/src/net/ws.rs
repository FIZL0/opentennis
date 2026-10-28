//! M156: browser guests with no WebRTC (an iPhone in Lockdown Mode, WebKitGTK, a privacy extension) join a native
//! host over a WebSocket on its adhoc http server (`adhoc.rs`, `GET /adhoc/ws?name=<peer>`) instead of WebRTC's data
//! channels. Once open it's the same lobby protocol as `rtc.rs`'s: text frames carry the JSON messages, binary frames
//! rollback's datagrams (in order and never lost here, as TCP is: they just come later on a bad link). A guest picks
//! its own peer name and sends it in the request, so it knows the name the host knows it by.
//!
//! The browser picks the transport itself ([`join`]): a host code (`rtch<host:port>`) dials WebRTC where the browser
//! has `RTCPeerConnection` and falls back to the WebSocket when it hasn't, or when WebRTC can't get through (UDP
//! blocked by the host's firewall while TCP is let in, M152c1).

#[cfg(not(target_arch = "wasm32"))]
pub use native::{Peer, accept, upgrade};
#[cfg(target_arch = "wasm32")]
pub use web::Peer;

/// The adhoc server's WebSocket path.
pub const PATH: &str = "/adhoc/ws";

/// A guest's peer name as it may be asked for: 1 to 32 letters, digits, `-` or `_`.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn good_name(name: &str) -> bool {
    (1..=32).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Connect to a host's address from a host code: WebRTC where it works, else (in the browser) the host's WebSocket.
pub(in crate::net) async fn join(addr: &str) -> Result<(super::Conn, super::Link, super::Inbox, String), String> {
    #[cfg(target_arch = "wasm32")]
    {
        if !web::has_rtc() {
            bevy::log::info!("online: this browser has no WebRTC: joining {addr} over a WebSocket");
            return web::dial(addr).await;
        }
        match super::rtc::dial(addr).await {
            Ok(c) => Ok(c),
            Err(e) => {
                bevy::log::info!("online: WebRTC to {addr} failed ({e}): trying a WebSocket");
                // the WebRTC failure says more (no lobby, the firewall's UDP) when the WebSocket can't either
                web::dial(addr).await.map_err(|_| e)
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    super::rtc::dial(addr).await
}

// ---------------------------------------------------------------- the host's end (and a test's guest)

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::super::{Conn, Inbox, Link, Msg, rtc};
    use super::good_name;
    use std::io::{self, Write};
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
    use tokio::sync::mpsc as amp;

    /// RFC 6455's key suffix.
    const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    /// The biggest message taken (a resync's state is the largest, well under this).
    const MAX: u64 = 64 << 20;

    enum Out {
        Text(String),
        Bin(Vec<u8>),
        Pong(Vec<u8>),
        Close,
    }

    /// One WebSocket connection (a writer task sends what's queued, in order).
    #[derive(Clone)]
    pub struct Peer(amp::UnboundedSender<Out>);

    impl Peer {
        pub fn send_datagram(&self, d: &[u8]) {
            let _ = self.0.send(Out::Bin(d.to_vec()));
        }

        pub fn close(&self) {
            let _ = self.0.send(Out::Close);
        }
    }

    /// A WebSocket upgrade's head (`GET /adhoc/ws?name=…`): its key and the guest's name. None for any other request.
    pub fn upgrade(head: &str) -> Option<(String, String)> {
        let mut lines = head.split("\r\n");
        let mut first = lines.next()?.split(' ');
        let (method, target) = (first.next()?, first.next()?);
        let (path, query) = target.split_once('?').unwrap_or((target, ""));
        if method != "GET" || path != super::PATH {
            return None;
        }
        let (mut key, mut up) = (None, false);
        for l in lines {
            let Some((k, v)) = l.split_once(':') else { continue };
            if k.trim().eq_ignore_ascii_case("upgrade") {
                up = v.trim().eq_ignore_ascii_case("websocket");
            } else if k.trim().eq_ignore_ascii_case("sec-websocket-key") {
                key = Some(v.trim().to_string());
            }
        }
        let name = query.split('&').find_map(|kv| kv.strip_prefix("name=")).unwrap_or("").to_string();
        up.then_some((key?, name))
    }

    /// The handshake's `Sec-WebSocket-Accept` for a request's key.
    pub(super) fn accept_key(key: &str) -> String {
        data_encoding::BASE64.encode(&sha1(format!("{key}{GUID}").as_bytes()))
    }

    /// Takes over `c` after an upgrade's head ([`upgrade`]): switched to a WebSocket and handed to the hosting lobby
    /// as `name`, or refused (no lobby open, a bad name).
    pub fn accept(mut c: std::net::TcpStream, (key, name): (String, String)) -> io::Result<()> {
        let no = |c: &mut std::net::TcpStream, status: &str, why: &str| {
            let r = format!("HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{why}", why.len());
            c.write_all(r.as_bytes())
        };
        if !good_name(&name) {
            return no(&mut c, "400 Bad Request", "A guest's name is 1 to 32 letters or digits.\n");
        }
        if !rtc::lobby_open() {
            return no(&mut c, "503 Service Unavailable", "No lobby is open on this machine.\n");
        }
        let r = format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n", accept_key(&key));
        c.write_all(r.as_bytes())?;
        c.set_read_timeout(None)?;
        c.set_nodelay(true)?;
        c.set_nonblocking(true)?;
        crate::runtime().spawn(async move {
            match tokio::net::TcpStream::from_std(c) {
                Ok(s) => {
                    bevy::log::info!("online: guest {name} joined over a WebSocket");
                    let (conn, link, inbox) = open(s, false);
                    rtc::arrived(name, conn, link, inbox);
                }
                Err(e) => bevy::log::warn!("online: a guest's WebSocket: {e}"),
            }
        });
        Ok(())
    }

    /// A test's guest: a WebSocket to `host`'s adhoc server as `name`.
    #[cfg(test)]
    pub(in crate::net) async fn dial(host: &str, name: &str) -> Result<(Conn, Link, Inbox), String> {
        let mut s = tokio::net::TcpStream::connect(host).await.map_err(|e| e.to_string())?;
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        s.write_all(format!("GET {}?name={name} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n", super::PATH).as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        // the reply's head, a byte at a time (nothing of the frames after it is read)
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            head.push(s.read_u8().await.map_err(|e| e.to_string())?);
        }
        let head = String::from_utf8_lossy(&head);
        if !head.starts_with("HTTP/1.1 101") || !head.contains(&accept_key(key)) {
            return Err(head.into_owned());
        }
        s.set_nodelay(true).map_err(|e| e.to_string())?;
        Ok(open(s, true))
    }

    /// A WebSocket's connection, link and arrivals; `client`: our frames are masked (the guest's end).
    fn open(s: tokio::net::TcpStream, client: bool) -> (Conn, Link, Inbox) {
        let (mut rd, mut wr) = s.into_split();
        let (out, mut outs) = amp::unbounded_channel::<Out>();
        n0_future::task::spawn(async move {
            while let Some(o) = outs.recv().await {
                let (op, body) = match &o {
                    Out::Text(t) => (1, t.as_bytes()),
                    Out::Bin(b) => (2, b.as_slice()),
                    Out::Pong(b) => (10, b.as_slice()),
                    Out::Close => (8, &[][..]),
                };
                if wr.write_all(&frame(op, body, client)).await.is_err() || matches!(o, Out::Close) {
                    break;
                }
            }
            let _ = wr.shutdown().await;
        });
        let peer = Peer(out);
        let (mt, msgs) = amp::unbounded_channel();
        let (dt, dgrams) = amp::unbounded_channel();
        let p = peer.clone();
        n0_future::task::spawn(async move {
            let why = read(&mut rd, &p, &mt, &dt).await;
            let _ = mt.send(Err(why));
        });
        (Conn::Ws(peer.clone()), link(peer), Inbox { msgs, dgrams })
    }

    /// Messages out as text frames, each after [`super::super::LAG_MS`], in order.
    fn link(peer: Peer) -> Link {
        let (s, mut r) = amp::unbounded_channel::<(n0_future::time::Instant, Msg)>();
        n0_future::task::spawn(async move {
            while let Some((due, m)) = r.recv().await {
                n0_future::time::sleep_until(due).await;
                if peer.0.send(Out::Text(serde_json::to_string(&m).unwrap())).is_err() {
                    break;
                }
            }
        });
        Link(s)
    }

    /// Reads messages and datagrams until the connection ends: why it did.
    async fn read(
        rd: &mut (impl AsyncRead + Unpin),
        peer: &Peer,
        msgs: &amp::UnboundedSender<Result<Msg, String>>,
        dgrams: &amp::UnboundedSender<Vec<u8>>,
    ) -> String {
        // a message in fragments so far (its opcode); control frames can come between them
        let mut part: Option<(u8, Vec<u8>)> = None;
        loop {
            let (fin, op, body) = match next_frame(rd).await {
                Ok(f) => f,
                Err(e) => return format!("connection lost: {e}"),
            };
            match op {
                8 => {
                    peer.close();
                    return "closed".into();
                }
                9 => drop(peer.0.send(Out::Pong(body))),
                10 => {}
                0 => match part.as_mut() {
                    Some((_, b)) if (b.len() + body.len()) as u64 <= MAX => b.extend(body),
                    _ => return "a bad WebSocket frame".into(),
                },
                1 | 2 if part.is_none() => part = Some((op, body)),
                _ => return "a bad WebSocket frame".into(),
            }
            if fin && op < 8 {
                let gone = match part.take() {
                    Some((1, b)) => msgs.send(String::from_utf8(b).map_err(|e| e.to_string()).and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))).is_err(),
                    Some((_, b)) => dgrams.send(b).is_err(),
                    None => false,
                };
                if gone {
                    return "dropped".into();
                }
            }
        }
    }

    /// One frame: final, opcode, payload (unmasked).
    pub(super) async fn next_frame(rd: &mut (impl AsyncRead + Unpin)) -> io::Result<(bool, u8, Vec<u8>)> {
        let mut h = [0u8; 2];
        rd.read_exact(&mut h).await?;
        let len = match h[1] & 0x7f {
            126 => rd.read_u16().await? as u64,
            127 => rd.read_u64().await?,
            n => n as u64,
        };
        if len > MAX {
            return Err(io::Error::other("a message too big"));
        }
        let mut mask = [0u8; 4];
        if h[1] & 0x80 != 0 {
            rd.read_exact(&mut mask).await?;
        }
        let mut body = vec![0u8; len as usize];
        rd.read_exact(&mut body).await?;
        if h[1] & 0x80 != 0 {
            body.iter_mut().enumerate().for_each(|(i, b)| *b ^= mask[i % 4]);
        }
        Ok((h[0] & 0x80 != 0, h[0] & 0x0f, body))
    }

    /// One whole frame of `op` (a client's masked, as RFC 6455 wants; the key needn't be secret here).
    pub(super) fn frame(op: u8, body: &[u8], masked: bool) -> Vec<u8> {
        let mut f = vec![0x80 | op];
        let m = if masked { 0x80 } else { 0 };
        match body.len() {
            n if n < 126 => f.push(m | n as u8),
            n if n <= 0xffff => {
                f.push(m | 126);
                f.extend((n as u16).to_be_bytes());
            }
            n => {
                f.push(m | 127);
                f.extend((n as u64).to_be_bytes());
            }
        }
        if masked {
            let key = (body.len() as u32 ^ 0x5a3c_96e1).to_be_bytes();
            f.extend(key);
            f.extend(body.iter().enumerate().map(|(i, b)| b ^ key[i % 4]));
        } else {
            f.extend_from_slice(body);
        }
        f
    }

    /// SHA-1 (only the handshake's key needs it).
    pub(super) fn sha1(data: &[u8]) -> [u8; 20] {
        let mut h: [u32; 5] = [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476, 0xc3d2_e1f0];
        let mut m = data.to_vec();
        m.push(0x80);
        while m.len() % 64 != 56 {
            m.push(0);
        }
        m.extend((data.len() as u64 * 8).to_be_bytes());
        for block in m.chunks(64) {
            let mut w = [0u32; 80];
            for i in 0..16 {
                w[i] = u32::from_be_bytes(block[4 * i..4 * i + 4].try_into().unwrap());
            }
            for i in 16..80 {
                w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
            }
            let [mut a, mut b, mut c, mut d, mut e] = h;
            for (i, wi) in w.iter().enumerate() {
                let (f, k) = match i {
                    0..20 => ((b & c) | (!b & d), 0x5a82_7999),
                    20..40 => (b ^ c ^ d, 0x6ed9_eba1),
                    40..60 => ((b & c) | (b & d) | (c & d), 0x8f1b_bcdc),
                    _ => (b ^ c ^ d, 0xca62_c1d6),
                };
                let t = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*wi);
                (e, d, c, b, a) = (d, c, b.rotate_left(30), a, t);
            }
            for (x, y) in h.iter_mut().zip([a, b, c, d, e]) {
                *x = x.wrapping_add(y);
            }
        }
        let mut out = [0u8; 20];
        for (i, x) in h.iter().enumerate() {
            out[4 * i..4 * i + 4].copy_from_slice(&x.to_be_bytes());
        }
        out
    }
}

// ---------------------------------------------------------------- the browser's end

#[cfg(target_arch = "wasm32")]
mod web {
    use super::super::{Conn, Inbox, Link, Msg};
    use n0_future::time::{Duration, Instant, sleep};
    use tokio::sync::mpsc as amp;
    use wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use web_sys::{BinaryType, MessageEvent, WebSocket};

    /// One WebSocket to the host.
    #[derive(Clone)]
    pub struct Peer(WebSocket);

    impl Peer {
        pub fn send_datagram(&self, d: &[u8]) {
            let _ = self.0.send_with_u8_array(d);
        }

        pub fn close(&self) {
            let _ = self.0.close();
        }
    }

    /// Whether this browser has WebRTC (off in Lockdown Mode, some privacy settings, WebKitGTK).
    pub fn has_rtc() -> bool {
        js_sys::Reflect::has(&js_sys::global(), &"RTCPeerConnection".into()).unwrap_or(false)
    }

    fn js(e: JsValue) -> String {
        e.as_string().or_else(|| e.dyn_ref::<js_sys::Error>().map(|e| String::from(e.message()))).unwrap_or_else(|| format!("{e:?}"))
    }

    /// A WebSocket to `host`'s adhoc server (10 s to open at most): the connection, its link and arrivals, and our
    /// peer name (made up here, sent in the request).
    pub(in crate::net) async fn dial(host: &str) -> Result<(Conn, Link, Inbox, String), String> {
        let me: String = (0..12).map(|_| char::from_digit((js_sys::Math::random() * 16.0) as u32 % 16, 16).unwrap()).collect();
        let ws = WebSocket::new(&format!("ws://{host}{}?name=ws{me}", super::PATH)).map_err(js)?;
        ws.set_binary_type(BinaryType::Arraybuffer);
        let (mt, msgs) = amp::unbounded_channel();
        let (dt, dgrams) = amp::unbounded_channel();
        // ponytail: the handlers are leaked (a few closures a connection), as rtc.rs's are
        let m = mt.clone();
        let on_msg = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
            let d = e.data();
            match d.as_string() {
                Some(s) => drop(m.send(serde_json::from_str::<Msg>(&s).map_err(|e| e.to_string()))),
                None => drop(dt.send(js_sys::Uint8Array::new(&d).to_vec())),
            }
        });
        ws.set_onmessage(Some(on_msg.as_ref().unchecked_ref()));
        on_msg.forget();
        let on_close = Closure::<dyn FnMut()>::new(move || {
            let _ = mt.send(Err("closed".into()));
        });
        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
        on_close.forget();
        let t = Instant::now();
        loop {
            match ws.ready_state() {
                WebSocket::OPEN => break,
                WebSocket::CONNECTING if t.elapsed() < Duration::from_secs(10) => sleep(Duration::from_millis(50)).await,
                _ => {
                    let _ = ws.close();
                    return Err("Couldn't join the host over its web server: is its lobby still open?".into());
                }
            }
        }
        let peer = Peer(ws);
        Ok((Conn::Ws(peer.clone()), link(peer), Inbox { msgs, dgrams }, format!("ws{me}")))
    }

    /// Messages out as text frames, each after [`super::super::LAG_MS`], in order.
    fn link(peer: Peer) -> Link {
        let (s, mut r) = amp::unbounded_channel::<(Instant, Msg)>();
        n0_future::task::spawn(async move {
            while let Some((due, m)) = r.recv().await {
                n0_future::time::sleep_until(due).await;
                if peer.0.send_with_str(&serde_json::to_string(&m).unwrap()).is_err() {
                    break;
                }
            }
        });
        Link(s)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::native::*;
    use super::super::{Msg, rtc};
    use std::time::Duration;

    #[test]
    fn handshake_parts() {
        // RFC 6455's own example, and SHA-1's
        assert_eq!(accept_key("dGhlIHNhbXBsZSBub25jZQ=="), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
        let hex = |b: [u8; 20]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
        assert_eq!(hex(sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(hex(sha1(&[b'a'; 1000])), "291e9a6c66994949b57ba5e650361e98fc36b1ba");
        let head = "GET /adhoc/ws?name=ws1a2b HTTP/1.1\r\nHost: x\r\nUpgrade: WebSocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: abc==";
        assert_eq!(upgrade(head), Some(("abc==".into(), "ws1a2b".into())));
        assert_eq!(upgrade("GET /adhoc/ws HTTP/1.1\r\nSec-WebSocket-Key: abc"), None);
        assert_eq!(upgrade("GET /adhoc/info.json HTTP/1.1\r\nUpgrade: websocket\r\nSec-WebSocket-Key: abc"), None);
        assert!(super::good_name("ws0123abcdef") && !super::good_name("") && !super::good_name("a b") && !super::good_name(&"a".repeat(33)));
    }

    /// M156: a guest joins the hosting lobby over the adhoc server's WebSocket, and both ends talk (a message of
    /// 100 kB: 64-bit frame lengths; datagrams as binary frames); a close is seen at once.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_guest_joins_over_a_websocket() {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap().to_string();
        // the adhoc server's part: an upgrade's head read, then `accept`
        std::thread::spawn(move || {
            for c in l.incoming().flatten() {
                let mut c = c;
                let mut head = Vec::new();
                let mut b = [0u8; 1];
                while !head.ends_with(b"\r\n\r\n") && std::io::Read::read(&mut c, &mut b).unwrap_or(0) == 1 {
                    head.push(b[0]);
                }
                let _ = accept(c, upgrade(&String::from_utf8_lossy(&head)).unwrap());
            }
        });
        let _one = rtc::tests::SIGNALLING.lock().await;
        drop(rtc::hosting());
        // no lobby open: refused
        assert!(super::native::dial(&addr, "g1").await.err().unwrap().starts_with("HTTP/1.1 503"));
        let mut lobby = rtc::hosting();
        assert!(super::native::dial(&addr, "a.b").await.err().unwrap().starts_with("HTTP/1.1 400"));
        let (gconn, glink, mut ginbox) = super::native::dial(&addr, "g1").await.unwrap();
        let within = Duration::from_secs(5);
        let (name, hconn, hlink, mut hinbox) = tokio::time::timeout(within, lobby.recv()).await.unwrap().unwrap();
        assert_eq!(name, "g1");
        glink.send(&Msg::Ping(7));
        assert_eq!(tokio::time::timeout(within, hinbox.msgs.recv()).await.unwrap(), Some(Ok(Msg::Ping(7))));
        hlink.send(&Msg::Bye("x".repeat(100_000)));
        assert_eq!(tokio::time::timeout(within, ginbox.msgs.recv()).await.unwrap(), Some(Ok(Msg::Bye("x".repeat(100_000)))));
        hconn.send_datagram(vec![1, 2, 3]);
        gconn.send_datagram(vec![4; 300]);
        assert_eq!(tokio::time::timeout(within, ginbox.dgrams.recv()).await.unwrap(), Some(vec![1, 2, 3]));
        assert_eq!(tokio::time::timeout(within, hinbox.dgrams.recv()).await.unwrap(), Some(vec![4; 300]));
        gconn.close(b"bye");
        assert!(matches!(tokio::time::timeout(within, hinbox.msgs.recv()).await.unwrap(), Some(Err(_))));
        assert!(matches!(tokio::time::timeout(within, ginbox.msgs.recv()).await.unwrap(), Some(Err(_))));
    }

    #[test]
    fn frames_carry_any_length() {
        for n in [0, 125, 126, 65535, 65536] {
            let body: Vec<u8> = (0..n).map(|i| i as u8).collect();
            for masked in [false, true] {
                let f = frame(2, &body, masked);
                let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
                let (fin, op, back) = rt.block_on(super::native::next_frame(&mut f.as_slice())).unwrap();
                assert_eq!((fin, op, back), (true, 2, body.clone()), "{n} bytes, masked {masked}");
            }
        }
    }
}

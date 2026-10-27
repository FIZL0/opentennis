//! M152b: the browser's LAN on a native build. A desktop or phone hosts browser guests over WebRTC (str0m, a sans-IO
//! stack driven here on tokio's UDP sockets), on the same two channels as the browser's own (`web` in `rtc.rs`): JSON
//! lobby messages on a reliable ordered channel 0, rollback's datagrams on channel 1 (unordered, no retransmits).
//!
//! No codes to pass by hand: the guest POSTs its offer code to the host's `/rtc` and gets the answer code back in the
//! reply ([`listen`]; M152a's page server can route `/rtc` to [`signal`] instead). The guest offers, so the host keeps
//! nothing between requests. The browser's own candidates are usually mDNS names, which we can't resolve: its ICE
//! checks reach our host candidates anyway, and str0m takes the address they come from (peer-reflexive).

use super::super::{Conn, Inbox, Link, Msg};
use super::{ANSWER, OFFER, Sig};
use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use str0m::change::{SdpAnswer, SdpOffer, SdpPendingOffer};
use str0m::channel::{ChannelConfig, ChannelId, Reliability};
use str0m::net::{Protocol, Receive};
use str0m::{Candidate, Event, IceConnectionState, Input, Output, Rtc};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc as amp, oneshot, watch};

/// Out to the connection's driver.
enum Out {
    Msg(String),
    Dgram(Vec<u8>),
    Close,
}

/// One WebRTC connection (its driver task owns the `Rtc`).
#[derive(Clone)]
pub struct Peer(amp::UnboundedSender<Out>);

impl Peer {
    pub fn send_datagram(&self, d: &[u8]) {
        let _ = self.0.send(Out::Dgram(d.to_vec()));
    }

    pub fn close(&self) {
        let _ = self.0.send(Out::Close);
    }
}

/// Tests: candidates on loopback only (a sandbox may have no LAN).
#[cfg(test)]
pub static LOOPBACK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// This machine's addresses a guest on the LAN can reach: IPv4 of the interfaces that are up, not loopback.
fn addresses() -> Vec<IpAddr> {
    #[cfg(test)]
    if LOOPBACK.load(std::sync::atomic::Ordering::Relaxed) {
        return vec![IpAddr::from([127, 0, 0, 1])];
    }
    let mut ips: Vec<IpAddr> = netdev::get_interfaces()
        .into_iter()
        .filter(|i| i.is_up() && !i.is_loopback())
        .flat_map(|i| i.ipv4.into_iter().map(|n| IpAddr::V4(n.addr())))
        .filter(|ip| !ip.is_loopback() && !ip.is_unspecified())
        .collect();
    ips.dedup();
    ips
}

/// A packet in: its bytes, where from, which of our addresses it came to.
type Packet = (Vec<u8>, SocketAddr, SocketAddr);

/// A connection's sockets: its own (each read on a task of its own until the connection's packet receiver drops), or
/// the pinned port's, shared with every other guest's ([`Mux`]: what we send to an address routes it back to us).
struct Socks {
    socks: Vec<Arc<UdpSocket>>,
    mux: Option<(Arc<Mux>, amp::Sender<Packet>)>,
}

const NO_ADDRESS: &str = "No network address: join a Wi-Fi network (or a phone's hotspot) first.";

/// A fresh `Rtc` with a socket and host candidate per address, the sockets' packets into the receiver.
async fn bind() -> Result<(Rtc, Socks, amp::Receiver<Packet>), String> {
    let mut rtc = Rtc::builder().build(Instant::now());
    let (pt, packets) = amp::channel::<Packet>(256);
    let mut socks = Vec::new();
    for ip in addresses() {
        let Ok(s) = UdpSocket::bind(SocketAddr::new(ip, 0)).await else {
            continue;
        };
        let Ok(at) = s.local_addr() else { continue };
        if let Ok(c) = Candidate::host(at, "udp") {
            rtc.add_local_candidate(c);
            let s = Arc::new(s);
            n0_future::task::spawn(read_own(s.clone(), at, pt.clone()));
            socks.push(s);
        }
    }
    if socks.is_empty() {
        return Err(NO_ADDRESS.into());
    }
    Ok((rtc, Socks { socks, mux: None }, packets))
}

/// One of a connection's own sockets, read until the connection ends (`recv_from` waits on one socket).
async fn read_own(s: Arc<UdpSocket>, at: SocketAddr, pt: amp::Sender<Packet>) {
    let mut buf = vec![0; 2000];
    loop {
        tokio::select! {
            r = s.recv_from(&mut buf) => match r {
                Ok((n, from)) => if pt.send((buf[..n].to_vec(), from, at)).await.is_err() { return },
                Err(_) => continue,
            },
            _ = pt.closed() => return,
        }
    }
}

// ---------------------------------------------------------------- the pinned port

/// M152c1: while adhoc serves, every guest's WebRTC goes through one UDP port, the page's own port number, so one
/// firewall rule (TCP and UDP on that port) covers adhoc. One socket per address for all guests; packets go to the
/// connection by the address they come from: one we sent to, or, the first time, the ICE check's user name (it starts
/// with our side's ufrag).
struct Mux {
    port: u16,
    socks: std::sync::Mutex<Vec<Arc<UdpSocket>>>,
    routes: std::sync::Mutex<HashMap<SocketAddr, amp::Sender<Packet>>>,
    ufrags: std::sync::Mutex<HashMap<String, amp::Sender<Packet>>>,
    /// Dropped with the mux: its readers stop and the port closes.
    alive: watch::Sender<()>,
}

/// The pinned port; the mux on it once a guest has come (connections hold it too, so a match outlives `pin(None)`).
static PINNED: std::sync::Mutex<(Option<u16>, Option<Arc<Mux>>)> =
    std::sync::Mutex::new((None, None));

/// Pins browser guests' WebRTC to UDP `port` (`None`: each connection on ports of its own again, the default).
pub fn pin(port: Option<u16>) {
    let mut p = PINNED.lock().unwrap();
    if p.0 != port {
        *p = (port, None);
    }
}

/// The pinned port's mux, made the first time (in the runtime: its readers are tasks); None when nothing is pinned.
fn mux() -> Option<Arc<Mux>> {
    let mut p = PINNED.lock().unwrap();
    let port = p.0?;
    Some(
        p.1.get_or_insert_with(|| {
            Arc::new(Mux {
                port,
                socks: Default::default(),
                routes: Default::default(),
                ufrags: Default::default(),
                alive: watch::channel(()).0,
            })
        })
        .clone(),
    )
}

impl Mux {
    /// Its sockets, one per address now up (bound the first time an address is seen; one that's taken is left out).
    fn sockets(self: &Arc<Self>) -> Vec<Arc<UdpSocket>> {
        let mut socks = self.socks.lock().unwrap();
        for ip in addresses() {
            let at = SocketAddr::new(ip, self.port);
            if socks.iter().any(|s| s.local_addr().is_ok_and(|a| a == at)) {
                continue;
            }
            let bound = std::net::UdpSocket::bind(at)
                .and_then(|s| s.set_nonblocking(true).map(|_| s))
                .and_then(UdpSocket::from_std);
            match bound {
                Ok(s) => {
                    let s = Arc::new(s);
                    n0_future::task::spawn(read_mux(
                        Arc::downgrade(self),
                        s.clone(),
                        at,
                        self.alive.subscribe(),
                    ));
                    socks.push(s);
                }
                Err(e) => {
                    bevy::log::info!("LAN: UDP {at} is taken ({e}): guests can't use that address")
                }
            }
        }
        socks.clone()
    }

    /// A packet to the connection it belongs to, if any.
    fn deliver(&self, data: Vec<u8>, from: SocketAddr, at: SocketAddr) {
        let mut routes = self.routes.lock().unwrap();
        let to = match routes.get(&from).filter(|t| !t.is_closed()) {
            Some(t) => Some(t.clone()),
            None => stun_ufrag(&data)
                .and_then(|u| {
                    self.ufrags
                        .lock()
                        .unwrap()
                        .get(u)
                        .filter(|t| !t.is_closed())
                        .cloned()
                })
                .inspect(|t| drop(routes.insert(from, t.clone()))),
        };
        if let Some(t) = to {
            let _ = t.try_send((data, from, at));
        }
    }

    /// `ufrag` (our side's) and what we send to belong to the connection reading `t`; ended connections' go.
    fn claim(&self, ufrag: Option<&str>, to: Option<SocketAddr>, t: &amp::Sender<Packet>) {
        if let Some(u) = ufrag {
            let mut ufrags = self.ufrags.lock().unwrap();
            ufrags.retain(|_, t| !t.is_closed());
            ufrags.insert(u.to_string(), t.clone());
        }
        if let Some(a) = to {
            let mut routes = self.routes.lock().unwrap();
            if !routes.get(&a).is_some_and(|r| r.same_channel(t)) {
                routes.retain(|_, t| !t.is_closed());
                routes.insert(a, t.clone());
            }
        }
    }
}

/// Tests: the pinned port's sockets' addresses and how many live connections it routes to.
#[cfg(test)]
pub(super) fn pinned() -> Option<(Vec<SocketAddr>, usize)> {
    let m = PINNED.lock().unwrap().1.clone()?;
    let at = m
        .socks
        .lock()
        .unwrap()
        .iter()
        .filter_map(|s| s.local_addr().ok())
        .collect();
    let mut conns: Vec<amp::Sender<Packet>> = Vec::new();
    for t in m.routes.lock().unwrap().values().filter(|t| !t.is_closed()) {
        if !conns.iter().any(|c| c.same_channel(t)) {
            conns.push(t.clone());
        }
    }
    Some((at, conns.len()))
}

/// One of the pinned port's sockets, read until the mux goes.
async fn read_mux(
    mux: std::sync::Weak<Mux>,
    s: Arc<UdpSocket>,
    at: SocketAddr,
    mut alive: watch::Receiver<()>,
) {
    let mut buf = vec![0; 2000];
    loop {
        tokio::select! {
            r = s.recv_from(&mut buf) => match (r, mux.upgrade()) {
                (Ok((n, from)), Some(m)) => m.deliver(buf[..n].to_vec(), from, at),
                (_, None) => return,
                (Err(_), _) => continue,
            },
            _ = alive.changed() => return,
        }
    }
}

/// The ufrag a STUN binding request is for (its user name is `<receiver's ufrag>:<sender's>`); None for anything else.
fn stun_ufrag(p: &[u8]) -> Option<&str> {
    let word = |i: usize| {
        p.get(i..i + 2)
            .map(|b| u16::from_be_bytes([b[0], b[1]]) as usize)
    };
    if word(0)? != 1 || p.get(4..8)? != [0x21, 0x12, 0xa4, 0x42] {
        return None;
    }
    let end = (20 + word(2)?).min(p.len());
    let mut i = 20;
    while i + 4 <= end {
        let (kind, len) = (word(i)?, word(i + 2)?);
        let v = p.get(i + 4..i + 4 + len)?;
        if kind == 6 {
            return std::str::from_utf8(v).ok()?.split(':').next();
        }
        i += 4 + len.div_ceil(4) * 4;
    }
    None
}

/// The host's sockets for a guest: the pinned port's while one is pinned (and bound), else its own.
async fn bind_host() -> Result<(Rtc, Socks, amp::Receiver<Packet>), String> {
    let Some(mux) = mux() else {
        return bind().await;
    };
    let socks = mux.sockets();
    if socks.is_empty() {
        return bind().await;
    }
    let mut rtc = Rtc::builder().build(Instant::now());
    for s in &socks {
        if let Ok(c) = s
            .local_addr()
            .map_err(|e| e.to_string())
            .and_then(|a| Candidate::host(a, "udp").map_err(|e| e.to_string()))
        {
            rtc.add_local_candidate(c);
        }
    }
    let (pt, packets) = amp::channel::<Packet>(256);
    Ok((
        rtc,
        Socks {
            socks,
            mux: Some((mux, pt)),
        },
        packets,
    ))
}

/// `sig` as the description str0m reads: IP candidates only (no mDNS names); `setup`: the DTLS role the remote takes.
fn remote_sdp(sig: &Sig, offer: bool, setup: &str) -> String {
    let ips = Sig {
        cands: sig
            .cands
            .iter()
            .filter(|(a, _)| a.parse::<IpAddr>().is_ok())
            .cloned()
            .collect(),
        ..sig.clone()
    };
    let sdp = ips.sdp(offer);
    let from = if offer {
        "a=setup:actpass"
    } else {
        "a=setup:active"
    };
    sdp.replace(from, &format!("a=setup:{setup}"))
}

fn ours(sdp: String) -> Result<Sig, String> {
    Sig::from_sdp(&sdp).ok_or_else(|| "our own description has no ICE/DTLS details".into())
}

const CHANNELS: [(u16, &str, bool); 2] = [(0, "msgs", true), (1, "dgrams", false)];

fn config((id, label, reliable): (u16, &str, bool)) -> ChannelConfig {
    ChannelConfig {
        label: label.into(),
        ordered: reliable,
        reliability: if reliable {
            Reliability::Reliable
        } else {
            Reliability::MaxRetransmits { retransmits: 0 }
        },
        negotiated: Some(id),
        protocol: String::new(),
    }
}

/// The host's side: the answer to a guest's offer `code` (its code) and the connection, opening in the background.
pub(in crate::net) async fn accept(code: &str) -> Result<(String, Opening), String> {
    let o = Sig::parse(code, OFFER).ok_or("not a LAN offer")?;
    let (mut rtc, socks, packets) = bind_host().await?;
    // the browser offers actpass and the answer code can't say which role we took: the browser reads it as active
    // (`Sig::sdp`), so we're the DTLS client, which str0m picks for a passive remote
    let offer = SdpOffer::from_sdp_string(&remote_sdp(&o, true, "passive"))
        .map_err(|e| format!("the guest's offer: {e}"))?;
    let answer = rtc
        .sdp_api()
        .accept_offer(offer)
        .map_err(|e| e.to_string())?;
    let mut a = ours(answer.to_sdp_string())?;
    if let Some((m, t)) = &socks.mux {
        m.claim(Some(&a.ufrag), None, t);
    }
    let _ = drain(&mut rtc, &socks, &mut |_| {});
    a.to = o.ufrag.clone();
    let ids = channels(&mut rtc, &socks, |rtc, c| {
        rtc.direct_api().create_data_channel(c)
    });
    Ok((a.code(ANSWER), drive(rtc, socks, packets, ids, o.ufrag)))
}

/// Our channels' ids, made one at a time (str0m: every change is drained before the next).
fn channels(
    rtc: &mut Rtc,
    socks: &Socks,
    mut make: impl FnMut(&mut Rtc, ChannelConfig) -> ChannelId,
) -> [ChannelId; 2] {
    CHANNELS.map(|c| {
        let id = make(rtc, config(c));
        let _ = drain(rtc, socks, &mut |_| {});
        id
    })
}

/// The guest's side, natively: offer to the host's signalling address (`host:port`), connect with its answer.
pub(in crate::net) async fn dial(host: &str) -> Result<(Conn, Link, Inbox, String), String> {
    let (mut rtc, socks, packets) = bind().await?;
    let mut change = rtc.sdp_api();
    let ids = CHANNELS.map(|c| change.add_channel_with_config(config(c)));
    let (offer, pending): (SdpOffer, SdpPendingOffer) = change.apply().ok_or("no offer")?;
    let _ = drain(&mut rtc, &socks, &mut |_| {});
    let o = ours(offer.to_sdp_string())?;
    let reply = crate::modstore::client()
        .post(format!("http://{host}/rtc"))
        .body(o.code(OFFER))
        .send()
        .await
        .map_err(|e| format!("Couldn't reach the host: {e}"))?;
    let ok = reply.status().is_success();
    let text = reply.text().await.map_err(|e| e.to_string())?;
    let a = Sig::parse(&text, ANSWER)
        .filter(|a| ok && a.to == o.ufrag)
        .ok_or(if ok {
            "The host's answer made no sense.".into()
        } else {
            text
        })?;
    // the answer code says active (`Sig::sdp`): the host is the DTLS client
    // and its media is ours (a code has no mid: `Sig::sdp` says 0, a browser's own; str0m picks another)
    let offer = offer.to_sdp_string();
    let mid = offer
        .lines()
        .find_map(|l| l.trim().strip_prefix("a=mid:"))
        .unwrap_or("0");
    let answer = remote_sdp(&a, false, "active")
        .replace("a=mid:0\r\n", &format!("a=mid:{mid}\r\n"))
        .replace("BUNDLE 0\r\n", &format!("BUNDLE {mid}\r\n"));
    let answer =
        SdpAnswer::from_sdp_string(&answer).map_err(|e| format!("the host's answer: {e}"))?;
    rtc.sdp_api()
        .accept_answer(pending, answer)
        .map_err(|e| e.to_string())?;
    let (conn, link, inbox, _) = drive(rtc, socks, packets, ids, a.ufrag).open().await?;
    Ok((conn, link, inbox, o.ufrag))
}

/// A connection on its way up.
pub(in crate::net) struct Opening {
    opened: oneshot::Receiver<Result<(), String>>,
    peer: Peer,
    inbox: Inbox,
    /// The remote's name (its ufrag).
    name: String,
}

impl Opening {
    /// Both channels open (30 s at most): the connection, its message link, arrivals and the remote's name.
    pub(in crate::net) async fn open(self) -> Result<(Conn, Link, Inbox, String), String> {
        let lost = "A player's connection timed out: are both on the same network?";
        match tokio::time::timeout(Duration::from_secs(30), self.opened).await {
            Ok(Ok(Ok(()))) => Ok((
                Conn::Rtc(self.peer.clone()),
                link(self.peer),
                self.inbox,
                self.name,
            )),
            Ok(Ok(Err(e))) => Err(e),
            _ => {
                self.peer.close();
                Err(lost.into())
            }
        }
    }
}

/// Messages out on the reliable channel, each after [`super::super::LAG_MS`], in order.
fn link(peer: Peer) -> Link {
    let (s, mut r) = amp::unbounded_channel::<(n0_future::time::Instant, Msg)>();
    n0_future::task::spawn(async move {
        while let Some((due, m)) = r.recv().await {
            n0_future::time::sleep_until(due).await;
            if peer
                .0
                .send(Out::Msg(serde_json::to_string(&m).unwrap()))
                .is_err()
            {
                break;
            }
        }
    });
    Link(s)
}

/// The connection's driver: packets in, the app's sends out, timers; until it closes.
fn drive(
    mut rtc: Rtc,
    socks: Socks,
    mut packets: amp::Receiver<Packet>,
    [msgs, dgrams]: [ChannelId; 2],
    name: String,
) -> Opening {
    let (out, mut outs) = amp::unbounded_channel::<Out>();
    let (mt, msgs_rx) = amp::unbounded_channel();
    let (dt, dgrams_rx) = amp::unbounded_channel();
    let (done, opened) = oneshot::channel();
    n0_future::task::spawn(async move {
        let socks = &socks;
        let mut done = Some(done);
        let mut open = [false; 2];
        // reliable messages the send buffer had no room for yet, oldest first
        let mut backlog: VecDeque<String> = VecDeque::new();
        let mut end: Option<String> = None;
        loop {
            let mut on = |e: Event| match e {
                Event::ChannelOpen(id, _) => {
                    open[0] |= id == msgs;
                    open[1] |= id == dgrams;
                    if open == [true; 2]
                        && let Some(d) = done.take()
                    {
                        let _ = d.send(Ok(()));
                    }
                }
                Event::ChannelData(d) if d.id == msgs => {
                    let m = String::from_utf8(d.data)
                        .map_err(|e| e.to_string())
                        .and_then(|s| serde_json::from_str::<Msg>(&s).map_err(|e| e.to_string()));
                    let _ = mt.send(m);
                }
                Event::ChannelData(d) if d.id == dgrams => drop(dt.send(d.data)),
                Event::ChannelClose(id) if id == msgs => end = Some("closed".into()),
                Event::IceConnectionStateChange(IceConnectionState::Disconnected) => {
                    end = Some("connection lost".into())
                }
                _ => {}
            };
            let next = drain(&mut rtc, socks, &mut on);
            if next.is_none() && end.is_none() {
                end = Some("connection lost".into());
            }
            if let Some(why) = end.take() {
                let _ = mt.send(Err(why.clone()));
                if let Some(d) = done.take() {
                    let _ = d.send(Err(why));
                }
                rtc.disconnect();
                return;
            }
            // what the buffer couldn't take before, now it may have drained
            while let Some(m) = backlog.front() {
                match rtc.channel(msgs).map(|mut c| c.write(false, m.as_bytes())) {
                    Some(Ok(true)) => drop(backlog.pop_front()),
                    _ => break,
                }
                let _ = drain(&mut rtc, socks, &mut |_| {});
            }
            let wake = tokio::time::Instant::from_std(
                next.unwrap_or_else(Instant::now).max(Instant::now()),
            );
            let wake = wake.min(
                tokio::time::Instant::now()
                    + Duration::from_millis(if backlog.is_empty() { 1000 } else { 5 }),
            );
            tokio::select! {
                p = packets.recv() => {
                    let Some((data, source, destination)) = p else { return };
                    if let Ok(contents) = data.as_slice().try_into() {
                        let _ = rtc.handle_input(Input::Receive(Instant::now(), Receive { proto: Protocol::Udp, source, destination, contents }));
                    }
                }
                o = outs.recv() => match o {
                    Some(Out::Msg(m)) => {
                        if !backlog.is_empty() || !matches!(rtc.channel(msgs).map(|mut c| c.write(false, m.as_bytes())), Some(Ok(true))) {
                            backlog.push_back(m);
                        }
                    }
                    Some(Out::Dgram(d)) => {
                        if let Some(mut c) = rtc.channel(dgrams) {
                            let _ = c.write(true, &d);
                        }
                    }
                    Some(Out::Close) | None => {
                        // the stream reset tells the other end at once (a bare disconnect: only its ICE timeout)
                        rtc.direct_api().close_data_channel(msgs);
                        let _ = drain(&mut rtc, socks, &mut |_| {});
                        rtc.disconnect();
                        let _ = drain(&mut rtc, socks, &mut |_| {});
                        return;
                    }
                },
                _ = tokio::time::sleep_until(wake) => {
                    let _ = rtc.handle_input(Input::Timeout(Instant::now()));
                }
            }
        }
    });
    Opening {
        opened,
        peer: Peer(out),
        inbox: Inbox {
            msgs: msgs_rx,
            dgrams: dgrams_rx,
        },
        name,
    }
}

/// Send what the connection has to send; its events to `on`; the next deadline (None: it failed).
fn drain(rtc: &mut Rtc, socks: &Socks, on: &mut dyn FnMut(Event)) -> Option<Instant> {
    loop {
        match rtc.poll_output() {
            Ok(Output::Timeout(t)) => return Some(t),
            Ok(Output::Transmit(t)) => {
                if let Some((m, pt)) = &socks.mux {
                    m.claim(None, Some(t.destination), pt);
                }
                if let Some(s) = socks
                    .socks
                    .iter()
                    .find(|s| s.local_addr().is_ok_and(|a| a == t.source))
                {
                    let _ = s.try_send_to(&t.contents, t.destination);
                }
            }
            Ok(Output::Event(e)) => on(e),
            Err(e) => {
                bevy::log::info!("LAN connection: {e}");
                return None;
            }
        }
    }
}

// ---------------------------------------------------------------- signalling

/// What `/rtc` answers: a guest's offer code in, (HTTP status, body) out; the body is the answer code, else why not.
/// The connection goes on in the background, to the lobby hosting now ([`super::hosting`]).
pub async fn signal(offer: &str) -> (u16, String) {
    if !super::is_hosting() {
        return (503, "No lobby is open on this machine.".into());
    }
    match accept(offer).await {
        Ok((answer, opening)) => {
            n0_future::task::spawn(async move {
                match opening.open().await {
                    Ok((conn, link, inbox, name)) => super::arrived(name, conn, link, inbox),
                    Err(e) => bevy::log::info!("LAN guest didn't connect: {e}"),
                }
            });
            (200, answer)
        }
        Err(e) => (400, e),
    }
}

/// The signalling listener on `port` (all addresses), for the rest of the run; a second call with the listener up
/// is a no-op. The bound address.
pub async fn listen(port: u16) -> std::io::Result<SocketAddr> {
    static UP: std::sync::Mutex<Option<SocketAddr>> = std::sync::Mutex::new(None);
    if let Some(a) = *UP.lock().unwrap() {
        return Ok(a);
    }
    let l = tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).await?;
    let at = l.local_addr()?;
    *UP.lock().unwrap() = Some(at);
    n0_future::task::spawn(async move {
        while let Ok((s, _)) = l.accept().await {
            n0_future::task::spawn(serve(s));
        }
    });
    Ok(at)
}

/// One HTTP request: `POST /rtc` with an offer code, or a CORS preflight (the guest page may come from elsewhere).
async fn serve(mut s: tokio::net::TcpStream) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut buf = Vec::new();
    let mut chunk = [0u8; 2048];
    // the head, then a body of Content-Length (an offer code is ~150 bytes; 8 KiB at most)
    let (head, body) = loop {
        let Ok(Ok(n)) = tokio::time::timeout(Duration::from_secs(10), s.read(&mut chunk)).await
        else {
            return;
        };
        if n == 0 || buf.len() > 8192 {
            return;
        }
        buf.extend_from_slice(&chunk[..n]);
        let Some(at) = buf.windows(4).position(|w| w == b"\r\n\r\n") else {
            continue;
        };
        let head = String::from_utf8_lossy(&buf[..at]).to_string();
        let len: usize = head
            .lines()
            .find_map(|l| {
                l.split_once(':')
                    .filter(|(k, _)| k.trim().eq_ignore_ascii_case("content-length"))
                    .and_then(|(_, v)| v.trim().parse().ok())
            })
            .unwrap_or(0);
        if buf.len() >= at + 4 + len {
            break (
                head,
                String::from_utf8_lossy(&buf[at + 4..at + 4 + len]).to_string(),
            );
        }
    };
    let mut first = head.lines().next().unwrap_or_default().split_whitespace();
    let (method, path) = (
        first.next().unwrap_or_default(),
        first.next().unwrap_or_default(),
    );
    let (status, text) = match (method, path.split('?').next()) {
        ("OPTIONS", _) => (204, String::new()),
        ("POST", Some("/rtc")) => signal(body.trim()).await,
        _ => (404, "Not found".into()),
    };
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Service Unavailable",
    };
    let reply = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: POST, OPTIONS\r\nAccess-Control-Allow-Headers: content-type\r\nAccess-Control-Allow-Private-Network: true\r\n\
         Connection: close\r\n\r\n{text}",
        text.len()
    );
    let _ = s.write_all(reply.as_bytes()).await;
    let _ = s.shutdown().await;
}

#[cfg(test)]
mod tests {
    use super::stun_ufrag;

    #[test]
    fn stun_user_names() {
        // a binding request: PRIORITY, then USERNAME "abcd:wxyz" (9 bytes, padded to 12)
        let mut p = vec![0, 1, 0, 24, 0x21, 0x12, 0xa4, 0x42];
        p.extend([7; 12]);
        p.extend([0x00, 0x24, 0, 4, 1, 2, 3, 4]);
        p.extend([0x00, 0x06, 0, 9]);
        p.extend(b"abcd:wxyz\0\0\0");
        assert_eq!(stun_ufrag(&p), Some("abcd"));
        // a response (no user name to go by), DTLS, a short packet
        p[1] = 0x01;
        p[0] = 0x01;
        assert_eq!(stun_ufrag(&p), None);
        assert_eq!(stun_ufrag(&[22, 254, 253, 0, 0]), None);
        assert_eq!(stun_ufrag(&p[..30]), None);
    }
}

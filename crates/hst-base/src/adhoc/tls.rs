//! M152f: the adhoc server over HTTPS, so a guest's page is a secure context (WebGPU, OPFS for the disc between visits,
//! a service worker for Add to Home Screen). The server's one port takes both: a connection that opens with a TLS
//! handshake gets TLS, anything else plain http as before (native guests, a typed `http://` address).
//!
//! The certificate is self-signed (ECDSA P-256, serverAuth, a year) for this machine's LAN addresses and localhost, so
//! each guest's browser warns once and the guest taps past it. It's kept in `<data dir>/adhoc-tls` and made again only
//! when it's within a month of its end or an address it doesn't name turns up (keeping the names it had), so the
//! exception a guest's browser keeps for it holds from one session to the next.

use std::io::{self, Read, Write};
use std::net::{IpAddr, TcpStream};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};

/// How long a new certificate lasts.
const LIFE: Duration = Duration::from_secs(365 * 24 * 3600);
/// A certificate this close to its end is made again.
const RENEW: Duration = Duration::from_secs(30 * 24 * 3600);
/// The most names one carries (the addresses wanted now come first; older ones fill the rest).
const MOST: usize = 32;

/// A TLS connection's first byte: a handshake record.
pub(super) const HANDSHAKE: u8 = 0x16;

/// The certificate kept in `dir`: its DER, its key's (PKCS #8) and what it says about itself.
#[derive(Debug, PartialEq)]
struct Kept {
    cert: Vec<u8>,
    key: Vec<u8>,
    /// Its end, in seconds since 1970.
    until: u64,
    names: Vec<String>,
}

/// The names a certificate for `ips` carries: each address, then the loopback ones and `localhost`.
fn wanted(ips: &[IpAddr]) -> Vec<String> {
    let mut v: Vec<String> = ips.iter().map(IpAddr::to_string).collect();
    for n in ["127.0.0.1", "::1", "localhost"] {
        if !v.iter().any(|w| w == n) {
            v.push(n.into());
        }
    }
    v
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn read(dir: &Path) -> Option<Kept> {
    let about = std::fs::read_to_string(dir.join("names.txt")).ok()?;
    let mut lines = about.lines();
    let until = lines.next()?.strip_prefix("until ")?.trim().parse().ok()?;
    let names = lines.map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect();
    Some(Kept { cert: std::fs::read(dir.join("cert.der")).ok()?, key: std::fs::read(dir.join("key.der")).ok()?, until, names })
}

/// Whether `k` still does for `want` at `now`: it names them all and has more than [`RENEW`] left.
fn still_good(k: &Kept, want: &[String], now: u64) -> bool {
    k.until > now + RENEW.as_secs() && want.iter().all(|w| k.names.contains(w))
}

/// A new certificate for `names` from `now`.
fn make(names: Vec<String>, now: u64) -> Result<Kept, String> {
    let e = |e: rcgen::Error| e.to_string();
    let mut p = rcgen::CertificateParams::new(names.clone()).map_err(e)?;
    p.distinguished_name.push(rcgen::DnType::CommonName, "OpenTennis adhoc host");
    p.distinguished_name.push(rcgen::DnType::OrganizationName, "OpenTennis");
    p.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];
    p.key_usages = vec![rcgen::KeyUsagePurpose::DigitalSignature];
    let at = |s: u64| time::OffsetDateTime::from_unix_timestamp(s as i64).map_err(|e| e.to_string());
    // a day back, for a guest whose clock is a little behind
    p.not_before = at(now.saturating_sub(24 * 3600))?;
    let until = now + LIFE.as_secs();
    p.not_after = at(until)?;
    // a serial of its own each time (Firefox refuses a second certificate with the same issuer and serial)
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
    p.serial_number = Some(rcgen::SerialNumber::from_slice(&[&[0x01u8][..], &nanos.to_be_bytes()].concat()));
    let key = rcgen::KeyPair::generate().map_err(e)?;
    let cert = p.self_signed(&key).map_err(e)?;
    Ok(Kept { cert: cert.der().to_vec(), key: key.serialize_der(), until, names })
}

fn write(dir: &Path, k: &Kept) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join("cert.der"), &k.cert)?;
    std::fs::write(dir.join("key.der"), &k.key)?;
    std::fs::write(dir.join("names.txt"), format!("until {}\n{}\n", k.until, k.names.join("\n")))
}

/// The certificate for `ips` from `dir`: the kept one while it does, else a new one (written back; the names the old
/// one had stay on it, so a laptop moving between networks keeps one certificate).
fn certificate(dir: &Path, ips: &[IpAddr], now: u64) -> Result<Kept, String> {
    let want = wanted(ips);
    let old = read(dir);
    if let Some(k) = old.as_ref().filter(|k| still_good(k, &want, now)) {
        return Ok(Kept { cert: k.cert.clone(), key: k.key.clone(), until: k.until, names: k.names.clone() });
    }
    let mut names = want;
    for n in old.map(|k| k.names).unwrap_or_default() {
        if names.len() < MOST && !names.contains(&n) {
            names.push(n);
        }
    }
    let k = make(names, now)?;
    if let Err(e) = write(dir, &k) {
        bevy::log::warn!("adhoc: couldn't keep the https certificate in {}: {e}", dir.display());
    }
    bevy::log::info!("adhoc: a new https certificate for {:?}", k.names);
    Ok(k)
}

/// The server's TLS setup for `ips`, its certificate kept in `dir`.
pub(crate) fn config(dir: &Path, ips: &[IpAddr]) -> Result<Arc<rustls::ServerConfig>, String> {
    let build = |k: &Kept| {
        rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .and_then(|b| b.with_no_client_auth().with_single_cert(vec![CertificateDer::from(k.cert.clone())], PrivatePkcs8KeyDer::from(k.key.clone()).into()))
            .map_err(|e| e.to_string())
    };
    let k = certificate(dir, ips, now())?;
    let mut c = match build(&k) {
        Ok(c) => c,
        // a kept one that doesn't load (a file cut short): a new one
        Err(e) => {
            bevy::log::warn!("adhoc: the kept https certificate didn't load ({e}): making a new one");
            let k = make(wanted(ips), now())?;
            let _ = write(dir, &k);
            build(&k)?
        }
    };
    c.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(Arc::new(c))
}

/// A TLS connection to a guest (blocking, on its own thread as the plain ones are).
pub(crate) type Tls = rustls::StreamOwned<rustls::ServerConnection, TcpStream>;

pub(crate) fn accept(c: TcpStream, cfg: Arc<rustls::ServerConfig>) -> io::Result<Tls> {
    let conn = rustls::ServerConnection::new(cfg).map_err(io::Error::other)?;
    Ok(rustls::StreamOwned::new(conn, c))
}

/// M156 over TLS: the WebSocket after an upgrade's head on `c` goes to the hosting lobby as `name`, the TLS records
/// pumped on the game's runtime between the socket and the lobby's end of a pipe.
pub(crate) fn websocket(mut c: Tls, up: (String, String)) -> io::Result<()> {
    let Some(name) = crate::net::ws::answer(&mut c, up)? else { return Ok(()) };
    c.flush()?;
    let rustls::StreamOwned { conn, sock } = c;
    sock.set_read_timeout(None)?;
    sock.set_nodelay(true)?;
    sock.set_nonblocking(true)?;
    crate::runtime().spawn(async move {
        match tokio::net::TcpStream::from_std(sock) {
            Ok(sock) => {
                let (ours, lobby) = tokio::io::duplex(256 << 10);
                crate::net::ws::arrive(lobby, name);
                if let Err(e) = pump(conn, sock, ours).await {
                    bevy::log::info!("adhoc: a guest's secure WebSocket ended: {e}");
                }
            }
            Err(e) => bevy::log::warn!("online: a guest's WebSocket: {e}"),
        }
    });
    Ok(())
}

/// Carries `conn`'s plaintext between `sock` (its TLS records) and `plain` until either end closes.
async fn pump(mut conn: rustls::ServerConnection, mut sock: tokio::net::TcpStream, mut plain: tokio::io::DuplexStream) -> io::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (mut net, mut text, mut up) = (vec![0u8; 16 << 10], vec![0u8; 16 << 10], vec![0u8; 16 << 10]);
    loop {
        // what's decrypted to the lobby, what's queued to the guest
        loop {
            match conn.reader().read(&mut text) {
                Ok(0) => return Ok(()),
                Ok(n) => plain.write_all(&text[..n]).await?,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        while conn.wants_write() {
            let mut out = Vec::new();
            conn.write_tls(&mut out)?;
            sock.write_all(&out).await?;
        }
        tokio::select! {
            r = sock.read(&mut net) => {
                let n = r?;
                if n == 0 {
                    return Ok(());
                }
                let mut rd = &net[..n];
                while !rd.is_empty() {
                    conn.read_tls(&mut rd)?;
                    conn.process_new_packets().map_err(io::Error::other)?;
                    // drained as it comes, so the plaintext buffer never fills
                    loop {
                        match conn.reader().read(&mut text) {
                            Ok(0) => return Ok(()),
                            Ok(k) => plain.write_all(&text[..k]).await?,
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                            Err(e) => return Err(e),
                        }
                    }
                }
            }
            r = plain.read(&mut up) => {
                let n = r?;
                if n == 0 {
                    conn.send_close_notify();
                    let mut out = Vec::new();
                    while conn.wants_write() {
                        conn.write_tls(&mut out)?;
                    }
                    return sock.write_all(&out).await;
                }
                conn.writer().write_all(&up[..n])?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn ip(a: [u8; 4]) -> IpAddr {
        IpAddr::V4(Ipv4Addr::from(a))
    }

    #[test]
    fn a_kept_certificate_holds_until_an_address_is_new_or_it_nears_its_end() {
        let dir = std::env::temp_dir().join(format!("hst-adhoc-tls-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let t = 1_800_000_000;
        let a = certificate(&dir, &[ip([192, 168, 1, 5])], t).unwrap();
        assert_eq!(a.names, ["192.168.1.5", "127.0.0.1", "::1", "localhost"]);
        assert_eq!(a.until, t + LIFE.as_secs());
        // the same addresses (or fewer) half a year on: the same one
        assert_eq!(certificate(&dir, &[ip([192, 168, 1, 5])], t + 180 * 86400).unwrap(), a);
        assert_eq!(certificate(&dir, &[], t + 1).unwrap(), a);
        // a new address: a new one that keeps the old names too
        let b = certificate(&dir, &[ip([10, 0, 0, 2])], t + 2).unwrap();
        assert_ne!(b.cert, a.cert);
        assert_eq!(b.names, ["10.0.0.2", "127.0.0.1", "::1", "localhost", "192.168.1.5"]);
        assert_eq!(certificate(&dir, &[ip([192, 168, 1, 5])], t + 3).unwrap(), b);
        // within a month of its end: a new one
        let c = certificate(&dir, &[ip([10, 0, 0, 2])], b.until - 20 * 86400).unwrap();
        assert_ne!(c.cert, b.cert);
        assert!(c.until > b.until);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_certificate_loads_into_rustls() {
        let dir = std::env::temp_dir().join(format!("hst-adhoc-tls-load-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(config(&dir, &[ip([192, 168, 7, 7])]).is_ok());
        // a cut-short key is replaced
        std::fs::write(dir.join("key.der"), b"x").unwrap();
        assert!(config(&dir, &[ip([192, 168, 7, 7])]).is_ok());
        assert_ne!(std::fs::read(dir.join("key.der")).unwrap(), b"x");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

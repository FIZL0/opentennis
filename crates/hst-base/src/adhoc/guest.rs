//! M152g: a native app with no disc of its own joins an adhoc host (M152a) on the LAN and plays from the host's disc,
//! as the browser guest page does (M152c).
//!
//! - Finding hosts: the guest broadcasts a question on UDP [`FIND_PORT`] and every host with the server on answers
//!   with its http port and name ([`answer_finds`]); a host the broadcast doesn't reach is typed in as its address.
//! - Getting the disc: `/adhoc/disc` (a pack, ≈ 170 MB) is streamed into `cache/adhoc-guest/` in the game's folder, and
//!   the host's address beside it once the host has a lobby open. The app then restarts (a window can be made only
//!   once per process): the new start reads the pack into memory and deletes the folder before anything else
//!   ([`resume`]), so the disc is never left on the device as one the game could run from again.
//! - The mods: the host's served mods go into the mod files held in memory (`modfs`): manifests, previews and
//!   texture packs at the start, the rest of a mod when the game first wants it ([`fetch_lazy`]), from the host.
//! - The lobby: the menu joins `rtch<host:port>` at once ([`code`]), as a lobby link does.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The UDP port hosts listen on for a guest's broadcast.
pub const FIND_PORT: u16 = 47152;
const ASK: &[u8] = b"opentennis-find 1";

/// A host that answered: its address as `ip:port` and the name it gave.
#[derive(Clone, Debug, PartialEq)]
pub struct Host {
    pub addr: String,
    pub name: String,
}

/// The host side: answers guests' broadcasts on [`FIND_PORT`] with the server's `port` until it stops.
pub(super) fn answer_finds(port: u16, sh: Arc<super::Shared>) {
    // a server switched off and on again: the last one's socket goes within half a second
    let mut bound = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, FIND_PORT));
    for _ in 0..4 {
        if bound.is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(250));
        bound = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, FIND_PORT));
    }
    match bound {
        Ok(sock) => answer_on(sock, port, &sh.stop),
        Err(e) => bevy::log::warn!("adhoc: guests can't find this host by broadcast (UDP {FIND_PORT}: {e}); they can type its address"),
    }
}

fn answer_on(sock: UdpSocket, port: u16, stop: &AtomicBool) {
    let _ = sock.set_read_timeout(Some(Duration::from_millis(500)));
    let reply = serde_json::json!({ "game": "opentennis", "port": port, "name": machine_name() }).to_string();
    let mut buf = [0u8; 64];
    while !stop.load(Relaxed) {
        if let Ok((n, from)) = sock.recv_from(&mut buf)
            && &buf[..n] == ASK
        {
            let _ = sock.send_to(reply.as_bytes(), from);
        }
    }
}

/// This machine's name as a guest's list shows it ("" if it can't be read).
fn machine_name() -> String {
    let env = |k| std::env::var(k).ok().filter(|v| !v.is_empty());
    env("COMPUTERNAME").or_else(|| env("HOSTNAME")).or_else(|| std::fs::read_to_string("/etc/hostname").ok()).map(|s| s.trim().to_string()).unwrap_or_default()
}

/// The guest side: hosts that answer, asked again every two seconds until dropped.
pub struct Finder {
    found: Arc<Mutex<Vec<(Host, Instant)>>>,
    stop: Arc<AtomicBool>,
}

impl Finder {
    pub fn start() -> Self {
        Self::asking(FIND_PORT)
    }

    /// Asking on UDP `port`.
    fn asking(port: u16) -> Self {
        let (found, stop) = (Arc::new(Mutex::new(Vec::new())), Arc::new(AtomicBool::new(false)));
        let (f, s) = (found.clone(), stop.clone());
        let _ = std::thread::Builder::new().name("adhoc-find".into()).spawn(move || find(port, &f, &s));
        Finder { found, stop }
    }

    /// The hosts heard from in the last few seconds, in the order they first answered.
    pub fn hosts(&self) -> Vec<Host> {
        let mut f = self.found.lock().unwrap();
        f.retain(|(_, t)| t.elapsed() < Duration::from_secs(7));
        f.iter().map(|(h, _)| h.clone()).collect()
    }
}

impl Drop for Finder {
    fn drop(&mut self) {
        self.stop.store(true, Relaxed);
    }
}

fn find(port: u16, found: &Mutex<Vec<(Host, Instant)>>, stop: &AtomicBool) {
    let Ok(sock) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)) else { return };
    let _ = sock.set_broadcast(true);
    let _ = sock.set_read_timeout(Some(Duration::from_millis(250)));
    let mut asked = Instant::now() - Duration::from_secs(10);
    let mut buf = [0u8; 1024];
    while !stop.load(Relaxed) {
        if asked.elapsed() > Duration::from_secs(2) {
            asked = Instant::now();
            for to in broadcasts() {
                let _ = sock.send_to(ASK, (to, port));
            }
        }
        let Ok((n, from)) = sock.recv_from(&mut buf) else { continue };
        let Some(host) = answer(&buf[..n], from) else { continue };
        let mut f = found.lock().unwrap();
        match f.iter_mut().find(|(h, _)| h.addr == host.addr) {
            Some(e) => *e = (host, Instant::now()),
            None => f.push((host, Instant::now())),
        }
    }
}

/// Where the question goes: everyone on this network, each interface's own broadcast address (a phone's hotspot) and
/// this machine (a host on it).
fn broadcasts() -> Vec<Ipv4Addr> {
    let mut to = vec![Ipv4Addr::BROADCAST, Ipv4Addr::LOCALHOST];
    for i in netdev::get_interfaces().into_iter().filter(|i| i.is_up() && !i.is_loopback()) {
        to.extend(i.ipv4.iter().filter(|n| n.prefix_len() < 31).map(|n| n.broadcast()));
    }
    to.dedup();
    to
}

/// A host's answer from `from`.
fn answer(b: &[u8], from: SocketAddr) -> Option<Host> {
    let j: serde_json::Value = serde_json::from_slice(b).ok()?;
    if j["game"] != "opentennis" {
        return None;
    }
    let port = u16::try_from(j["port"].as_u64()?).ok()?;
    let name = j["name"].as_str().unwrap_or("").chars().filter(|c| !c.is_control()).take(40).collect();
    Some(Host { addr: format!("{}:{port}", from.ip()), name })
}

/// A typed address (`192.168.1.5`, `192.168.1.5:8080`, `http://host/`, `https://host:8080`…) as `host:port`; None if it isn't one.
pub fn address(typed: &str) -> Option<String> {
    let t = typed.trim();
    // M152f: the card's https address too (the server takes plain http on the same port)
    let (t, port) = match t.strip_prefix("https://") {
        Some(t) => (t, 443),
        None => (t.strip_prefix("http://").unwrap_or(t), 80),
    };
    let t = t.trim_end_matches('/');
    let (host, port) = match t.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().ok().filter(|&p| p > 0)?),
        None => (t, port),
    };
    let ok = !host.is_empty() && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    ok.then(|| format!("{host}:{port}"))
}

// ---------------------------------------------------------------- http

/// A keep-alive connection to the host's server (ours: every answer has a `Content-Length`).
pub struct Http {
    addr: String,
    conn: Option<BufReader<TcpStream>>,
}

impl Http {
    pub fn new(addr: &str) -> Self {
        Http { addr: addr.into(), conn: None }
    }

    fn connect(&self) -> io::Result<BufReader<TcpStream>> {
        let to = self.addr.to_socket_addrs()?.next().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no such address"))?;
        let s = TcpStream::connect_timeout(&to, Duration::from_secs(4))?;
        s.set_read_timeout(Some(Duration::from_secs(20)))?;
        let _ = s.set_nodelay(true);
        Ok(BufReader::with_capacity(1 << 16, s))
    }

    /// GET `path`: the status, the body's length, and the body handed to `sink` a piece at a time.
    pub fn get(&mut self, path: &str, mut sink: impl FnMut(&[u8], u64) -> io::Result<()>) -> io::Result<u16> {
        // a kept connection the server has closed meanwhile fails at once: one fresh try
        for fresh in [self.conn.is_none(), true] {
            if fresh {
                self.conn = Some(self.connect()?);
            }
            match self.try_get(path, &mut sink) {
                Ok(s) => return Ok(s),
                Err((_, false)) if !fresh => self.conn = None,
                Err((e, _)) => {
                    self.conn = None;
                    return Err(e);
                }
            }
        }
        unreachable!()
    }

    /// The error, and whether any of the answer had come (then a retry would repeat what `sink` already took).
    fn try_get(&mut self, path: &str, sink: &mut impl FnMut(&[u8], u64) -> io::Result<()>) -> Result<u16, (io::Error, bool)> {
        let c = self.conn.as_mut().unwrap();
        let host = self.addr.clone();
        let req = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: OpenTennis\r\n\r\n");
        c.get_mut().write_all(req.as_bytes()).map_err(|e| (e, false))?;
        let mut line = String::new();
        if c.read_line(&mut line).map_err(|e| (e, false))? == 0 {
            return Err((io::ErrorKind::UnexpectedEof.into(), false));
        }
        let bad = || (io::Error::new(io::ErrorKind::InvalidData, "not an OpenTennis host"), true);
        let status: u16 = line.split(' ').nth(1).and_then(|s| s.parse().ok()).ok_or_else(bad)?;
        let (mut len, mut close) = (None, false);
        loop {
            line.clear();
            c.read_line(&mut line).map_err(|e| (e, true))?;
            let l = line.trim_end();
            if l.is_empty() {
                break;
            }
            let (k, v) = l.split_once(':').ok_or_else(bad)?;
            match k.trim().to_ascii_lowercase().as_str() {
                "content-length" => len = v.trim().parse::<u64>().ok(),
                "connection" => close = v.trim().eq_ignore_ascii_case("close"),
                _ => {}
            }
        }
        let len = len.ok_or_else(bad)?;
        let mut left = len;
        let mut buf = vec![0u8; 1 << 16];
        while left > 0 {
            let n = c.read(&mut buf[..left.min(1 << 16) as usize]).map_err(|e| (e, true))?;
            if n == 0 {
                return Err((io::ErrorKind::UnexpectedEof.into(), true));
            }
            sink(&buf[..n], len).map_err(|e| (e, true))?;
            left -= n as u64;
        }
        if close {
            self.conn = None;
        }
        Ok(status)
    }

    /// GET `path` whole; an answer other than 200 is an error that says the status.
    pub fn bytes(&mut self, path: &str) -> io::Result<Vec<u8>> {
        let mut body = Vec::new();
        match self.get(path, |b, _| {
            body.extend_from_slice(b);
            Ok(())
        })? {
            200 => Ok(body),
            s => Err(io::Error::other(format!("{path}: HTTP {s}"))),
        }
    }

    /// The host's `/adhoc/info.json`, if it is an OpenTennis host.
    pub fn info(&mut self) -> io::Result<serde_json::Value> {
        let j: serde_json::Value = serde_json::from_slice(&self.bytes("/adhoc/info.json")?).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "not an OpenTennis host"))?;
        if j["game"] != "opentennis" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "not an OpenTennis host"));
        }
        Ok(j)
    }
}

// ---------------------------------------------------------------- getting the disc

/// Where a guest's download waits for the restart, in the game's folder `dir`.
fn cache(dir: &Path) -> PathBuf {
    dir.join("cache").join("adhoc-guest")
}

/// How getting the host's disc goes (the page shows it).
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    Asking,
    /// The host is still packing its ISO.
    HostPacking,
    /// Bytes so far, of.
    Disc(u64, u64),
    /// Disc in; the host has no lobby open yet.
    WaitingForLobby,
    /// All set: restart to join.
    Ready,
    Failed(String),
}

/// Gets the disc from the host at `addr` (`host:port`) into `dir`'s cache, on a thread; `step` says how it goes, and
/// `cancel` stops it (what's half-fetched is dropped).
pub fn get(addr: String, dir: PathBuf, step: Arc<Mutex<Step>>, cancel: Arc<AtomicBool>) {
    let _ = std::thread::Builder::new().name("adhoc-guest".into()).spawn(move || {
        let out = fetch(&addr, &dir, &step, &cancel);
        if let Err(e) = out {
            let _ = std::fs::remove_dir_all(cache(&dir));
            if !cancel.load(Relaxed) {
                bevy::log::warn!("adhoc guest: {addr}: {e}");
                *step.lock().unwrap() = Step::Failed(e);
            }
        }
    });
}

fn fetch(addr: &str, dir: &Path, step: &Mutex<Step>, cancel: &AtomicBool) -> Result<(), String> {
    let set = |s| *step.lock().unwrap() = s;
    let stopped = || cancel.load(Relaxed).then(|| "stopped".to_string());
    let words = |e: io::Error| match e.kind() {
        io::ErrorKind::InvalidData => format!("{addr} isn't an OpenTennis host."),
        _ => format!("Couldn't reach {addr} ({e}). Is Adhoc on there, and this device on the same network?"),
    };
    set(Step::Asking);
    let mut http = Http::new(addr);
    let info = http.info().map_err(words)?;
    let mine = super::VERSION.unwrap_or("dev");
    if let Some(v) = info["version"].as_str().filter(|v| *v != mine && *v != "dev" && mine != "dev") {
        return Err(format!("The host runs OpenTennis {v} and this is {mine}: update both to the same version."));
    }
    let tmp = cache(dir);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| format!("Couldn't make {}: {e}", tmp.display()))?;
    let part = tmp.join("disc.part");
    loop {
        if let Some(s) = stopped() {
            return Err(s);
        }
        let mut f = io::BufWriter::new(std::fs::File::create(&part).map_err(|e| e.to_string())?);
        let mut got = 0;
        let status = http
            .get("/adhoc/disc", |b, of| {
                if cancel.load(Relaxed) {
                    return Err(io::Error::other("stopped"));
                }
                f.write_all(b)?;
                got += b.len() as u64;
                set(Step::Disc(got, of));
                Ok(())
            })
            .map_err(words)?;
        f.flush().map_err(|e| e.to_string())?;
        match status {
            200 => break,
            503 => {
                set(Step::HostPacking);
                std::thread::sleep(Duration::from_secs(2));
            }
            s => return Err(format!("The host wouldn't hand out its disc (HTTP {s}).")),
        }
    }
    if hst_data::iso::Iso::open(&part).ok().and_then(|i| i.pack_hash).is_none() {
        return Err("What the host sent isn't a disc pack.".into());
    }
    std::fs::rename(&part, tmp.join("pending.pack")).map_err(|e| e.to_string())?;
    loop {
        if let Some(s) = stopped() {
            return Err(s);
        }
        if http.info().map_err(words)?["lobby"] == true {
            break;
        }
        set(Step::WaitingForLobby);
        std::thread::sleep(Duration::from_secs(2));
    }
    std::fs::write(tmp.join("join.txt"), addr).map_err(|e| e.to_string())?;
    set(Step::Ready);
    Ok(())
}

// ---------------------------------------------------------------- after the restart

/// The host this run joins (after [`resume`]).
static HOST: Mutex<Option<String>> = Mutex::new(None);

/// At the start, in the game's folder `dir`: a disc a guest got from a host (see the module doc), read into memory and
/// mounted; the path to run from (no file is there). The download is deleted either way.
pub fn resume(dir: &Path) -> Option<PathBuf> {
    let tmp = cache(dir);
    if !tmp.exists() {
        return None;
    }
    let addr = std::fs::read_to_string(tmp.join("join.txt")).ok().and_then(|a| address(&a));
    let pack = addr.as_ref().and_then(|_| std::fs::read(tmp.join("pending.pack")).ok());
    let _ = std::fs::remove_dir_all(&tmp);
    let (addr, pack) = (addr?, pack?);
    let pack: Arc<[u8]> = pack.into();
    let hash = hst_data::iso::Iso::from_source(Box::new(io::Cursor::new(pack.clone()))).ok()?.pack_hash?;
    let at = dir.join("adhoc-host.pack");
    hst_data::iso::mount(&at, pack);
    bevy::log::info!("adhoc guest: playing from {addr}'s disc (pack {hash:016x}, in memory)");
    *HOST.lock().unwrap() = Some(addr);
    Some(at)
}

/// The host this run joins, as a join code (`net::rtc::HOST`): the menu joins it at once.
pub fn code() -> Option<String> {
    HOST.lock().unwrap().as_ref().map(|a| format!("{}{a}", crate::net::rtc::HOST))
}

/// One served mod: its folder under the mods folder, kind, and files (path in it, size).
struct ModFiles {
    folder: String,
    kind: String,
    files: Vec<(String, u64)>,
}

fn mod_list(info: &serde_json::Value) -> Vec<ModFiles> {
    let list = info["mods"].as_array().map(Vec::as_slice).unwrap_or_default();
    list.iter()
        .filter_map(|m| {
            let folder = m["folder"].as_str()?.to_string();
            let safe = |p: &str| !p.is_empty() && Path::new(p).components().all(|c| matches!(c, std::path::Component::Normal(_)));
            if !safe(&folder) {
                return None;
            }
            let files = m["files"].as_array()?.iter().filter_map(|f| Some((f["path"].as_str()?.to_string(), f["size"].as_u64()?))).filter(|(p, _)| safe(p) && crate::modfs::plain(p)).collect();
            Some(ModFiles { folder, kind: m["kind"].as_str().unwrap_or("").into(), files })
        })
        .collect()
}

/// The host's mods into the mod files in memory under `root` (the mods folder): what's listed at once, the rest when a
/// mod is first wanted ([`fetch_lazy`]). Nothing unless this run is a guest's.
pub fn mods(root: &Path) {
    let Some(addr) = HOST.lock().unwrap().clone() else { return };
    let mut http = Http::new(&addr);
    let info = match http.info() {
        Ok(i) => i,
        Err(e) => return bevy::log::warn!("adhoc guest: the host's mods: {e}"),
    };
    let list = mod_list(&info);
    let _ = ROOT.set(root.to_path_buf());
    let mut now = 0;
    for m in &list {
        for (p, size) in &m.files {
            let at = root.join(&m.folder).join(p);
            if m.kind == "textures" || crate::modfs::small(&format!("{}/{p}", m.folder)) {
                match http.bytes(&url(&m.folder, p)) {
                    Ok(b) => {
                        now += b.len();
                        crate::modfs::add(&at, b.into());
                    }
                    Err(e) => bevy::log::warn!("adhoc guest: {}/{p}: {e}", m.folder),
                }
            } else {
                crate::modfs::add_lazy(&at, *size);
            }
        }
    }
    bevy::log::info!("adhoc guest: {} of the host's mods ({} MB read now)", list.len(), now >> 20);
    crate::modfs::fetch_with(fetch_lazy);
}

/// The mods folder [`mods`] filled.
static ROOT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// `/adhoc/mods/<folder>/<file>`.
fn url(folder: &str, file: &str) -> String {
    format!("/adhoc/mods/{}/{}", super::encode(folder), super::encode(file))
}

/// `modfs`'s fetch for mod folder `dir` (a key under the mods folder): its files not in memory, from the host, on a
/// thread; then `modfs::loaded` (a file that won't come reads as missing, as in the browser).
fn fetch_lazy(dir: &str) {
    let (Some(addr), Some(root)) = (HOST.lock().unwrap().clone(), ROOT.get()) else { return crate::modfs::loaded(dir) };
    let (dir, root) = (dir.to_string(), crate::modfs::key(root));
    let _ = std::thread::Builder::new().name("adhoc-mod".into()).spawn(move || {
        let mut http = Http::new(&addr);
        for (key, _) in crate::modfs::lazy_under(Path::new(&dir)) {
            let Some(rel) = key.strip_prefix(&root).and_then(|r| r.strip_prefix('/')) else { continue };
            let Some((folder, file)) = split(rel, &dir[root.len().min(dir.len())..]) else { continue };
            match http.bytes(&url(&folder, &file)) {
                Ok(b) => crate::modfs::add(Path::new(&key), b.into()),
                Err(e) => bevy::log::warn!("adhoc guest: {rel}: {e}"),
            }
        }
        crate::modfs::loaded(&dir);
    });
}

/// `rel` (`<folder>/<file>` under the mods folder) split at the mod folder `dir_rel` (`/<folder>` as the wanted dir's
/// key has it past the root).
fn split(rel: &str, dir_rel: &str) -> Option<(String, String)> {
    let folder = dir_rel.trim_start_matches('/');
    let file = rel.strip_prefix(folder)?.strip_prefix('/')?;
    Some((folder.to_string(), file.to_string()))
}

// ---------------------------------------------------------------- restarting

/// Starts the game over (the desktop; the phones restart their own way), with `args` in front of this run's. Never
/// returns but with why it couldn't.
pub fn restart(args: &[&str]) -> io::Error {
    let Ok(exe) = std::env::current_exe() else { return io::ErrorKind::NotFound.into() };
    // an AppImage's own path, not its mount (which goes when this process does)
    let exe = std::env::var_os("APPIMAGE").filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or(exe);
    let mut cmd = std::process::Command::new(exe);
    cmd.args(args).args(std::env::args().skip(1));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.exec()
    }
    #[cfg(not(unix))]
    match cmd.spawn() {
        Ok(_) => std::process::exit(0),
        Err(e) => e,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_addresses() {
        assert_eq!(address("192.168.1.5").as_deref(), Some("192.168.1.5:80"));
        assert_eq!(address(" http://192.168.1.5:8080/ ").as_deref(), Some("192.168.1.5:8080"));
        assert_eq!(address("https://192.168.1.5:8080").as_deref(), Some("192.168.1.5:8080"));
        assert_eq!(address("https://192.168.1.5/").as_deref(), Some("192.168.1.5:443"));
        assert_eq!(address("host.local:8888").as_deref(), Some("host.local:8888"));
        assert_eq!(address("192.168.1.5:0"), None);
        assert_eq!(address("192.168.1.5/x"), None);
        assert_eq!(address(""), None);
    }

    #[test]
    fn answers_name_the_server() {
        let from: SocketAddr = "192.168.43.7:47152".parse().unwrap();
        let h = answer(br#"{"game":"opentennis","port":8080,"name":"den"}"#, from).unwrap();
        assert_eq!(h, Host { addr: "192.168.43.7:8080".into(), name: "den".into() });
        assert!(answer(br#"{"game":"other","port":8080}"#, from).is_none());
        assert!(answer(b"opentennis-find 1", from).is_none());
    }

    #[test]
    fn mod_files_split_at_their_folder() {
        assert_eq!(split("chars/ann/model.glb", "/chars/ann"), Some(("chars/ann".into(), "model.glb".into())));
        assert_eq!(split("chars/annie/model.glb", "/chars/ann"), None);
    }

    /// A host's server, found by broadcast and asked for its disc: the download lands in the cache, and `resume` takes
    /// it into memory and leaves nothing on disk.
    #[test]
    fn a_guest_gets_the_disc_and_keeps_it_in_memory() {
        use super::super::{Shared, Web, serve};
        let Ok(mut iso) = hst_data::fixture::iso() else { return };
        let t = std::env::temp_dir().join(format!("hst-guest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&t);
        std::fs::create_dir_all(&t).unwrap();
        // a pack to serve (the host packs an ISO the same way), on a server of its own (`set`'s is one per process)
        let pack = t.join("host.pack");
        iso.write_pack(&pack, |_, _| {}).unwrap();
        let sh = Arc::new(Shared { disc: pack.clone(), web: Mutex::new(Web::Looking), mods: Mutex::new(Vec::new()), stop: AtomicBool::new(false), reached: AtomicBool::new(false), tls: None });
        let l = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = l.local_addr().unwrap().port();
        let s2 = sh.clone();
        std::thread::spawn(move || {
            for c in l.incoming().flatten() {
                let s = s2.clone();
                std::thread::spawn(move || serve(c, &s));
            }
        });
        let udp = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).unwrap();
        let find_port = udp.local_addr().unwrap().port();
        let stop = Arc::new(AtomicBool::new(false));
        let s2 = stop.clone();
        std::thread::spawn(move || answer_on(udp, port, &s2));
        let finder = Finder::asking(find_port);
        let until = Instant::now() + Duration::from_secs(5);
        let here = |f: &Finder| f.hosts().iter().any(|h| h.addr == format!("127.0.0.1:{port}"));
        while !here(&finder) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(50));
        }
        let found = here(&finder);
        stop.store(true, Relaxed);
        // no lobby is open here: the download stops at waiting for one; its pack is what `resume` reads
        let step = Arc::new(Mutex::new(Step::Asking));
        let cancel = Arc::new(AtomicBool::new(false));
        get(format!("127.0.0.1:{port}"), t.clone(), step.clone(), cancel.clone());
        let until = Instant::now() + Duration::from_secs(60);
        let done = |s: &Step| matches!(s, Step::WaitingForLobby | Step::Failed(_));
        while !done(&step.lock().unwrap()) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(50));
        }
        let got = step.lock().unwrap().clone();
        assert!(found, "the host answered the broadcast");
        assert_eq!(got, Step::WaitingForLobby);
        // as `fetch` does once the lobby opens
        std::fs::write(cache(&t).join("join.txt"), format!("127.0.0.1:{port}")).unwrap();
        let at = resume(&t);
        let left = cache(&t).exists();
        cancel.store(true, Relaxed);
        sh.stop.store(true, Relaxed);
        let want = hst_data::iso::Iso::open(&pack).ok().and_then(|i| i.pack_hash);
        let _ = std::fs::remove_dir_all(&t);
        let at = at.expect("resumed");
        assert!(!left && !at.exists(), "nothing on disk to run from");
        assert!(want.is_some());
        assert_eq!(hst_data::iso::Iso::open(&at).unwrap().pack_hash, want);
        assert_eq!(code(), Some(format!("rtch127.0.0.1:{port}")));
    }
}

//! M152a: adhoc hosting. A native build serves the web build, its disc and its mods over plain http on every LAN
//! interface (a phone's hotspot too), so a guest opens `http://<this machine>` in any browser and plays with no install
//! (the guest page, M152c, and the native WebRTC host, M152b, build on it). Switched on from Settings → Online & Mods
//! for this run only (it hands out the disc, so it never starts by itself).
//!
//! What it serves:
//! - `/` and anything else: the web build, from the first of `HST_WEB`, a `web/` beside the game, the copy fetched
//!   before (`<data dir>/web`, a release's `web.tar.gz`, fetched once per version when there's none) or, in a cargo
//!   checkout, its `web/` (`tools/web.sh` builds it)
//! - `/adhoc/info.json`: this build's version, the web build's state, the disc (its URL, name, size) and the mods
//!   (each one's id, name, kind, folder URL and files with their sizes)
//! - `/adhoc/disc`: the disc the game runs from as a pack (M152c: an ISO is packed once into `cache/adhoc.pack`
//!   first, so a guest downloads ~170 MB, not the whole image; the guest page reads it into memory)
//! - `/adhoc/mods/<folder>/<file>`: a listed mod's files (characters, courts and umpires, and the texture packs that
//!   are switched on)
//! - `POST /rtc`: M152b's signalling (a guest's WebRTC offer in, the answer out) for the lobby hosting now, so the
//!   guest page (M152c) joins on the same address and port it was served from
//!
//! M152g: a native app with no disc finds the server by a UDP broadcast and plays from its disc too (`guest`, `page`).
//!
//! M152e: the host can't see a guest that never gets through, so the card says how to let them in: at once when this
//! PC's firewall (ufw) is on and has no rule letting the port in, and after a while with no guest otherwise; never
//! once a request has come from another machine.
//!
//! A plain thread per connection (a handful of guests), HTTP/1.1 keep-alive, GET and HEAD, byte ranges (a big disc can
//! be read in parts or resumed).

pub mod guest;
pub mod page;

use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The ports tried in turn: 80 gives the shortest address; a desktop that may not open it (below 1024) gets the next.
const PORTS: [u16; 4] = [80, 8080, 8000, 8888];
/// This build's release tag (`tools/release.sh` sets it), whose `web.tar.gz` matches it.
const VERSION: Option<&str> = option_env!("HST_VERSION");
const RELEASE: &str = "https://github.com/FIZL0/opentennis/releases/download";

/// Where the server is: its addresses as guests type them, and what's still missing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Status {
    /// `http://<ip>[:port]` on each LAN interface, the likeliest (Wi-Fi, Ethernet) first.
    pub urls: Vec<String>,
    /// The web build's state in words ("" once it's there).
    pub note: String,
    /// How to let guests through this PC's firewall, while none has got through and it may be in the way ("" else).
    pub hint: String,
}

/// Where the web build is.
#[derive(Clone, Debug)]
enum Web {
    Looking,
    /// Fetching this release's: bytes so far, of.
    Fetching(u64, u64),
    Ready(PathBuf),
    Missing(String),
}

/// One mod as served: its folder under the mods folder (`/`-separated), where it is, and its manifest's facts.
#[derive(Clone, Debug)]
struct Served {
    rel: String,
    dir: PathBuf,
    id: String,
    name: String,
    kind: &'static str,
}

struct Shared {
    disc: PathBuf,
    web: Mutex<Web>,
    mods: Mutex<Vec<Served>>,
    stop: AtomicBool,
    /// A request has come from another machine.
    reached: AtomicBool,
}

struct Server {
    port: u16,
    shared: Arc<Shared>,
    since: Instant,
    /// The firewall's verdict on the port and when it was read (it's read again now and then: the host may open it).
    firewall: Mutex<(Instant, Firewall)>,
}

/// How long the server runs with no guest before the card says the firewall may be in the way.
const NO_GUEST: Duration = Duration::from_secs(20);

static SERVER: Mutex<Option<Server>> = Mutex::new(None);

/// Whether the server is running.
pub fn on() -> bool {
    SERVER.lock().unwrap().is_some()
}

/// Starts (serving `disc`) or stops the server; an error says why it couldn't start.
pub fn set(on: bool, disc: &Path) -> Result<(), String> {
    let mut s = SERVER.lock().unwrap();
    if !on {
        if let Some(old) = s.take() {
            old.shared.stop.store(true, Relaxed);
            // wakes the accept so its thread sees `stop`
            let _ = TcpStream::connect_timeout(&SocketAddr::from((Ipv4Addr::LOCALHOST, old.port)), Duration::from_millis(200));
            crate::net::rtc::pin(None);
        }
        return Ok(());
    }
    if s.is_some() {
        return Ok(());
    }
    // M152c1: guests' WebRTC comes in on UDP at the same port number (`net::rtc::pin`), so a port that's free for both
    let listener = PORTS
        .iter()
        .find_map(|&p| {
            let _udp = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, p)).ok()?;
            TcpListener::bind((Ipv4Addr::UNSPECIFIED, p)).ok()
        }).map_or_else(|| TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)), Ok).map_err(|e| format!("Couldn't open a port: {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let shared = Arc::new(Shared { disc: served_disc(disc), web: Mutex::new(Web::Looking), mods: Mutex::new(mods()), stop: AtomicBool::new(false), reached: AtomicBool::new(false) });
    let own = own_addrs();
    let sh = shared.clone();
    std::thread::Builder::new().name("adhoc-web".into()).spawn(move || find_web(&sh)).map_err(|e| e.to_string())?;
    let sh = shared.clone();
    std::thread::Builder::new()
        .name("adhoc".into())
        .spawn(move || {
            for c in listener.incoming() {
                if sh.stop.load(Relaxed) {
                    break;
                }
                let Ok(c) = c else { continue };
                if c.peer_addr().is_ok_and(|a| !a.ip().is_loopback() && !own.contains(&a.ip())) {
                    sh.reached.store(true, Relaxed);
                }
                let sh = sh.clone();
                let _ = std::thread::Builder::new().name("adhoc-conn".into()).spawn(move || {
                    let _ = serve(c, &sh);
                });
            }
        })
        .map_err(|e| e.to_string())?;
    // M152g: native guests with no disc find this host by broadcast
    let sh = shared.clone();
    std::thread::Builder::new().name("adhoc-answer".into()).spawn(move || guest::answer_finds(port, sh)).map_err(|e| e.to_string())?;
    crate::net::rtc::pin(Some(port));
    let fw = firewall(port);
    bevy::log::info!("adhoc: serving on port {port}: {:?}, firewall: {fw:?}", urls(port));
    let now = Instant::now();
    *s = Some(Server { port, shared, since: now, firewall: Mutex::new((now, fw)) });
    Ok(())
}

/// M152c: the pack a guest downloads for `disc`: `disc` itself if it's a pack, else `cache/adhoc.pack` in the game's folder,
/// packed from the ISO in the background unless an earlier run left a good one (until then `/adhoc/disc` says so).
fn served_disc(disc: &Path) -> PathBuf {
    let pack = |p: &Path| hst_data::iso::Iso::open(p).ok().and_then(|i| i.pack_hash);
    if pack(disc).is_some() {
        return disc.to_path_buf();
    }
    let out = crate::store::path(&disc.to_string_lossy(), "cache").join("adhoc.pack");
    if pack(&out).is_some_and(|h| hst_data::iso::KNOWN_PACKS.iter().any(|k| k.0 == h)) {
        return out;
    }
    let (from, to) = (disc.to_path_buf(), out.clone());
    let _ = std::thread::Builder::new().name("adhoc-pack".into()).spawn(move || {
        let made = std::fs::create_dir_all(to.parent().unwrap()).and_then(|_| hst_data::iso::Iso::open(&from)?.write_pack(&to, |_, _| {}));
        match made {
            Ok(_) => bevy::log::info!("adhoc: packed {} for guests", from.display()),
            Err(e) => bevy::log::warn!("adhoc: couldn't pack {} for guests: {e}", from.display()),
        }
    });
    out
}

/// The running server's addresses and state.
pub fn status() -> Option<Status> {
    let s = SERVER.lock().unwrap();
    let s = s.as_ref()?;
    let note = match &*s.shared.web.lock().unwrap() {
        Web::Looking => "Looking for the web build...".into(),
        Web::Fetching(n, of) if *of > 0 => format!("Downloading the web build: {}%", n * 100 / of),
        Web::Fetching(..) => "Downloading the web build...".into(),
        Web::Ready(_) => String::new(),
        Web::Missing(why) => why.clone(),
    };
    let hint = if s.shared.reached.load(Relaxed) {
        String::new()
    } else {
        let mut fw = s.firewall.lock().unwrap();
        if fw.0.elapsed() > Duration::from_secs(2) {
            *fw = (Instant::now(), firewall(s.port));
        }
        fw.1.hint(s.port, s.since.elapsed() > NO_GUEST)
    };
    Some(Status { urls: urls(s.port), note, hint })
}

/// This machine's own addresses (a request from one of them is the host itself, not a guest).
fn own_addrs() -> Vec<std::net::IpAddr> {
    netdev::get_interfaces().into_iter().flat_map(|i| i.ipv4.into_iter().map(|n| n.addr().into()).chain(i.ipv6.into_iter().map(|n| n.addr().into()))).collect()
}

/// What this PC's firewall does with guests' requests to the server's port, as far as the game can read it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Firewall {
    /// ufw is on and lets nothing in on the port.
    UfwBlocks,
    /// ufw is on and lets the port in.
    UfwOpen,
    /// ufw is on but its rules can't be read.
    Ufw,
    /// firewalld is installed (its rules need root to read).
    Firewalld,
    /// Nothing the game can read.
    Unknown,
}

impl Firewall {
    /// The card's words on letting guests in: at once when the firewall blocks the port, after `late` (no guest for a
    /// while) when it might, never when it lets the port in.
    fn hint(self, port: u16, late: bool) -> String {
        match self {
            Firewall::UfwBlocks => format!("This PC's firewall blocks guests: allow them with sudo ufw allow {port}"),
            Firewall::UfwOpen => String::new(),
            _ if !late => String::new(),
            Firewall::Ufw => format!("No one yet? This PC's firewall may block them: sudo ufw allow {port}"),
            Firewall::Firewalld => format!("No one yet? This PC's firewall may block them: sudo firewall-cmd --add-port={port}/tcp --add-port={port}/udp"),
            Firewall::Unknown if cfg!(windows) => "No one yet? Let OpenTennis through Windows Firewall on private networks.".into(),
            Firewall::Unknown if cfg!(target_os = "macos") => "No one yet? Let OpenTennis through the firewall: System Settings > Network > Firewall.".into(),
            Firewall::Unknown => format!("No one yet? This PC's firewall may block them: allow TCP and UDP port {port}."),
        }
    }
}

/// Reads this PC's firewall setup for `port` (Linux: ufw's own files, which most installs let anyone read).
fn firewall(port: u16) -> Firewall {
    if !cfg!(target_os = "linux") {
        return Firewall::Unknown;
    }
    let read = |p: &str| std::fs::read_to_string(p).ok();
    let ufw_on = read("/etc/ufw/ufw.conf").is_some_and(|c| ufw_enabled(&c));
    if !ufw_on {
        return if Path::new("/etc/firewalld").is_dir() { Firewall::Firewalld } else { Firewall::Unknown };
    }
    let policy = read("/etc/default/ufw").and_then(|c| c.lines().find_map(|l| l.trim().strip_prefix("DEFAULT_INPUT_POLICY=").map(|v| v.trim_matches('"').to_string())));
    match read("/etc/ufw/user.rules") {
        // M152c1: the page comes in on TCP, guests' WebRTC on UDP at the same port
        Some(rules) => match ["tcp", "udp"].iter().all(|proto| ufw_lets_in(&rules, port, proto, policy.as_deref().unwrap_or("DROP"))) {
            true => Firewall::UfwOpen,
            false => Firewall::UfwBlocks,
        },
        None => Firewall::Ufw,
    }
}

/// Whether ufw's config switches it on (`ENABLED=yes`).
fn ufw_enabled(conf: &str) -> bool {
    conf.lines().filter_map(|l| l.trim().strip_prefix("ENABLED=")).next_back().is_some_and(|v| v.trim_matches(['"', '\'', ' ']).eq_ignore_ascii_case("yes"))
}

/// Whether ufw's rules (`user.rules`, whose `### tuple ###` lines list them in order) let `proto` (tcp, udp) from the
/// LAN in on `port`: the first incoming rule for it decides; none falls to the default `policy`. A rule for some
/// sources only (a LAN range) counts as letting guests in.
fn ufw_lets_in(rules: &str, port: u16, want: &str, policy: &str) -> bool {
    for l in rules.lines() {
        let Some(t) = l.strip_prefix("### tuple ###") else { continue };
        let f: Vec<&str> = t.split_whitespace().collect();
        // action proto dport dst sport src [dapp sapp] direction
        let [action, proto, dport, ..] = f[..] else { continue };
        let dir = f.iter().skip(6).rfind(|w| !w.starts_with("comment=")).copied().unwrap_or("in");
        if !dir.starts_with("in") || !(proto == want || proto == "any") {
            continue;
        }
        let has = dport == "any"
            || dport.split(',').any(|d| match d.split_once(':') {
                Some((a, b)) => a.parse().is_ok_and(|a: u16| a <= port) && b.parse().is_ok_and(|b: u16| port <= b),
                None => d.parse() == Ok(port),
            });
        if has {
            return matches!(action, "allow" | "limit");
        }
    }
    policy == "ACCEPT"
}

/// `http://<ip>[:port]` for each private IPv4 address of this machine's interfaces that are up (no loopback, no
/// container or VPN bridges), Wi-Fi and Ethernet first.
fn urls(port: u16) -> Vec<String> {
    let skip = ["docker", "br-", "veth", "virbr", "vmnet", "vboxnet", "tailscale", "zt", "tun", "utun", "wg", "lo"];
    let mut found: Vec<(u8, Ipv4Addr)> = netdev::get_interfaces()
        .into_iter()
        .filter(|i| i.is_up() && !i.is_loopback() && !skip.iter().any(|s| i.name.starts_with(s)))
        .flat_map(|i| {
            let rank = match i.if_type {
                netdev::prelude::InterfaceType::Wireless80211 => 0,
                netdev::prelude::InterfaceType::Ethernet => 1,
                _ => 2,
            };
            i.ipv4.into_iter().map(move |n| (rank, n.addr()))
        })
        .filter(|(_, a)| a.is_private() || a.is_link_local())
        .collect();
    // link-local last: a hotspot hands out private ones
    found.sort_by_key(|&(r, a)| (a.is_link_local(), r, a));
    found.dedup_by_key(|(_, a)| *a);
    found.into_iter().map(|(_, a)| url(a, port)).collect()
}

fn url(a: Ipv4Addr, port: u16) -> String {
    if port == 80 { format!("http://{a}") } else { format!("http://{a}:{port}") }
}

/// What the mods folder holds to serve: every mod (`mods::dirs`) but the texture packs switched off, the kinds this
/// build doesn't play and the characters the roster leaves out.
fn mods() -> Vec<Served> {
    let root = crate::modfs::root();
    let on: Vec<PathBuf> = crate::mods_textures::active(root).into_iter().map(|p| p.dir).collect();
    crate::mods::dirs(root)
        .into_iter()
        .filter_map(|dir| {
            use crate::mods::Kind;
            let kind = match Kind::of_dir(&dir) {
                Kind::Character => "character",
                Kind::Court => "court",
                Kind::Umpire => "umpire",
                Kind::Textures if on.contains(&dir) => "textures",
                _ => return None,
            };
            let m = crate::mods::read_any(&dir).ok()?;
            if kind == "character" && (crate::mods::dropped(&m.id) || crate::mods::retired(&m.id, &m.source)) {
                return None;
            }
            let rel = crate::modfs::relative(root, &dir)?.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
            Some(Served { rel, dir, id: m.id, name: m.name, kind })
        })
        .collect()
}

/// Every file under `dir` (dot files and folders left out), `/`-separated and relative to it, with its size.
fn files(dir: &Path) -> Vec<(String, u64)> {
    fn walk(base: &Path, d: &Path, out: &mut Vec<(String, u64)>) {
        let mut es: Vec<_> = std::fs::read_dir(d).into_iter().flatten().flatten().filter(|e| !e.file_name().to_string_lossy().starts_with('.')).collect();
        es.sort_by_key(|e| e.file_name());
        for e in es {
            let p = e.path();
            match e.metadata() {
                Ok(m) if m.is_dir() => walk(base, &p, out),
                Ok(m) => {
                    let rel = p.strip_prefix(base).unwrap().components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
                    out.push((rel, m.len()));
                }
                Err(_) => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out
}

/// `/adhoc/info.json` (the mods looked at again: one installed meanwhile shows up).
fn info(sh: &Shared) -> Vec<u8> {
    let mods = mods();
    let list: Vec<serde_json::Value> = mods
        .iter()
        .map(|m| {
            let files: Vec<_> = files(&m.dir).into_iter().map(|(p, n)| serde_json::json!({ "path": p, "size": n })).collect();
            serde_json::json!({ "id": m.id, "name": m.name, "kind": m.kind, "folder": m.rel, "url": format!("/adhoc/mods/{}/", encode(&m.rel)), "files": files })
        })
        .collect();
    *sh.mods.lock().unwrap() = mods;
    let web = match &*sh.web.lock().unwrap() {
        Web::Ready(_) => "ready",
        Web::Missing(_) => "missing",
        _ => "fetching",
    };
    let name = sh.disc.file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let size = std::fs::metadata(&sh.disc).map_or(0, |m| m.len());
    let j = serde_json::json!({
        "game": "opentennis",
        "version": VERSION.unwrap_or("dev"),
        "web": web,
        "disc": { "url": "/adhoc/disc", "name": name, "size": size, "ready": sh.disc.is_file() },
        "lobby": crate::net::rtc::lobby_open(),
        "mods": list,
    });
    serde_json::to_vec_pretty(&j).unwrap()
}

// ---------------------------------------------------------------- the web build

/// Finds the web build (see the module doc), fetching this release's when there's none.
fn find_web(sh: &Shared) {
    let ok = |d: &Path| d.join("index.html").is_file() && d.join("pkg/hst_bg.wasm").is_file();
    let env = std::env::var_os("HST_WEB").filter(|v| !v.is_empty()).map(PathBuf::from);
    let exe = std::env::current_exe().ok().and_then(|e| e.canonicalize().ok());
    let beside = exe.as_ref().and_then(|e| e.parent()).map(|d| d.join("web"));
    let fetched = crate::disc::data_dir().join("web");
    let checkout = exe.as_ref().and_then(|e| e.ancestors().find(|a| a.join("Cargo.lock").is_file()).map(|a| a.join("web")));
    let stamp = |d: &Path| std::fs::read_to_string(d.join(".version")).ok();
    let fetched_ok = ok(&fetched) && VERSION.is_none_or(|v| stamp(&fetched).as_deref() == Some(v));
    let found = env.into_iter().chain(beside).chain(fetched_ok.then(|| fetched.clone())).chain(checkout).find(|d| ok(d));
    let web = match (found, VERSION) {
        (Some(d), _) => Web::Ready(d),
        (None, Some(v)) => match fetch(sh, v, &fetched) {
            Ok(()) => Web::Ready(fetched),
            Err(e) => {
                bevy::log::warn!("adhoc: the web build: {e}");
                Web::Missing(format!("Couldn't download the web build ({e}). Connect to the internet once and switch this on again."))
            }
        },
        (None, None) => Web::Missing("No web build here: build it with tools/web.sh, or point HST_WEB at one.".into()),
    };
    *sh.web.lock().unwrap() = web;
}

/// Release `tag`'s `web.tar.gz`, unpacked into `into` (stamped with the tag once whole).
fn fetch(sh: &Shared, tag: &str, into: &Path) -> Result<(), String> {
    let part = into.with_extension("part");
    let _ = std::fs::remove_dir_all(&part);
    std::fs::create_dir_all(&part).map_err(|e| e.to_string())?;
    let body = crate::runtime().block_on(async {
        let r = crate::modstore::client().get(format!("{RELEASE}/{tag}/web.tar.gz")).header("User-Agent", "hot-shots-tennis").send().await.map_err(|e| e.to_string())?;
        if !r.status().is_success() {
            return Err(format!("{}", r.status()));
        }
        let of = r.content_length().unwrap_or(0);
        let mut r = r;
        let mut body = Vec::with_capacity(of as usize);
        while let Some(c) = r.chunk().await.map_err(|e| e.to_string())? {
            if sh.stop.load(Relaxed) {
                return Err("stopped".into());
            }
            body.extend_from_slice(&c);
            *sh.web.lock().unwrap() = Web::Fetching(body.len() as u64, of);
        }
        Ok(body)
    })?;
    untar(flate2::read::GzDecoder::new(&body[..]), &part).map_err(|e| format!("unpacking it: {e}"))?;
    std::fs::write(part.join(".version"), tag).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(into);
    std::fs::rename(&part, into).map_err(|e| e.to_string())
}

/// Unpacks tar stream `r` (ustar / GNU: files, folders, long names) into `dir`; anything with a `..` or an absolute
/// path is refused.
fn untar(mut r: impl Read, dir: &Path) -> io::Result<()> {
    let bad = || io::Error::new(io::ErrorKind::InvalidData, "not a tar");
    let mut long: Option<String> = None;
    loop {
        let mut h = [0u8; 512];
        if let Err(e) = r.read_exact(&mut h) {
            return if e.kind() == io::ErrorKind::UnexpectedEof { Ok(()) } else { Err(e) };
        }
        if h.iter().all(|&b| b == 0) {
            return Ok(());
        }
        let field = |a: usize, b: usize| String::from_utf8_lossy(&h[a..b]).trim_end_matches('\0').to_string();
        let size = u64::from_str_radix(field(124, 136).trim(), 8).map_err(|_| bad())?;
        let kind = h[156];
        let name = long.take().unwrap_or_else(|| {
            let (prefix, name) = (field(345, 500), field(0, 100));
            if &h[257..262] == b"ustar" && !prefix.is_empty() { format!("{prefix}/{name}") } else { name }
        });
        let mut data = (&mut r).take(size);
        let pad = (512 - size % 512) % 512;
        let rel = Path::new(&name);
        let safe = rel.components().all(|c| matches!(c, Component::Normal(_) | Component::CurDir));
        match kind {
            b'L' => {
                let mut s = String::new();
                data.read_to_string(&mut s)?;
                long = Some(s.trim_end_matches('\0').to_string());
            }
            b'0' | 0 if safe => {
                let p = dir.join(rel);
                std::fs::create_dir_all(p.parent().unwrap())?;
                io::copy(&mut data, &mut std::fs::File::create(p)?)?;
            }
            b'5' if safe => std::fs::create_dir_all(dir.join(rel))?,
            _ => {}
        }
        io::copy(&mut data, &mut io::sink())?;
        io::copy(&mut (&mut r).take(pad), &mut io::sink())?;
    }
}

// ---------------------------------------------------------------- http

/// One request's line and the headers it needs.
#[derive(Debug, PartialEq)]
struct Request {
    head: bool,
    /// A POST (`/rtc`), whose body is `len` bytes.
    post: bool,
    len: usize,
    path: String,
    range: Option<String>,
    close: bool,
}

/// Answers `c`'s requests until it closes, idles 30 s or the server stops.
fn serve(mut c: TcpStream, sh: &Shared) -> io::Result<()> {
    c.set_read_timeout(Some(Duration::from_secs(30)))?;
    let mut buf = Vec::new();
    while !sh.stop.load(Relaxed) {
        let Some(end) = read_head(&mut c, &mut buf)? else { return Ok(()) };
        let text = String::from_utf8_lossy(&buf[..end]).into_owned();
        buf.drain(..end + 4);
        let Some(req) = parse(&text) else {
            reply(&mut c, "405 Method Not Allowed", "text/plain", b"GET only\n", true, false)?;
            return Ok(());
        };
        if req.post {
            // an offer code is ~150 bytes
            if req.len > 8 << 10 {
                return reply(&mut c, "413 Payload Too Large", "text/plain", b"too big\n", true, false);
            }
            while buf.len() < req.len {
                let mut chunk = [0u8; 4096];
                match c.read(&mut chunk)? {
                    0 => return Ok(()),
                    n => buf.extend_from_slice(&chunk[..n]),
                }
            }
            let body: Vec<u8> = buf.drain(..req.len).collect();
            post(&mut c, &req, &String::from_utf8_lossy(&body))?;
        } else {
            route(&mut c, sh, &req)?;
        }
        if req.close {
            return Ok(());
        }
    }
    Ok(())
}

/// Reads until `buf` holds a whole request head; its length (up to the blank line), or None at the end.
fn read_head(c: &mut TcpStream, buf: &mut Vec<u8>) -> io::Result<Option<usize>> {
    loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            return Ok(Some(i));
        }
        if buf.len() > 16 << 10 {
            return Ok(None);
        }
        let mut chunk = [0u8; 4096];
        let n = match c.read(&mut chunk) {
            Ok(0) => return Ok(None),
            Ok(n) => n,
            Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => return Ok(None),
            Err(e) => return Err(e),
        };
        buf.extend_from_slice(&chunk[..n]);
    }
}

/// A GET or HEAD request's head; None for any other method.
fn parse(head: &str) -> Option<Request> {
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let (method, target, version) = (first.next()?, first.next()?, first.next().unwrap_or("HTTP/1.0"));
    if method != "GET" && method != "HEAD" && method != "POST" {
        return None;
    }
    let mut req = Request { head: method == "HEAD", post: method == "POST", len: 0, path: decode(target.split(['?', '#']).next()?), range: None, close: version == "HTTP/1.0" };
    for l in lines {
        let Some((k, v)) = l.split_once(':') else { continue };
        let v = v.trim();
        if k.eq_ignore_ascii_case("range") {
            req.range = Some(v.to_string());
        } else if k.eq_ignore_ascii_case("content-length") {
            req.len = v.parse().ok()?;
        } else if k.eq_ignore_ascii_case("connection") {
            req.close = v.eq_ignore_ascii_case("close") || req.close && !v.eq_ignore_ascii_case("keep-alive");
        }
    }
    Some(req)
}

/// `%xx` escapes decoded (a bad one stays as it is).
fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2]))
        {
            out.push((h * 16 + l) as u8);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `s` with what a URL path can't hold as it is `%xx`-escaped (`/` kept).
fn encode(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"/-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// `rel` (`/`-separated, from a URL) under `dir`, if it stays there and names a file.
fn under(dir: &Path, rel: &str) -> Option<PathBuf> {
    let rel = Path::new(rel.trim_start_matches('/'));
    if !rel.components().all(|c| matches!(c, Component::Normal(n) if !n.to_string_lossy().starts_with('.'))) {
        return None;
    }
    Some(dir.join(rel)).filter(|p| p.is_file())
}

/// Answers a POST: `/rtc` takes a guest's offer code for the hosting lobby (M152b's `signal`, on the game's runtime: the
/// connection it opens carries on there) and answers with the answer code, else why not.
fn post(c: &mut TcpStream, req: &Request, body: &str) -> io::Result<()> {
    if req.path != "/rtc" {
        return reply(c, "404 Not Found", "text/plain", b"not found\n", req.close, false);
    }
    let (code, text) = crate::runtime().block_on(crate::net::rtc::signal(body.trim()));
    let status = match code {
        200 => "200 OK",
        400 => "400 Bad Request",
        _ => "503 Service Unavailable",
    };
    reply(c, status, "text/plain; charset=utf-8", text.as_bytes(), req.close, false)
}

/// Answers `req`.
fn route(c: &mut TcpStream, sh: &Shared, req: &Request) -> io::Result<()> {
    let p = req.path.as_str();
    if p == "/adhoc/info.json" {
        let body = info(sh);
        return reply(c, "200 OK", "application/json", &body, req.close, req.head);
    }
    if p == "/adhoc/disc" {
        if !sh.disc.is_file() {
            return reply(c, "503 Service Unavailable", "text/plain", b"The host is still getting the disc ready.\n", req.close, req.head);
        }
        return file(c, &sh.disc, "application/octet-stream", req, sh);
    }
    if let Some(rest) = p.strip_prefix("/adhoc/mods/") {
        let found = sh.mods.lock().unwrap().iter().find_map(|m| rest.strip_prefix(&m.rel).and_then(|f| f.strip_prefix('/')).and_then(|f| under(&m.dir, f)));
        return match found {
            Some(f) => file(c, &f, mime(&f), req, sh),
            None => reply(c, "404 Not Found", "text/plain", b"no such mod file\n", req.close, req.head),
        };
    }
    let web = sh.web.lock().unwrap().clone();
    match web {
        Web::Ready(dir) => {
            let rel = if p.ends_with('/') { format!("{p}index.html") } else { p.to_string() };
            match under(&dir, &rel) {
                Some(f) => file(c, &f, mime(&f), req, sh),
                None => reply(c, "404 Not Found", "text/plain", b"not found\n", req.close, req.head),
            }
        }
        other => {
            let words = match other {
                Web::Missing(why) => why,
                _ => "The host is getting the game ready: this page reloads by itself.".into(),
            };
            let page = format!("<!doctype html><meta charset=utf-8><meta name=viewport content='width=device-width'><meta http-equiv=refresh content=3><title>OpenTennis</title><p style='font:18px sans-serif;margin:2em'>{words}</p>");
            reply(c, "503 Service Unavailable", "text/html; charset=utf-8", page.as_bytes(), req.close, req.head)
        }
    }
}

/// A file's type by its extension (the browser compiles wasm as it streams in only as `application/wasm`).
fn mime(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript",
        Some("wasm") => "application/wasm",
        Some("css") => "text/css",
        Some("json") => "application/json",
        Some("webmanifest") => "application/manifest+json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("txt") => "text/plain; charset=utf-8",
        Some("xml") => "application/xml",
        _ => "application/octet-stream",
    }
}

/// A byte range header's span of a `len`-byte file (first, last; both in it), or None for the whole file or one that
/// can't be met (several ranges at once get the whole file).
fn span(range: &str, len: u64) -> Option<Result<(u64, u64), ()>> {
    let r = range.strip_prefix("bytes=")?;
    if r.contains(',') {
        return None;
    }
    let (a, b) = r.split_once('-')?;
    let (a, b) = (a.trim(), b.trim());
    let (first, last) = match (a.parse::<u64>().ok(), b.parse::<u64>().ok()) {
        (Some(a), Some(b)) => (a, b.min(len.saturating_sub(1))),
        (Some(a), None) if b.is_empty() => (a, len.saturating_sub(1)),
        (None, Some(n)) if a.is_empty() && n > 0 => (len.saturating_sub(n), len.saturating_sub(1)),
        _ => return None,
    };
    Some(if first > last || first >= len { Err(()) } else { Ok((first, last)) })
}

/// Sends file `f` (the part `req` asks for), stopping if the server does.
fn file(c: &mut TcpStream, f: &Path, mime: &str, req: &Request, sh: &Shared) -> io::Result<()> {
    let Ok(mut fh) = std::fs::File::open(f) else {
        return reply(c, "404 Not Found", "text/plain", b"not found\n", req.close, req.head);
    };
    let len = fh.metadata()?.len();
    let (status, first, n, extra) = match req.range.as_deref().and_then(|r| span(r, len)) {
        None => ("200 OK", 0, len, String::new()),
        Some(Ok((a, b))) => ("206 Partial Content", a, b - a + 1, format!("Content-Range: bytes {a}-{b}/{len}\r\n")),
        Some(Err(())) => {
            let head = format!("HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{len}\r\nContent-Length: 0\r\n{}\r\n", connection(req.close));
            return c.write_all(head.as_bytes());
        }
    };
    let head = format!("HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {n}\r\nAccept-Ranges: bytes\r\nCache-Control: no-cache\r\n{extra}{}\r\n", connection(req.close));
    c.write_all(head.as_bytes())?;
    if req.head {
        return Ok(());
    }
    io::Seek::seek(&mut fh, io::SeekFrom::Start(first))?;
    let mut left = n;
    let mut chunk = vec![0u8; 256 << 10];
    while left > 0 {
        if sh.stop.load(Relaxed) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "stopped"));
        }
        let want = left.min(chunk.len() as u64) as usize;
        let k = fh.read(&mut chunk[..want])?;
        if k == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        c.write_all(&chunk[..k])?;
        left -= k as u64;
    }
    Ok(())
}

fn connection(close: bool) -> &'static str {
    if close { "Connection: close\r\n" } else { "Connection: keep-alive\r\n" }
}

fn reply(c: &mut TcpStream, status: &str, mime: &str, body: &[u8], close: bool, head: bool) -> io::Result<()> {
    let h = format!("HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-store\r\n{}\r\n", body.len(), connection(close));
    c.write_all(h.as_bytes())?;
    if !head {
        c.write_all(body)?;
    }
    Ok(())
}

// ---------------------------------------------------------------- QR

/// `text` as a QR code (medium error correction): its side in modules and the dark ones, row by row; None if it's too
/// long for one.
pub fn qr(text: &str) -> Option<(usize, Vec<bool>)> {
    let code = qrcode::QrCode::with_error_correction_level(text, qrcode::EcLevel::M).ok()?;
    Some((code.width(), code.to_colors().into_iter().map(|c| c == qrcode::Color::Dark).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_parse() {
        let r = parse("GET /mods/a%20b/x.png?v=1 HTTP/1.1\r\nHost: x\r\nRange: bytes=5-9").unwrap();
        assert_eq!(r, Request { head: false, post: false, len: 0, path: "/mods/a b/x.png".into(), range: Some("bytes=5-9".into()), close: false });
        assert!(parse("HEAD / HTTP/1.0\r\n").unwrap().close);
        assert!(!parse("GET / HTTP/1.0\r\nConnection: keep-alive").unwrap().close);
        assert!(parse("PUT / HTTP/1.1\r\n").is_none());
        let p = parse("POST /rtc HTTP/1.1\r\nContent-Length: 12").unwrap();
        assert!(p.post && p.len == 12);
        assert_eq!(decode("%2"), "%2");
        assert_eq!(encode("hst-mods/Ann é"), "hst-mods/Ann%20%C3%A9");
    }

    #[test]
    fn ranges() {
        assert_eq!(span("bytes=0-99", 50), Some(Ok((0, 49))));
        assert_eq!(span("bytes=10-", 50), Some(Ok((10, 49))));
        assert_eq!(span("bytes=-5", 50), Some(Ok((45, 49))));
        assert_eq!(span("bytes=60-", 50), Some(Err(())));
        assert_eq!(span("bytes=0-1,4-5", 50), None);
    }

    #[test]
    fn paths_stay_inside() {
        let d = std::env::temp_dir().join(format!("hst-adhoc-{}", std::process::id()));
        std::fs::create_dir_all(d.join("pkg")).unwrap();
        std::fs::write(d.join("pkg/a.js"), "x").unwrap();
        assert!(under(&d, "/pkg/a.js").is_some());
        assert!(under(&d, "/pkg/../pkg/a.js").is_none());
        assert!(under(&d, "/../etc/passwd").is_none());
        assert!(under(&d, "/pkg").is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn tar_unpacks() {
        // a ustar file, a GNU long name, a `..` refused
        fn entry(name: &[u8], kind: u8, body: &[u8]) -> Vec<u8> {
            let mut h = [0u8; 512];
            h[..name.len().min(100)].copy_from_slice(&name[..name.len().min(100)]);
            h[124..135].copy_from_slice(format!("{:011o}", body.len()).as_bytes());
            h[156] = kind;
            h[257..262].copy_from_slice(b"ustar");
            let mut v = h.to_vec();
            v.extend_from_slice(body);
            v.resize(v.len().div_ceil(512) * 512, 0);
            v
        }
        let long = "pkg/".to_string() + &"x".repeat(120);
        let mut t = [entry(b"./index.html", b'0', b"hi"), entry(b"././@LongLink", b'L', long.as_bytes()), entry(b"short", b'0', b"long one"), entry(b"../out", b'0', b"no")].concat();
        t.extend_from_slice(&[0; 1024]);
        let d = std::env::temp_dir().join(format!("hst-untar-{}", std::process::id()));
        untar(&t[..], &d.join("w")).unwrap();
        assert_eq!(std::fs::read(d.join("w/index.html")).unwrap(), b"hi");
        assert_eq!(std::fs::read(d.join("w").join(&long)).unwrap(), b"long one");
        assert!(!d.join("out").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn serves_over_http() {
        let d = std::env::temp_dir().join(format!("hst-adhoc-http-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("index.html"), "<p>hi").unwrap();
        std::fs::write(d.join("disc.pack"), (0..=255u8).collect::<Vec<_>>()).unwrap();
        let sh = Arc::new(Shared { disc: d.join("disc.pack"), web: Mutex::new(Web::Ready(d.clone())), mods: Mutex::new(Vec::new()), stop: AtomicBool::new(false), reached: AtomicBool::new(false) });
        let l = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = l.local_addr().unwrap().port();
        let s2 = sh.clone();
        std::thread::spawn(move || {
            for c in l.incoming().flatten() {
                let s = s2.clone();
                std::thread::spawn(move || serve(c, &s));
            }
        });
        let mut c = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        // two requests on one connection: the page, then part of the disc
        c.write_all(b"GET / HTTP/1.1\r\nHost: x\r\n\r\nGET /adhoc/disc HTTP/1.1\r\nRange: bytes=16-19\r\nConnection: close\r\n\r\n").unwrap();
        let mut out = Vec::new();
        c.read_to_end(&mut out).unwrap();
        let text = String::from_utf8_lossy(&out);
        assert!(text.starts_with("HTTP/1.1 200 OK\r\nContent-Type: text/html"), "{text}");
        assert!(text.contains("<p>hi") && text.contains("206 Partial Content") && text.contains("Content-Range: bytes 16-19/256"));
        assert!(out.ends_with(&[16, 17, 18, 19]));
        // M152c: signalling on the same port; no lobby is hosting in this test
        let mut c = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        c.write_all(b"POST /rtc HTTP/1.1\r\nContent-Length: 5\r\nConnection: close\r\n\r\nrtco1").unwrap();
        let mut out = String::new();
        c.read_to_string(&mut out).unwrap();
        assert!(out.starts_with("HTTP/1.1 503") && out.ends_with("No lobby is open on this machine."), "{out}");
        sh.stop.store(true, Relaxed);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn ufw_rules() {
        let rules = "### tuple ### allow udp 8080 0.0.0.0/0 any 0.0.0.0/0 in\n\
                     ### tuple ### allow tcp 9943:9944 0.0.0.0/0 any 0.0.0.0/0 alvr - in\n\
                     ### tuple ### allow tcp 8000 0.0.0.0/0 any 192.168.0.0/16 in comment=6f\n\
                     ### tuple ### deny tcp 8888 0.0.0.0/0 any 0.0.0.0/0 in\n\
                     ### tuple ### allow tcp 8888 0.0.0.0/0 any 0.0.0.0/0 in\n\
                     ### tuple ### allow tcp 80 0.0.0.0/0 any 0.0.0.0/0 out\n\
                     ### tuple ### allow any 81,90:99 0.0.0.0/0 any 0.0.0.0/0 in_wlan0\n";
        // udp only, a range, a LAN-only rule, a deny first, outgoing only, a list
        assert!(!ufw_lets_in(rules, 8080, "tcp", "DROP"));
        assert!(ufw_lets_in(rules, 9944, "tcp", "DROP"));
        assert!(ufw_lets_in(rules, 8000, "tcp", "DROP"));
        assert!(!ufw_lets_in(rules, 8888, "tcp", "ACCEPT"));
        assert!(!ufw_lets_in(rules, 80, "tcp", "DROP"));
        assert!(ufw_lets_in(rules, 80, "tcp", "ACCEPT"));
        assert!(ufw_lets_in(rules, 95, "tcp", "REJECT"));
        assert_eq!(Firewall::UfwOpen.hint(8080, true), "");
        assert!(ufw_lets_in(rules, 8080, "udp", "DROP"));
        assert!(!ufw_lets_in(rules, 9944, "udp", "DROP"));
        assert!(Firewall::UfwBlocks.hint(8080, false).ends_with("sudo ufw allow 8080"));
        assert!(ufw_enabled("# comment\nENABLED=yes\nLOGLEVEL=low"));
        assert!(!ufw_enabled("ENABLED=no\n"));
        assert!(!ufw_enabled("#ENABLED=yes\n"));
        assert_eq!(Firewall::Unknown.hint(8080, false), "");
        assert!(!Firewall::Unknown.hint(8080, true).is_empty());
    }

    #[test]
    fn qr_codes() {
        let (n, dark) = qr("http://192.168.43.1:8080").unwrap();
        assert_eq!(dark.len(), n * n);
        // the finder pattern's corner
        assert!(dark[0] && dark[6] && !dark[7]);
    }
}

//! Lockstep netplay (M2g): a tick runs only once every seat's pads for it are in. Each machine sends its own seats'
//! pads `delay` ticks ahead (the host picks the delay from the slowest guest's round trip, `net::delay_for`), so at
//! steady speed nobody waits; a peer that falls behind stalls everyone, and after [`STALL`] the screen says whose pads
//! it waits for. The first `delay` ticks are idle for every seat.
//!
//! The gate (`mode::Gate`) runs before each of the match's ticks: it hands the tick's `[Pad; 4]` to the match as a
//! one-tick `Replay` (`tick_input`), so the match never reads local devices itself (while an instant replay plays its
//! own queue, only the tick's presses go in, for its ✕/○ skip: `run_on`). Before every [`HASH_EVERY`]th tick
//! each machine sends its `snapshot::state_hash`; on a mismatch each machine logs it and saves the inputs (every
//! seat's, every tick, the same on every machine) with the line-up as `desync-*.json` (beside the disc; in the browser a
//! download and the console; the ignored test `replays_a_desync` runs one again), says so on screen and plays on
//! (B154), and the host sends the peers its state to take on (B154b, `resync.rs`). The pause screen opens and closes
//! on every machine; the post-point press is a pad press, so it travels with the pads. A peer going silent pauses
//! everyone until it rejoins, a bot takes its seat or the match ends (B196, `recover.rs`). The court intro before the
//! match runs its ticks on a lockstep of its own (`Msg::Intro`, `intro_tick`), so a ✕ on any machine skips it on the
//! same tick everywhere.

use super::tick_input::{self, IDLE, Replay};
use super::{Game, Pads, front_end, snapshot};
use crate::net::{Cmd, Event, Msg, Net, Room};
use bevy::prelude::*;
use hst_sim::replay::Pad;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// Hashes are compared before every this many ticks.
pub const HASH_EVERY: u32 = 30;
/// A stall this long (real seconds) mid-match puts up "Waiting for ..." (B642: at 0.25 s every short hitch flashed it
/// up mid-rally; shorter stalls just hold the match silently).
const STALL: f32 = 2.0;
/// A stall this long before every peer has loaded the match puts up "Waiting for ... to load...".
const LOAD_STALL: f32 = 0.25;
/// How long the end message stays up before the match closes.
const END: f32 = 5.0;
/// The desync message's start (a peer that hears it saves its inputs too).
const DESYNC: &str = "Out of sync";
/// How long a desync's note stays up (real seconds).
const NOTE: f32 = 3.0;

/// `--pads`' gamepad for a peer's seat.
const REMOTE: usize = 99;

/// The online match's seed for the match setup (`main.rs`, the weather generator's), `u64::MAX` offline.
pub static SEED: AtomicU64 = AtomicU64::new(u64::MAX);

/// The online match's seed, once (the next match is offline unless it sets one again).
pub fn seed() -> Option<u32> {
    u32::try_from(SEED.swap(u64::MAX, Ordering::Relaxed)).ok()
}

pub fn wire(p: Pad) -> [u8; 6] {
    let b = p.buttons.to_le_bytes();
    [b[0], b[1], p.rx, p.ry, p.lx, p.ly]
}

pub fn unwire(w: [u8; 6]) -> Pad {
    Pad { buttons: u16::from_le_bytes([w[0], w[1]]), rx: w[2], ry: w[3], lx: w[4], ly: w[5] }
}

/// The lockstep bookkeeping: every seat's pads by tick as they come in, this machine's hashes and the peers'.
pub struct Lockstep {
    /// The next tick to run.
    pub tick: u32,
    pub delay: u32,
    /// Seats 1P..4P taken (an empty seat is always idle).
    seats: [bool; 4],
    /// This machine's seats.
    pub mine: Vec<usize>,
    pending: BTreeMap<u32, [Option<Pad>; 4]>,
    /// Every tick's pads as run.
    pub log: Vec<[Pad; 4]>,
    own: BTreeMap<u32, u64>,
    theirs: Vec<(u32, u64)>,
}

impl Lockstep {
    pub fn new(seats: [bool; 4], mine: Vec<usize>, delay: u32) -> Self {
        let mut l = Lockstep { tick: 0, delay, seats, mine, pending: BTreeMap::new(), log: Vec::new(), own: BTreeMap::new(), theirs: Vec::new() };
        for t in 0..delay {
            *l.slot(t) = [Some(IDLE); 4];
        }
        l
    }

    /// B196: the lockstep from tick `tick` on (a restart), on the ticks before it that `log` ran (`last` the one
    /// before `tick`; a rejoiner's are idle).
    pub fn from(seats: [bool; 4], mine: Vec<usize>, delay: u32, tick: u32, mut log: Vec<[Pad; 4]>, last: [Pad; 4]) -> Self {
        let mut l = Lockstep::new(seats, mine, delay);
        l.pending = (tick..tick + delay).map(|t| (t, [Some(IDLE); 4])).collect();
        log.resize(tick as usize, [IDLE; 4]);
        if let Some(p) = log.last_mut() {
            *p = last;
        }
        (l.tick, l.log) = (tick, log);
        l
    }

    fn slot(&mut self, t: u32) -> &mut [Option<Pad>; 4] {
        let seats = self.seats;
        self.pending.entry(t).or_insert(seats.map(|s| if s { None } else { Some(IDLE) }))
    }

    /// A peer's pads for `tick`. B602: none for this machine's own seats, and none past [`Lockstep::ahead`] (no peer
    /// gets that far without this machine's pads, so the ticks waiting stay few).
    pub fn heard(&mut self, tick: u32, seats: &[(u8, [u8; 6])]) {
        if tick < self.tick || tick - self.tick > self.ahead() {
            return;
        }
        let mine = std::mem::take(&mut self.mine);
        let slot = self.slot(tick);
        for &(s, p) in seats.iter().filter(|(s, _)| !mine.contains(&(*s as usize))) {
            if let Some(x) = slot.get_mut(s as usize) {
                *x = Some(unwire(p));
            }
        }
        self.mine = mine;
    }

    /// B602: how many ticks past the next one a peer's pads can be for: a peer runs at most `delay` ticks ahead of
    /// this machine and sends its pads `delay` ticks on from there, plus a second's slack.
    pub fn ahead(&self) -> u32 {
        2 * self.delay + 60
    }

    /// The seats whose pads for the next tick aren't in.
    pub fn missing(&self) -> Vec<usize> {
        match self.pending.get(&self.tick) {
            Some(s) => (0..4).filter(|&i| s[i].is_none()).collect(),
            None => (0..4).filter(|&i| self.seats[i]).collect(),
        }
    }

    /// Runs the next tick (`missing` empty): its pads, and the message carrying this machine's `local` pads (its seats
    /// in order) for the tick `delay` on.
    pub fn advance(&mut self, local: &[Pad]) -> ([Pad; 4], Msg) {
        let now = self.pending.remove(&self.tick).expect("the tick's pads").map(|p| p.expect("every seat's pad"));
        let at = self.tick + self.delay;
        // P52: lent out while the slot is borrowed, not copied
        let mine = std::mem::take(&mut self.mine);
        let slot = self.slot(at);
        for (&s, &p) in mine.iter().zip(local) {
            slot[s] = Some(p);
        }
        self.log.push(now);
        self.tick += 1;
        let msg = Msg::Pads { tick: at, seats: mine.iter().zip(local).map(|(&s, &p)| (s as u8, wire(p))).collect() };
        self.mine = mine;
        (now, msg)
    }

    /// This machine's hash before tick `t`; a mismatch with a peer's: (tick, ours, theirs).
    pub fn own_hash(&mut self, t: u32, h: u64) -> Option<(u32, u64, u64)> {
        self.own.insert(t, h);
        self.check()
    }

    pub fn their_hash(&mut self, t: u32, h: u64) -> Option<(u32, u64, u64)> {
        self.theirs.push((t, h));
        self.check()
    }

    fn check(&mut self) -> Option<(u32, u64, u64)> {
        let mut bad = None;
        // ponytail: our hashes are all kept (one per 30 ticks, ~16 bytes each)
        self.theirs.retain(|&(t, h)| match self.own.get(&t) {
            Some(&o) => {
                if o != h && bad.is_none() {
                    bad = Some((t, o, h));
                }
                false
            }
            None => true,
        });
        bad
    }
}

/// The online match: the lockstep, the line-up, what goes out to the peers.
#[derive(Resource)]
pub struct Netplay {
    pub lock: Lockstep,
    pub room: Room,
    /// This machine's peer name in `room`.
    pub me: String,
    /// For the peers (the pump hands them to `Net`; without one, e.g. in tests, whoever runs it carries them).
    pub out: Vec<Msg>,
    /// Why the match ended; it closes `END` seconds later.
    pub end: Option<String>,
    /// The states went apart (a report is saved for the first mismatch; until a resync, B154b), and the note on
    /// screen with its seconds left.
    pub desynced: bool,
    /// Where the match's one desync report went (B276: a match that keeps going out of sync and back saved its whole
    /// input log again each time, until the browser ran out of memory).
    report: Option<String>,
    note: Option<(String, f32)>,
    paused: bool,
    /// The last hashed tick and its state's text (`snapshot::state_text`), for a desync's report.
    pub(super) state: (u32, String),
    pub(super) stalled: f32,
    ended: f32,
    /// `HST_NET_MASH=seed`: this machine's seats press random buttons (unattended tests).
    pub(super) mash: Option<u64>,
    /// Bringing the states back together after a desync (B154b).
    pub resync: super::resync::Resync,
    /// The court intro's own lockstep while it runs (`intro.rs` sets it; B42e): each tick's pad says whether that
    /// seat pressed ✕.
    pub(super) intro: Option<Lockstep>,
    /// A peer gone silent: waiting, a bot taking over, a rejoin (B196).
    pub recover: super::recover::Recover,
    /// B738: the last [`KEEP`] hashed ticks' state text and state, for a desync's report (the last in-sync state to
    /// replay from, this machine's text at the bad tick).
    pub(super) kept: BTreeMap<u32, (String, Kept)>,
    /// B738: the peers' state texts for their bad tick (`Msg::Diverged`), by peer: (tick, text, all pieces in).
    pub(super) theirs: BTreeMap<String, (u32, String, bool)>,
    /// B738: the desync report's file and contents, written again as the peers' texts come in.
    filed: Option<(std::path::PathBuf, serde_json::Value)>,
}

/// B738: a hashed tick's state as rollback keeps it: the snapshot and an instant replay's queue while one plays.
pub(super) type Kept = std::sync::Arc<(snapshot::Snapshot, Option<Replay>)>;

/// B738: how many hashed ticks' states a match keeps for its desync report (ggrs tells a desync a few checksums late).
const KEEP: usize = 12;

/// B738: a `Msg::Diverged` piece's length (bytes of state text; under a browser data channel's 256 KB as JSON).
const TEXT_PIECE: usize = 96 << 10;

impl Netplay {
    pub fn new(room: Room, me: String) -> Self {
        let taken = |i: usize| room.seats.get(i).is_some_and(|s| s.as_ref().is_some_and(crate::net::Seat::human));
        let mine = (0..room.seats.len()).filter(|&i| room.seats[i].as_ref().is_some_and(|s| s.peer == me)).collect();
        let rejoin = room.rejoin;
        let others: Vec<String> = room.seats.iter().flatten().filter(|s| s.human() && s.peer != me).map(|s| s.peer.clone()).collect();
        let mash = std::env::var("HST_NET_MASH").ok().and_then(|v| v.parse::<u64>().ok()).map(|s| s | 1);
        Netplay {
            lock: Lockstep::new(std::array::from_fn(taken), mine, room.delay.max(1)),
            room,
            me,
            out: Vec::new(),
            end: None,
            desynced: false,
            report: None,
            note: None,
            paused: false,
            state: (0, String::new()),
            stalled: 0.0,
            ended: 0.0,
            mash,
            intro: None,
            resync: Default::default(),
            recover: super::recover::Recover::new(rejoin, others.into_iter()),
            kept: BTreeMap::new(),
            theirs: BTreeMap::new(),
            filed: None,
        }
        .resuming()
    }

    /// M197: a resumed match holds until the host's restart from the save (as a rejoin's), the host's too.
    fn resuming(mut self) -> Self {
        self.recover.hold |= self.room.resuming();
        self
    }

    /// B738: the state hashed before tick `t` (a rollback's re-run replaces it), the oldest beyond [`KEEP`] dropped.
    pub(super) fn keep(&mut self, t: u32, text: String, state: Kept) {
        self.kept.insert(t, (text, state));
        while self.kept.len() > KEEP {
            self.kept.pop_first();
        }
    }

    /// The court intro's lockstep starts: the match's seats and delay.
    pub(super) fn start_intro(&mut self) {
        self.intro = Some(Lockstep::new(self.lock.seats, self.lock.mine.clone(), self.lock.delay));
    }

    /// Ends the match (the first reason stays), telling the peers if `tell`.
    pub(super) fn finish(&mut self, why: String, tell: bool) {
        if self.end.is_none() {
            if tell {
                self.out.push(Msg::Bye(why.clone()));
            }
            info!("online match over: {why}");
            self.end = Some(why);
        }
    }

    /// The next random pad for seat `s` (`HST_NET_MASH`): the d-pad changes every half second, a face button pressed
    /// about every 20 ticks.
    pub(super) fn mashed(&mut self, s: usize) -> Pad {
        let r = self.mash.as_mut().unwrap();
        *r ^= *r << 13;
        *r ^= *r >> 7;
        *r ^= *r << 17;
        let t = self.lock.tick as u64 + s as u64 * 7;
        let dir = [0, 0x10, 0x20, 0x40, 0x80, 0x30, 0x60, 0xc0, 0x90][((t / 30).wrapping_mul(0x9e37_79b9) >> 7) as usize % 9];
        let press = if (*r).is_multiple_of(20) { [0x1000, 0x2000, 0x4000, 0x8000][(*r >> 8) as usize % 4] } else { 0 };
        Pad { buttons: dir | press, ..IDLE }
    }
}

/// Every seat's pads, every tick, and the line-up: written when hashes differ, once a match (later calls say where).
/// B738: with the last in-sync state (`snapshot::encode`, the newest kept before the bad tick `bad`) and its tick and
/// hash, this machine's state text at the bad tick, and the peers' as they send theirs (each machine sends its own
/// here, `Msg::Diverged`), so `replays_a_desync` runs the pads on from the in-sync state and names the first field
/// the machines' states differ in.
fn save_desync(w: &mut World, why: &str, bad: Option<u32>) -> String {
    if let Some(at) = &w.resource::<Netplay>().report {
        return at.clone();
    }
    let np = w.resource::<Netplay>();
    let iso = w.get_resource::<crate::Args>().map_or(String::new(), |a| a.iso.clone());
    let unix = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let me: String = np.me.chars().take(8).collect();
    // tests running at once (threads, processes) share one folder and the names host/g: two desyncs in the same
    // second took each other's report (one test deleted the file the other then expected)
    let path = if cfg!(test) {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir().join(format!("hst-desync-{}-{n}-{me}.json", std::process::id()))
    } else {
        crate::net::Online::path(&iso).with_file_name(format!("desync-{unix}-{me}.json"))
    };
    let pads: Vec<[[u8; 6]; 4]> = np.lock.log.iter().map(|t| t.map(wire)).collect();
    // the bad tick's state (else the last hashed) and the newest kept before it
    let state = bad.and_then(|t| np.kept.get(&t).map(|k| (t, k.0.clone()))).unwrap_or_else(|| np.state.clone());
    let before = np.kept.range(..bad.unwrap_or(u32::MAX)).next_back().map(|(&t, k)| (t, snapshot::fnv(&k.0), k.1.clone()));
    let mut json = serde_json::json!({ "why": why, "room": np.room, "me": np.me, "pads": pads, "state_tick": state.0, "state": state.1 });
    let from = np.me.clone();
    if let Some((t, hash, kept)) = before {
        let data = encode_kept(w, &kept);
        json["snapshot_tick"] = t.into();
        json["snapshot_hash"] = format!("{hash:016x}").into();
        json["snapshot"] = hex(&data).into();
    }
    // the peers hear this machine's text for the bad tick, in pieces
    let mut np = w.resource_mut::<Netplay>();
    let pieces = text_pieces(&state.1);
    let n = pieces.len();
    np.out.extend(pieces.into_iter().enumerate().map(|(i, text)| Msg::Diverged { from: from.clone(), tick: state.0, text, last: i + 1 == n }));
    json["peers"] = peers_json(&np.theirs);
    let at = write_report(&path, &json);
    np.report = Some(at.clone());
    np.filed = Some((path, json));
    at
}

/// B738: `text` in [`TEXT_PIECE`]-byte pieces cut at character boundaries (one, maybe empty, at least).
fn text_pieces(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    loop {
        let mut cut = rest.len().min(TEXT_PIECE);
        while !rest.is_char_boundary(cut) {
            cut -= 1;
        }
        out.push(rest[..cut].to_string());
        rest = &rest[cut..];
        if rest.is_empty() {
            return out;
        }
    }
}

/// B738: the peers' complete texts for the report: `{peer: {"tick": t, "state": text}}`.
fn peers_json(theirs: &BTreeMap<String, (u32, String, bool)>) -> serde_json::Value {
    theirs.iter().filter(|p| p.1.2).map(|(peer, (t, text, _))| (peer.clone(), serde_json::json!({ "tick": t, "state": text }))).collect::<serde_json::Map<_, _>>().into()
}

/// The report's JSON to `path` (in the browser a download of that name): where it went, or why it didn't.
fn write_report(path: &std::path::Path, json: &serde_json::Value) -> String {
    #[cfg(target_arch = "wasm32")]
    let saved = crate::web::save_file(&path.file_name().unwrap().to_string_lossy(), &json.to_string());
    #[cfg(not(target_arch = "wasm32"))]
    let saved = std::fs::write(path, json.to_string()).map(|()| path.display().to_string());
    match saved {
        Ok(at) => at,
        Err(e) => format!("(not saved: {e})"),
    }
}

/// B738: a piece of a peer's state text for its bad tick; once all are in, the report (if saved) is written again
/// with it (in the browser a second download).
fn heard_text(w: &mut World, from: String, tick: u32, text: String, last: bool) {
    let mut np = w.resource_mut::<Netplay>();
    let have = np.theirs.entry(from).or_insert((tick, String::new(), false));
    if have.0 != tick || have.2 {
        *have = (tick, String::new(), false);
    }
    have.1.push_str(&text);
    have.2 = last;
    if !last {
        return;
    }
    let peers = peers_json(&np.theirs);
    if let Some((path, json)) = &mut np.filed {
        json["peers"] = peers;
        let at = write_report(path, json);
        info!("online: a peer's state for the desync's tick added to {at}");
    }
}

/// B738: a kept state as `snapshot::encode` bytes (the match is put back as it is now), as `rollback::confirmed`.
fn encode_kept(w: &mut World, kept: &Kept) -> Vec<u8> {
    let (now, queue) = (snapshot::save(w), w.get_resource::<Replay>().cloned());
    snapshot::restore(w, &kept.0);
    if let Some(q) = &kept.1 {
        w.insert_resource(q.clone());
    }
    let data = snapshot::encode(w);
    snapshot::restore(w, &now);
    if let Some(q) = queue {
        w.insert_resource(q);
    }
    data
}

/// Bytes as lowercase hex (the report's snapshot).
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// The states differ at tick `t`: logged every time, the first time also the report and a note on screen; the match
/// plays on.
pub(super) fn desync(w: &mut World, (t, ours, theirs): (u32, u64, u64)) {
    // B249: a guest's checksums can tell it only after it took the host's state: still noted and saved, once
    let np = w.resource::<Netplay>();
    let mended = np.resync.stale(t);
    if mended && np.note.is_some() {
        return;
    }
    let why = format!("{DESYNC} at tick {t} (hash {ours:016x}, a peer's {theirs:016x})");
    warn!("online: {why}");
    if !mended {
        super::resync::ask(&mut w.resource_mut::<Netplay>());
        if w.resource::<Netplay>().desynced {
            return;
        }
    }
    let path = save_desync(w, &why, Some(t));
    info!("online: inputs saved to {path}");
    let mut np = w.resource_mut::<Netplay>();
    np.desynced = !mended;
    // a brief line at the top, not the notice window: the match plays on (the report's path is in the log)
    np.note = Some(("Out of sync, catching up".into(), NOTE));
}

/// A network event into the match.
pub fn take(w: &mut World, e: Event) {
    let over = w.get_resource::<Game>().is_some_and(|g| g.match_stats.result.is_some());
    let Some(mut np) = w.get_resource_mut::<Netplay>() else { return };
    // B602: the host's word only from the host (a guest hears everything from it; the host its guests, and its own
    // network's word that a player came back)
    if let Event::Match(from, m) = &e
        && host_only(m)
        && from != "host"
    {
        return warn!("online: dropped a {} from {from}: only the host says that", label(m));
    }
    np.recover.heard_from(&e);
    match e {
        Event::Match(_, Msg::Pads { tick, seats }) => np.lock.heard(tick, &seats),
        Event::Match(_, Msg::Intro { tick, seats }) => {
            if let Some(l) = &mut np.intro {
                l.heard(tick, &seats);
            }
        }
        Event::Match(_, Msg::Hash { tick, hash }) => {
            if let Some(bad) = np.lock.their_hash(tick, hash) {
                desync(w, bad);
            }
        }
        Event::Match(_, Msg::Pause(open)) => {
            np.paused = open;
            front_end::set_paused(w, open);
        }
        Event::Datagram(from, data) => super::rollback::heard(w, from, &data),
        Event::Match(_, Msg::Diverged { from, tick, text, last }) => heard_text(w, from, tick, text, last),
        Event::Match(_, m @ (Msg::State { .. } | Msg::Resynced(_))) => super::resync::heard(&mut np, m),
        // B279: the host left the result screen: its lobby once this machine leaves it too (`again`)
        Event::Match(_, Msg::Lobby(room)) if over => np.recover.lobby = Some(room),
        Event::Match(_, m @ (Msg::Away { .. } | Msg::Choose(_) | Msg::Restart { .. } | Msg::Rejoined { .. } | Msg::Lobby(_))) => super::recover::heard(&mut np, m),
        Event::Match(_, Msg::Bye(why)) | Event::Dropped(why) => {
            if why.starts_with(DESYNC) && np.end.is_none() {
                let path = save_desync(w, &why, None);
                w.resource_mut::<Netplay>().finish(format!("{why}\nInputs saved to {path}"), false);
            } else if why.contains("The match is over") {
                np.finish(why, false);
            } else {
                np.finish(format!("{why} The match is over."), false);
            }
        }
        _ => {}
    }
}

/// B602: the messages only the host sends in the match: who's away, its state, a restart, the way back to the lobby,
/// and its network's word that a player came back.
fn host_only(m: &Msg) -> bool {
    matches!(m, Msg::Away { .. } | Msg::State { .. } | Msg::Restart { .. } | Msg::Lobby(_) | Msg::Rejoined { .. })
}

/// The kind of match message, for the log.
fn label(m: &Msg) -> &'static str {
    match m {
        Msg::Away { .. } => "who's-away word",
        Msg::State { .. } => "match state",
        Msg::Restart { .. } => "restart",
        Msg::Lobby(_) => "lobby line-up",
        Msg::Rejoined { .. } => "rejoin",
        _ => "message",
    }
}

/// The network's events in, the match's messages (and rollback's datagrams) out.
pub(super) fn pump(w: &mut World) {
    loaded(w);
    let Some(net) = w.get_resource::<Net>() else { return };
    // B196: going back to the lobby: its events are the menu's from now on
    let lobby = w.get_resource::<Netplay>().is_some_and(|n| n.recover.lobby.is_some());
    for e in if lobby { Vec::new() } else { net.poll() } {
        take(w, e);
    }
    let out = w.get_resource_mut::<Netplay>().map(|mut n| std::mem::take(&mut n.out)).unwrap_or_default();
    let datagrams = super::rollback::outgoing(w);
    let net = w.resource::<Net>();
    for m in out {
        net.send(Cmd::Send(m));
    }
    for (to, d) in datagrams {
        net.send(Cmd::Datagram(to, d));
    }
}

/// B200: this machine's loading screen down: the peers hear it. Once everyone's is, presses reach the match again.
pub(super) fn loaded(w: &mut World) {
    let covered = crate::mode::covered(w);
    let Some(mut np) = w.get_resource_mut::<Netplay>() else { return };
    // P52: the name is copied only the once it goes in
    if !covered && !np.recover.loaded.contains(&np.me) {
        let me = np.me.clone();
        np.recover.loaded.insert(me.clone());
        info!("online: loaded the match");
        np.out.push(Msg::Loaded(me));
    }
    if np.recover.all_loaded(&np.room) {
        w.remove_resource::<crate::mode::Hush>();
    }
}

/// B200: every peer has loaded the match (its ticks, the court intro's too, wait until then).
pub(super) fn started(np: &Netplay) -> bool {
    np.recover.all_loaded(&np.room)
}

/// The match's tick gate: true runs the next tick on every seat's pads.
pub fn gate(w: &mut World) -> bool {
    pump(w);
    super::resync::lockstep(w);
    super::recover::restarted(w);
    let Some(np) = w.get_resource::<Netplay>() else { return true };
    if np.end.is_some() || np.recover.hold || !started(np) || !np.lock.missing().is_empty() {
        return false;
    }
    let t = np.lock.tick;
    if t % HASH_EVERY == 0 {
        let text = snapshot::state_text(w);
        let h = snapshot::fnv(&text);
        if t % (HASH_EVERY * 60) == 0 {
            info!("online tick {t}: hash {h:016x}");
        }
        let kept = std::sync::Arc::new((snapshot::save(w), super::instant_replay::playing(w).then(|| w.get_resource::<Replay>().cloned()).flatten()));
        let mut np = w.resource_mut::<Netplay>();
        np.keep(t, text.clone(), kept);
        np.state = (t, text);
        np.out.push(Msg::Hash { tick: t, hash: h });
        if let Some(bad) = np.lock.own_hash(t, h) {
            desync(w, bad);
        }
    }
    // this machine's seats' pads (each seat's local device sits in the slot of the same number, `--pads`)
    // P52: lent out (not copied) while the pads are read
    let mine = std::mem::take(&mut w.resource_mut::<Netplay>().lock.mine);
    let mut local: Vec<Pad> = {
        let mut pads = w.resource_mut::<Pads>();
        mine.iter().map(|&s| tick_input::take(&mut pads.slots[s])).collect()
    };
    round_touch(w, &mine, &mut local);
    let mut np = w.resource_mut::<Netplay>();
    if np.mash.is_some() {
        local = mine.iter().map(|&s| np.mashed(s)).collect();
    }
    np.lock.mine = mine;
    let (now, msg) = np.lock.advance(&local);
    np.out.push(msg);
    np.stalled = 0.0;
    let last = np.lock.log.len().checked_sub(2).map_or([IDLE; 4], |p| np.lock.log[p]);
    run_on(w, now, last);
    pump(w);
    true
}

/// B736: each on-screen stick's rounding, by seat (`device::TouchStick`).
#[derive(Resource, Default)]
struct TouchSticks([hst_base::device::TouchStick; 4]);

/// B736: this machine's seats' pads (`seats`, `pads` in step) with an on-screen stick's axes rounded, before the
/// match uses them or they go out: a thumb's jitter changed the stick nearly every tick, and every change is a
/// rollback guess gone wrong. A seat on a controller or the keyboard is left as it is.
pub(super) fn round_touch(w: &mut World, seats: &[usize], pads: &mut [Pad]) {
    let tilts: Vec<Option<Vec2>> = seats.iter().map(|&s| touch_tilt(w, s)).collect();
    let mut sticks = w.get_resource_or_insert_with(TouchSticks::default);
    for ((&s, p), tilt) in seats.iter().zip(pads).zip(tilts) {
        match tilt {
            Some(t) => (p.lx, p.ly) = sticks.0[s].round(t),
            None => sticks.0[s] = Default::default(),
        }
    }
}

/// Seat `s`'s on-screen stick's tilt, if an on-screen pad drives it (its device sits in the slot of the same number).
fn touch_tilt(w: &World, s: usize) -> Option<Vec2> {
    let e = w.get_resource::<super::controls::PadSlots>()?.0.get(s).copied().flatten()?;
    let named = w.get::<Name>(e).is_some_and(|n| super::emote::is_touch(n));
    named.then(|| w.get::<Gamepad>(e).map(|g| g.left_stick())).flatten()
}

/// The court intro's next tick in lockstep (B42e), this machine's seats pressing ✕ if `skip`: `None` while a seat's
/// pad for it isn't in, else whether any seat pressed ✕ on it (the same tick on every machine).
pub(super) fn intro_tick(w: &mut World, skip: bool) -> Option<bool> {
    pump(w);
    let mut np = w.resource_mut::<Netplay>();
    let np = &mut *np;
    // B196: a restart ended the intro (it skips to its end)
    if np.intro.is_none() {
        return Some(true);
    }
    if !started(np) {
        return None;
    }
    let l = np.intro.as_mut().filter(|l| l.missing().is_empty())?;
    let local = vec![Pad { buttons: skip as u16, ..IDLE }; l.mine.len()];
    let (now, msg) = l.advance(&local);
    let Msg::Pads { tick, seats } = msg else { unreachable!() };
    np.out.push(Msg::Intro { tick, seats });
    np.stalled = 0.0;
    let skip = now.iter().any(|p| p.buttons != 0);
    pump(w);
    Some(skip)
}

/// Hands a tick's synced pads (`last`: the tick before's) to the match: as a one-tick `Replay`, or while an instant
/// replay plays its own queue, as the presses its ✕/○ skip reads (every seat's, the same on every machine; this
/// machine's unsent latches are dropped), as rollback's `handle` does.
pub(super) fn run_on(w: &mut World, now: [Pad; 4], last: [Pad; 4]) {
    if !super::instant_replay::playing(w) {
        w.get_resource_or_insert_with(|| Replay(Default::default())).0.push_back(now);
        return;
    }
    let Some(mut pads) = w.get_resource_mut::<Pads>() else { return };
    for s in 0..4 {
        let d = tick_input::decode(now[s], last[s]);
        (pads.slots[s].shot, pads.slots[s].serve) = (d.shot, d.serve);
    }
}

fn setup(mut commands: Commands, np: Option<Res<Netplay>>) {
    let Some(np) = np else { return };
    if np.room.lockstep {
        commands.insert_resource(crate::mode::Gate(gate));
    } else {
        let rb = super::rollback::Rollback::new(&np.room, &np.me, crate::mode::run_fixed).expect("the room's seats");
        commands.insert_resource(rb);
        commands.insert_resource(crate::mode::Gate(super::rollback::gate));
    }
    // the match reads only the pads the gate hands it
    commands.insert_resource(Replay(Default::default()));
    // B200: and no press at all until every peer has loaded it
    commands.insert_resource(crate::mode::Hush);
}

/// Each frame: the network, the pause going out, the stall/end text, and closing the match after the end.
fn watch(w: &mut World) {
    pump(w);
    super::recover::keep(w);
    super::rollback::poll(w);
    // a long frame (loading, a hidden tab) isn't everyone else going silent
    let dt = w.resource::<Time<Real>>().delta_secs().min(0.25);
    let notice = super::recover::tick(w, dt);
    let remote = w.get_resource::<super::rollback::Rollback>().map(|r| r.remote_seats());
    let dt = w.resource::<Time<Real>>().delta_secs();
    let open = front_end::paused(w);
    let result = w.get_resource::<Game>().is_some_and(|g| g.match_stats.result.is_some());
    let mut np = w.resource_mut::<Netplay>();
    if open != np.paused {
        np.paused = open;
        np.out.push(Msg::Pause(open));
    }
    if np.end.is_some() {
        np.ended += dt;
    } else if !open {
        np.stalled += dt;
    }
    let mut toast = false;
    let text = match &np.end {
        Some(why) => why.clone(),
        None if !notice.is_empty() => notice,
        None if np.stalled > LOAD_STALL && !started(&np) => {
            let who: Vec<String> = np.recover.loading(&np.room).iter().map(|p| np.room.names.get(p).cloned().unwrap_or_else(|| p.chars().take(8).collect())).collect();
            format!("Waiting for {} to load...", who.join(", "))
        }
        None if np.stalled > STALL && !result => {
            let who: Vec<String> = np.intro.as_ref().map(Lockstep::missing).or(remote).unwrap_or_else(|| np.lock.missing()).iter().map(|s| format!("{}P", s + 1)).collect();
            format!("Waiting for {}...", who.join(", "))
        }
        None => match &mut np.note {
            Some((say, left)) if *left > 0.0 => {
                *left -= dt;
                toast = true;
                say.clone()
            }
            _ => String::new(),
        },
    };
    let close = np.ended > END;
    pump(w);
    // B196a: in the pause dialog's window (`main_menu/pause.rs`), the bot-or-lobby question on its pills
    let ask = w.resource::<Netplay>().recover.ask_me && w.resource::<Netplay>().end.is_none();
    if let Some(mut n) = w.get_resource_mut::<OnlineNotice>() {
        if ask && !n.ask {
            n.sel = 0;
        }
        if n.text != text || n.ask != ask || n.toast != toast {
            (n.text, n.ask, n.toast) = (text, ask, toast);
        }
    }
    if close {
        lobby_saved(w);
        // B196: the connection goes on into the lobby (`main_menu/online.rs` `launch`'s way back)
        if let Some(room) = w.resource_mut::<Netplay>().recover.lobby.take()
            && let Some(net) = w.remove_resource::<Net>()
        {
            *LOBBY.lock().unwrap() = Some((net, room, false));
        }
        w.write_message(AppExit::Success);
    }
}

/// B279: the result screen left (its Continue): back to the same lobby, seats, picks and court as they were. The host
/// takes everyone there (`Msg::Lobby`; its relay goes back to its lobby loop); a guest takes the lobby the host sent,
/// or, ahead of the host, waits in this one for it (`main_menu/online.rs`).
fn again(w: &mut World) {
    let over = w.get_resource::<Game>().is_some_and(|g| g.match_stats.result.is_some());
    if !over || w.resource::<Messages<AppExit>>().is_empty() || w.resource::<Netplay>().end.is_some() || !w.contains_resource::<Net>() {
        return;
    }
    let mut np = w.resource_mut::<Netplay>();
    let wait = np.me != "host" && np.recover.lobby.is_none();
    let room = np.recover.lobby.take().unwrap_or_else(|| {
        let mut r = np.room.clone();
        r.again();
        r
    });
    let net = w.remove_resource::<Net>().expect("checked");
    if w.resource::<Netplay>().me == "host" {
        info!("online: back to the lobby after the match");
        net.send(Cmd::Send(Msg::Lobby(room.clone())));
    }
    *LOBBY.lock().unwrap() = Some((net, room, wait));
}

/// B196a: an online match's notice (a peer away, the bot-or-lobby question, why it ended), drawn in the pause
/// dialog's window: dimmed court, our box, the words in the game's font, and while asking the two choices on pills
/// with the 3D hand. `netplay.rs` fills it each frame; `recover.rs` reads the choice (`sel`, a tap in `tapped`).
#[derive(Resource, Default)]
pub struct OnlineNotice {
    pub text: String,
    /// The question is up: pill 0 "A bot plays for them", 1 "Everyone to the lobby", 2 "Leave".
    pub ask: bool,
    /// A brief line at the top of the screen instead of the window (a desync the match played through).
    pub toast: bool,
    pub sel: usize,
    pub tapped: bool,
}

/// The choices' pills.
pub const CHOICES: [&str; 3] = ["A bot plays for them", "Everyone to the lobby", "Leave"];

pub fn plugin(app: &mut App) {
    app.init_resource::<OnlineNotice>();
    app.add_systems(Startup, setup).add_systems(Update, watch.run_if(resource_exists::<Netplay>)).add_systems(Last, again.run_if(resource_exists::<Netplay>));
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("HST_NET_THROTTLE").is_some() {
        app.add_systems(Last, super::rollback::throttle.run_if(resource_exists::<Netplay>));
    }
}

/// The match's flags (as `main_menu`'s) for `room`, this machine being `me`: its seats driven by `devices` in seat
/// order, the other taken seats by their peers: a human seat on a gamepad no machine has (`REMOTE`; it may pick up a
/// spare local pad, which the gate never reads), so every machine sets the match up with the same humans.
pub fn flags(room: &Room, me: &str, devices: &[Option<super::controls::Seat>], extra: &[String]) -> Vec<String> {
    let order: &[usize] = if room.doubles { &[0, 2, 1, 3] } else { &[0, 1] };
    let pick = |s: usize| room.seats.get(s).cloned().flatten().map(|x| x.pick);
    let join = |v: Vec<usize>| v.iter().map(usize::to_string).collect::<Vec<_>>().join(",");
    let mut mine = devices.iter();
    let seats: Vec<_> = (0..room.seats.len())
        .map(|s| match &room.seats[s] {
            Some(x) if x.peer == me => mine.next().copied().flatten().or(Some(super::controls::Seat::Pad(REMOTE))),
            Some(x) if x.human() => Some(super::controls::Seat::Pad(REMOTE)),
            // open, or the computer's (`net::CPU`, started with open seats)
            _ => None,
        })
        .collect();
    let court = (room.court + 1).to_string();
    let mut a = vec!["--play".to_string(), "--stage".into(), court.clone(), "--court".into(), court];
    if !room.doubles {
        a.push("--singles".into());
    }
    a.extend([
        "--chars".into(),
        join(order.iter().map(|&s| pick(s).map_or(0, |p| p.char)).collect()),
        "--outfits".into(),
        join(order.iter().map(|&s| pick(s).map_or(0, |p| p.costume)).collect()),
        "--umpire".into(),
        room.umpire.to_string(),
        "--sets".into(),
        (([1, 3, 5][room.sets.min(2)] + 1) / 2).to_string(),
        "--games".into(),
        room.games.to_string(),
        "--pads".into(),
        super::controls::Seat::flag(&seats),
    ]);
    // M8f3: the host's umpire mod, heard by a peer that has it (the donor, `--umpire`, times it either way)
    if let Some((id, _)) = room.umpire_mod.as_ref().filter(|_| !room.vanilla) {
        a.extend(["--umpire-mod-id".into(), id.clone()]);
    }
    // M8f8: the host's court mod over the layout court (`--stage`/`--court` above), drawn by a peer that has it
    if let Some(c) = room.court_mod.as_ref().filter(|_| !room.vanilla) {
        a.extend(["--court-mod-id".into(), c.id.clone()]);
    }
    if room.no_marks {
        a.push("--no-shot-markers".into());
    }
    // M176: the host's Match Speed
    a.extend(crate::match_speed::flags(room.match_hz()));
    // B370: the select's hand toggles, by match slot
    if order.iter().any(|&s| pick(s).is_some_and(|p| p.swap)) {
        a.extend(["--swap-hands".into(), order.iter().map(|&s| if pick(s).is_some_and(|p| p.swap) { "1" } else { "0" }).collect::<Vec<_>>().join(",")]);
    }
    // B40g3: the host's handicaps, by match slot
    if let Some(h) = super::handicap::Handicaps::flag(&order.iter().map(|&s| pick(s).map_or(0, |p| p.handicap)).collect::<Vec<_>>()) {
        a.extend(["--handicap".into(), h]);
    }
    a.extend(extra.iter().cloned());
    a
}

/// The online match as a mode: `game`, `flags(…)`'s match (`main.rs` `match_mode`), with the lockstep on `net`.
pub fn mode(game: crate::mode::Mode, net: Net, room: Room, me: String) -> crate::mode::Mode {
    SEED.store(room.seed as u64, Ordering::Relaxed);
    // M197: the host's save, for a resumed match
    let resume = if me == "host" && room.resuming() { RESUME.lock().unwrap().take() } else { None };
    game.and(move |app| {
        let mut np = Netplay::new(room, me);
        np.recover.resume = resume;
        app.insert_resource(np).insert_resource(net);
    })
}

/// M197: the lobby's offer of a saved match (`recover::offer`).
pub use super::recover::offer;

/// B737: Settings → Online → Netplay Delay (this machine's own rollback input delay; 0 Auto).
pub use super::rollback::{MAX_DELAY as AUTO_DELAY, MOST as MOST_DELAY, choose_delay, chosen_delay};

/// M197: the match the host saved going back to the lobby after a drop, and its line-up (the lobby offers it).
pub static SAVED: Mutex<Option<(std::sync::Arc<super::save_game::MatchSave>, Room)>> = Mutex::new(None);

/// M199: host the online save `save` again (Continue's Open lobby): its lobby (seats, court, rules, the players' ids)
/// back with it, for the lobby to offer (`saved_offer`) and a resumed match to start from, as after a drop. False for
/// a save that kept no lobby.
pub fn host_again(save: super::save_game::MatchSave) -> bool {
    let Some(room) = save.head.setup.lobby.as_ref().map(super::save_game::Lobby::room) else { return false };
    *SAVED.lock().unwrap() = Some((std::sync::Arc::new(save), room));
    true
}

/// M199: the lobby's offer of the match the host keeps (`SAVED`).
pub fn saved_offer() -> Option<crate::net::Resume> {
    SAVED.lock().unwrap().as_ref().map(|(save, room)| offer(save, room))
}

/// M197: the save a resumed match starts from (the host's lobby sets it at Start).
pub static RESUME: Mutex<Option<std::sync::Arc<super::save_game::MatchSave>>> = Mutex::new(None);

/// M197: the host's match saved for its lobby (`recover::lobby_save`) into the saves and to the lobby (`SAVED`).
fn lobby_saved(w: &mut World) {
    super::recover::lobby_save(w);
    let Some(saved) = w.resource_mut::<Netplay>().recover.saved.take() else { return };
    if let Err(e) = super::save_game::Saves::user().write(&saved.0) {
        warn!("online: the unfinished match wasn't saved: {e}");
    }
    *SAVED.lock().unwrap() = Some(saved);
}

/// B196: the lobby an ended match goes back to, with its connection; B279: true for a guest whose host hasn't left
/// the result screen yet (its `Msg::Lobby` to come).
pub static LOBBY: Mutex<Option<(Net, Room, bool)>> = Mutex::new(None);

/// `hst net … --play`: the match `main` starts instead of the menu (it was waiting for the lobby).
pub static PENDING: Mutex<Option<(Net, Room, String)>> = Mutex::new(None);

#[cfg(test)]
mod tests {
    use super::*;

    /// B602: match message `m` from peer `from` as the network hands it to peer `to`: a guest hears everything from
    /// the host (which passes the other guests' on).
    pub(super) fn via(from: &str, to: &str, m: Msg) -> Event {
        Event::Match(if to == "host" { from } else { "host" }.into(), m)
    }

    /// `via` into the match on `w`, from "g" on the host (its match doesn't look at which guest).
    pub(super) fn hear(w: &mut World, m: Msg) {
        let to = w.resource::<Netplay>().me.clone();
        take(w, via("g", &to, m));
    }

    fn pad(b: u16) -> Pad {
        Pad { buttons: b, ..IDLE }
    }

    /// B738: a state text goes to the peers in pieces under the piece size, cut between characters, that join back.
    #[test]
    fn a_state_text_goes_in_pieces_that_join_back() {
        let text: String = "é→x".repeat(super::TEXT_PIECE / 2);
        let pieces = super::text_pieces(&text);
        assert!(pieces.len() > 2 && pieces.iter().all(|p| p.len() <= super::TEXT_PIECE && !p.is_empty()));
        assert_eq!(pieces.concat(), text);
        assert_eq!(super::text_pieces(""), [""]);
    }

    /// B736: an on-screen pad on 1P and a controller on 2P, both held near full tilt with a thumb's wobble (two
    /// pixels of a 60 px reach every tick), then turned a quarter: the on-screen seat's pad changes only a handful of
    /// times in 600 ticks (each change a rollback guess gone wrong and its ticks re-run, every tick before), the
    /// controller's is left as it was, and the buttons pass through.
    #[test]
    fn a_jittery_touch_stick_changes_its_pad_rarely() {
        let mut w = World::new();
        let mut spawn = |name: &str| w.spawn((Gamepad::default(), Name::new(name.to_string()))).id();
        let (touch, xbox) = (spawn("Touch controls (iPhone)"), spawn("Xbox Controller"));
        w.insert_resource(super::super::controls::PadSlots([Some(touch), Some(xbox), None, None], None));
        let mut seed = 0x9e37_79b9_7f4a_7c15u64;
        let mut noise = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % 2001) as f32 / 1000.0 - 1.0
        };
        let byte = |v: f32| hst_sim::player::pad_deadzone(super::super::stick_byte(v));
        let mut changes = [0; 2];
        let mut last = [IDLE; 2];
        for t in 0..600 {
            let a = if t < 400 { 0.0 } else { (t - 400).min(180) as f32 / 180.0 * std::f32::consts::FRAC_PI_2 };
            let tilt = Vec2::from_angle(a) * 0.95 + Vec2::new(noise(), noise()) / 30.0;
            for e in [touch, xbox] {
                let mut g = w.get_mut::<Gamepad>(e).unwrap();
                g.analog_mut().set(bevy::input::gamepad::GamepadInput::Axis(GamepadAxis::LeftStickX), tilt.x);
                g.analog_mut().set(bevy::input::gamepad::GamepadInput::Axis(GamepadAxis::LeftStickY), tilt.y);
            }
            let raw = Pad { buttons: 0x2000, lx: byte(tilt.x), ly: byte(-tilt.y), ..IDLE };
            let mut pads = [raw; 2];
            round_touch(&mut w, &[0, 1], &mut pads);
            assert_eq!(pads[1], raw);
            assert_eq!(pads[0].buttons, 0x2000);
            for s in 0..2 {
                changes[s] += (pads[s] != last[s]) as u32;
            }
            last = pads;
        }
        assert!(changes[0] <= 10 && changes[1] > 400, "{changes:?}");
        // pointing up at the end
        assert!(last[0].lx == 0x80 && last[0].ly <= 0x10, "{:?}", last[0]);
    }

    /// Two machines, 2 seats each, a link `lag` ticks long: both run the same pads, every tick, in order.
    #[test]
    fn lockstep_over_a_slow_link() {
        let (lag, delay) = (6u32, 7u32);
        let mut m = [Lockstep::new([true; 4], vec![0, 2], delay), Lockstep::new([true; 4], vec![1, 3], delay)];
        let mut wire: Vec<(usize, u32, Msg)> = Vec::new();
        let mut stalls = [0; 2];
        for round in 0..2000u32 {
            for (to, at, msg) in std::mem::take(&mut wire) {
                if at <= round {
                    if let Msg::Pads { tick, seats } = &msg {
                        m[to].heard(*tick, seats);
                    }
                } else {
                    wire.push((to, at, msg));
                }
            }
            for i in 0..2 {
                if !m[i].missing().is_empty() {
                    stalls[i] += 1;
                    continue;
                }
                let t = m[i].tick;
                let local: Vec<Pad> = m[i].mine.iter().map(|&s| pad((t * 4 + s as u32) as u16)).collect();
                let (_, msg) = m[i].advance(&local);
                wire.push((1 - i, round + lag, msg));
            }
        }
        let n = m[0].tick.min(m[1].tick) as usize;
        assert!(n > 1900, "ran {n} of 2000 ticks; stalls {stalls:?}");
        assert_eq!(m[0].log[..n], m[1].log[..n]);
        // seat s's pad for tick t was sent at tick t - delay
        assert_eq!(m[0].log[100][3], pad(((100 - delay) * 4 + 3) as u16));
        assert_eq!(m[0].log[3], [IDLE; 4]);
        // a delay shorter than the link stalls
        let mut a = Lockstep::new([true, true, false, false], vec![0], 2);
        a.advance(&[IDLE]);
        a.advance(&[IDLE]);
        assert_eq!(a.missing(), vec![1]);
    }

    #[test]
    fn hashes_compare_whichever_comes_first() {
        let mut l = Lockstep::new([true; 4], vec![0], 3);
        assert_eq!(l.their_hash(30, 5), None);
        assert_eq!(l.own_hash(30, 5), None);
        assert_eq!(l.own_hash(60, 6), None);
        assert_eq!(l.their_hash(60, 7), Some((60, 6, 7)));
    }

    #[test]
    fn pads_wire_round_trip() {
        let p = Pad { buttons: 0xa0f1, rx: 1, ry: 2, lx: 0x80, ly: 0xff };
        assert_eq!(unwire(wire(p)), p);
    }

    #[test]
    fn flags_put_each_seat_in_match_order() {
        use crate::net::{Pick, Seat};
        let s = |peer: &str, char| Some(Seat { peer: peer.into(), pick: Pick::disc(char, char % 3) });
        let room = Room { doubles: true, court: 2, sets: 1, games: 3, umpire: 1, seats: vec![s("host", 5), s("g", 6), s("g", 7), s("host", 8)], ..Default::default() };
        let a = flags(&room, "g", &[Some(super::super::controls::Seat::Keys), Some(super::super::controls::Seat::Pad(0))], &[]);
        let flag = |f: &str| a[a.iter().position(|x| x == f).unwrap() + 1].clone();
        assert_eq!((flag("--chars"), flag("--outfits")), ("5,7,6,8".into(), "2,1,0,2".into()));
        assert_eq!((flag("--pads"), flag("--court"), flag("--sets"), flag("--games")), ("99,k,0,99".into(), "3".into(), "2".into(), "3".into()));
        assert!(!a.contains(&"--swap-hands".to_string()), "no toggle on: no flag");
        // B370: 2P's (the guest's first) hand toggle, at its match slot
        let mut room = room;
        room.seats[1].as_mut().unwrap().pick.swap = true;
        let a = flags(&room, "host", &[None, None], &[]);
        assert_eq!(a[a.iter().position(|x| x == "--swap-hands").unwrap() + 1], "0,0,1,0");
    }

    /// B40g3: the host's handicaps reach every peer's match by match slot; none set, no flag.
    #[test]
    fn flags_carry_the_handicaps() {
        use crate::net::{Pick, Seat};
        let s = |peer: &str| Some(Seat { peer: peer.into(), pick: Pick::disc(0, 0) });
        let mut room = Room { doubles: true, seats: vec![s("host"), s("g"), s("cpu"), s("g")], ..Default::default() };
        let flag = |room: &Room, me| {
            let a = flags(room, me, &[None, None], &[]);
            a.iter().position(|x| x == "--handicap").map(|i| a[i + 1].clone())
        };
        assert_eq!(flag(&room, "g"), None);
        room.seats[1].as_mut().unwrap().pick.handicap = 1;
        room.seats[2].as_mut().unwrap().pick.handicap = 2;
        // seats 0 2 1 3 are match slots 0 1 2 3; every peer reads the same
        assert_eq!((flag(&room, "g"), flag(&room, "host")), (Some("0,2,1,0".into()), Some("0,2,1,0".into())));
    }

    /// The host's Markers Off reaches every peer's match.
    #[test]
    fn flags_carry_markers_off() {
        use crate::net::{Pick, Seat};
        let s = |peer: &str| Some(Seat { peer: peer.into(), pick: Pick::disc(0, 0) });
        let mut room = Room { seats: vec![s("host"), s("g")], ..Default::default() };
        let off = |room: &Room| flags(room, "g", &[None], &[]).contains(&"--no-shot-markers".to_string());
        assert!(!off(&room));
        room.no_marks = true;
        assert!(off(&room));
    }

    /// M176: the host's Match Speed reaches every peer's match (nothing at the default 60, so a 60 Hz lobby's wire and
    /// flags are the ones it always had), and the rollback clock and input delay run in its ticks.
    #[test]
    fn flags_carry_the_match_speed() {
        use crate::net::{Pick, Seat};
        let s = |peer: &str| Some(Seat { peer: peer.into(), pick: Pick::disc(0, 0) });
        let mut room = Room { seats: vec![s("host"), s("g")], ..Default::default() };
        let hz = |room: &Room| {
            let a = flags(room, "g", &[None], &[]);
            a.iter().position(|f| f == "--match-hz").map(|i| a[i + 1].clone())
        };
        assert_eq!((room.match_hz(), hz(&room)), (60, None));
        assert!(!serde_json::to_string(&room).unwrap().contains("hz"));
        room.hz = 50;
        assert_eq!((room.match_hz(), hz(&room).as_deref()), (50, Some("50")));
        room.hz = 42;
        assert_eq!((room.match_hz(), hz(&room)), (60, None));
        let rtt = std::time::Duration::from_millis(300);
        assert_eq!((crate::net::delay_at(rtt, 60), crate::net::delay_at(rtt, 50)), (crate::net::delay_for(rtt), 9));
    }

    /// M8f3: the host's umpire mod goes in the flags by id (each peer looks it up under its own `mods/`, the donor
    /// `--umpire` either way); none in a vanilla lobby.
    #[test]
    fn flags_carry_the_umpire_mod_id() {
        use crate::net::{Pick, Seat};
        let s = |peer: &str| Some(Seat { peer: peer.into(), pick: Pick::disc(0, 0) });
        let mut room = Room { umpire: 2, umpire_mod: Some(("gag_ump00".into(), "Umpire 00".into())), seats: vec![s("host"), s("g")], ..Default::default() };
        let a = flags(&room, "g", &[None], &[]);
        let flag = |a: &[String], f: &str| a.iter().position(|x| x == f).map(|i| a[i + 1].clone());
        assert_eq!((flag(&a, "--umpire"), flag(&a, "--umpire-mod-id")), (Some("2".into()), Some("gag_ump00".into())));
        room.vanilla = true;
        assert_eq!(flag(&flags(&room, "g", &[None], &[]), "--umpire-mod-id"), None);
    }

    /// M8f8: the host's court mod goes in the flags by id over the room's court (its layout, `--stage`/`--court`);
    /// none in a vanilla lobby, where the layout plays as a disc court.
    #[test]
    fn flags_carry_the_court_mod_id() {
        use crate::net::{Pick, Seat};
        let s = |peer: &str| Some(Seat { peer: peer.into(), pick: Pick::disc(0, 0) });
        let mut room = Room { court: 4, court_mod: Some(crate::mods_online::CourtWire { id: "gag_c01_yamate".into(), name: "Yamate".into(), looks: 1, dir: None }), seats: vec![s("host"), s("g")], ..Default::default() };
        let a = flags(&room, "g", &[None], &[]);
        let flag = |a: &[String], f: &str| a.iter().position(|x| x == f).map(|i| a[i + 1].clone());
        assert_eq!((flag(&a, "--stage"), flag(&a, "--court"), flag(&a, "--court-mod-id")), (Some("5".into()), Some("5".into()), Some("gag_c01_yamate".into())));
        room.vanilla = true;
        let a = flags(&room, "g", &[None], &[]);
        assert_eq!((flag(&a, "--stage"), flag(&a, "--court-mod-id")), (Some("5".into()), None));
    }

    /// The court intro online (B42e): the guest's ✕ on its intro tick 10 skips on both machines at tick 10 + delay,
    /// over a link 3 ticks long; a machine waits for the other's ticks.
    #[test]
    fn intro_skip_reaches_both() {
        use crate::net::{Pick, Seat};
        let s = |peer: &str| Some(Seat { peer: peer.into(), pick: Pick::disc(0, 0) });
        let room = Room { seats: vec![s("host"), s("g")], delay: 4, ..Default::default() };
        let mut w = ["host", "g"].map(|me| {
            let mut w = World::new();
            let mut np = Netplay::new(room.clone(), me.into());
            np.start_intro();
            w.insert_resource(np);
            w
        });
        let mut wire: Vec<(usize, u32, Msg)> = Vec::new();
        let mut skipped = [None; 2];
        let mut ticks = [0u32; 2];
        for round in 0..40u32 {
            for (to, at, m) in std::mem::take(&mut wire) {
                if at <= round {
                    hear(&mut w[to], m);
                } else {
                    wire.push((to, at, m));
                }
            }
            for i in 0..2 {
                if let Some(skip) = intro_tick(&mut w[i], i == 1 && ticks[i] == 10) {
                    if skip && skipped[i].is_none() {
                        skipped[i] = Some(ticks[i]);
                    }
                    ticks[i] += 1;
                }
                let out = std::mem::take(&mut w[i].resource_mut::<Netplay>().out);
                wire.extend(out.into_iter().map(|m| (1 - i, round + 3, m)));
            }
        }
        assert_eq!(skipped, [Some(14), Some(14)]);
        assert!(ticks[0] > 30 && ticks[0].abs_diff(ticks[1]) <= 4, "{ticks:?}");
    }

    /// B200: the match's ticks (and its court intro's) wait for every peer's loading screen to be down, and no press
    /// reaches anything until then.
    #[test]
    fn the_match_waits_for_every_peer_to_load() {
        use crate::net::{Pick, Seat};
        let s = |peer: &str| Some(Seat { peer: peer.into(), pick: Pick::disc(0, 0) });
        let room = Room { seats: vec![s("host"), s("g")], delay: 2, ..Default::default() };
        let mut w = World::new();
        let mut np = Netplay::new(room, "host".into());
        np.start_intro();
        w.insert_resource(np);
        w.insert_resource(crate::mode::Hush);
        assert_eq!(intro_tick(&mut w, false), None);
        assert!(!gate(&mut w));
        let out = std::mem::take(&mut w.resource_mut::<Netplay>().out);
        assert!(matches!(&out[..], [Msg::Loaded(p)] if p == "host"), "{out:?}");
        assert!(w.contains_resource::<crate::mode::Hush>());
        hear(&mut w, Msg::Loaded("g".into()));
        assert_eq!(intro_tick(&mut w, false), Some(false));
        assert!(!w.contains_resource::<crate::mode::Hush>());
        assert!(w.resource::<Netplay>().out.iter().all(|m| !matches!(m, Msg::Loaded(_))), "said once");
    }

    /// B602: a guest takes the host's state and lobby only from the host (a forged one from another guest is
    /// dropped); the host takes neither from a guest.
    #[test]
    fn host_only_words_come_from_the_host() {
        use crate::net::{Pick, Seat};
        let s = |peer: &str| Some(Seat { peer: peer.into(), pick: Pick::disc(0, 0) });
        let room = Room { seats: vec![s("host"), s("g"), s("g2"), s("g")], doubles: true, delay: 4, ..Default::default() };
        let state = || Msg::State { tick: 3, data: vec![1; 8], last: true };
        let mut w = World::new();
        w.insert_resource(Netplay::new(room.clone(), "g".into()));
        take(&mut w, Event::Match("g2".into(), state()));
        take(&mut w, Event::Match("g2".into(), Msg::Lobby(room.clone())));
        let np = w.resource::<Netplay>();
        assert!(np.resync.pending.is_none() && np.recover.lobby.is_none() && np.end.is_none());
        take(&mut w, Event::Match("host".into(), state()));
        take(&mut w, Event::Match("host".into(), Msg::Lobby(room.clone())));
        let np = w.resource::<Netplay>();
        assert!(np.resync.pending.is_some() && np.recover.lobby.is_some());

        let mut h = World::new();
        h.insert_resource(Netplay::new(room.clone(), "host".into()));
        h.resource_mut::<Netplay>().recover.away = vec!["g2".into()];
        take(&mut h, Event::Match("g".into(), Msg::Rejoined { old: "g2".into(), new: "g".into(), name: "x".into() }));
        assert_eq!(h.resource::<Netplay>().recover.away, ["g2"], "a guest's forged rejoin");
        take(&mut h, Event::Match("host".into(), Msg::Rejoined { old: "g2".into(), new: "g3".into(), name: "x".into() }));
        assert!(h.resource::<Netplay>().recover.away.is_empty());
    }

    /// B602: a peer's pads for this machine's own seats, or too far ahead, are dropped (the ticks waiting stay few).
    #[test]
    fn pads_only_for_others_seats_and_near_ticks() {
        let mut l = Lockstep::new([true; 4], vec![0, 2], 4);
        let far = l.tick + l.ahead();
        l.heard(far + 1, &[(1, [1; 6])]);
        l.heard(u32::MAX, &[(1, [1; 6])]);
        assert_eq!(l.pending.len(), 4, "the delay's idle ticks only");
        l.heard(far, &[(0, [1; 6]), (1, [2; 6]), (2, [3; 6]), (3, [4; 6])]);
        assert_eq!(l.pending[&far], [None, Some(unwire([2; 6])), None, Some(unwire([4; 6]))]);
        l.heard(0, &[(0, [9; 6]), (1, [9; 6])]);
        assert_eq!(l.pending[&0][0], Some(IDLE), "its own seat stays its own");
    }

    /// B602: the host's state in pieces stops gathering past `STATE_MAX` (the pieces so far dropped, none taken).
    #[test]
    fn resync_pieces_are_capped() {
        use super::super::resync::{STATE_MAX, gather};
        let mut p = Vec::new();
        assert!(gather(&mut p, vec![0; STATE_MAX - 1]));
        assert!(gather(&mut p, vec![0; 1]));
        assert!(!gather(&mut p, vec![0; 1]));
        assert!(p.is_empty());
        let mut np = Netplay::new(Room { seats: vec![None, None], delay: 4, ..Default::default() }, "g".into());
        np.resync.pieces = vec![0; STATE_MAX];
        super::super::resync::heard(&mut np, Msg::State { tick: 3, data: vec![1], last: true });
        assert!(np.resync.pending.is_none() && np.resync.pieces.is_empty());
    }

    /// The match on two headless apps (`snapshot.rs`'s harness, doubles): what each gate sends reaches the other
    /// `lag` rounds (ticks) later.
    mod matches {
        use super::super::super::*;
        use super::super::{Lockstep, Netplay, gate, run_on, take, unwire, wire};
        use super::{hear, via};
        use crate::net::{Event, Msg, Pick, Room, Seat};
        use crate::{Args, GameSpace};
        use std::path::Path;

        const ISO: &str = hst_data::fixture::ISO_PATH;

        fn room(delay: u32) -> Room {
            let s = |peer: &str, char| Some(Seat { peer: peer.into(), pick: Pick::disc(char, 0) });
            // 1P and 2P (a team) on the host, 3P and 4P on the guest
            Room { doubles: true, games: 1, sets: 0, umpire: 4, seats: vec![s("host", 6), s("host", 1), s("g", 2), s("g", 3)], delay, seed: 0x28c7c4a1, ..Default::default() }
        }

        thread_local! {
            /// Instant replays in the harness's matches. A full match's mashed pads keep asking for them (△ during one
            /// asks again), so it never ends with them on.
            static REPLAYS: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
            /// M171: a peer's own disc (its name, the path) where it isn't the US one (`ISO`).
            static DISCS: std::cell::RefCell<Vec<(String, String)>> = const { std::cell::RefCell::new(Vec::new()) };
            /// B734: `app_in` seeds the match from the room's seed as `main.rs` does online (the generators, the
            /// weather schedule, the effects seed), the setup's cloud draws this many (`main.rs` counts them from the
            /// court's cloud models; the harness has no court).
            static CLOUDS: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
        }

        /// B734: what `play::plugin`'s tick runs beyond the harness's chain (the lens flare's draws, the point's
        /// reaction, the sounds with the jingles and the umpire's word timing, the scratch after the poses, bounce
        /// debris, the emotes' lines, the head turns, the carts, the tornado, the offbeat pace, the replay pace, the
        /// point start kept), in its order, so a saved match's ticks run as a machine ran them.
        fn real_tick(app: &mut App, args: &Args) {
            let mut iso = hst_data::iso::Iso::open(&args.iso).unwrap();
            app.insert_resource(crate::audio::Sound::test(&mut iso));
            app.init_resource::<ReactionRoots>();
            app.add_systems(FixedUpdate, (flare_tick.after(remember).before(control), react_first.after(effects::tick).before(control), reaction_roots.after(control).before(simulate)));
            app.add_systems(FixedUpdate, (effects::tick_trails.after(character::tick).before(played_out), play_sounds.after(held_ball).before(instant_replay::run)));
            app.add_systems(FixedUpdate, (doubles_ai::scratch::after_pose.after(bodyhit::pose).after(play_sounds), emote::play_lines.after(play_sounds)));
            app.add_systems(FixedUpdate, (bounce_debris::set_up.after(simulate).before(start_effects), bounce_debris::throw.after(start_effects).after(foot_fx::tick)));
            app.add_plugins((look::plugin, carts::plugin, tornado::plugin, offbeat::plugin, replay_pace::plugin, super::super::super::save_game::plugin));
        }

        /// B734: the room's seed into the match as `main.rs`' `match_weather` takes it online: the shared generator
        /// seeded with it, the weather schedule drawn from it, then the setup's `rand()`s with `clouds` cloud draws.
        fn seed_match(app: &mut App, room: &Room, args: &Args, clouds: usize) {
            let mut disc = hst_data::iso::Iso::open(&args.iso).unwrap();
            let (cnf, bin) = (disc.read("SYSTEM.CNF").unwrap(), disc.read("ZZBIN/GAME.BIN").unwrap());
            let exe = hst_data::exe::Game::new(&cnf, &bin).unwrap();
            let n = room.court as u32 + 1;
            let o = exe.weather_odds(n);
            let odds = hst_sim::weather::Odds { cloudy: o[0], cloudy_len: [o[1], o[2]], rain: o[3], rain_len: [o[4], o[5]], no_border: o[7] != 0, heavy: o[8] != 0 };
            let ((directions, speed), (chance, kind)) = (exe.wind(n), exe.gusts(n));
            let wind = hst_sim::weather::Wind { chance, kind, directions, speed };
            let seed = hst_sim::rng::Seed::shared(room.seed);
            let mut mt = hst_sim::weather::Mt::new(seed.shared);
            let players = if args.singles { 2 } else { 4 };
            let schedule = hst_sim::weather::schedule(&odds, &wind, args.games, args.sets, players, || mt.next());
            let mut rngs = hst_sim::rng::Rngs::new(seed.rand, mt);
            let (effects, _) = rngs.setup_rand(clouds);
            set_effects_seed(effects);
            app.insert_resource(MatchRng(rngs));
            app.insert_resource(crate::weather::Weather { schedule, game: 0, fixed: None });
        }

        fn app(room: &Room, me: &str) -> App {
            app_in(room, me, None)
        }

        /// The folder with a mod's looks, if this machine has it (`None`: the stand-in, as a peer that declined them).
        type Looks<'a> = &'a dyn Fn(&crate::mods_online::Wire) -> Option<std::path::PathBuf>;

        /// `app`, or with `looks` the room's own line-up as `flags` starts it (court, costumes, sets, the seats' mods by
        /// `looks`); the setup's seed and weather stay the harness's (`main.rs` draws those).
        fn app_in(room: &Room, me: &str, looks: Option<Looks>) -> App {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, AssetPlugin::default()));
            app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
            app.init_asset::<crate::gs::GsMaterial>();
            // match order (`flags`'): doubles 1P, 3P, 2P, 4P; singles the two seats
            let order: &[usize] = if room.doubles { &[0, 2, 1, 3] } else { &[0, 1] };
            let chars = order.iter().map(|&s| room.seats[s].as_ref().unwrap().pick.char).collect();
            let iso = DISCS.with_borrow(|d| d.iter().find(|(p, _)| p == me).map(|(_, iso)| iso.clone())).unwrap_or_else(|| ISO.into());
            let mut args = Args { iso, archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: !room.doubles, chars, outfits: vec![], viewer: None, sound: None, music: false, umpire: room.umpire, sets: 1, games: room.games, pads: None };
            if let Some(looks) = looks {
                (args.court, args.stage) = (room.court + 1, Some(room.court as u32 + 1));
                args.outfits = order.iter().map(|&s| room.seats[s].as_ref().unwrap().pick.costume).collect();
                args.sets = ([1, 3, 5][room.sets.min(2)] + 1) / 2;
                let mut mm = crate::mods::MatchMod::default();
                for (k, &s) in order.iter().enumerate() {
                    if let Some(w) = room.seats[s].as_ref().and_then(|x| x.pick.r#mod.as_deref()) {
                        mm.set(k, crate::mods_online::for_match(w, looks(w).as_deref()));
                    }
                }
                app.insert_resource(mm);
            }
            if let Some(clouds) = CLOUDS.get() {
                seed_match(&mut app, room, &args, clouds);
                real_tick(&mut app, &args);
            }
            app.insert_resource(args);
            // the computer plays the seats no person has (B151: started with open seats)
            let human = std::array::from_fn(|i| room.seats.get(i).and_then(Option::as_ref).is_some_and(Seat::human));
            app.insert_resource(Pads { seats: Some(human), autoplay: false, ..default() });
            app.init_resource::<tick_input::TickPads>();
            app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
            app.add_plugins((cutaway::plugin, bodyhit::plugin, surprise::plugin, reaction_voices::plugin, match_stats::plugin, panel::plugin, popups::plugin, npcs::plugin, foot_fx::plugin, weather::plugin, heard::plugin, change_ends::plugin));
            app.add_systems(PostStartup, setup);
            app.add_systems(FixedUpdate, (remember, effects::tick, tick_input::systems(), control, simulate, start_effects, age_balloons, motions, character::tick, played_out, held_ball).chain());
            if REPLAYS.get() {
                app.add_systems(FixedUpdate, instant_replay::run.after(held_ball));
            }
            app.finish();
            app.cleanup();
            app.world_mut().run_schedule(Startup);
            app.world_mut().run_schedule(PostStartup);
            let mut np = Netplay::new(room.clone(), me.into());
            np.mash = Some(if me == "host" { 0x1234_5679 } else { 0x9abc_def1 });
            np.resync.step = step;
            app.insert_resource(np).insert_resource(tick_input::Replay(Default::default()));
            app
        }

        /// Rounds until the match is over or `limit`, `bad` changes the guest's pads at that tick; the two apps.
        fn play(lag: u32, limit: u32, bad: Option<u32>) -> [App; 2] {
            play_room(room(crate::net::delay_for(std::time::Duration::from_millis(2 * lag as u64 * 1000 / 60))), lag, limit, bad)
        }

        fn play_room(room: Room, lag: u32, limit: u32, bad: Option<u32>) -> [App; 2] {
            let mut apps = [app(&room, "host"), app(&room, "g")];
            let mut wire: Vec<(usize, u32, Msg)> = Vec::new();
            // an instant replay's queue as each tick of it runs: it only plays its own pads (M2h1)
            let mut queue = [None; 2];
            for round in 0..limit {
                for (to, at, msg) in std::mem::take(&mut wire) {
                    if at <= round {
                        hear(apps[to].world_mut(), msg);
                    } else {
                        wire.push((to, at, msg));
                    }
                }
                for i in 0..2 {
                    let w = apps[i].world_mut();
                    if gate(w) {
                        if i == 1 && bad == Some(w.resource::<Netplay>().lock.tick) {
                            w.resource_mut::<tick_input::Replay>().0.back_mut().unwrap()[2].buttons ^= !0;
                        }
                        let q = instant_replay::playing(w).then(|| (w.resource::<Game>().instant.replays, w.resource::<tick_input::Replay>().0.len()));
                        if let (Some((r, a)), Some((s, b))) = (queue[i], q) {
                            assert!(r != s || b < a, "a network tick's pads went into instant replay {r}'s queue");
                        }
                        queue[i] = q;
                        w.run_schedule(FixedUpdate);
                    }
                    let out = std::mem::take(&mut w.resource_mut::<Netplay>().out);
                    wire.extend(out.into_iter().map(|m| (1 - i, round + lag, m)));
                }
                let over = |a: &App| a.world().resource::<Game>().match_stats.result.is_some() || a.world().resource::<Netplay>().end.is_some();
                if apps.iter().all(over) && wire.is_empty() {
                    break;
                }
            }
            apps
        }

        /// M2g's acceptance: a doubles match, 2 seats on each machine, 100 ms each way, random pads, to the end
        /// with the same state on both (every 30 ticks' hashes agree: no desync). ~20 min unoptimised:
        /// `cargo test -p hst full_doubles -- --ignored`; `tools/check.sh` runs its first 2400 ticks.
        #[test]
        #[ignore]
        fn full_doubles_match_at_100ms_stays_in_sync() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let _ = in_sync(200_000, true);
        }

        /// One 100 ms doubles match (2400 ticks) for three checks: both machines stay in sync, a state sent from
        /// it hashes the same elsewhere (B154b), and its saved inputs replay to the same state.
        #[test]
        fn doubles_at_100ms_stays_in_sync_sends_and_replays() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let [mut host, mut g] = in_sync(2400, false);
            sent_state_hashes_the_same(&mut host, &mut g);
            saved_inputs_replay_the_same(&mut host);
        }

        /// B151: the host started with two seats open, the computer plays them on both machines alike.
        #[test]
        fn doubles_with_computer_seats_stays_in_sync() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let mut room = room(crate::net::delay_for(std::time::Duration::from_millis(200)));
            room.seats[1] = Some(Seat { peer: crate::net::CPU.into(), pick: Pick::disc(1, 0) });
            room.seats[3] = Some(Seat { peer: crate::net::CPU.into(), pick: Pick::disc(3, 0) });
            let apps = play_room(room, 6, 1500, None);
            let [a, b] = apps.each_ref().map(|a| a.world().resource::<Netplay>());
            assert_eq!((a.end.as_deref(), b.end.as_deref()), (None, None));
            assert_eq!((a.lock.mine.clone(), b.lock.mine.clone()), (vec![0], vec![2]));
            let n = a.lock.log.len().min(b.lock.log.len());
            assert!(n > 1400, "{n} ticks");
            eprintln!("computer seats in sync over {n} ticks");
            assert_eq!(a.lock.log[..n], b.lock.log[..n]);
        }

        fn in_sync(limit: u32, to_the_end: bool) -> [App; 2] {
            REPLAYS.set(!to_the_end);
            let apps = play(6, limit, None);
            for a in &apps {
                let np = a.world().resource::<Netplay>();
                assert_eq!(np.end, None);
                assert!(!to_the_end || a.world().resource::<Game>().match_stats.result.is_some(), "the match didn't finish in {} ticks", np.lock.tick);
            }
            let [a, b] = apps.each_ref().map(|a| a.world().resource::<Netplay>());
            let n = a.lock.log.len().min(b.lock.log.len());
            assert_eq!(a.lock.log[..n], b.lock.log[..n]);
            // a mashed match asks for instant replays all the time: they start and end in lockstep (M2h1)
            let replays = apps[0].world().resource::<Game>().instant.replays;
            assert!(to_the_end || replays > 1, "{replays} instant replays");
            eprintln!("in sync over {n} ticks, {replays} instant replays");
            apps
        }

        /// A tick run on different pads: both see it at the next hash, save the inputs, say so, the guest takes the
        /// host's state and they play on in sync (B154, B154b).
        #[test]
        fn a_desync_is_saved_and_play_goes_on() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            // tick 119's pads are in the state hashed before tick 120
            for mut a in play(6, 400, Some(119)) {
                let path = reported(&a, 120);
                let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).expect("saved")).unwrap();
                assert!(saved["pads"].as_array().unwrap().len() >= 120);
                assert!(saved["why"].as_str().unwrap().starts_with("Out of sync at tick 120 "), "{}", saved["why"]);
                std::fs::remove_file(&path).unwrap();
                // B276: going out of sync again after the note is down notes it, but the match's report isn't saved again
                let t = a.world().resource::<Netplay>().state.0 + 30;
                a.world_mut().resource_mut::<Netplay>().note = None;
                super::super::desync(a.world_mut(), (t, 1, 2));
                let np = a.world().resource::<Netplay>();
                assert!(np.note.is_some(), "the second desync's note");
                assert!(np.report.as_deref() == Some(path.as_str()) && !Path::new(&path).exists());
            }
        }

        /// A desync at tick `t` was noted and saved, resynced once, and the match went on past it with every hash
        /// since agreeing: the report's path.
        fn reported(a: &App, t: u32) -> String {
            let np = a.world().resource::<Netplay>();
            assert_eq!(np.end, None);
            assert!(np.lock.log.len() > t as usize + 60, "played on to {}", np.lock.log.len());
            assert_eq!(np.resync.done, 1, "resyncs");
            assert!(!np.desynced && np.state.0 >= np.resync.fresh + 2 * super::super::HASH_EVERY, "out of sync again by tick {}", np.state.0);
            eprintln!("{}: resynced from tick {}, hashed to {}", np.me, np.resync.fresh, np.state.0);
            assert!(np.note.is_some(), "the desync's note");
            np.report.clone().expect("the desync's report")
        }

        /// B154b: a state from `snapshot::encode` taken on by a match that's elsewhere hashes the same, and its
        /// pieces fit a message.
        fn sent_state_hashes_the_same(host: &mut App, g: &mut App) {
            let data = super::super::snapshot::encode(host.world_mut());
            let json = serde_json::to_vec(&Msg::State { tick: 0, data: vec![255; crate::play::resync::PIECE], last: false }).unwrap().len();
            eprintln!("state {} KB, a piece {} KB as JSON", data.len() / 1024, json / 1024);
            // a browser's data channel takes 256 KB
            assert!(json < 256 << 10, "{json} bytes");
            let mut fresh = app(&room(0), "g");
            for b in [g.world_mut(), fresh.world_mut()] {
                super::super::snapshot::decode(b, &data).unwrap();
                assert_eq!(super::super::snapshot::state_text(b), super::super::snapshot::state_text(host.world_mut()));
            }
        }

        /// A lockstep match's inputs run again as `replays_a_desync` does come out the same, instant replays and all.
        fn saved_inputs_replay_the_same(host: &mut App) {
            let room = room(crate::net::delay_for(std::time::Duration::from_millis(200)));
            let log = host.world().resource::<Netplay>().lock.log.clone();
            assert!(host.world().resource::<Game>().instant.replays > 1);
            let mut a = app(&room, "replay");
            a.world_mut().remove_resource::<Netplay>();
            for (t, &p) in log.iter().enumerate() {
                run_on(a.world_mut(), p, t.checked_sub(1).map_or([tick_input::IDLE; 4], |l| log[l]));
                a.world_mut().run_schedule(FixedUpdate);
            }
            assert_eq!(super::super::snapshot::state_hash(a.world_mut()), super::super::snapshot::state_hash(host.world_mut()));
        }

        /// `HST_DESYNC=desync-….json cargo test -p hst replays_a_desync -- --ignored`: runs the saved inputs again,
        /// printing the hash every 30 ticks (the machines' `Hash` ticks). Singles or doubles; the room's mods play
        /// with the looks from a `mods-online` folder beside the file (as the saving machine received them; else
        /// the donor stands in, which plays alike, B556). At the saved state's tick it compares this run's state with
        /// the saving machine's and prints where they first differ (`HST_DESYNC_TEXT=file`: this run's state there;
        /// `HST_DESYNC_START=file`: its first, the last cloud count tried's). The match is seeded from the room
        /// with the setup's cloud draws found from the log's tick-0 hash (`HST_DESYNC_CLOUDS=n` sets them) and ticks
        /// with the real match's systems; `HST_DESYNC_RERUN=n` instead rolls every tick back from `n` guessed ticks
        /// and panics where that state leaves the straight run's.
        #[test]
        #[ignore]
        fn replays_a_desync() {
            let path = std::path::PathBuf::from(std::env::var("HST_DESYNC").expect("HST_DESYNC=file"));
            let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let room: Room = serde_json::from_value(saved["room"].clone()).unwrap();
            let pads: Vec<[[u8; 6]; 4]> = serde_json::from_value(saved["pads"].clone()).unwrap();
            let at = saved["state_tick"].as_u64().map(|t| t as usize);
            let theirs = saved["state"].as_str().unwrap_or_default().to_string();
            let beside = path.parent().unwrap_or(Path::new(".")).join("mods-online");
            let caches = [beside.join("mods-online"), beside];
            let looks = |w: &crate::mods_online::Wire| {
                let got = caches.iter().find_map(|c| crate::mods_online::find(w, 0, &[], c));
                println!("mod {}: looks {}", w.id, got.as_ref().map_or("(the donor stands in)".into(), |d| d.display().to_string()));
                got
            };
            // B738: the machines' states at the bad tick side by side, and a run on from the last in-sync state
            machines_differ(&saved);
            if saved.get("snapshot").is_some() && std::env::var("HST_DESYNC_RERUN").is_err() {
                let mut a = from_snapshot(&saved, &room, Some(&looks));
                let ours = replay_from(&mut a, &saved, &room, &pads, true);
                let peers = saved["peers"].as_object().cloned().unwrap_or_default();
                if let Some(ours) = ours {
                    report_state(&ours, &theirs, at.unwrap_or(0));
                    for (peer, p) in &peers {
                        println!("against {peer}'s state:");
                        report_state(&ours, p["state"].as_str().unwrap_or_default(), p["tick"].as_u64().unwrap_or(0) as usize);
                    }
                }
                return;
            }
            // the match's start as the saving machine logged it (`rollback tick 0: hash …` in `hst-log.txt` beside
            // the file): the setup's cloud draws are the count that starts the same (`HST_DESYNC_CLOUDS` sets it)
            let log = std::fs::read_to_string(path.with_file_name("hst-log.txt")).unwrap_or_default();
            let logged = |t: usize| log.lines().filter_map(|l| l.split_once(&format!("rollback tick {t}: hash "))).map(|(_, h)| h.trim().to_string()).collect::<Vec<_>>();
            let start = logged(0).first().and_then(|h| u64::from_str_radix(h, 16).ok());
            let fixed = std::env::var("HST_DESYNC_CLOUDS").ok().and_then(|c| c.parse().ok());
            let (mut a, mut found) = (None, None);
            for clouds in fixed.map_or(0..=64, |c| c..=c) {
                CLOUDS.set(Some(clouds));
                let mut b = app_in(&room, "replay", Some(&looks));
                let text = super::super::snapshot::state_text(b.world_mut());
                let h = super::super::snapshot::fnv(&text);
                if let Ok(f) = std::env::var("HST_DESYNC_START") {
                    std::fs::write(f, &text).unwrap();
                }
                if fixed.is_some() || start == Some(h) {
                    println!("tick 0: {clouds} cloud draws start as the saving machine logged ({h:016x}, logged {start:016x?})");
                    (a, found) = (Some(b), Some(clouds));
                    break;
                }
            }
            CLOUDS.set(None);
            let mut a = a.unwrap_or_else(|| panic!("no cloud count 0..=64 starts at the logged hash {start:016x?}: the setup differs"));
            a.world_mut().remove_resource::<Netplay>();
            if let Some(n) = std::env::var("HST_DESYNC_RERUN").ok().and_then(|n| n.parse().ok()) {
                let me = saved["me"].as_str().unwrap_or_default();
                let remote = std::array::from_fn(|s| room.seats.get(s).and_then(Option::as_ref).is_some_and(|x| x.peer != me));
                let clouds = found;
                let make = || {
                    CLOUDS.set(clouds);
                    let mut a = app_in(&room, "replay", Some(&looks));
                    CLOUDS.set(None);
                    a.world_mut().remove_resource::<Netplay>();
                    a
                };
                return rerun_check(&make, &pads, remote, n);
            }
            for (t, p) in pads.iter().enumerate() {
                if t % 1800 == 0 && t > 0 {
                    println!("tick {t}: {:016x}, logged {:?} (a tick rolled back logs again)", super::super::snapshot::state_hash(a.world_mut()), logged(t));
                }
                if t % 30 == 0 {
                    println!("tick {t}: {:016x}", super::super::snapshot::state_hash(a.world_mut()));
                }
                if Some(t) == at {
                    report_state(&super::super::snapshot::state_text(a.world_mut()), &theirs, t);
                }
                let last = t.checked_sub(1).map_or([tick_input::IDLE; 4], |l| pads[l].map(unwire));
                run_on(a.world_mut(), p.map(unwire), last);
                a.world_mut().run_schedule(FixedUpdate);
            }
            let _ = Lockstep::new;
        }

        /// B734: `replays_a_desync` with `HST_DESYNC_RERUN=n`: the saved pads run as this machine's rollback ran
        /// them, every tick first `n` ticks ahead on ggrs' guess (the `remote` seats holding their last pads, this
        /// machine's its own), then back to the state saved before them (`snapshot`, and an instant replay's queue as
        /// `rollback::save` keeps it) and the tick on the real pads. Each tick's state must be the one the same pads
        /// run straight on (a first run of `make`'s match) reach; panics at the first that isn't, with where the
        /// states part: a state the snapshot misses, or one the guessed ticks leave behind.
        fn rerun_check(make: &dyn Fn() -> App, pads: &[[[u8; 6]; 4]], remote: [bool; 4], n: usize) {
            use super::super::snapshot;
            let pad = |t: usize| pads[t].map(unwire);
            // a tick as `rollback::handle` runs one: the queue is this tick's pads alone (an instant replay's, left
            // by a run ahead, goes), or in an instant replay its latches
            let tick = |w: &mut bevy::ecs::world::World, now: [hst_sim::replay::Pad; 4], last: [hst_sim::replay::Pad; 4]| {
                if instant_replay::playing(w) {
                    let mut p = w.resource_mut::<Pads>();
                    for s in 0..4 {
                        p.slots[s] = tick_input::decode(now[s], last[s]);
                    }
                } else {
                    w.insert_resource(tick_input::Replay([now].into()));
                }
                w.run_schedule(FixedUpdate);
            };
            let last = |t: usize| t.checked_sub(1).map_or([tick_input::IDLE; 4], pad);
            let straight = |to: usize| {
                let mut a = make();
                let mut hashes = Vec::with_capacity(to);
                for t in 0..to {
                    hashes.push(snapshot::state_hash(a.world_mut()));
                    tick(a.world_mut(), pad(t), last(t));
                }
                (a, hashes)
            };
            let (_, hashes) = straight(pads.len());
            let mut a = make();
            for (t, &hash) in hashes.iter().enumerate() {
                let w = a.world_mut();
                let now = snapshot::state_text(w);
                if snapshot::fnv(&now) != hash {
                    let (mut b, _) = straight(t);
                    let text = snapshot::state_text(b.world_mut());
                    let at = text.bytes().zip(now.bytes()).position(|(x, y)| x != y).unwrap_or(text.len().min(now.len()));
                    let from = now[..at].char_indices().rev().nth(300).map_or(0, |(i, _)| i);
                    let cut = |s: &str| s[from..].chars().take(700).collect::<String>();
                    if let Ok(f) = std::env::var("HST_DESYNC_TEXT") {
                        std::fs::write(&f, &now).unwrap();
                        std::fs::write(format!("{f}.straight"), &text).unwrap();
                    }
                    panic!("tick {t}: rolled back from {n} guessed ticks every tick, the state differs from the run straight on\nstraight: {}\nrollback: {}", cut(&text), cut(&now));
                }
                if t % 1800 == 0 {
                    println!("tick {t}: {hash:016x}, every rollback alike so far");
                }
                let saved = snapshot::save(w);
                let queue = instant_replay::playing(w).then(|| w.get_resource::<tick_input::Replay>().cloned()).flatten();
                let mut prev = last(t);
                for u in t..(t + n).min(pads.len()) {
                    let real = pad(u);
                    let guess = std::array::from_fn(|s| if remote[s] { last(t)[s] } else { real[s] });
                    tick(w, guess, prev);
                    prev = guess;
                }
                snapshot::restore(w, &saved);
                if let Some(q) = queue {
                    w.insert_resource(q);
                }
                tick(w, pad(t), last(t));
            }
            println!("{} ticks, each rolled back from {n} guessed: every state alike", pads.len());
        }

        /// B738: the desync report's machines' states at the bad tick (the saving machine's, each peer's that came in):
        /// where they first differ, by field.
        fn machines_differ(saved: &serde_json::Value) {
            let (me, mine) = (saved["me"].as_str().unwrap_or("this machine"), saved["state"].as_str().unwrap_or_default());
            let peers = saved["peers"].as_object().cloned().unwrap_or_default();
            if peers.is_empty() {
                println!("no peer's state in the report (saved before B738, or the peer's text never came)");
            }
            for (peer, p) in &peers {
                let theirs = p["state"].as_str().unwrap_or_default();
                let (t, u) = (saved["state_tick"].as_u64().unwrap_or(0), p["tick"].as_u64().unwrap_or(0));
                match first_difference(mine, theirs) {
                    None => println!("tick {t}: {me}'s state and {peer}'s (tick {u}) are the same"),
                    Some((field, at)) => {
                        let from = mine[..at].char_indices().rev().nth(200).map_or(0, |(i, _)| i);
                        let cut = |s: &str| s.get(from..).unwrap_or_default().chars().take(400).collect::<String>();
                        println!("tick {t}: {me} and {peer} (tick {u}) first differ at `{field}` (byte {at})\n{me}: {}\n{peer}: {}", cut(mine), cut(theirs));
                    }
                }
            }
        }

        /// B738: where two state texts first differ: the field's path (`ball.pos`, from the `Debug` text's
        /// `name: ` keys and their nesting) and the byte; `None` if they're the same.
        fn first_difference(a: &str, b: &str) -> Option<(String, usize)> {
            if a == b {
                return None;
            }
            let at = a.bytes().zip(b.bytes()).position(|(x, y)| x != y).unwrap_or(a.len().min(b.len()));
            let (mut stack, mut key, mut word) = (Vec::<String>::new(), String::new(), String::new());
            let mut chars = a[..at].chars().peekable();
            while let Some(c) = chars.next() {
                match c {
                    '"' => {
                        // a string's insides aren't structure
                        let mut esc = false;
                        for c in chars.by_ref() {
                            if !esc && c == '"' {
                                break;
                            }
                            esc = !esc && c == '\\';
                        }
                        word.clear();
                    }
                    c if c.is_alphanumeric() || c == '_' => word.push(c),
                    ':' if chars.peek() == Some(&' ') && !word.is_empty() => key = std::mem::take(&mut word),
                    '{' | '[' | '(' => {
                        stack.push(std::mem::take(&mut key));
                        word.clear();
                    }
                    '}' | ']' | ')' => {
                        key = stack.pop().unwrap_or_default();
                        word.clear();
                    }
                    _ => word.clear(),
                }
            }
            let path: Vec<&str> = stack.iter().chain([&key]).filter(|k| !k.is_empty()).map(String::as_str).collect();
            Some((path.join("."), at))
        }

        /// B738: a match for the report's room with the report's last in-sync state put in (`snapshot::decode`; the
        /// per-match constants are this one's own), its hash checked against the saving machine's.
        fn from_snapshot(saved: &serde_json::Value, room: &Room, looks: Option<Looks>) -> App {
            let t = saved["snapshot_tick"].as_u64().expect("the snapshot's tick");
            let hex = saved["snapshot"].as_str().expect("the snapshot");
            let data: Vec<u8> = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect();
            let mut a = app_in(room, "replay", looks);
            a.world_mut().remove_resource::<Netplay>();
            super::super::snapshot::decode(a.world_mut(), &data).expect("the snapshot loads");
            let h = super::super::snapshot::fnv(&super::super::snapshot::state_text(a.world_mut()));
            let logged = saved["snapshot_hash"].as_str().unwrap_or_default();
            println!("tick {t}: the saved in-sync state put in, hash {h:016x} (the saving machine's {logged})");
            assert_eq!(format!("{h:016x}"), logged, "the snapshot doesn't hash as it did on the saving machine");
            a
        }

        /// B738: the report's pads from its snapshot's tick to its bad tick (`state_tick`) as the room's ticks run
        /// them (lockstep's `run_on`, rollback's `handle`), the hash every 30 ticks if `say`: the state text there
        /// (`None` if the pads end first).
        fn replay_from(a: &mut App, saved: &serde_json::Value, room: &Room, pads: &[[[u8; 6]; 4]], say: bool) -> Option<String> {
            let (from, to) = (saved["snapshot_tick"].as_u64().unwrap() as usize, saved["state_tick"].as_u64().unwrap() as usize);
            let pad = |t: usize| pads[t].map(unwire);
            for t in from..=to {
                let w = a.world_mut();
                if say && t % 30 == 0 {
                    println!("tick {t}: {:016x}", super::super::snapshot::state_hash(w));
                }
                if t == to {
                    return Some(super::super::snapshot::state_text(w));
                }
                let (now, last) = (*pads.get(t)?, t.checked_sub(1).map_or([tick_input::IDLE; 4], pad));
                let now = now.map(unwire);
                if room.lockstep {
                    run_on(w, now, last);
                } else if instant_replay::playing(w) {
                    let mut p = w.resource_mut::<Pads>();
                    for s in 0..4 {
                        p.slots[s] = tick_input::decode(now[s], last[s]);
                    }
                } else {
                    w.insert_resource(tick_input::Replay([now].into()));
                }
                w.run_schedule(FixedUpdate);
            }
            None
        }

        /// B738: a desync report has the last in-sync state, the peer's state at the bad tick beside its own, and
        /// the pads from that state reach the in-sync machine's (the host's) state at the bad tick, not the nudged
        /// guest's; the first difference between the machines is named.
        fn report_replays(path: &str) {
            let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).expect("saved")).unwrap();
            let room: Room = serde_json::from_value(saved["room"].clone()).unwrap();
            let pads: Vec<[[u8; 6]; 4]> = serde_json::from_value(saved["pads"].clone()).unwrap();
            let me = saved["me"].as_str().unwrap();
            let peer = if me == "host" { "g" } else { "host" };
            let (t, s) = (saved["state_tick"].as_u64().unwrap(), saved["snapshot_tick"].as_u64().expect("the snapshot's tick"));
            assert!(s < t && t - s <= super::super::HASH_EVERY as u64 * super::super::KEEP as u64, "{me}: snapshot at {s}, bad tick {t}");
            let theirs = &saved["peers"][peer];
            assert_eq!(theirs["tick"].as_u64(), Some(t), "{me}: {peer}'s state's tick");
            let (mine, theirs) = (saved["state"].as_str().unwrap(), theirs["state"].as_str().unwrap());
            let (field, _) = first_difference(mine, theirs).expect("the machines' states differ");
            eprintln!("{me}: tick {t}, the machines first differ at `{field}`, snapshot {s}");
            machines_differ(&saved);
            let mut a = from_snapshot(&saved, &room, None);
            let ran = replay_from(&mut a, &saved, &room, &pads, false).expect("the pads reach the bad tick");
            let host = if me == "host" { mine } else { theirs };
            assert!(ran == host, "{me}: replayed from tick {s}, differs from the host's state at {t}: {:?}", first_difference(&ran, host));
        }

        /// `replays_a_desync`'s verdict at tick `t`: this run's state `ours` against the saving machine's `theirs`.
        fn report_state(ours: &str, theirs: &str, t: usize) {
            if let Ok(f) = std::env::var("HST_DESYNC_TEXT") {
                std::fs::write(f, ours).unwrap();
            }
            println!("tick {t}: this run {:016x}, the saving machine {:016x}", super::super::snapshot::fnv(ours), super::super::snapshot::fnv(theirs));
            if ours == theirs {
                return println!("tick {t}: the same state as the saving machine's (the other machine went its own way)");
            }
            let at = ours.bytes().zip(theirs.bytes()).position(|(x, y)| x != y).unwrap_or(ours.len().min(theirs.len()));
            let from = ours[..at].char_indices().rev().nth(300).map_or(0, |(i, _)| i);
            let cut = |s: &str| s[from..].chars().take(700).collect::<String>();
            println!("tick {t}: differs from the saving machine's state\nthis run: {}\nsaved:    {}", cut(ours), cut(theirs));
        }

        /// C2: one machine's mashed doubles match, every tick's state hash written to `HST_HASH_TRACE` (one hex line
        /// a tick, `HST_HASH_TICKS` ticks, default 30000, or until the match of `HST_HASH_GAMES` games, default 1, ends). Run before and after a change
        /// that must not change the sim and diff the files:
        /// `HST_HASH_TRACE=context/c2/a.txt tools/check.sh -p hst hash_trace -- --ignored` (~1 min optimised).
        #[test]
        #[ignore]
        fn hash_trace() {
            let path = std::env::var("HST_HASH_TRACE").expect("HST_HASH_TRACE=file");
            let ticks: u32 = std::env::var("HST_HASH_TICKS").map_or(30_000, |t| t.parse().unwrap());
            REPLAYS.set(false);
            let mut room = room(0);
            room.games = std::env::var("HST_HASH_GAMES").map_or(1, |g| g.parse().unwrap());
            room.seats.iter_mut().for_each(|s| s.as_mut().unwrap().peer = "host".into());
            let mut a = app(&room, "replay");
            let mut np = a.world_mut().remove_resource::<Netplay>().unwrap();
            let mut out = String::new();
            let mut last = [tick_input::IDLE; 4];
            for t in 0..ticks {
                out += &format!("{:016x}\n", super::super::snapshot::state_hash(a.world_mut()));
                if a.world().resource::<Game>().match_stats.result.is_some() {
                    break;
                }
                np.lock.tick = t;
                let now = [0, 1, 2, 3].map(|s| np.mashed(s));
                run_on(a.world_mut(), now, last);
                a.world_mut().run_schedule(FixedUpdate);
                last = now;
            }
            std::fs::write(path, out).unwrap();
        }

        /// B556: a mod character plays the same match on a machine with its looks as on one where its donor stands in
        /// (a peer whose download failed): the B556 report's line-up (Anya, `mods/hst-mods/oob_pc12_anya`, on 1P),
        /// every seat mashed, the two machines' state compared every tick for `HST_LOOKS_TICKS` (default 4000).
        #[test]
        fn a_mod_plays_alike_with_or_without_its_looks() {
            const ANYA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods/hst-mods/oob_pc12_anya");
            if !hst_data::fixture::has_iso() || !Path::new(ANYA).is_dir() {
                return eprintln!("no ISO or mods/hst-mods/oob_pc12_anya, skipped");
            }
            let ticks: u32 = std::env::var("HST_LOOKS_TICKS").map_or(4000, |t| t.parse().unwrap());
            REPLAYS.set(false);
            let w = crate::mods_online::Wire::of(&crate::mods::read(Path::new(ANYA)).unwrap(), 0).unwrap();
            let seat = |char, costume| Some(Seat { peer: "host".into(), pick: Pick::disc(char, costume) });
            let anya = Some(Seat { peer: "host".into(), pick: Pick { r#mod: Some(Box::new(w)), ..Pick::disc(8, 0) } });
            let room = Room { doubles: true, games: 4, sets: 0, umpire: 2, court: 3, seats: vec![anya, seat(10, 6), seat(7, 5), seat(8, 1)], seed: 1502962241, ..Default::default() };
            let mut apps = [Some(ANYA.into()), None].map(|d: Option<std::path::PathBuf>| app_in(&room, "host", Some(&|_| d.clone())));
            let mut np = apps.each_mut().map(|a| a.world_mut().remove_resource::<Netplay>().unwrap());
            let mut last = [tick_input::IDLE; 4];
            for t in 0..ticks {
                let [a, b] = apps.each_mut().map(|a| super::super::snapshot::state_text(a.world_mut()));
                if a != b {
                    let at = a.bytes().zip(b.bytes()).position(|(x, y)| x != y).unwrap_or(a.len().min(b.len()));
                    let from = a[..at].char_indices().rev().nth(300).map_or(0, |(i, _)| i);
                    let cut = |s: &str| s[from..].chars().take(700).collect::<String>();
                    panic!("tick {t}: the looks and the stand-in differ\nlooks:    {}\nstand-in: {}", cut(&a), cut(&b));
                }
                if apps[0].world().resource::<Game>().match_stats.result.is_some() {
                    break;
                }
                np[0].lock.tick = t;
                let now = [0, 1, 2, 3].map(|s| np[0].mashed(s));
                for a in &mut apps {
                    run_on(a.world_mut(), now, last);
                    a.world_mut().run_schedule(FixedUpdate);
                }
                last = now;
            }
        }

        /// M151d: a mod with generated tables (§6 `shots` `power`/`arc`) and a disc body, scaled and paced, plays the
        /// same match on its owner's machine as on one without its folder: each builds the tables from the wire and
        /// its own disc (every peer's tables fingerprint agreeing, `handshake`). Its serves fly faster than its
        /// donor's on both; `HST_LOOKS_TICKS` ticks (default 2500), every seat mashed.
        #[test]
        fn a_mod_with_generated_tables_plays_alike_on_every_peer() {
            if !hst_data::fixture::has_iso() {
                return eprintln!("no ISO, skipped");
            }
            let dir = std::env::temp_dir().join(format!("hst-m151d-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("mod.json"), r#"{"standard":1,"id":"m151d","name":"Big Server","disc_body":true,"donor":7,"hand":"right","params":{"base":7,"override":{}},"ai_row":7,"face":"morph","scale":1.25,"swing_speed":1.5,"run_speed":0.8,"stats":"S","shots":{"serve":{"power":1.4,"arc":1.5},"stroke":{"from":9,"preset":"cannon"}},"version":1}"#).unwrap();
            let ticks: u32 = std::env::var("HST_LOOKS_TICKS").map_or(2500, |t| t.parse().unwrap());
            REPLAYS.set(false);
            let w = crate::mods_online::Wire::of(&crate::mods::read(&dir).unwrap(), 2).unwrap();
            assert_eq!(w.check(), Ok(()));
            assert!(w.disc && w.power[0] == 1.4 && w.arc[0] == 1.5 && w.scale == 1.25 && w.names.is_empty(), "{w:?}");
            // what the other peer gets
            let w: crate::mods_online::Wire = serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
            let seat = |char, costume| Some(Seat { peer: "host".into(), pick: Pick::disc(char, costume) });
            let big = Some(Seat { peer: "host".into(), pick: Pick { r#mod: Some(Box::new(w)), ..Pick::disc(7, 2) } });
            let room = Room { doubles: true, games: 4, sets: 0, umpire: 2, court: 1, seats: vec![big, seat(7, 0), seat(3, 1), seat(5, 0)], seed: 151, ..Default::default() };
            let mut apps = [Some(dir.clone()), None].map(|d: Option<std::path::PathBuf>| app_in(&room, "host", Some(&|_| d.clone())));
            let _ = std::fs::remove_dir_all(&dir);
            for a in &apps {
                let g = a.world().resource::<Game>();
                // match order: the mod is player 0, the stock Big Chief player 2
                let top = |i: usize| g.serve_tables[i][0][2].0.top_speed();
                assert!(top(0) > 1.3 * top(2), "{} vs stock {}", top(0), top(2));
            }
            let mut np = apps.each_mut().map(|a| a.world_mut().remove_resource::<Netplay>().unwrap());
            let mut last = [tick_input::IDLE; 4];
            for t in 0..ticks {
                let [a, b] = apps.each_mut().map(|a| super::super::snapshot::state_text(a.world_mut()));
                if a != b {
                    let at = a.bytes().zip(b.bytes()).position(|(x, y)| x != y).unwrap_or(a.len().min(b.len()));
                    let from = a[..at].char_indices().rev().nth(300).map_or(0, |(i, _)| i);
                    let cut = |s: &str| s[from..].chars().take(700).collect::<String>();
                    panic!("tick {t}: the owner and the other peer differ\nowner: {}\nother: {}", cut(&a), cut(&b));
                }
                if apps[0].world().resource::<Game>().match_stats.result.is_some() {
                    break;
                }
                np[0].lock.tick = t;
                let now = [0, 1, 2, 3].map(|s| np[0].mashed(s));
                for a in &mut apps {
                    run_on(a.world_mut(), now, last);
                    a.world_mut().run_schedule(FixedUpdate);
                }
                last = now;
            }
        }

        // ---------------------------------------------------------------- rollback (M2h)

        use super::super::super::rollback::{self, Rollback};

        fn step(w: &mut bevy::prelude::World) {
            w.run_schedule(FixedUpdate);
        }

        /// A tick that comes out differently on one machine: tick 300 reseeds the AI's generator.
        fn nudged(w: &mut bevy::prelude::World) {
            if w.resource::<Rollback>().frame() == 300 {
                w.resource_mut::<Game>().rng.ai = hst_sim::rng::Mt::new(7);
            }
            step(w);
        }

        /// The two apps with rollback sessions; their datagrams reach the other `lag` rounds (ticks) later. `bad`:
        /// the guest's tick 300 comes out differently.
        fn play_rollback(lag: u32, limit: u32, bad: bool) -> [App; 2] {
            play_rollback_with(lag, limit, bad, 0..0, |_, _| {})
        }

        /// `play_rollback`, the guest frozen for the rounds in `hitch` (a hiccup: nothing runs, sent or heard), and
        /// `each(round, apps)` after every round. A round is a tick of time by ggrs' clock (B494), so the outcome
        /// doesn't depend on how long it really takes (a loaded machine).
        fn play_rollback_with(lag: u32, limit: u32, bad: bool, hitch: std::ops::Range<u32>, each: impl FnMut(u32, &mut [App; 2])) -> [App; 2] {
            play_rollback_in(room(0), lag, limit, bad, hitch, each)
        }

        /// B731: `play_rollback_with` on `room` (its delay is the sessions' input delay).
        fn play_rollback_in(room: Room, lag: u32, limit: u32, bad: bool, hitch: std::ops::Range<u32>, each: impl FnMut(u32, &mut [App; 2])) -> [App; 2] {
            play_rollback_at(room, None, lag, limit, bad, hitch, each)
        }

        /// B737: `play_rollback_in` with the host's and the guest's own input delays (`None`: the setting's).
        fn play_rollback_at(room: Room, delays: Option<[usize; 2]>, lag: u32, limit: u32, bad: bool, hitch: std::ops::Range<u32>, mut each: impl FnMut(u32, &mut [App; 2])) -> [App; 2] {
            ggrs::clock::drive(true);
            let mut i = 0;
            let mut apps = ["host", "g"].map(|me| {
                let mut a = app(&room, me);
                let f = if bad && me == "g" { nudged } else { step };
                let delay = delays.map_or(rollback::input_delay(&room), |d| d[i]);
                i += 1;
                a.insert_resource(Rollback::with_delay(&room, me, f as fn(&mut _), delay).unwrap());
                a
            });
            let mut wire: Vec<(usize, u32, String, Vec<u8>)> = Vec::new();
            for round in 0..limit {
                ggrs::clock::advance(std::time::Duration::from_micros(16_667));
                for (to, at, from, d) in std::mem::take(&mut wire) {
                    if at <= round && !(to == 1 && hitch.contains(&round)) {
                        take(apps[to].world_mut(), Event::Datagram(from, d));
                    } else {
                        wire.push((to, at, from, d));
                    }
                }
                for i in 0..2 {
                    if i == 1 && hitch.contains(&round) {
                        continue;
                    }
                    let w = apps[i].world_mut();
                    rollback::gate(w);
                    let me = w.resource::<Netplay>().me.clone();
                    wire.extend(rollback::outgoing(w).into_iter().map(|(_, d)| (1 - i, round + lag, me.clone(), d)));
                    let out = std::mem::take(&mut w.resource_mut::<Netplay>().out);
                    for m in out {
                        hear(apps[1 - i].world_mut(), m);
                    }
                }
                each(round, &mut apps);
                let over = |a: &App| a.world().resource::<Game>().match_stats.result.is_some() || a.world().resource::<Netplay>().end.is_some();
                if apps.iter().all(over) {
                    break;
                }
            }
            ggrs::clock::drive(false);
            apps
        }

        /// M2h's acceptance: a doubles match (2 seats on each machine, random pads) at 150 ms round trip (5 ticks each
        /// way: 167 ms) with 2 ticks of input delay, to the end: ggrs' checksums (every 30th tick's state hash) agree
        /// on every confirmed tick and both ran the same pads. ~1 h unoptimised:
        /// `cargo test --release -p hst full_doubles_match_rollback -- --ignored`; `tools/check.sh` runs 1200 ticks.
        #[test]
        #[ignore]
        fn full_doubles_match_rollback_at_150ms_stays_in_sync() {
            rollback_in_sync(400_000, true);
        }

        #[test]
        fn rollback_at_150ms_stays_in_sync() {
            rollback_in_sync(1200, false);
        }

        fn rollback_in_sync(limit: u32, to_the_end: bool) {
            if !hst_data::fixture::has_iso() {
                return;
            }
            // a room with no measured round trip: the default delay
            assert_eq!(rollback::input_delay(&room(0)), rollback::MAX_DELAY);
            const { assert!(rollback::MAX_DELAY <= 2) };
            let t0 = std::time::Instant::now();
            REPLAYS.set(!to_the_end);
            let apps = play_rollback(5, limit, false);
            for a in &apps {
                let np = a.world().resource::<Netplay>();
                assert_eq!(np.end, None);
                assert!(!to_the_end || a.world().resource::<Game>().match_stats.result.is_some(), "the match didn't finish in {} ticks", np.lock.log.len());
            }
            let [a, b] = apps.each_ref().map(|a| a.world().resource::<Netplay>());
            // the last few may still be guesses
            let n = a.lock.log.len().min(b.lock.log.len()) - rollback::WINDOW;
            assert_eq!(a.lock.log[..n], b.lock.log[..n]);
            let reruns = apps.each_ref().map(|a| a.world().resource::<Rollback>().reruns);
            // a guess is wrong whenever a peer's pad changes: that's what rollback is for
            assert!(reruns.iter().all(|&r| r > 100), "re-run ticks {reruns:?}");
            let checked = a.state.0.min(b.state.0);
            assert!(checked >= (n as u32).saturating_sub(120), "the checksums stopped at tick {checked} of {n}");
            let replays = apps[0].world().resource::<Game>().instant.replays;
            assert!(to_the_end || replays > 1, "{replays} instant replays");
            eprintln!("in sync over {n} ticks, {reruns:?} ticks re-run, {replays} instant replays, {:.0?}", t0.elapsed());
        }

        /// B737: Netplay Delay is each machine's own: Auto is [`rollback::MAX_DELAY`] whatever the host measured,
        /// 1..=4 as picked. Peers on different delays (the host 1 tick, the guest 4) over a slow link (200 ms: 6
        /// ticks each way) run the same pads and ggrs' checksums agree.
        #[test]
        fn rollback_peers_with_different_delays_stay_in_sync() {
            let room = room(crate::net::delay_for(std::time::Duration::from_millis(200)));
            assert_eq!((rollback::delay_of(0), rollback::delay_of(1), rollback::delay_of(4), rollback::delay_of(9)), (rollback::MAX_DELAY, 1, 4, 4));
            if !hst_data::fixture::has_iso() {
                return;
            }
            REPLAYS.set(true);
            let apps = play_rollback_at(room, Some([1, 4]), 6, 600, false, 0..0, |_, _| {});
            for (a, d) in apps.iter().zip([1, 4]) {
                assert_eq!(a.world().resource::<Netplay>().end, None);
                assert_eq!(a.world().resource::<Rollback>().delay, d);
            }
            let [a, b] = apps.each_ref().map(|a| a.world().resource::<Netplay>());
            let n = a.lock.log.len().min(b.lock.log.len()) - rollback::WINDOW;
            assert!(n > 300, "only {n} confirmed ticks");
            assert_eq!(a.lock.log[..n], b.lock.log[..n]);
            let checked = a.state.0.min(b.state.0);
            assert!(checked >= (n as u32).saturating_sub(120), "the checksums stopped at tick {checked} of {n}");
        }

        /// B162: the guest's frame loop stops (a hidden browser tab, a minimised window) at tick 300 for `away` of real
        /// time while its network still queues the host's datagrams; the host stalls, nobody is dropped, and after
        /// it comes back both run on in sync.
        fn tabbed_out(away: std::time::Duration) {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let room = room(0);
            let mut apps = ["host", "g"].map(|me| {
                let mut a = app(&room, me);
                own_camera::plugin(&mut a);
                a.insert_resource(Rollback::new(&room, me, step as fn(&mut _)).unwrap());
                a
            });
            let mut wire: Vec<(usize, String, Vec<u8>)> = Vec::new();
            let mut gone: Option<std::time::Instant> = None;
            // B299: per round after the guest is back, whether its picture is its own camera's (it looks from the
            // other end, so the shared camera's is a different view)
            let mut own = Vec::new();
            let mut round = 0;
            while round < 900 {
                for (to, from, d) in std::mem::take(&mut wire) {
                    take(apps[to].world_mut(), Event::Datagram(from, d));
                }
                let g = apps[1].world().resource::<Rollback>().frame();
                if g == 300 && gone.is_none() {
                    gone = Some(std::time::Instant::now());
                }
                let hidden = gone.is_some_and(|t| t.elapsed() < away);
                for i in 0..2 {
                    if i == 1 && hidden {
                        continue;
                    }
                    let w = apps[i].world_mut();
                    rollback::gate(w);
                    let me = w.resource::<Netplay>().me.clone();
                    wire.extend(rollback::outgoing(w).into_iter().map(|(_, d)| (1 - i, me.clone(), d)));
                    for m in std::mem::take(&mut w.resource_mut::<Netplay>().out) {
                        hear(apps[1 - i].world_mut(), m);
                    }
                }
                // B376: a cut-away or an instant replay has the shared camera (the harness's replays can start on the
                // tick after a cut-away's first, and a replay's first tick cuts): those aren't flips
                let w = apps[1].world();
                if gone.is_some() && !hidden && !w.resource::<Game>().instant.replaying && !w.resource::<cutaway::Director>().showing() {
                    let g = w.resource::<Game>();
                    own.push(own_camera::shown(g, w.get_resource()) != own_camera::shown(g, None));
                }
                if hidden {
                    std::thread::sleep(std::time::Duration::from_millis(16));
                    assert!(apps[0].world().resource::<Rollback>().frame() <= 300 + rollback::WINDOW + rollback::MAX_DELAY + 1, "the host ran on without the guest");
                } else {
                    round += 1;
                }
            }
            let [a, b] = apps.each_ref().map(|a| a.world().resource::<Netplay>());
            assert_eq!((&a.end, &b.end), (&None, &None));
            let n = a.lock.log.len().min(b.lock.log.len()) - rollback::WINDOW;
            assert!(n > 800, "only {n} ticks");
            assert_eq!(a.lock.log[..n], b.lock.log[..n]);
            // a cut-away or replay has the shared camera for seconds; a round or two of it is the catch-up's re-runs
            // flipping the picture to the other end
            let runs: Vec<_> = own.chunk_by(|a, b| a == b).filter(|r| !r[0]).map(<[bool]>::len).collect();
            assert!(runs.iter().all(|&r| r > 10), "the guest's picture flipped to the shared camera for {runs:?} rounds");
        }

        /// Longer than ggrs' 2 s default disconnect timeout.
        #[test]
        fn rollback_survives_a_tab_out() {
            tabbed_out(std::time::Duration::from_secs(5));
        }

        /// The task's 2 minutes: `cargo test --release -p hst tab_out_for_2_min -- --ignored`.
        #[test]
        #[ignore]
        fn rollback_survives_a_tab_out_for_2_min() {
            tabbed_out(std::time::Duration::from_secs(120));
        }


        /// B163: after the guest freezes for 20 ticks the host runs ahead; it catches the guest up by sitting out one
        /// tick at a time (never two in a row once it's running again: ggrs' own advice froze 3+ at once), and the
        /// two end up within a tick of each other.
        #[test]
        fn rollback_time_sync_spreads_the_waits() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let frame = |a: &App| a.world().resource::<Rollback>().frame();
            let (mut last, mut idle, mut longest) = (0, 0, 0);
            let apps = play_rollback_with(3, 900, false, 300..320, |round, apps| {
                let f = frame(&apps[0]);
                // the host waits on the guest's pads until a little after it's back
                if round > 340 {
                    idle = if f == last { idle + 1 } else { 0 };
                    longest = longest.max(idle);
                }
                last = f;
            });
            assert_eq!(longest, 1, "the host sat out {longest} ticks in a row");
            let [a, b] = apps.each_ref().map(frame);
            assert!(a.abs_diff(b) <= 2, "host at tick {a}, guest at {b}");
            // B197: time sync moves each clock at most `SHIFT` off real time; this guest's 20 lost ticks (the harness
            // doesn't hand them back as Bevy's fixed loop now does) take both up to it, a tick or two short
            assert!(apps[0].world().resource::<Rollback>().sess.frames_ahead() <= 2);
        }

        /// B228: the host's own emote (1P's flick, a local seat on the host) plays on both machines, in sync. The
        /// host's mashed pads stop for the tick it flicks, so its latch goes out as a real device's would.
        #[test]
        fn the_hosts_emote_plays_on_both() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let (mut flicked, mut seen, mut mash) = (None, [false; 2], None);
            let apps = play_rollback_with(5, 900, false, 0..0, |round, apps| {
                if let Some(m) = mash.take() {
                    apps[0].world_mut().resource_mut::<Netplay>().mash = Some(m);
                }
                for (i, a) in apps.iter().enumerate() {
                    seen[i] |= a.world().resource::<Game>().players[0].emote > 0;
                }
                let w = apps[0].world_mut();
                let g = w.resource::<Game>();
                // from 1P's first serve (the host serves first: before the toss) or a rally after it
                let open = g.phase == Phase::Rally || g.phase == Phase::Serve && g.serving.toss.is_none();
                if !seen[0] && round % 10 == 0 && open {
                    flicked.get_or_insert(round);
                    mash = w.resource_mut::<Netplay>().mash.take();
                    w.resource_mut::<Pads>().slots[0].emote = Some(2);
                }
            });
            assert!(flicked.is_some(), "no serve or rally to flick in");
            assert_eq!(seen, [true, true], "the host's emote played (host, guest), flicked from round {flicked:?}");
            let [a, b] = apps.each_ref().map(|a| a.world().resource::<Netplay>());
            assert_eq!((a.end.clone(), b.end.clone()), (None, None));
            let n = a.lock.log.len().min(b.lock.log.len()) - rollback::WINDOW;
            assert_eq!(a.lock.log[..n], b.lock.log[..n]);
            assert!(!a.desynced && !b.desynced);
        }

        /// B735: what each machine draws for the ball, before (the match's own ball) and with rollback corrections
        /// gliding (`smooth`), against where it really was (the same pads on one machine), `lag` ticks each way
        /// (`EXP_LAG`, 9) over `EXP_ROUNDS` frames: frames drawn off, separate warps, frames that jump over 25 cm past
        /// the ball's true move and the worst of those short of a snap, and snaps (3 m or more). This machine's own players are never moved.
        fn drawn_ball_corrections(lag: u32, rounds: u32) -> [[(u32, u32, u32, f32, u32); 2]; 2] {
            use super::super::super::smooth::{Smooth, fade};
            REPLAYS.set(false);
            let mut room = room(0);
            room.games = 2;
            // per machine, per drawn tick (its first frame): the match's ball and the drawn one
            let mut shown: [std::collections::BTreeMap<usize, ([f32; 3], [f32; 3])>; 2] = Default::default();
            let apps = play_rollback_in(room.clone(), lag, rounds, false, 0..0, |_, apps| {
                for i in 0..2 {
                    let w = apps[i].world_mut();
                    let own = w.resource::<Rollback>().local_seats();
                    let mut s = w.get_resource_or_init::<Smooth>().clone();
                    let n = w.resource::<Game>().players.len();
                    for (p, d) in s.players.iter().enumerate().take(n) {
                        let seat = if n == 4 { [0, 2, 1, 3][p] } else { p };
                        assert!(!own.contains(&seat) || *d == Vec3::ZERO, "machine {i}'s own player {p} moved by {d}");
                    }
                    let f = w.resource::<Rollback>().frame();
                    let pos = w.resource::<Game>().flight.ball.pos;
                    shown[i].entry(f).or_insert((pos, s.ball_at(pos)));
                    fade(&mut s, 1.0 / 60.0);
                    w.insert_resource(s);
                }
            });
            let log = apps[0].world().resource::<Netplay>().lock.log.clone();
            let n = log.len() - rollback::WINDOW;
            let mut a = app(&room, "replay");
            a.world_mut().remove_resource::<Netplay>();
            let mut truth = Vec::new();
            for (t, &p) in log[..n].iter().enumerate() {
                run_on(a.world_mut(), p, t.checked_sub(1).map_or([tick_input::IDLE; 4], |l| log[l]));
                a.world_mut().run_schedule(FixedUpdate);
                truth.push(Vec3::from(a.world().resource::<Game>().flight.ball.pos));
            }
            std::array::from_fn(|i| {
                [0, 1].map(|k| {
                    let (mut bad, mut runs, mut jumps, mut jump, mut snaps, mut prev) = (0, 0, 0, 0f32, 0, None::<(usize, Vec3, Vec3)>);
                    for (&f, (raw, smooth)) in &shown[i] {
                        let Some(&q) = f.checked_sub(1).and_then(|k| truth.get(k)) else { continue };
                        let p = Vec3::from(if k == 0 { *raw } else { *smooth });
                        let d = p.distance(q);
                        let off = d > 1e-3;
                        let was = prev.is_some_and(|(_, pp, pq)| pp.distance(pq) > 1e-3);
                        bad += off as u32;
                        runs += (off && !was) as u32;
                        if let Some((_, pp, pq)) = prev.filter(|(pf, _, _)| *pf + 1 == f) {
                            // past the ball's own move this frame: a correction showing (one past SNAP_BALL is a snap)
                            let j = ((p - pp) - (q - pq)).length();
                            if j >= super::super::super::smooth::SNAP_BALL {
                                snaps += 1;
                            } else {
                                jumps += (j > 0.25) as u32;
                                jump = jump.max(j);
                            }
                        }
                        prev = Some((f, p, q));
                    }
                    (bad, runs, jumps, jump, snaps)
                })
            })
        }

        /// B735's measure: `cargo test --release -p hst-play drawn_ball_corrections_at -- --ignored --nocapture`.
        #[test]
        #[ignore]
        fn drawn_ball_corrections_at_9_ticks() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let lag = std::env::var("EXP_LAG").map_or(9, |v| v.parse().unwrap());
            let rounds = std::env::var("EXP_ROUNDS").map_or(9000, |v| v.parse().unwrap());
            for (i, m) in drawn_ball_corrections(lag, rounds).iter().enumerate() {
                for (k, (bad, runs, jumps, jump, snaps)) in m.iter().enumerate() {
                    eprintln!("B735 lag {lag}: machine {i} {}: {bad} drawn frames off, {runs} warps, {jumps} frames jump over 25 cm (worst {jump:.3} m), {snaps} snaps", ["before", "smoothed"][k]);
                }
            }
        }

        /// B735: at 9 ticks each way the smoothed ball jumps (over 25 cm past its true move, short of a snap) on fewer
        /// frames and by less than the match's own, and neither machine's own players are ever moved (per frame).
        #[test]
        fn drawn_ball_corrections_glide() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            for (i, [before, smoothed]) in drawn_ball_corrections(9, 900).into_iter().enumerate() {
                eprintln!("machine {i}: before {before:?}, smoothed {smoothed:?}");
                assert!(smoothed.2 <= before.2 && smoothed.3 <= before.3, "machine {i}: smoothed {smoothed:?}, before {before:?}");
            }
        }

        /// A tick that runs differently on one machine: both see it at the next checksum, save their inputs, the
        /// guest takes the host's state, and the hashes from then on agree (B154b).
        #[test]
        fn rollback_desync_is_saved_and_play_goes_on() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            // ggrs compares the confirmed ticks' checksums: none differ after the resync (`reported`)
            let apps = play_rollback(5, 900, true);
            for a in &apps {
                let path = reported(a, 330);
                report_replays(&path);
                std::fs::remove_file(path).expect("saved");
            }
        }

        /// B494: the harness's outcome doesn't depend on how long its rounds really take (ggrs' pings, resends and
        /// time sync and `rollback`'s resend go by ggrs' clock, a tick a round): rounds slowed now and then (a loaded
        /// machine) play the forced desync's match the same, tick for tick.
        #[test]
        fn rollback_rounds_play_the_same_however_long_they_take() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let outcome = |slow: bool| {
                let apps = play_rollback_with(5, 420, true, 0..0, |r, _| {
                    if slow && r % 9 == 0 {
                        std::thread::sleep(std::time::Duration::from_millis(10 + (r as u64 * 7) % 40));
                    }
                });
                apps.each_ref().map(|a| {
                    let (np, rb) = (a.world().resource::<Netplay>(), a.world().resource::<Rollback>());
                    if let Some(at) = &np.report {
                        let _ = std::fs::remove_file(at);
                    }
                    (np.lock.log.clone(), (np.state.0, np.resync.fresh, np.resync.done, rb.frame(), rb.reruns, rb.skipped, rb.held, rb.resent))
                })
            };
            let (quick, slow) = (outcome(false), outcome(true));
            for (q, s) in quick.iter().zip(&slow) {
                assert_eq!(q.1, s.1, "state, fresh, resyncs, frame, re-runs, sat out, held, resent");
                let at = q.0.iter().zip(&s.0).position(|(a, b)| a != b);
                assert!(at.is_none() && q.0.len() == s.0.len(), "pads differ from tick {at:?} (ticks {} and {})", q.0.len(), s.0.len());
                assert_eq!(q.1.2, 1, "resyncs");
            }
        }

        /// B154b's acceptance: the forced desync's match, resynced, plays to the end in sync (~25 s optimised):
        /// `cargo test --release -p hst a_resynced_match_finishes -- --ignored`.
        #[test]
        #[ignore]
        fn a_resynced_match_finishes() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            REPLAYS.set(false);
            for a in &play_rollback(5, 400_000, true) {
                assert!(a.world().resource::<Game>().match_stats.result.is_some(), "the match didn't finish");
                std::fs::remove_file(reported(a, 330)).expect("saved");
            }
        }

        /// ggrs' `SyncTestSession`: every tick, back 7 ticks and those again, each tick's state hash compared with its
        /// first run's (it panics on a difference). The determinism check for rollback, in CI.
        #[test]
        fn synctest_reruns_every_tick_the_same() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            // into the first point's rally (the serve is around tick 200), mashing
            synctest(|t, _| t >= 420);
        }

        /// B154: re-runs through the first change of ends stay the same (its camera glide kept state outside the
        /// snapshot, so a rollback mid-glide came out differently and the match stopped on the Change Sides screen).
        /// ~40 s: `tools/check.sh -p hst synctest_through -- --ignored` after touching the snapshot or change of ends.
        #[test]
        #[ignore]
        fn synctest_through_a_change_of_ends() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            REPLAYS.set(false);
            let mut seen = false;
            synctest(|t, g| {
                assert!(t < 30_000, "no change of ends in {t} ticks");
                let now = matches!(g.phase, Phase::ChangeEnds(_));
                let done = seen && !now;
                seen |= now;
                done
            });
        }

        /// ggrs' synctest on the doubles room, all seats on one machine and mashed, until `stop(tick, game)`.
        fn synctest(mut stop: impl FnMut(u32, &Game) -> bool) {
            let mut room = room(0);
            room.seats.iter_mut().for_each(|s| s.as_mut().unwrap().peer = "host".into());
            let mut a = app(&room, "host");
            a.insert_resource(Rollback::new(&room, "host", step as fn(&mut _)).unwrap());
            let mut sess = ggrs::SessionBuilder::<rollback::Cfg>::new().with_num_players(4).unwrap().with_check_distance(7).with_input_delay(rollback::MAX_DELAY).start_synctest_session().unwrap();
            for t in 0.. {
                let w = a.world_mut();
                if stop(t, w.resource::<Game>()) {
                    break;
                }
                let mut np = w.resource_mut::<Netplay>();
                np.lock.tick = t;
                for s in 0..4 {
                    let p = np.mashed(s);
                    sess.add_local_input(s, rollback::Input(wire(p))).unwrap();
                }
                let reqs = sess.advance_frame().expect("the re-run ticks' hashes match the first run's");
                rollback::handle(w, reqs, 1);
            }
            assert!(a.world().resource::<Rollback>().reruns > 2000);
        }

        // ---------------------------------------------------------------- a peer gone silent (B196)

        use super::super::super::recover;

        /// Three peers (host 1P, `h` 2P and 4P, `g` 3P; `two`: host 1P 2P, `g` 3P 4P), rollback, every message 3 rounds (ticks) on its way; a peer
        /// not in `live` hears and does nothing (killed). `each(round, apps, live)` after every round may kill one or
        /// add one (a rejoiner).
        fn peers(two: bool, limit: u32, mut each: impl FnMut(u32, &mut Vec<App>, &mut Vec<bool>)) -> Vec<App> {
            let mut room = room(0);
            let names: &[&str] = if two { &["host", "g"] } else { &["host", "h", "g"] };
            if !two {
                room.seats[1].as_mut().unwrap().peer = "h".into();
                room.seats[3].as_mut().unwrap().peer = "h".into();
            }
            let mut apps: Vec<App> = names
                .iter()
                .map(|&me| {
                    let mut a = app(&room, me);
                    a.insert_resource(Rollback::new(&room, me, step as fn(&mut _)).unwrap());
                    a
                })
                .collect();
            let mut live = vec![true; names.len()];
            let mut wire: Vec<(String, u32, Event)> = Vec::new();
            let me = |a: &App| a.world().resource::<Netplay>().me.clone();
            for round in 0..limit {
                for (to, at, e) in std::mem::take(&mut wire) {
                    if at > round {
                        wire.push((to, at, e));
                    } else if let Some(i) = (0..apps.len()).find(|&i| live[i] && me(&apps[i]) == to) {
                        take(apps[i].world_mut(), e);
                    }
                }
                for i in (0..apps.len()).filter(|&i| live[i]) {
                    let w = apps[i].world_mut();
                    rollback::gate(w);
                    recover::tick(w, 1.0 / 60.0);
                    let from = w.resource::<Netplay>().me.clone();
                    wire.extend(rollback::outgoing(w).into_iter().map(|(to, d)| (to, round + 3, Event::Datagram(from.clone(), d))));
                    let out = std::mem::take(&mut w.resource_mut::<Netplay>().out);
                    let others: Vec<String> = apps.iter().map(me).filter(|p| *p != from).collect();
                    for m in out {
                        wire.extend(others.iter().map(|p| (p.clone(), round + 3, via(&from, p, m.clone()))));
                    }
                }
                each(round, &mut apps, &mut live);
            }
            apps
        }

        /// The notice the next frame would show (a frame of no time).
        fn notice(a: &mut App) -> String {
            recover::tick(a.world_mut(), 0.0)
        }

        // ---------------------------------------------------------------- crossplay (M171)

        /// M171: a US host and a guest on the Japanese disc with the host's data over it play the host's match in
        /// sync, in lockstep and with rollback; without the data they go apart (the control).
        #[test]
        fn a_japanese_disc_plays_a_us_hosts_match() {
            crossplay(false);
        }

        /// M171: the other way round: a Japanese host, a guest on the US disc, the Japanese rules on both.
        #[test]
        fn a_us_disc_plays_a_japanese_hosts_match() {
            crossplay(true);
        }

        fn crossplay(host_jp: bool) {
            use hst_data::{crossplay, digest, iso::Iso};
            use hst_sim::build::Build;
            let jp = hst_data::fixture::path("fixtures/discs/SCPS-15113.iso");
            if !hst_data::fixture::has_iso() || !jp.exists() {
                // the Japanese disc isn't the user's: skipped even under HST_REQUIRE_DISC
                return eprintln!("skipped: no ISO or fixtures/discs/SCPS-15113.iso");
            }
            let us = std::path::PathBuf::from(ISO);
            let (host, own) = if host_jp { (jp, us) } else { (us, jp) };
            // the guest's disc under a path of its own, so the host's data goes over it and nobody else's
            let dir = std::env::temp_dir().join(format!("hst-crossplay-{}-{host_jp}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            let link = dir.join("disc.iso");
            let _ = std::fs::remove_file(&link);
            std::os::unix::fs::symlink(std::fs::canonicalize(&own).unwrap(), &link).unwrap();
            let survey = |p: &Path| {
                let mut iso = Iso::open_own(p).unwrap();
                digest::survey(|f| iso.read_unlogged(f).ok()).unwrap()
            };
            let ((hd, _), (gd, gm)) = (survey(&host), survey(&link));
            assert_ne!(hd, gd, "the two discs play alike");
            let set = |g: &Path| DISCS.set(vec![("host".into(), host.to_string_lossy().into()), ("g".into(), g.to_string_lossy().into())]);
            let want = if host_jp { Build::Japan } else { Build::Us };
            let builds = |apps: &[App]| apps.iter().map(|a| a.world().resource::<Game>().build).collect::<Vec<_>>();
            REPLAYS.set(true);

            // the control: the guest's own disc, the host's match: they go apart
            set(&own);
            let apps = play_room(room(crate::net::delay_for(std::time::Duration::from_millis(200))), 6, 900, None);
            // (a resync hands the guest the host's state, its rules included, so only the desync report tells)
            assert!(apps.iter().any(|a| a.world().resource::<Netplay>().report.is_some()), "the discs played alike without the host's data");

            // the host's data, over the wire's encoding, laid over the guest's disc and checked against its digest
            let mut hiso = Iso::open_own(&host).unwrap();
            let patch = crossplay::patch_for(|f| hiso.read_unlogged(f).ok(), &gm).unwrap().encode();
            eprintln!("crossplay: {} KB of the host's data", patch.len() / 1024);
            crossplay::verify(&link, crossplay::Patch::decode(&patch).unwrap(), hd).unwrap();
            set(&link);

            // lockstep
            let apps = play_room(room(crate::net::delay_for(std::time::Duration::from_millis(200))), 6, 900, None);
            assert_eq!(builds(&apps), [want, want]);
            let [a, b] = apps.each_ref().map(|a| a.world().resource::<Netplay>());
            for np in [a, b] {
                assert_eq!((np.end.as_deref(), np.desynced, np.note.as_ref().map(|n| &n.0)), (None, false, None), "{}", np.me);
            }
            let n = a.lock.log.len().min(b.lock.log.len());
            assert!(n > 600, "{n} ticks");
            assert_eq!(a.lock.log[..n], b.lock.log[..n]);
            eprintln!("crossplay lockstep: in sync over {n} ticks, {:?}", builds(&apps));

            // rollback
            let apps = peers(true, 900, |_, _, _| {});
            assert_eq!(builds(&apps), [want, want]);
            let logs: Vec<_> = apps.iter().map(|a| a.world().resource::<Netplay>()).collect();
            let n = logs.iter().map(|np| np.lock.log.len()).min().unwrap() - rollback::WINDOW;
            assert!(n > 600, "only {n} ticks in step");
            for np in &logs {
                assert_eq!((np.end.as_deref(), np.desynced, np.note.as_ref().map(|n| &n.0)), (None, false, None), "{}", np.me);
                assert_eq!(np.lock.log[..n], logs[0].lock.log[..n], "{} ran other pads", np.me);
            }
            eprintln!("crossplay rollback: in step over {n} ticks");

            DISCS.set(Vec::new());
            assert!(crossplay::remove(&link));
            let _ = std::fs::remove_file(&link);
            let _ = std::fs::remove_dir(&dir);
        }

        fn press(a: &mut App, k: KeyCode) {
            let mut keys = ButtonInput::<KeyCode>::default();
            keys.press(k);
            a.insert_resource(keys);
        }

        /// The peers in `apps` ran the same pads from the restart on, ggrs' checksums agreeing, `ran` ticks or more.
        fn in_step(apps: &[&App], ran: usize) -> usize {
            let base = apps[0].world().resource::<Rollback>().base;
            let logs: Vec<_> = apps.iter().map(|a| a.world().resource::<Netplay>()).collect();
            let n = logs.iter().map(|np| np.lock.log.len()).min().unwrap() - rollback::WINDOW;
            assert!(n >= base + ran, "ran {} ticks after the restart at {base}", n - base);
            for (a, np) in apps.iter().zip(&logs) {
                assert_eq!(a.world().resource::<Rollback>().base, base);
                assert_eq!((np.end.as_deref(), np.desynced), (None, false), "{}", np.me);
                assert!(np.state.0 as usize >= n - 120, "{}: checksums stopped at {}", np.me, np.state.0);
                assert_eq!(np.lock.log[base..n], logs[0].lock.log[base..n], "{} ran other pads", np.me);
            }
            base
        }

        /// B197: rollback peers (`two`: host 1P 2P, `g` 3P 4P; else one seat each: host, `h`, `g`, `k`), every datagram 3
        /// rounds on its way, a round being a 60th of a second of real time; peer `i` runs the tick gate `calls(i, round)`
        /// times in it (Bevy's fixed loop: a 20 fps peer 3 times every third round, one hitching none for a while and
        /// then all it missed at once). Each peer's ticks run (`Rollback::frame`) after every round.
        fn paced(two: bool, limit: u32, calls: impl Fn(usize, u32) -> u32) -> (Vec<App>, Vec<Vec<usize>>) {
            let mut room = room(0);
            let names: &[&str] = if two { &["host", "g"] } else { &["host", "h", "g", "k"] };
            if !two {
                for (s, p) in [(1, "h"), (2, "g"), (3, "k")] {
                    room.seats[s].as_mut().unwrap().peer = p.into();
                }
            }
            let mut apps: Vec<App> = names.iter().map(|&me| {
                let mut a = app(&room, me);
                a.insert_resource(Rollback::new(&room, me, step as fn(&mut _)).unwrap());
                a
            }).collect();
            let mut ran = vec![Vec::new(); apps.len()];
            let mut wire: Vec<(usize, u32, String, Vec<u8>)> = Vec::new();
            for round in 0..limit {
                for (to, at, from, d) in std::mem::take(&mut wire) {
                    if at <= round {
                        take(apps[to].world_mut(), Event::Datagram(from, d));
                    } else {
                        wire.push((to, at, from, d));
                    }
                }
                for i in 0..apps.len() {
                    for _ in 0..calls(i, round) {
                        let w = apps[i].world_mut();
                        // a frozen peer's datagrams wait for it in the network's queue (`net.rs`), as they would
                        rollback::gate(w);
                        let me = w.resource::<Netplay>().me.clone();
                        for (to, d) in rollback::outgoing(w) {
                            let j = names.iter().position(|n| **n == to).unwrap();
                            wire.push((j, round + 3, me.clone(), d));
                        }
                        let out = std::mem::take(&mut w.resource_mut::<Netplay>().out);
                        for j in (0..names.len()).filter(|&j| j != i) {
                            for m in &out {
                                hear(apps[j].world_mut(), m.clone());
                            }
                        }
                    }
                    ran[i].push(apps[i].world().resource::<Rollback>().frame());
                }
            }
            (apps, ran)
        }

        /// B197: the slow peer's schedule: 20 fps (3 ticks every third 60th of a second) and, every 3 s, a 1 s hitch
        /// (nothing for 60 rounds, then 60 gate calls in one frame).
        fn slow(round: u32) -> u32 {
            match round % 180 {
                120..179 => 0,
                179 => 60,
                r => (r % 3 == 2) as u32 * 3,
            }
        }

        /// B197's acceptance: one slow peer (20 fps with 1 s hitches) leaves the others at full speed (60 ticks a
        /// second of real time, never two rounds without a tick, at most `SHIFT` behind real time) and every peer in
        /// sync; the slow one keeps up in real time too.
        fn a_slow_peer(two: bool) {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let limit = 900;
            let (apps, ran) = paced(two, limit, |i, r| if i == 1 { slow(r) } else { 1 });
            for (i, f) in ran.iter().enumerate() {
                // its session's start (ggrs' handshake) and first tick
                let start = f.iter().position(|&f| f > 0).unwrap();
                let behind = (limit as usize - start) as i32 - f[limit as usize - 1] as i32;
                let rb = apps[i].world().resource::<Rollback>();
                eprintln!("peer {i}: {behind} ticks behind real time, {} held, {} sat out, {} re-run, ahead {}", rb.held, rb.skipped, rb.reruns, rb.sess.frames_ahead());
                assert!(behind.abs() <= rollback::SHIFT + 3 + 60 * (i == 1) as i32, "peer {i} {behind} ticks behind real time");
                if i == 1 {
                    continue;
                }
                let idle = f[start + 30..].windows(3).filter(|w| w[0] == w[2]).count();
                assert_eq!(idle, 0, "peer {i} went two rounds without a tick");
            }
            let logs: Vec<_> = apps.iter().map(|a| a.world().resource::<Netplay>()).collect();
            let n = logs.iter().map(|np| np.lock.log.len()).min().unwrap() - rollback::WINDOW;
            assert!(n > 600, "only {n} ticks in step");
            for np in &logs {
                assert_eq!((np.end.as_deref(), np.desynced), (None, false), "{}", np.me);
                assert_eq!(np.lock.log[..n], logs[0].lock.log[..n], "{} ran other pads", np.me);
            }
        }

        #[test]
        fn a_slow_peer_leaves_the_other_at_full_speed() {
            a_slow_peer(true);
        }

        #[test]
        fn a_slow_peer_leaves_three_others_at_full_speed() {
            a_slow_peer(false);
        }

        /// B196: `g` killed mid-rally: everyone waits, nobody answers, the others are asked, the host chooses a bot
        /// (✕), which plays 3P from the same tick on both machines left, in lockstep.
        #[test]
        fn a_bot_takes_a_silent_peers_seat() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let (kill, s) = (600, |x: f32| (x * 60.0) as u32);
            let (mut said, mut asked) = (Vec::new(), Vec::new());
            let apps = peers(false, kill + s(recover::SILENT + recover::WAIT) + 1200, |round, apps, live| {
                if round == kill {
                    live[2] = false;
                }
                if round == kill + s(recover::SILENT) + 30 || round == kill + s(recover::SILENT + recover::WAIT) + 30 {
                    said.push([0, 1].map(|i| notice(&mut apps[i])));
                    asked.push([0, 1].map(|i| apps[i].world().resource::<Netplay>().recover.ask_me));
                }
                if round == kill + s(recover::SILENT + recover::WAIT) + 60 {
                    press(&mut apps[0], KeyCode::Enter);
                }
            });
            eprintln!("{said:#?}");
            assert!(said[0].iter().all(|t| t.starts_with("Waiting for g to come back or rejoin... 0:2")), "{:?}", said[0]);
            assert!(said[1].iter().all(|t| t.starts_with("g didn't come back.")), "{:?}", said[1]);
            // both machines left get the pills, while they wait too
            assert_eq!(asked, [[true; 2], [true; 2]]);
            let base = in_step(&[&apps[0], &apps[1]], 600);
            assert!(base > kill as usize - 30 && base <= kill as usize + rollback::WINDOW, "restarted at {base}");
            for a in &apps[..2] {
                assert_eq!(a.world().resource::<Pads>().seats, Some([true, true, false, true]));
                // B196: no desync after the restart (the instant replay's old copy brought the old seats back)
                assert_eq!(a.world().resource::<Netplay>().resync.done, 0, "resynced after the restart");
                assert_eq!(a.world().resource::<Netplay>().room.seats[2].as_ref().unwrap().peer, crate::net::CPU);
            }
        }

        /// B196, two peers: `g` killed, the host alone chooses a bot while it waits (no need to sit out the wait),
        /// which plays 3P and 4P on.
        #[test]
        fn a_bot_takes_the_only_guests_seats() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let (kill, s) = (600, |x: f32| (x * 60.0) as u32);
            let apps = peers(true, kill + s(recover::SILENT) + 120 + 800, |round, apps, live| {
                if round == kill {
                    live[1] = false;
                }
                if round == kill + s(recover::SILENT) + 120 {
                    press(&mut apps[0], KeyCode::Enter);
                }
            });
            in_step(&[&apps[0]], 600);
            assert_eq!(apps[0].world().resource::<Pads>().seats, Some([true, true, false, false]));
        }

        /// B162: peer `who` (of `host`, `h`, `g`) switches away at tick 600 for `away` seconds: its frames stop and its
        /// keeper says `Hidden` once a second; everyone else holds the match with "… switched away" (no bot question,
        /// nobody ends it), and back, it plays on in step with them. Returns the apps and the notices 20 s in.
        fn switched_away(who: usize, away: f32) -> (Vec<App>, Vec<String>) {
            let (gone, back, s) = (600, 600 + (away * 60.0) as u32, |x: f32| (x * 60.0) as u32);
            let peer = ["host", "h", "g"][who];
            let mut said = Vec::new();
            let apps = peers(false, back + 900, |round, apps, live| {
                if round == gone {
                    live[who] = false;
                }
                if round == back {
                    live[who] = true;
                }
                if (gone + 30..back).contains(&round) && (round - gone) % 60 == 30 {
                    for i in (0..3).filter(|&i| i != who) {
                        hear(apps[i].world_mut(), Msg::Hidden(peer.into()));
                    }
                }
                if round == gone + s(20.0) {
                    said = (0..3).filter(|&i| i != who).map(|i| notice(&mut apps[i])).collect();
                    for a in apps.iter() {
                        assert!(!a.world().resource::<Netplay>().recover.ask_me);
                    }
                }
            });
            for a in &apps {
                let np = a.world().resource::<Netplay>();
                assert_eq!(np.end, None, "{}", np.me);
                assert!(np.recover.hidden.is_empty() && np.recover.away.is_empty(), "{}", np.me);
            }
            in_step(&apps.iter().collect::<Vec<_>>(), 600);
            (apps, said)
        }

        /// B162: a guest tabbed out for 30 s (2 min: `a_player_switched_away_for_2_min_comes_back`).
        #[test]
        fn a_player_switched_away_comes_back() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let (mut apps, said) = switched_away(2, 30.0);
            assert!(said.iter().all(|t| t.starts_with("g switched away from the game.\nWaiting for them to come back... 4:4")), "{said:?}");
            assert_eq!(notice(&mut apps[0]), "");
        }

        /// B162: the host tabbed out for 30 s: the guests hold too (a silent host ends it after 13 s).
        #[test]
        fn the_host_switched_away_comes_back() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let (_, said) = switched_away(0, 30.0);
            assert!(said.iter().all(|t| t.starts_with("host switched away from the game.")), "{said:?}");
        }

        /// B162, the task's 2 minutes (~1 min; `tools/check.sh -p hst-play a_player_switched_away_for_2_min -- --ignored`
        /// after touching `recover.rs`' waits).
        #[test]
        #[ignore]
        fn a_player_switched_away_for_2_min_comes_back() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            switched_away(2, 120.0);
        }

        /// B162: a player switched away for [`recover::LONG`] (forced: its wait starts 2 s short of it) is away: the
        /// others wait [`recover::WAIT`] for it as for a silent one.
        #[test]
        fn a_player_switched_away_too_long_is_away() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let s = |x: f32| (x * 60.0) as u32;
            let mut said = Vec::new();
            peers(false, 610 + s(5.0), |round, apps, live| {
                if round == 600 {
                    live[2] = false;
                }
                // past its last keep-alive's way here
                if round == 610 {
                    for a in &mut apps[..2] {
                        hear(a.world_mut(), Msg::Hidden("g".into()));
                        *a.world_mut().resource_mut::<Netplay>().recover.hidden.get_mut("g").unwrap() = recover::LONG - 2.0;
                    }
                }
                if round > 600 && round % 60 == 0 {
                    for a in &mut apps[..2] {
                        hear(a.world_mut(), Msg::Hidden("g".into()));
                    }
                }
                if round == 610 + s(1.0) || round == 610 + s(4.0) {
                    said.push(notice(&mut apps[1]));
                }
            });
            assert!(said[0].starts_with("g switched away from the game.\nWaiting for them to come back... 0:0"), "{said:?}");
            assert!(said[1].starts_with("Waiting for g to come back or rejoin... "), "{said:?}");
        }

        /// B196: nobody wants a bot (○ on `h`): both machines left go back to the lobby together.
        #[test]
        fn a_silent_peer_can_end_the_match() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let (kill, s) = (600, |x: f32| (x * 60.0) as u32);
            let apps = peers(false, kill + s(recover::SILENT + recover::WAIT) + 200, |round, apps, live| {
                if round == kill {
                    live[2] = false;
                }
                if round == kill + s(recover::SILENT + recover::WAIT) + 60 {
                    press(&mut apps[1], KeyCode::Backspace);
                }
            });
            // everyone still in goes back to the lobby together, g's seat open again
            for a in &apps[..2] {
                let np = a.world().resource::<Netplay>();
                let end = np.end.clone().unwrap_or_default();
                assert!(end.starts_with("g didn't come back. Back to the lobby..."), "{end}");
                let lobby = np.recover.lobby.as_ref().expect("back to the lobby");
                let peers: Vec<_> = lobby.seats.iter().map(|s| s.as_ref().map(|s| s.peer.as_str())).collect();
                assert_eq!(peers, [Some("host"), Some("h"), None, Some("h")]);
                assert!(lobby.ready.is_empty() && !lobby.names.contains_key("g"));
            }
        }

        /// M197: `g` drops, the host takes everyone back to the lobby (the match saved as its point started), `g` comes
        /// back as `g2` and the host resumes: the host's restart from the save puts both machines at the save's state,
        /// and they play on from it in step.
        #[test]
        fn a_dropped_match_resumes_from_the_lobby() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let (kill, s) = (600, |x: f32| (x * 60.0) as u32);
            let mut apps = peers(true, kill + s(recover::SILENT) + 240, |round, apps, live| {
                if round == kill {
                    live[1] = false;
                }
                if round == kill + s(recover::SILENT) + 120 {
                    press(&mut apps[0], KeyCode::Backspace);
                }
            });
            let host = &mut apps[0];
            notice(host);
            let np = host.world().resource::<Netplay>();
            assert!(np.recover.lobby.is_some(), "back to the lobby: {:?}", np.end);
            let (save, saved) = np.recover.saved.clone().expect("the match saved going back to the lobby");
            assert_eq!(saved.seats, room(0).seats, "the line-up as the match had it");
            // M199: the save on disk keeps its lobby: the rest goes from the save as read back, as Continue's Open
            // lobby hosts it again
            let disk = super::super::super::save_game::MatchSave::from_bytes(&save.to_bytes()).unwrap();
            let saved = disk.head.setup.lobby.as_ref().map(super::super::super::save_game::Lobby::room).expect("its lobby kept");
            assert_eq!((&saved.seats, &saved.names, disk.state == save.state), (&room(0).seats, &np.room.names, true));
            let save = std::sync::Arc::new(disk);
            // the save's state, as a fresh match takes it
            let mut room = saved.clone();
            room.rejoined("g", "g2", "g");
            room.resume = Some(super::super::offer(&save, &saved));
            let mut fresh = app(&room, "host");
            snapshot::decode(fresh.world_mut(), &save.state).unwrap();
            let want = snapshot::fnv(&snapshot::state_text(fresh.world_mut()));
            // the lobby's Start: the host with the save, g back as g2, both held for the restart
            let mut apps: Vec<App> = ["host", "g2"]
                .iter()
                .map(|&me| {
                    let mut a = app(&room, me);
                    a.insert_resource(Rollback::new(&room, me, step as fn(&mut _)).unwrap());
                    assert!(a.world().resource::<Netplay>().recover.hold, "{me} waits for the restart");
                    a
                })
                .collect();
            apps[0].world_mut().resource_mut::<Netplay>().recover.resume = Some(save.clone());
            let mut wire: Vec<(String, u32, Event)> = Vec::new();
            let me = |a: &App| a.world().resource::<Netplay>().me.clone();
            let mut got = [None; 2];
            for round in 0..900u32 {
                for (to, at, e) in std::mem::take(&mut wire) {
                    if at > round {
                        wire.push((to, at, e));
                    } else if let Some(i) = (0..2).find(|&i| me(&apps[i]) == to) {
                        take(apps[i].world_mut(), e);
                    }
                }
                for i in 0..2 {
                    let w = apps[i].world_mut();
                    rollback::gate(w);
                    recover::tick(w, 1.0 / 60.0);
                    // the restart just taken on: this machine's state then
                    if got[i].is_none() && !w.resource::<Netplay>().recover.hold {
                        got[i] = Some(snapshot::fnv(&snapshot::state_text(w)));
                    }
                    let from = w.resource::<Netplay>().me.clone();
                    wire.extend(rollback::outgoing(w).into_iter().map(|(to, d)| (to, round + 3, Event::Datagram(from.clone(), d))));
                    let out = std::mem::take(&mut w.resource_mut::<Netplay>().out);
                    let to = if from == "host" { "g2" } else { "host" };
                    wire.extend(out.into_iter().map(|m| (to.to_string(), round + 3, via(&from, to, m))));
                }
            }
            assert_eq!(got, [Some(want); 2], "both machines start from the host's save");
            for a in &apps {
                let np = a.world().resource::<Netplay>();
                assert!(np.room.resume.is_none(), "{}: taken on once", np.me);
                let g = &a.world().resource::<Game>().score;
                assert_eq!((g.sets, g.games), (save.head.summary.sets, save.head.summary.games), "{}", np.me);
            }
            in_step(&[&apps[0], &apps[1]], 300);
        }

        /// B279: the match over, nobody waits for anybody on the result screen (the host gone quiet there is it in the
        /// lobby); leaving it, the host takes everyone to the same lobby, nobody ready, and a guest goes to the lobby
        /// the host sent, or ahead of it to this one, waiting.
        #[test]
        fn the_result_screen_leads_back_to_the_lobby() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let over = 120;
            let mut apps = peers(true, over + ((recover::SILENT + 12.0) * 60.0) as u32, |round, apps, live| {
                if round == over {
                    for a in apps.iter_mut() {
                        let mut g = a.world_mut().resource_mut::<Game>();
                        g.match_stats.result = Some(g.match_stats.finish(0));
                    }
                    live[0] = false;
                }
            });
            let g = &mut apps[1];
            assert_eq!((notice(g), g.world().resource::<Netplay>().end.clone()), (String::new(), None));
            let mut room = g.world().resource::<Netplay>().room.clone();
            room.ready.insert("g".into());
            g.world_mut().resource_mut::<Netplay>().room = room.clone();
            let leave = |a: &mut App| {
                let (net, rx) = crate::net::Net::wired();
                a.insert_resource(net);
                a.world_mut().write_message(AppExit::Success);
                super::super::again(a.world_mut());
                assert!(!a.world().contains_resource::<crate::net::Net>());
                (super::super::LOBBY.lock().unwrap().take().map(|(_, r, wait)| (r, wait)).expect("a lobby"), rx)
            };
            // the guest ahead of the host: this lobby, nobody ready, waiting
            let ((r, wait), _) = leave(g);
            assert!(wait && r.ready.is_empty() && r.seats == room.seats && r.court == room.court);
            // the host: everyone told
            let ((r, wait), mut rx) = leave(&mut apps[0]);
            assert!(!wait && r.ready.is_empty() && r.seats == room.seats);
            let Ok(crate::net::Cmd::Send(Msg::Lobby(sent))) = rx.try_recv() else { panic!("no Lobby sent") };
            assert_eq!(sent.ver, r.ver);
            // the guest still on the results when the host's lobby comes: that one once it leaves too
            let g = &mut apps[1];
            hear(g.world_mut(), Msg::Lobby(sent.clone()));
            assert_eq!(g.world().resource::<Netplay>().end, None);
            let ((r, wait), _) = leave(g);
            assert!(!wait && r.ver == sent.ver);
        }

        /// B196: `g` killed, then a new process for it joins (the relay gives it `g`'s seat as `g2`): the host restarts
        /// the match with it in, and all three play on from the same tick in lockstep. And the host killed: the
        /// guests end the match. ~25 s: `tools/check.sh -p hst a_silent_peer_rejoins -- --ignored` after touching
        /// `recover.rs` or the rejoin flow.
        #[test]
        #[ignore]
        fn a_silent_peer_rejoins_and_a_silent_host_ends_it() {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let (kill, back) = (600, 900);
            let mut room = None;
            let mut base = 0;
            let apps = peers(false, back + 1300 + 1200, |round, apps, live| {
                if round == kill {
                    live[2] = false;
                    room = Some(apps[0].world().resource::<Netplay>().room.clone());
                }
                if round == back {
                    let mut r = room.clone().unwrap();
                    r.rejoined("g", "g2", "g");
                    r.rejoin = true;
                    let mut a = app(&r, "g2");
                    a.insert_resource(Rollback::new(&r, "g2", step as fn(&mut _)).unwrap());
                    apps.push(a);
                    live.push(true);
                    take(apps[0].world_mut(), Event::Match("host".into(), Msg::Rejoined { old: "g".into(), new: "g2".into(), name: "g".into() }));
                }
                if round == back + 1300 {
                    base = in_step(&[&apps[0], &apps[1], &apps[3]], 600);
                    assert!(base > kill as usize - 30 && base <= kill as usize + rollback::WINDOW, "restarted at {base}");
                    live[0] = false;
                }
            });
            for a in [&apps[1], &apps[3]] {
                let end = a.world().resource::<Netplay>().end.clone().unwrap_or_default();
                assert!(end.starts_with("The host stopped answering. The match is over.\nScore"), "{end}");
            }
        }

        /// B198: rollback peers (as `paced`, one gate call a round) over a bad network: each datagram lost with
        /// probability `loss`, the rest `lag` rounds on their way (the match's messages too, never lost: a stream),
        /// one bigger than [`crate::net::DATAGRAM_MAX`] lost always (as iroh's), and nothing at all through the
        /// rounds in `outage`. A round in which no peer runs a tick takes a 60th of a second of real time (ggrs
        /// resends on its clock). Each peer's ticks run after every round, and the biggest datagram sent.
        fn lossy(two: bool, loss: f64, lag: u32, down: u32, oneway: bool) -> (Vec<App>, Vec<Vec<usize>>, usize, std::ops::Range<u32>) {
            let mut room = room(0);
            let names: &[&str] = if two { &["host", "g"] } else { &["host", "h", "g", "k"] };
            if !two {
                for (s, p) in [(1, "h"), (2, "g"), (3, "k")] {
                    room.seats[s].as_mut().unwrap().peer = p.into();
                }
            }
            let mut apps: Vec<App> = names.iter().map(|&me| {
                let mut a = app(&room, me);
                a.insert_resource(Rollback::new(&room, me, step as fn(&mut _)).unwrap());
                a
            }).collect();
            let mut ran = vec![Vec::new(); apps.len()];
            let (mut wire, mut big) = (Vec::<(usize, u32, Event)>::new(), 0);
            let mut rng = 0x2545_f491_4f6c_dd1du64;
            let mut lost = move || {
                rng ^= rng << 13;
                rng ^= rng >> 7;
                rng ^= rng << 17;
                (rng >> 11) as f64 / (1u64 << 53) as f64 <= loss
            };
            let mut outage = u32::MAX..u32::MAX;
            for round in 0.. {
                // the outage 2 s after every session is running (ggrs' handshake, over the lossy network, first)
                if outage.start == u32::MAX && apps.iter().all(|a| a.world().resource::<Rollback>().sess.current_state() == ggrs::SessionState::Running) {
                    outage = round + 120..round + 120 + down;
                }
                if round == outage.end.saturating_add(300) {
                    break;
                }
                let t0 = std::time::Instant::now();
                for (to, at, e) in std::mem::take(&mut wire) {
                    if at <= round {
                        take(apps[to].world_mut(), e);
                    } else {
                        wire.push((to, at, e));
                    }
                }
                let before: Vec<usize> = apps.iter().map(|a| a.world().resource::<Rollback>().frame()).collect();
                for i in 0..apps.len() {
                    let w = apps[i].world_mut();
                    rollback::gate(w);
                    let me = w.resource::<Netplay>().me.clone();
                    for (to, d) in rollback::outgoing(w) {
                        big = big.max(d.len());
                        if (outage.contains(&round) && (!oneway || i == 0)) || lost() || !crate::net::fits(&d) {
                            continue;
                        }
                        let j = names.iter().position(|n| **n == to).unwrap();
                        wire.push((j, round + lag, Event::Datagram(me.clone(), d)));
                    }
                    let out = std::mem::take(&mut w.resource_mut::<Netplay>().out);
                    for j in (0..names.len()).filter(|&j| j != i) {
                        wire.extend(out.iter().map(|m| (j, round + lag, via(&me, names[j], m.clone()))));
                    }
                    ran[i].push(apps[i].world().resource::<Rollback>().frame());
                }
                if apps.iter().zip(&before).all(|(a, &b)| a.world().resource::<Rollback>().frame() == b) {
                    std::thread::sleep(std::time::Duration::from_micros(16_667).saturating_sub(t0.elapsed()));
                }
            }
            (apps, ran, big, outage)
        }

        /// B198's acceptance: 2 or 4 peers, 20 % of datagrams lost and 150 ms round trip, then for 3 s nothing gets
        /// through (`oneway`: nothing from the host): every peer's ticks go on within a second of the network
        /// coming back (no mutual wait), and between, nobody waits a second; all in sync, no datagram too big.
        fn on_a_bad_network(two: bool, oneway: bool) {
            if !hst_data::fixture::has_iso() {
                return;
            }
            let (apps, ran, big, outage) = lossy(two, 0.2, 5, 180, oneway);
            for (i, f) in ran.iter().enumerate() {
                // the longest run of rounds without a tick once every session is running, outside the outage and
                // its first second after
                let (mut still, mut worst) = (0, 0);
                for r in outage.start as usize - 120..f.len() {
                    still = if f[r] == f[r - 1] { still + 1 } else { 0 };
                    if !(outage.start..outage.end + 60).contains(&(r as u32)) {
                        worst = worst.max(still);
                    }
                }
                let rb = apps[i].world().resource::<Rollback>();
                eprintln!("peer {i}: {} ticks, longest wait {worst} rounds, {} held, {} re-run, biggest datagram {big} bytes", f[f.len() - 1], rb.held, rb.reruns);
                assert!(worst < 60, "peer {i} waited {worst} rounds");
            }
            let logs: Vec<_> = apps.iter().map(|a| a.world().resource::<Netplay>()).collect();
            let n = logs.iter().map(|np| np.lock.log.len()).min().unwrap() - rollback::WINDOW;
            assert!(n > 400, "only {n} ticks in step");
            for np in &logs {
                assert_eq!((np.end.as_deref(), np.desynced), (None, false), "{}", np.me);
                assert_eq!(np.lock.log[..n], logs[0].lock.log[..n], "{} ran other pads", np.me);
            }
        }

        #[test]
        fn two_peers_on_a_bad_network() {
            on_a_bad_network(true, false);
        }

        #[test]
        fn four_peers_on_a_bad_network() {
            on_a_bad_network(false, false);
        }

        /// Only the host's datagrams lost for 3 s: its unacked inputs pile up past ggrs' 128.
        #[test]
        fn a_one_way_outage() {
            on_a_bad_network(true, true);
        }

        /// What a tick costs (the match tick, the snapshot ggrs keeps, a state hash): sets `rollback::WINDOW`.
        /// `cargo test --release -p hst rollback_tick_cost -- --ignored --nocapture`
        #[test]
        #[ignore]
        fn rollback_tick_cost() {
            let room = room(0);
            let mut a = app(&room, "host");
            let (mut tick, mut save, mut hash) = (Vec::new(), Vec::new(), Vec::new());
            for t in 0..3000u32 {
                let w = a.world_mut();
                let mut np = w.resource_mut::<Netplay>();
                np.lock.tick = t;
                let p = [0, 1, 2, 3].map(|s| np.mashed(s));
                w.insert_resource(tick_input::Replay([p].into()));
                let c = std::time::Instant::now();
                w.run_schedule(FixedUpdate);
                tick.push(c.elapsed());
                let c = std::time::Instant::now();
                let s = super::super::super::snapshot::save(w);
                save.push(c.elapsed());
                let c = std::time::Instant::now();
                super::super::super::snapshot::state_hash(w);
                hash.push(c.elapsed());
                drop(s);
            }
            for (what, mut v) in [("tick", tick), ("save", save), ("hash", hash)] {
                v.sort();
                let mean = v.iter().sum::<std::time::Duration>() / v.len() as u32;
                println!("{what}: mean {mean:?}, median {:?}, 99% {:?}, max {:?}", v[v.len() / 2], v[v.len() * 99 / 100], v[v.len() - 1]);
            }
        }
    }
}

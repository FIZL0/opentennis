//! M126: the online match this machine plays in as a guest, kept so a browser refresh or a crash can rejoin it: the
//! lobby's code and this player's first seat, written when the match starts (`main_menu/online.rs` `launch`), read once
//! by a fresh menu (it goes straight to joining that code, as a lobby link does) and gone once the match is over. In
//! the browser it's the tab's sessionStorage (a reload keeps it, a closed tab doesn't); natively `rejoin.txt` beside
//! settings.txt. A record older than [`FRESH`] is ignored: the match has long given the seat to a bot or ended.
//! The host can't rejoin (there's no new host: `play/recover.rs`), so it never writes one.

/// How long after it was written a record still rejoins (the others wait ~2 minutes for a silent player, 5 for one
/// that switched away).
pub const FRESH: f64 = 600.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Rejoin {
    /// The code the match's lobby was joined with (a short code or a ticket: `net::code_in`'s).
    pub code: String,
    /// This player's first seat (0–3) in the lobby's line-up.
    pub seat: u8,
}

const KEY: &str = "hst-rejoin";

/// `code seat seconds` in one line.
fn encode(r: &Rejoin, at: f64) -> String {
    match ticket_for(&r.code) {
        Some(t) => format!("{} {} {} {t}", r.code, r.seat, at as u64),
        None => format!("{} {} {}", r.code, r.seat, at as u64),
    }
}

/// B643: the short code this session last looked up and the host's ticket it led to. The host stops publishing its
/// short code once the match starts, so a rejoin more than a couple of minutes in can't look it up again: the record
/// keeps the ticket after the code (an optional fourth field) and the rejoin dials that ([`dial`]).
static RESOLVED: std::sync::Mutex<Option<(String, String)>> = std::sync::Mutex::new(None);
/// B643: the ticket a rejoin taken from the record dials instead of looking its short code up, once.
static DIAL: std::sync::Mutex<Option<(String, String)>> = std::sync::Mutex::new(None);

/// B643: the short code `code` was looked up and led to the host's `ticket` (`net.rs` join).
pub fn resolved(code: &str, ticket: &str) {
    *RESOLVED.lock().unwrap_or_else(|p| p.into_inner()) = Some((code.into(), ticket.into()));
}

fn ticket_for(code: &str) -> Option<String> {
    RESOLVED.lock().unwrap_or_else(|p| p.into_inner()).as_ref().filter(|(c, _)| c == code).map(|(_, t)| t.clone())
}

/// B643: the host's ticket to dial for the short code `code` instead of looking it up, if a rejoin just took a record
/// for it (once).
pub fn dial(code: &str) -> Option<String> {
    let mut d = DIAL.lock().unwrap_or_else(|p| p.into_inner());
    d.take_if(|(c, _)| c == code).map(|(_, t)| t)
}

fn decode(text: &str, now: f64) -> Option<Rejoin> {
    let mut f = text.split_whitespace();
    let (code, seat, at) = (f.next()?, f.next()?.parse::<u8>().ok()?, f.next()?.parse::<f64>().ok()?);
    (seat < 4 && !code.is_empty() && (0.0..FRESH).contains(&(now - at))).then(|| Rejoin { code: code.into(), seat })
}

/// Remember `r` (the menu's `iso`: natively the record is beside its settings).
pub fn save(iso: &str, r: &Rejoin) {
    write(iso, Some(&encode(r, crate::unix_secs() as f64)));
}

/// The match to rejoin, if one was saved lately; read once (it's forgotten here, and saved again when the rejoin
/// starts).
pub fn take(iso: &str) -> Option<Rejoin> {
    let text = read(iso);
    let r = text.as_deref().and_then(|t| decode(t, crate::unix_secs() as f64));
    forget(iso);
    // B643: the host's ticket kept with it, for the rejoin to dial and the next record to keep
    if let (Some(r), Some(t)) = (&r, text.as_deref().and_then(|t| t.split_whitespace().nth(3))) {
        resolved(&r.code, t);
        *DIAL.lock().unwrap_or_else(|p| p.into_inner()) = Some((r.code.clone(), t.into()));
    }
    r
}

/// B643: how often [`keep`] writes the record again while the match goes on.
pub const KEEP: u64 = 60;

/// B643: the match still going on, so a crash late in a long one still rejoins: the record (if this machine has one)
/// written again with the time now, at most every [`KEEP`] seconds. Without it a record went stale [`FRESH`] after
/// the match started, and a refresh 10 minutes in came back to the menu instead.
pub fn keep(iso: &str) {
    static LAST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let now = crate::unix_secs();
    if now.saturating_sub(LAST.load(std::sync::atomic::Ordering::Relaxed)) >= KEEP {
        LAST.store(now, std::sync::atomic::Ordering::Relaxed);
        keep_at(iso, now as f64);
    }
}

fn keep_at(iso: &str, now: f64) {
    if let Some(r) = read(iso).and_then(|t| decode(&t, now)) {
        write(iso, Some(&encode(&r, now)));
    }
}

/// The match is over (or was never this machine's to rejoin).
pub fn forget(iso: &str) {
    write(iso, None);
}

#[cfg(target_arch = "wasm32")]
fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.session_storage().ok()?
}

#[cfg(target_arch = "wasm32")]
fn read(_iso: &str) -> Option<String> {
    storage()?.get_item(KEY).ok()?
}

#[cfg(target_arch = "wasm32")]
fn write(_iso: &str, text: Option<&str>) {
    if let Some(s) = storage() {
        let _ = match text {
            Some(t) => s.set_item(KEY, t),
            None => s.remove_item(KEY),
        };
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn path(iso: &str) -> std::path::PathBuf {
    crate::store::settings(iso).with_file_name(format!("{}.txt", KEY.trim_start_matches("hst-")))
}

#[cfg(not(target_arch = "wasm32"))]
fn read(iso: &str) -> Option<String> {
    std::fs::read_to_string(path(iso)).ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn write(iso: &str, text: Option<&str>) {
    let p = path(iso);
    let _ = match text {
        Some(t) => std::fs::write(p, t),
        None if p.exists() => std::fs::remove_file(p),
        None => Ok(()),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_rejoins_while_fresh() {
        let r = Rejoin { code: "AB3Q".into(), seat: 2 };
        let t = encode(&r, 1000.0);
        assert_eq!(decode(&t, 1000.0), Some(r.clone()));
        assert_eq!(decode(&t, 1000.0 + FRESH - 1.0), Some(r));
        assert_eq!(decode(&t, 1000.0 + FRESH), None, "too old");
        assert_eq!(decode(&t, 900.0), None, "from the future (a clock change)");
        assert_eq!(decode("AB3Q 7 1000", 1000.0), None, "no such seat");
        assert_eq!(decode("", 1000.0), None);
    }

    #[test]
    fn a_record_is_read_once() {
        let dir = std::env::temp_dir().join(format!("hst-rejoin-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let iso = dir.join("hst.iso").display().to_string();
        let r = Rejoin { code: "endpointabc".into(), seat: 1 };
        save(&iso, &r);
        assert_eq!(take(&iso), Some(r.clone()));
        assert_eq!(take(&iso), None, "taken");
        save(&iso, &r);
        forget(&iso);
        assert_eq!(take(&iso), None, "forgotten");
        let _ = std::fs::remove_dir(&dir);
    }

    /// B643: a record kept up while the match goes on still rejoins long after the match started; a stale or
    /// forgotten one isn't brought back.
    #[test]
    fn a_kept_record_outlives_the_match_start() {
        let dir = std::env::temp_dir().join(format!("hst-rejoin-keep-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let iso = dir.join("hst.iso").display().to_string();
        let r = Rejoin { code: "AB3Q".into(), seat: 1 };
        write(&iso, Some(&encode(&r, 1000.0)));
        let mut now = 1000.0;
        while now < 1000.0 + 3.0 * FRESH {
            now += KEEP as f64;
            keep_at(&iso, now);
        }
        assert_eq!(read(&iso).and_then(|t| decode(&t, now + FRESH - 1.0)), Some(r.clone()), "kept fresh");
        keep_at(&iso, now + FRESH);
        assert_eq!(read(&iso).and_then(|t| decode(&t, now + FRESH)), None, "stale stays stale");
        forget(&iso);
        keep_at(&iso, now);
        assert_eq!(read(&iso), None, "forgotten stays forgotten");
        let _ = std::fs::remove_dir(&dir);
    }

    /// B643: a short code's ticket rides along in the record (kept fresh with it) and the rejoin dials it, once.
    #[test]
    fn a_rejoin_dials_the_ticket_its_code_led_to() {
        let dir = std::env::temp_dir().join(format!("hst-rejoin-ticket-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let iso = dir.join("hst.iso").display().to_string();
        let r = Rejoin { code: "TK7W".into(), seat: 1 };
        resolved("TK7W", "endpointticket");
        let now = crate::unix_secs() as f64;
        write(&iso, Some(&encode(&r, now - 30.0)));
        keep_at(&iso, now);
        assert_eq!(read(&iso).as_deref(), Some(format!("TK7W 1 {} endpointticket", now as u64).as_str()));
        *RESOLVED.lock().unwrap() = None;
        assert_eq!(dial("TK7W"), None, "no rejoin yet");
        assert_eq!(take(&iso), Some(r));
        assert_eq!(dial("OTHR"), None);
        assert_eq!(dial("TK7W").as_deref(), Some("endpointticket"));
        assert_eq!(dial("TK7W"), None, "once");
        // the rejoined match's record keeps it again
        assert!(encode(&Rejoin { code: "TK7W".into(), seat: 1 }, now).ends_with(" endpointticket"));
        forget(&iso);
        let _ = std::fs::remove_dir(&dir);
    }
}

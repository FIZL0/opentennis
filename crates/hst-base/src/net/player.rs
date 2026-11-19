//! M197: this player's own id, the same from one connection to the next: a host holding an unfinished match's seats
//! knows the player who comes back with a new connection (a new peer id every launch) as the one who dropped. It's a
//! random token, nothing about the player: in the browser the tab's sessionStorage (a reload keeps it, the same as a
//! rejoin's record, `rejoin.rs`), natively `player.txt` beside settings.txt. With neither it's this run's own.

const KEY: &str = "hst-player";

static ID: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// This player's id (the menu's `iso`: natively the file is beside its settings), made and kept the first time.
pub fn load(iso: &str) -> String {
    let mut id = ID.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(i) = id.as_ref() {
        return i.clone();
    }
    let i = read(iso).map(|t| t.trim().to_string()).filter(|t| valid(t)).unwrap_or_else(|| {
        let i = fresh();
        write(iso, &i);
        i
    });
    *id = Some(i.clone());
    i
}

/// The id loaded (`load`), if any: what a join's Hello carries.
pub fn id() -> Option<String> {
    ID.lock().unwrap_or_else(|p| p.into_inner()).clone()
}

fn valid(t: &str) -> bool {
    t.len() == 32 && t.bytes().all(|b| b.is_ascii_hexdigit())
}

fn fresh() -> String {
    iroh::SecretKey::generate().to_bytes()[..16].iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(target_arch = "wasm32")]
fn read(_iso: &str) -> Option<String> {
    web_sys::window()?.session_storage().ok()??.get_item(KEY).ok()?
}

#[cfg(target_arch = "wasm32")]
fn write(_iso: &str, text: &str) {
    if let Some(s) = web_sys::window().and_then(|w| w.session_storage().ok().flatten()) {
        let _ = s.set_item(KEY, text);
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
fn write(iso: &str, text: &str) {
    let _ = std::fs::write(path(iso), text);
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_fresh_id_is_valid() {
        let (a, b) = (super::fresh(), super::fresh());
        assert!(super::valid(&a) && super::valid(&b) && a != b);
        assert!(!super::valid("") && !super::valid("not an id"));
    }
}

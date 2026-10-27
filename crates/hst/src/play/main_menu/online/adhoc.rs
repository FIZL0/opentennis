//! M152a: Online's Adhoc row: ✕ starts or stops serving the game to browsers on this network (`crate::adhoc`); its
//! card shows the address to open and a QR code of it. Leaving Online stops it (it hands out the disc and mods). Native
//! builds only: a browser can't serve.

use super::super::{Draw, Tex, WHITE};

/// Adhoc's index in `ROWS` (after Back, which has no pill, so the other rows keep theirs).
pub(super) const ROW: usize = 7;
/// Whether the row is there.
pub(super) const SHOWN: bool = cfg!(not(target_arch = "wasm32"));

/// The row to step from and draw as chosen: Back (or anything past the pills) is Relay, as before; Adhoc is itself.
pub(super) fn row(sel: usize) -> usize {
    if SHOWN && sel == ROW { ROW } else { sel.min(5) }
}

/// Whether the server is running.
pub(super) fn on() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    return crate::adhoc::on();
    #[cfg(target_arch = "wasm32")]
    false
}

#[cfg(not(target_arch = "wasm32"))]
static ERROR: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Starts (serving the disc at `iso`) or stops the server; one that can't start says why on the card.
pub(super) fn set(on: bool, iso: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let r = crate::adhoc::set(on, std::path::Path::new(iso));
        *ERROR.lock().unwrap() = r.err();
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (on, iso);
}

/// The address the QR code gives: the server's first (Wi-Fi first), while it runs.
fn url() -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    return crate::adhoc::status().and_then(|s| s.urls.first().cloned());
    #[cfg(target_arch = "wasm32")]
    None
}

/// The card's words: what Adhoc does while it's off; the addresses to open (and what's still missing) while it's on.
pub(super) fn words() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(e) = ERROR.lock().unwrap().clone() {
            return e;
        }
        if let Some(s) = crate::adhoc::status() {
            if s.urls.is_empty() {
                return "Not on a network: join a Wi-Fi network or turn on this device's hotspot.".into();
            }
            // M152e: a firewall hint takes the room of the longer wording
            let mut w = if s.hint.is_empty() {
                format!("Open {} in a browser on the same Wi-Fi or hotspot (or scan the code).", s.urls.join(" or "))
            } else {
                format!("Open {} on the same Wi-Fi or hotspot.", s.urls.join(" or "))
            };
            for more in [&s.note, &s.hint] {
                if !more.is_empty() {
                    w += &format!(" {more}");
                }
            }
            return w;
        }
    }
    "Play with phones and computers on this network, no install: they open this device's address in a browser. Shares your disc and mods with them while it's on.".into()
}

/// The card's picture `[x, y, w, h]`: the QR code on its white square at the right while the server runs, "Off" else.
pub(super) fn picture(d: &mut Draw, [x, y, w, h]: [f32; 4]) {
    let Some((n, dark)) = url().and_then(|u| qr(&u)) else {
        let s = if on() { "..." } else { "Off" };
        d.outlined(|d| d.centred(s, x + w / 2.0, y + 50.0, 40.0, WHITE));
        return;
    };
    let side = h - 16.0;
    let (sx, sy) = (x + w - side - 12.0, y + 8.0);
    let m = side / (n + 8) as f32;
    d.q(Tex::Solid, [0.0; 4], [sx, sy, side, side], WHITE, 128.0);
    // each row's dark runs one quad, a little over their size so no seam shows between rows
    for r in 0..n {
        let mut c = 0;
        while c < n {
            if !dark[r * n + c] {
                c += 1;
                continue;
            }
            let start = c;
            while c < n && dark[r * n + c] {
                c += 1;
            }
            d.q(Tex::Solid, [0.0; 4], [sx + (start + 4) as f32 * m, sy + (r + 4) as f32 * m, (c - start) as f32 * m + 0.3, m + 0.3], [0.0; 3], 128.0);
        }
    }
}

fn qr(url: &str) -> Option<(usize, Vec<bool>)> {
    #[cfg(not(target_arch = "wasm32"))]
    return crate::adhoc::qr(url);
    #[cfg(target_arch = "wasm32")]
    {
        let _ = url;
        None
    }
}

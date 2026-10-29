//! The phones' start page on the desktop, to look at it: `cargo run -p hst-base --example start_page [W H]`
//! (the picker is a stand-in that closes at once).
use bevy::prelude::*;

fn main() {
    let a: Vec<u32> = std::env::args().skip(1).filter_map(|a| a.parse().ok()).collect();
    let (w, h) = if a.len() == 2 { (a[0], a[1]) } else { (1280, 720) };
    let window = Window { title: "Hot Shots Tennis (shot)".into(), resolution: (w, h).into(), ..default() };
    hst_base::start_page::run(
        DefaultPlugins.set(WindowPlugin { primary_window: Some(window), ..default() }),
        std::env::temp_dir().join("hst-start-page.pack"),
        "This remaster plays from your own copy of the original game. Choose its disc image (.iso) in Files: iCloud Drive, On My iPhone or a USB drive. The game copies the part it needs, then you can delete the ISO.",
        "No file picker? Put the ISO in the Files app under On My iPhone > OpenTennis (the folder with this app's icon), then close and reopen the app: it finds it there.",
    )
}

//! Desktop macOS and Linux: a launch nobody watches (from the Finder, Dock or an app menu, where stdout and stderr go
//! to /dev/null or the journal) logs into `hst-log.txt` beside settings.txt, the launch before's kept as
//! `hst-log-previous.txt` (reopening after a crash would otherwise overwrite the crash's log), with the frame-rate log
//! on: as the phones do (`android.rs`, `ios.rs`). A terminal, pipe or file on stderr keeps it.

use bevy::log::{BoxedFmtLayer, tracing_subscriber};
use bevy::prelude::*;
use std::os::fd::AsRawFd;

/// stdout and stderr into `dir/hst-log.txt` when nobody reads them. Call before the app's threads start.
pub fn unwatched_to(dir: &std::path::Path) {
    // SAFETY: isatty/fstat on fd 2 and dup2 of a file this function opened, before any thread of ours writes
    unsafe {
        let mut st: libc::stat = std::mem::zeroed();
        let kind = if libc::fstat(2, &mut st) == 0 { st.st_mode & libc::S_IFMT } else { 0 };
        if libc::isatty(2) == 1 || kind == libc::S_IFIFO || kind == libc::S_IFREG {
            return;
        }
        let path = dir.join("hst-log.txt");
        let _ = std::fs::rename(&path, dir.join("hst-log-previous.txt"));
        let Ok(f) = std::fs::File::create(&path) else { return };
        libc::dup2(f.as_raw_fd(), 1);
        libc::dup2(f.as_raw_fd(), 2);
    }
    // the frame-rate log (`main.rs` log_fps, log_assets; `graphics.rs` log_draws) every 5 s, slow frames and stalls
    crate::FPS_LOG.store(true, std::sync::atomic::Ordering::Relaxed);
    eprintln!("OpenTennis {} (build {})", env!("CARGO_PKG_VERSION"), option_env!("HST_BUILD").unwrap_or("?"));
}

/// `LogPlugin::fmt_layer`: the engine's log on stderr, coloured only on a terminal (the codes would litter the file).
pub fn layer(_: &mut App) -> Option<BoxedFmtLayer> {
    // SAFETY: isatty only reads fd 2's state
    let tty = unsafe { libc::isatty(2) } == 1;
    Some(Box::new(tracing_subscriber::fmt::Layer::default().with_ansi(tty).with_writer(std::io::stderr)))
}

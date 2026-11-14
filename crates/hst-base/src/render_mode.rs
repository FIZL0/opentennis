//! B707: Settings → Graphics → Rendering, in every build: Hardware (the graphics chip, the default) or Software (the
//! CPU: wgpu's fallback adapter, which is llvmpipe/lavapipe on Linux, WARP on Windows, SwiftShader in a browser's
//! WebGPU). `rendering = hardware|software` in settings.txt; the renderer is picked once, at start, so a change
//! applies after a restart (the menu asks first; [`restart`]). Where the device has no software adapter the game
//! draws with the graphics chip anyway and says so ([`MISSING`]); `graphics::SOFTWARE` says what actually runs.
//! A software driver that crashes before the game's first frames (one with no way to draw to a window) is left out
//! from the next start on, until the player picks Software again ([`TRY_FILE`]), so it can't lock them out.

use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

/// The setting's words: settings.txt's, then the Settings row's.
pub const MODES: [(&str, &str); 2] = [("hardware", "Hardware"), ("software", "Software")];

/// The mode this run started in (Software asked for, whether or not it was found).
pub static STARTED: AtomicBool = AtomicBool::new(false);
/// Software was asked for, but this device has no software adapter (or it crashed the last start): the graphics chip
/// draws.
pub static MISSING: AtomicBool = AtomicBool::new(false);
/// Beside settings.txt while a Software start hasn't reached its first frames yet (`trying`), or after one that never
/// did (`failed`: Software is skipped until the menu picks it again).
pub const TRY_FILE: &str = "rendering-try.txt";
/// The folder settings.txt is in (for [`TRY_FILE`]).
static DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
/// Frames a Software start draws before it counts as working.
const FRAMES_OK: u32 = 30;

/// Whether settings.txt's `text` asks for Software (`HST_SOFTWARE=1` too, for a test run).
pub fn wanted(text: &str) -> bool {
    std::env::var_os("HST_SOFTWARE").is_some_and(|v| v != "0") || text.lines().filter_map(|l| l.split('#').next()?.split_once('=')).any(|(k, v)| k.trim() == "rendering" && v.trim() == MODES[1].0)
}

/// The settings.txt line for `software`.
pub fn line(software: bool) -> String {
    format!("rendering = {}\n", MODES[software as usize].0)
}

/// Sets the renderer up for the mode settings.txt (`text`, in folder `dir`) asks for, before the app starts: Software
/// looks for a software adapter among the backends `render` would use and names it, so Bevy takes it. If the
/// window can't use it, Bevy's own pick (the graphics chip) runs instead; if it crashes, the next start skips it.
pub fn apply(render: &mut bevy::render::settings::WgpuSettings, text: &str, dir: Option<&std::path::Path>) {
    let soft = wanted(text);
    STARTED.store(soft, Relaxed);
    if let Some(d) = dir {
        let _ = DIR.set(d.to_path_buf());
    }
    if !soft {
        return;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let try_file = dir.map(|d| d.join(TRY_FILE));
        let last = try_file.as_ref().and_then(|f| std::fs::read_to_string(f).ok()).unwrap_or_default();
        if !last.trim().is_empty() {
            // the last Software start never drew its first frames (it crashed): the graphics chip, until it's picked again
            if let Some(f) = &try_file {
                let _ = std::fs::write(f, "failed\n");
            }
            eprintln!("rendering: software skipped: the last start with it didn't get going");
            MISSING.store(true, Relaxed);
            return;
        }
        add_bundled_driver();
        let backends = render.backends.unwrap_or(wgpu::Backends::all());
        match software_adapter(backends) {
            Some((backend, name)) => {
                eprintln!("rendering: software asked for: {backend:?} {name}");
                render.adapter_name = Some(name);
                if let Some(f) = &try_file {
                    let _ = std::fs::write(f, "trying\n");
                }
            }
            None => MISSING.store(true, Relaxed),
        }
    }
    // the page asked the browser for its fallback adapter before loading the game (web/index.html `hstSoftware`)
    #[cfg(target_arch = "wasm32")]
    {
        let found = js_sys::Reflect::get(&js_sys::global(), &"hstSoftware".into()).is_ok_and(|v| v.is_truthy());
        render.force_fallback_adapter = found;
        MISSING.store(!found, Relaxed);
    }
}

/// Logs how Software went, once the renderer is up (`graphics::SOFTWARE`; the log isn't set up yet in [`apply`]).
pub fn report(software: bool, adapter: &str) {
    if !STARTED.load(Relaxed) {
        return;
    }
    if software {
        bevy::log::info!("rendering: software ({adapter})");
    } else if MISSING.load(Relaxed) {
        bevy::log::warn!("rendering: software asked for, but none was found (or it crashed the last start); drawing with {adapter}");
    } else {
        // found, but the window couldn't present from it: Bevy took the graphics chip
        MISSING.store(true, Relaxed);
        bevy::log::warn!("rendering: the software adapter couldn't draw to the window; drawing with {adapter}");
    }
}

/// A system: once a Software start has drawn its first frames it works, and [`TRY_FILE`] goes.
pub fn confirm(mut frames: bevy::prelude::Local<u32>) {
    if *frames > FRAMES_OK || !STARTED.load(Relaxed) || MISSING.load(Relaxed) {
        return;
    }
    *frames += 1;
    if *frames > FRAMES_OK {
        forget_try();
    }
}

/// Removes [`TRY_FILE`]: Software is tried again at the next start that asks for it.
fn forget_try() {
    if let Some(d) = DIR.get() {
        let _ = std::fs::remove_file(d.join(TRY_FILE));
    }
}

/// B708: the Linux builds carry Mesa's software Vulkan driver (lavapipe, `vulkan/` beside the game: tools/lavapipe.sh)
/// for systems that have none of their own. When Software is picked it's added to the drivers the Vulkan loader looks
/// at (`VK_ADD_DRIVER_FILES`, read when the renderer starts), beside the system's, so Hardware stays available if the
/// window can't use it. A system that has its own software driver lists both; the first found draws. Android's loader
/// only takes the phone's own driver, and Apple's Metal has no CPU device ([`NO_SOFTWARE`]).
#[cfg(all(target_os = "linux", not(target_os = "android")))]
fn add_bundled_driver() {
    let Some(icd) = std::env::current_exe().ok().and_then(|e| Some(e.canonicalize().ok()?.parent()?.join("vulkan/lvp_icd.json"))).filter(|f| f.is_file()) else {
        return;
    };
    let list = match std::env::var_os("VK_ADD_DRIVER_FILES").filter(|v| !v.is_empty()) {
        Some(v) => {
            let mut v = v;
            v.push(":");
            v.push(&icd);
            v
        }
        None => icd.into_os_string(),
    };
    eprintln!("rendering: bundled software driver added: {}", list.to_string_lossy());
    // SAFETY: called from `apply`, before the app (and its render, audio and task threads) starts
    unsafe { std::env::set_var("VK_ADD_DRIVER_FILES", list) };
}

#[cfg(not(any(target_arch = "wasm32", all(target_os = "linux", not(target_os = "android")))))]
fn add_bundled_driver() {}

/// B708: the Settings row's text where the platform settles that nothing can draw on the CPU (shown once Software was
/// picked): Apple's Metal has no CPU device, and Android's Vulkan loader only takes the phone's own driver (B709: a
/// headless SwiftShader posting frames to the window was weighed and dropped: the game on a CPU renderer runs ~9 fps on
/// a 16-thread desktop, ~1 on two cores, so a phone couldn't play it). `None`
/// where a software renderer may be found (Linux, Windows, a browser).
pub const NO_SOFTWARE: Option<&str> = if cfg!(any(target_os = "ios", target_os = "macos")) {
    Some("Hardware draws with your graphics chip. Apple devices have no software renderer (Metal always needs the graphics chip), so with Software picked the game still draws with Hardware (shown in brackets).")
} else if cfg!(target_os = "android") {
    Some("Hardware draws with your graphics chip. Android only lets a game use the phone's own graphics driver, and a phone's processor is far too slow to draw the game by itself, so there's no software renderer here: with Software picked the game still draws with Hardware (shown in brackets).")
} else {
    None
};

/// The first software adapter (a CPU device, or a driver known to draw on the CPU) among `backends`.
#[cfg(not(target_arch = "wasm32"))]
fn software_adapter(backends: wgpu::Backends) -> Option<(wgpu::Backend, String)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends, ..wgpu::InstanceDescriptor::new_without_display_handle() });
    let adapters = bevy::tasks::block_on(instance.enumerate_adapters(backends));
    adapters.iter().map(|a| a.get_info()).find(|i| is_software(i.device_type, &i.name)).map(|i| (i.backend, i.name))
}

/// Whether an adapter of this type and name draws on the CPU: a browser's WebGPU may not report the type, and
/// WebGL2 only names the renderer it sits on.
pub fn is_software(kind: wgpu_types::DeviceType, name: &str) -> bool {
    let name = name.to_lowercase();
    kind == wgpu_types::DeviceType::Cpu || ["swiftshader", "llvmpipe", "lavapipe", "softpipe", "basic render", "software"].iter().any(|s| name.contains(s))
}

/// Whether this build can start itself again (iOS apps can't: the player reopens it).
pub fn can_restart() -> bool {
    !cfg!(target_os = "ios")
}

/// Called once the setting has been saved: a mode other than the one running restarts the game into it (the browser
/// reloads the page; a phone app restarts its activity; iOS waits for the player to reopen it).
pub fn restart(software: bool) {
    if software {
        // picked again: a start that failed before gets another go
        forget_try();
    }
    if software == STARTED.load(Relaxed) {
        return;
    }
    #[cfg(target_arch = "wasm32")]
    if let Ok(f) = js_sys::Reflect::get(&js_sys::global(), &"hstRendering".into()).and_then(|f| wasm_bindgen::JsCast::dyn_into::<js_sys::Function>(f).map_err(|_| wasm_bindgen::JsValue::NULL)) {
        let _ = f.call1(&wasm_bindgen::JsValue::NULL, &MODES[software as usize].0.into());
    }
    #[cfg(target_os = "android")]
    crate::android::restart();
    #[cfg(all(unix, not(any(target_os = "ios", target_os = "android"))))]
    if let Ok(exe) = std::env::current_exe() {
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new(exe).args(std::env::args_os().skip(1)).exec();
        bevy::log::warn!("rendering: couldn't restart: {err}");
    }
    #[cfg(windows)]
    if let Ok(exe) = std::env::current_exe() {
        match std::process::Command::new(exe).args(std::env::args_os().skip(1)).spawn() {
            Ok(_) => std::process::exit(0),
            Err(e) => bevy::log::warn!("rendering: couldn't restart: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_word() {
        assert!(wanted("aspect = auto\nrendering = software\n") || std::env::var_os("HST_SOFTWARE").is_some());
        assert!(!wanted("rendering = hardware\n") || std::env::var_os("HST_SOFTWARE").is_some());
        assert_eq!(line(true), "rendering = software\n");
        assert!(is_software(wgpu_types::DeviceType::Cpu, "x"));
        assert!(is_software(wgpu_types::DeviceType::Other, "llvmpipe (LLVM 19.1.7, 256 bits)"));
        assert!(is_software(wgpu_types::DeviceType::Other, "Microsoft Basic Render Driver"));
        assert!(!is_software(wgpu_types::DeviceType::DiscreteGpu, "AMD Radeon RX 7900 XTX (RADV NAVI31)"));
    }
}

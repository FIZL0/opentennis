//! M155: ARM handhelds with no display server (muOS, Knulli on the RG34XX SP): no Wayland, no X11, no `/dev/dri`, only
//! the Mali blob's EGL drawing straight to the framebuffer (`/dev/fb0`). winit can't open a window there, so on such a
//! system its plugin is left out and this one stands in: the primary window gets the framebuffer's size and a native
//! window the Mali fbdev EGL takes (`{ width, height }` as two 16-bit numbers, the driver's own `fbdev_window`), handed
//! to wgpu's GL backend as an OpenHarmony handle, the one raw-window-handle kind whose pointer wgpu's EGL path passes
//! to `eglCreateWindowSurface` unchanged on any Linux. The display is EGL's default one (the blob's fbdev display).
//! The runner is a plain loop: no window events, the pads come from gilrs as usual (`OpenTennis.sh` points it past
//! udev where there's none, and PortMaster's `SDL_GAMECONTROLLERCONFIG` maps the built-in controls).

use std::ptr::NonNull;

use bevy::app::{AppExit, PluginsState};
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, RawHandleWrapper, RawHandleWrapperHolder, WindowCreated, WindowWrapper};
use wgpu::rwh;

/// The framebuffer the game draws to.
const FB: &str = "/dev/fb0";

/// No display server to open a window on, but a framebuffer: winit would panic, so this plugin runs instead.
/// `HST_FBDEV=1` forces it, `HST_FBDEV=0` keeps winit.
pub fn wanted() -> bool {
    match std::env::var("HST_FBDEV").ok().as_deref() {
        Some("1") => return true,
        Some("0") => return false,
        _ => {}
    }
    let set = |v: &str| std::env::var_os(v).is_some_and(|v| !v.is_empty());
    !set("WAYLAND_DISPLAY") && !set("WAYLAND_SOCKET") && !set("DISPLAY") && std::path::Path::new(FB).exists()
}

/// The Mali fbdev EGL's native window: the surface's size.
#[repr(C)]
struct NativeWindow {
    width: u16,
    height: u16,
}

/// The primary window's handles; keeps the native window alive as long as a surface may draw to it.
struct FbWindow(Box<NativeWindow>);

impl rwh::HasWindowHandle for FbWindow {
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, rwh::HandleError> {
        let native = NonNull::from(&*self.0).cast();
        // SAFETY: the box lives as long as `self`
        Ok(unsafe { rwh::WindowHandle::borrow_raw(rwh::RawWindowHandle::OhosNdk(rwh::OhosNdkWindowHandle::new(native))) })
    }
}

impl rwh::HasDisplayHandle for FbWindow {
    fn display_handle(&self) -> Result<rwh::DisplayHandle<'_>, rwh::HandleError> {
        // SAFETY: a handle with no pointer in it; wgpu takes EGL's default display for it
        Ok(unsafe { rwh::DisplayHandle::borrow_raw(rwh::RawDisplayHandle::Ohos(rwh::OhosDisplayHandle::new())) })
    }
}

/// The framebuffer's visible size (`FBIOGET_VSCREENINFO`'s first two words), else its first sysfs mode (`U:720x480p-60`).
fn fb_size() -> Option<(u32, u32)> {
    use std::os::fd::AsRawFd;
    const FBIOGET_VSCREENINFO: libc::c_ulong = 0x4600;
    if let Ok(f) = std::fs::File::open(FB) {
        // fb_var_screeninfo: 40 32-bit words, xres and yres first
        let mut info = [0u32; 40];
        // SAFETY: the buffer is the struct's size
        if unsafe { libc::ioctl(f.as_raw_fd(), FBIOGET_VSCREENINFO, info.as_mut_ptr()) } == 0 && info[0] > 0 && info[1] > 0 {
            return Some((info[0], info[1]));
        }
    }
    let modes = std::fs::read_to_string("/sys/class/graphics/fb0/modes").ok()?;
    let mode = modes.lines().next()?.split(':').next_back()?;
    let (w, rest) = mode.split_once('x')?;
    let h: String = rest.chars().take_while(char::is_ascii_digit).collect();
    Some((w.parse().ok()?, h.parse().ok()?))
}

/// Stands in for winit: sizes the primary window to the framebuffer, gives the renderer its handles, runs the loop.
pub fn plugin(app: &mut App) {
    let size = fb_size();
    info!("no display server: drawing to {FB} ({size:?})");
    let world = app.world_mut();
    let mut primary = world.query_filtered::<(Entity, &mut Window, &RawHandleWrapperHolder), With<PrimaryWindow>>();
    let Ok((entity, mut window, holder)) = primary.single_mut(world) else {
        warn!("fbdev: no primary window");
        return;
    };
    // `HST_WINDOW` (main.rs) sets the size by hand; otherwise the framebuffer's
    let (w, h) = match size {
        Some(s) if std::env::var_os("HST_WINDOW").is_none() => s,
        _ => (window.physical_width(), window.physical_height()),
    };
    window.resolution.set_physical_resolution(w, h);
    window.resolution.set_scale_factor(1.0);
    window.focused = true;
    let native = FbWindow(Box::new(NativeWindow { width: w.min(u16::MAX as u32) as u16, height: h.min(u16::MAX as u32) as u16 }));
    let handles = RawHandleWrapper::new(&WindowWrapper::new(native)).expect("fbdev window handles");
    // the renderer's thread waits on the holder for the display (third_party/bevy_render `settings.rs`, M154)
    *holder.0.lock().unwrap() = Some(handles.clone());
    world.entity_mut(entity).insert(handles);
    world.write_message(WindowCreated { window: entity });
    app.set_runner(run);
}

/// winit's runner without the events: wait for the plugins (the renderer comes up off the main thread), then update
/// until something asks to quit. Frames are paced by the swap (`Fifo`).
fn run(mut app: App) -> AppExit {
    while app.plugins_state() == PluginsState::Adding {
        bevy::tasks::tick_global_task_pools_on_main_thread();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    if app.plugins_state() == PluginsState::Ready {
        app.finish();
        app.cleanup();
    }
    loop {
        app.update();
        if let Some(exit) = app.should_exit() {
            return exit;
        }
    }
}

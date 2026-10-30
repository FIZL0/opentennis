//! iOS start-up (M91): the app's Documents folder (Files app → On My iPhone > OpenTennis) holds the ISO, settings.txt,
//! mods/ and `hst-log.txt`, this launch's log (stderr, panics and the engine's log; iOS shows nothing else without a
//! Mac). Writing the log also makes iOS list the folder in Files, which it only does once there's a file in it.

use bevy::log::{BoxedLayer, tracing_subscriber};
use bevy::prelude::*;
use std::os::fd::AsRawFd;

/// stderr into `Documents/hst-log.txt`, fresh each launch. Call first thing in `main`.
pub fn log_to_documents(docs: &std::path::Path) {
    let _ = std::fs::create_dir_all(docs);
    if let Ok(f) = std::fs::File::create(docs.join("hst-log.txt")) {
        // SAFETY: plain dup2 of an open file over fd 2
        unsafe { libc::dup2(f.as_raw_fd(), 2) };
    }
    // the frame-rate log (`main.rs` log_fps, log_assets; `graphics.rs` log_draws) every 5 s: the phone's only profiler
    crate::FPS_LOG.store(true, std::sync::atomic::Ordering::Relaxed);
    eprintln!("Hot Shots Tennis {} (build {})", env!("CARGO_PKG_VERSION"), option_env!("HST_BUILD").unwrap_or("?"));
    eprintln!("start: {}", memory());
    watch_memory_warnings();
    play_on_silent();
    // a close with no log line and no crash report says nothing: these say which kind of end it was
    // SAFETY: the handlers are `extern "C"` functions that live for the process; on_signal makes only
    // async-signal-safe calls
    unsafe {
        libc::atexit(on_exit);
        for sig in [libc::SIGABRT, libc::SIGSEGV, libc::SIGBUS, libc::SIGILL, libc::SIGTRAP, libc::SIGFPE] {
            libc::signal(sig, on_signal as *const () as libc::sighandler_t);
        }
        NSSetUncaughtExceptionHandler(on_exception);
    }
}

extern "C" fn on_exit() {
    eprintln!("exit() called");
}

extern "C" fn on_signal(sig: libc::c_int) {
    // only async-signal-safe calls here: write, then die of the same signal
    let msg: &[u8] = match sig {
        libc::SIGABRT => b"fatal signal: SIGABRT (abort)\n",
        libc::SIGSEGV => b"fatal signal: SIGSEGV (bad memory access)\n",
        libc::SIGBUS => b"fatal signal: SIGBUS (bad memory access)\n",
        libc::SIGILL => b"fatal signal: SIGILL\n",
        libc::SIGTRAP => b"fatal signal: SIGTRAP\n",
        _ => b"fatal signal: SIGFPE\n",
    };
    // SAFETY: write, signal and raise are async-signal-safe; `msg` is a static byte string
    unsafe {
        libc::write(2, msg.as_ptr().cast(), msg.len());
        libc::signal(sig, libc::SIG_DFL);
        libc::raise(sig);
    }
}

// UIKit failures are Objective-C exceptions: their name and reason, before the abort
unsafe extern "C" {
    fn NSSetUncaughtExceptionHandler(handler: extern "C" fn(*mut std::ffi::c_void));
    fn sel_registerName(name: *const std::ffi::c_char) -> *mut std::ffi::c_void;
    fn objc_msgSend();
    fn objc_getClass(name: *const std::ffi::c_char) -> *mut std::ffi::c_void;
}

/// The process's CPU time (all threads) in ms, for log_fps's ms/frame (iOS has no /proc).
pub fn cpu_ms() -> f32 {
    // SAFETY: rusage is plain integers (all-zero is valid); getrusage fills the struct we own
    let mut u: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut u) };
    let ms = |t: libc::timeval| t.tv_sec as f32 * 1e3 + t.tv_usec as f32 / 1e3;
    ms(u.ru_utime) + ms(u.ru_stime)
}

/// NSProcessInfo's thermal state: the phone slows the chip from "serious" on.
pub fn thermal() -> &'static str {
    // SAFETY: objc_msgSend cast to each message's signature as Foundation declares it (class method returning an
    // object, an NSInteger getter)
    unsafe {
        let obj: extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> *mut std::ffi::c_void = std::mem::transmute(objc_msgSend as *const ());
        let int: extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> isize = std::mem::transmute(objc_msgSend as *const ());
        let info = obj(objc_getClass(c"NSProcessInfo".as_ptr()), sel_registerName(c"processInfo".as_ptr()));
        ["nominal", "fair", "serious", "critical"].get(int(info, sel_registerName(c"thermalState".as_ptr())) as usize).copied().unwrap_or("?")
    }
}

extern "C" fn on_exception(e: *mut std::ffi::c_void) {
    // SAFETY: `e` is the NSException UIKit hands the handler; objc_msgSend cast to the getters' signatures, and the
    // UTF8String read only when the string isn't nil
    let text = |obj: *mut std::ffi::c_void, sel: &std::ffi::CStr| unsafe {
        let send: extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> *mut std::ffi::c_void = std::mem::transmute(objc_msgSend as *const ());
        let s = send(obj, sel_registerName(sel.as_ptr()));
        let utf8: extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> *const std::ffi::c_char = std::mem::transmute(objc_msgSend as *const ());
        if s.is_null() { "?".into() } else { std::ffi::CStr::from_ptr(utf8(s, sel_registerName(c"UTF8String".as_ptr()))).to_string_lossy().into_owned() }
    };
    eprintln!("Objective-C exception {}: {}", text(e, c"name"), text(e, c"reason"));
}

/// The first frame, a heartbeat every 5 s, quits asked for and iOS lifecycle changes, in the log; with the memory
/// iOS counts against the app (B538), and each new high of it 64 MB over the last.
pub fn heartbeat(time: Res<Time<Real>>, mut frames: Local<u64>, mut last: Local<f32>, mut high: Local<u64>, mut exits: MessageReader<AppExit>, mut life: MessageReader<bevy::window::AppLifecycle>) {
    *frames += 1;
    let t = time.elapsed_secs();
    let mem = memory();
    if *frames == 1 || t - *last >= 5.0 {
        *last = t;
        eprintln!("frame {} at {t:.1}s; {}", *frames, mem);
    }
    if mem.now > *high + (64 << 20) {
        *high = mem.now;
        eprintln!("memory high at {t:.1}s: {mem}");
    }
    for e in exits.read() {
        eprintln!("AppExit {e:?} at frame {}", *frames);
    }
    for l in life.read() {
        eprintln!("lifecycle {l:?} at {t:.1}s; {mem}");
    }
}

/// B538: the app's memory as iOS counts it for its limit (the phys footprint: heap, GPU buffers and textures, all of
/// it), its peak so far, and what's left before iOS closes the app.
pub struct Memory {
    pub now: u64,
    pub peak: u64,
    pub left: u64,
}

impl std::fmt::Display for Memory {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mb = |b: u64| b >> 20;
        write!(f, "memory {} MB (peak {} MB), {} MB left before iOS closes the app", mb(self.now), mb(self.peak), mb(self.left))
    }
}

unsafe extern "C" {
    static mach_task_self_: u32;
    fn task_info(task: u32, flavor: i32, info: *mut i32, count: *mut u32) -> i32;
    fn os_proc_available_memory() -> usize;
}

/// [`Memory`] now: `task_info(TASK_VM_INFO)`'s phys_footprint and (rev 3+) its peak, `os_proc_available_memory`.
pub fn memory() -> Memory {
    static PEAK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    // task_vm_info_data_t as u64s: phys_footprint is the 19th (byte 144), ledger_phys_footprint_peak the 22nd (168)
    let mut info = [0u64; 48];
    let mut count = (std::mem::size_of_val(&info) / 4) as u32;
    const TASK_VM_INFO: i32 = 22;
    // SAFETY: `info` holds `count` natural_t's; the kernel writes at most that many and sets `count`
    let ok = unsafe { task_info(mach_task_self_, TASK_VM_INFO, info.as_mut_ptr().cast(), &mut count) } == 0;
    let now = if ok { info[18] } else { 0 };
    let ledger = if ok && count >= 44 { info[21] } else { 0 };
    let peak = PEAK.fetch_max(now.max(ledger), std::sync::atomic::Ordering::Relaxed).max(now).max(ledger);
    // SAFETY: a plain query (iOS 13+)
    Memory { now, peak, left: unsafe { os_proc_available_memory() } as u64 }
}

/// B538: iOS's memory warnings, logged with the memory then (the last sign before it closes the app for memory).
fn watch_memory_warnings() {
    use objc2::runtime::AnyObject;
    // SAFETY: the messages as NSNotificationCenter declares them; the block and observer are retained for the process
    objc2::rc::autoreleasepool(|_| unsafe {
        let name = objc2_foundation::NSString::from_str("UIApplicationDidReceiveMemoryWarningNotification");
        let block = block2::RcBlock::new(|_: std::ptr::NonNull<AnyObject>| eprintln!("iOS memory warning: {}", memory()));
        let center: *mut AnyObject = objc2::msg_send![objc2::class!(NSNotificationCenter), defaultCenter];
        let observer: *mut AnyObject = objc2::msg_send![center, addObserverForName: &*name, object: std::ptr::null_mut::<AnyObject>(), queue: std::ptr::null_mut::<AnyObject>(), usingBlock: &*block];
        // kept for the process: the center holds the observer, the observer the block
        let _: *mut AnyObject = objc2::msg_send![observer, retain];
    });
}

/// M159a: what a sideload installer left in our own bundle, for Update to hand the new release back to it: every URL
/// scheme Info.plist declares (AltStore adds `altstore-<bundle id>`, SideStore `sidestore-<bundle id>` to each app
/// it signs) and whether TrollStore's `_TrollStore` marker file sits beside the .app. Nothing asks iOS which apps
/// are installed (B652: `canOpenURL` says yes to schemes stock apps own).
pub fn install_marks() -> (Vec<String>, bool) {
    use objc2::runtime::AnyObject;
    // the elements of an NSArray (none for nil or anything else)
    unsafe fn items(a: *mut AnyObject) -> Vec<*mut AnyObject> {
        // SAFETY: `a` is nil or an object; it's read as an NSArray only once it says it is one
        unsafe {
            let Some(a) = a.as_ref() else { return vec![] };
            let array: bool = objc2::msg_send![a, isKindOfClass: objc2::class!(NSArray)];
            if !array {
                return vec![];
            }
            let n: usize = objc2::msg_send![a, count];
            (0..n).map(|i| objc2::msg_send![a, objectAtIndex: i]).collect()
        }
    }
    let mut schemes = vec![];
    // SAFETY: the messages as NSBundle/NSArray/NSDictionary/NSString declare them; the info dictionary is immutable,
    // and each object's class is checked before it's read as one
    objc2::rc::autoreleasepool(|_| unsafe {
        let types_key = objc2_foundation::NSString::from_str("CFBundleURLTypes");
        let schemes_key = objc2_foundation::NSString::from_str("CFBundleURLSchemes");
        let bundle: *mut AnyObject = objc2::msg_send![objc2::class!(NSBundle), mainBundle];
        let Some(bundle) = bundle.as_ref() else { return };
        let types: *mut AnyObject = objc2::msg_send![bundle, objectForInfoDictionaryKey: &*types_key];
        for t in items(types) {
            let dict: bool = objc2::msg_send![&*t, isKindOfClass: objc2::class!(NSDictionary)];
            if !dict {
                continue;
            }
            let list: *mut AnyObject = objc2::msg_send![&*t, objectForKey: &*schemes_key];
            for s in items(list) {
                let string: bool = objc2::msg_send![&*s, isKindOfClass: objc2::class!(NSString)];
                let c: *const std::ffi::c_char = if string { objc2::msg_send![&*s, UTF8String] } else { std::ptr::null() };
                if !c.is_null() {
                    schemes.push(std::ffi::CStr::from_ptr(c).to_string_lossy().into_owned());
                }
            }
        }
    });
    // the executable is <container>/OpenTennis.app/OpenTennis; TrollStore marks the container
    let container = std::env::current_exe().ok().and_then(|e| Some(e.parent()?.parent()?.to_path_buf()));
    let troll = container.is_some_and(|c| c.join("_TrollStore").exists());
    (schemes, troll)
}

/// Sound even with the ring/silent switch on: the Playback audio category (the default, SoloAmbient, mutes with it).
/// Set before cpal starts its stream; AVFAudio is linked by cpal.
fn play_on_silent() {
    use objc2::runtime::AnyObject;
    // SAFETY: the messages as AVAudioSession declares them; the category constant's value is its own name
    objc2::rc::autoreleasepool(|_| unsafe {
        let category = objc2_foundation::NSString::from_str("AVAudioSessionCategoryPlayback");
        let session: *mut AnyObject = objc2::msg_send![objc2::class!(AVAudioSession), sharedInstance];
        let ok: bool = objc2::msg_send![session, setCategory: &*category, error: std::ptr::null_mut::<*mut AnyObject>()];
        if !ok {
            eprintln!("audio: couldn't set the Playback category");
        }
    });
}

/// `LogPlugin::custom_layer`: the engine's log as text on stderr (bevy only sends it to os_log on iOS).
pub fn log_layer(_: &mut App) -> Option<BoxedLayer> {
    Some(Box::new(tracing_subscriber::fmt::Layer::default().with_ansi(false).with_writer(std::io::stderr)))
}

/// No disc yet: the start page (`start_page.rs`) with the Files app's picker, instead of a launch that closes on a black
/// screen. An ISO dropped in the app's folder still works too (`disc::resolve` finds it on the next launch).
pub fn no_disc() -> ! {
    eprintln!("no disc image in Documents");
    let plugins = DefaultPlugins.set(bevy::log::LogPlugin { custom_layer: log_layer, ..default() }).set(WindowPlugin {
        // fullscreen like the game: a default (1280×720) window lays out bigger than the phone's screen
        primary_window: Some(Window { mode: bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Primary), ..default() }),
        ..default()
    });
    crate::start_page::run(plugins, crate::disc::data_dir().join("disc.pack"), INTRO, HELP)
}

const INTRO: &str = "This remaster plays from your own copy of the original game. Choose its disc image (.iso) in \
                     Files: iCloud Drive, On My iPhone or a USB drive. The game copies the part it needs, then you can \
                     delete the ISO.";

const HELP: &str = "No file picker? Put the ISO in the Files app under On My iPhone > OpenTennis (the folder with this \
                    app's icon), then close and reopen the app: it finds it there.";

/// iOS can't start an app again from inside: the player closes it.
pub(crate) const DONE: &str = "Done! Tap to close, then open OpenTennis again to play.";

pub(crate) fn restart() {}

/// The picker's answer for [`taken`]: `Some(None)` closed without a file.
static PICKED: std::sync::Mutex<Option<Option<std::fs::File>>> = std::sync::Mutex::new(None);

pub(crate) fn taken() -> Option<Option<std::fs::File>> {
    PICKED.lock().unwrap().take()
}

/// Opens the Files app's picker over the window (on the main thread); its answer arrives through [`taken`].
pub(crate) fn pick() -> bool {
    // SAFETY: the main queue lives for the process; `present_picker` takes no context
    unsafe { dispatch_async_f(&raw const _dispatch_main_q, std::ptr::null_mut(), present_picker) };
    true
}

objc2::define_class!(
    /// The document picker's delegate: the picked file, opened in place (a security-scoped URL, kept open for the
    /// run: the import reads it on its own thread), or the cancel.
    #[unsafe(super(objc2::runtime::NSObject))]
    #[name = "HstDiscPicker"]
    struct DiscPicker;

    impl DiscPicker {
        #[unsafe(method(documentPicker:didPickDocumentsAtURLs:))]
        fn picked(&self, _picker: &objc2::runtime::AnyObject, urls: &objc2::runtime::AnyObject) {
            use objc2::runtime::AnyObject;
            // SAFETY: `urls` is the NSArray<NSURL> UIKit hands the delegate; NSURL.path's UTF8String lives as long as it
            let file = unsafe {
                let url: *mut AnyObject = objc2::msg_send![urls, firstObject];
                let url = url.as_ref();
                url.and_then(|url| {
                    let _: bool = objc2::msg_send![url, startAccessingSecurityScopedResource];
                    let path: *mut AnyObject = objc2::msg_send![url, path];
                    let c: *const std::ffi::c_char = objc2::msg_send![path, UTF8String];
                    let path = std::ffi::CStr::from_ptr(c).to_string_lossy().into_owned();
                    std::fs::File::open(&path).inspect_err(|e| eprintln!("opening the picked {path}: {e}")).ok()
                })
            };
            *PICKED.lock().unwrap() = Some(file);
        }

        #[unsafe(method(documentPickerWasCancelled:))]
        fn cancelled(&self, _picker: &objc2::runtime::AnyObject) {
            *PICKED.lock().unwrap() = Some(None);
        }
    }
);

extern "C" fn present_picker(_: *mut std::ffi::c_void) {
    use objc2::ClassType;
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    thread_local! {
        // the picker holds its delegate weakly: main thread only, made on first use, kept
        static DELEGATE: std::cell::OnceCell<objc2::rc::Retained<DiscPicker>> = const { std::cell::OnceCell::new() };
    }
    // SAFETY: on the main thread (dispatched there); the messages' signatures as UIKit/Foundation declare them
    unsafe {
        let delegate = DELEGATE.with(|d| objc2::rc::Retained::as_ptr(d.get_or_init(|| msg_send![DiscPicker::class(), new])) as *const AnyObject);
        let app: *mut AnyObject = msg_send![class!(UIApplication), sharedApplication];
        let mut window: *mut AnyObject = msg_send![&*app, keyWindow];
        if window.is_null() {
            let windows: *mut AnyObject = msg_send![&*app, windows];
            window = msg_send![&*windows, firstObject];
        }
        let root: *mut AnyObject = if window.is_null() { std::ptr::null_mut() } else { msg_send![&*window, rootViewController] };
        if root.is_null() {
            eprintln!("disc picker: no view controller to show it over");
            *PICKED.lock().unwrap() = Some(None);
            return;
        }
        // any file: an ISO from a cloud drive often has no type; the import checks it's the disc
        // (the older initWithDocumentTypes:inMode: greys files out in open mode)
        let any = objc2_foundation::NSString::from_str("public.item");
        let item: *mut AnyObject = msg_send![class!(UTType), typeWithIdentifier: &*any];
        let types: *mut AnyObject = msg_send![class!(NSArray), arrayWithObject: item];
        let picker: *mut AnyObject = msg_send![class!(UIDocumentPickerViewController), alloc];
        // asCopy NO: the file in place, not a 4 GB copy into the app
        let picker: *mut AnyObject = msg_send![picker, initForOpeningContentTypes: types, asCopy: false];
        let _: () = msg_send![&*picker, setShouldShowFileExtensions: true];
        let _: () = msg_send![&*picker, setDelegate: delegate];
        let _: () = msg_send![&*root, presentViewController: picker, animated: true, completion: std::ptr::null_mut::<AnyObject>()];
    }
}

// UTType lives here; nothing else links it
#[link(name = "UniformTypeIdentifiers", kind = "framework")]
unsafe extern "C" {}

// UIKit only on the main thread: the on-screen pad's calls go through its dispatch queue
unsafe extern "C" {
    static _dispatch_main_q: std::ffi::c_void;
    fn dispatch_async_f(queue: *const std::ffi::c_void, context: *mut std::ffi::c_void, work: extern "C" fn(*mut std::ffi::c_void));
}

/// M91a1: the light haptic tick of an on-screen pad press (the Vibration setting; `play/touch_pad.rs`).
pub fn haptic() {
    // SAFETY: the main queue lives for the process; `tick` takes no context
    unsafe { dispatch_async_f(&raw const _dispatch_main_q, std::ptr::null_mut(), tick) };
}

extern "C" fn tick(_: *mut std::ffi::c_void) {
    use std::sync::atomic::{AtomicPtr, Ordering};
    // one generator, made on first use and kept (main thread only)
    static GENERATOR: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());
    // SAFETY: on the main thread (dispatched there); objc_msgSend cast to each UIKit message's signature, the
    // generator retained in GENERATOR for the process
    unsafe {
        let obj: extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> *mut std::ffi::c_void = std::mem::transmute(objc_msgSend as *const ());
        let with_style: extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void, isize) -> *mut std::ffi::c_void = std::mem::transmute(objc_msgSend as *const ());
        let mut g = GENERATOR.load(Ordering::Relaxed);
        if g.is_null() {
            let class = objc_getClass(c"UIImpactFeedbackGenerator".as_ptr());
            if class.is_null() {
                return;
            }
            // UIImpactFeedbackStyleLight
            g = with_style(obj(class, sel_registerName(c"alloc".as_ptr())), sel_registerName(c"initWithStyle:".as_ptr()), 0);
            GENERATOR.store(g, Ordering::Relaxed);
        }
        if !g.is_null() {
            obj(g, sel_registerName(c"impactOccurred".as_ptr()));
        }
    }
}

/// B501: opens `url` in Safari (`UIApplication.openURL`, on the main thread; there's no `xdg-open`).
pub fn open_url(url: &str) {
    let ctx = Box::into_raw(Box::new(url.to_owned())).cast();
    // SAFETY: the main queue lives for the process; `open_on_main` takes the box back
    unsafe { dispatch_async_f(&raw const _dispatch_main_q, ctx, open_on_main) };
}

extern "C" fn open_on_main(ctx: *mut std::ffi::c_void) {
    type Id = *mut std::ffi::c_void;
    // SAFETY: `ctx` is the `Box<String>` `open_url` leaked; the selectors' signatures as UIKit/Foundation declare them
    unsafe {
        let url = Box::from_raw(ctx.cast::<String>());
        let obj: extern "C" fn(Id, Id) -> Id = std::mem::transmute(objc_msgSend as *const ());
        let with: extern "C" fn(Id, Id, Id) -> Id = std::mem::transmute(objc_msgSend as *const ());
        let open: extern "C" fn(Id, Id, Id, Id, Id) = std::mem::transmute(objc_msgSend as *const ());
        let s = objc2_foundation::NSString::from_str(&url);
        let u = with(objc_getClass(c"NSURL".as_ptr()), sel_registerName(c"URLWithString:".as_ptr()), objc2::rc::Retained::as_ptr(&s) as Id);
        if u.is_null() {
            return bevy::log::warn!("opening {url}: not a URL");
        }
        let app = obj(objc_getClass(c"UIApplication".as_ptr()), sel_registerName(c"sharedApplication".as_ptr()));
        let options = obj(objc_getClass(c"NSDictionary".as_ptr()), sel_registerName(c"dictionary".as_ptr()));
        open(app, sel_registerName(c"openURL:options:completionHandler:".as_ptr()), u, options, std::ptr::null_mut());
    }
}

/// UIKit's `UIEdgeInsets`.
#[repr(C)]
#[derive(Clone, Copy)]
struct Insets {
    top: f64,
    left: f64,
    bottom: f64,
    right: f64,
}

/// The window's safe-area insets (left, top, right, bottom) in points, the window's logical px: the notch or Dynamic
/// Island's side and the home indicator's strip. As last read: each call asks the main thread for them anew.
pub fn safe_area(window: &bevy::window::RawHandleWrapper) -> [f32; 4] {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SAFE: [AtomicU32; 4] = [const { AtomicU32::new(0) }; 4];
    extern "C" fn read(view: *mut std::ffi::c_void) {
        // SAFETY: `view` is the window's live UIView (see the dispatch below); safeAreaInsets returns UIEdgeInsets
        // (`Insets`, four doubles) by value
        unsafe {
            let insets: extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> Insets = std::mem::transmute(objc_msgSend as *const ());
            let i = insets(view, sel_registerName(c"safeAreaInsets".as_ptr()));
            for (a, v) in SAFE.iter().zip([i.left, i.top, i.right, i.bottom]) {
                a.store((v as f32).to_bits(), Ordering::Relaxed);
            }
        }
    }
    if let raw_window_handle::RawWindowHandle::UiKit(h) = window.get_window_handle() {
        // SAFETY: the view lives as long as the window, which outlives this frame's request
        unsafe { dispatch_async_f(&raw const _dispatch_main_q, h.ui_view.as_ptr(), read) };
    }
    SAFE.each_ref().map(|a| f32::from_bits(a.load(Ordering::Relaxed)))
}

/// P49: the display's refresh rate for the frame limit (`graphics.rs`; 0 = uncapped). A ProMotion iPhone keeps an
/// app's CAMetalLayer at 60 Hz unless something asks for more (Info.plist's CADisableMinimumFrameDurationOnPhone only
/// allows it), and wgpu's Metal surface just presents with no display link. So a display link of our own on the main
/// run loop asks for 80–120 Hz above a 60 cap and for the cap itself at or below it, so a 60 cap stays on 60 Hz vsyncs.
pub fn frame_rate(fps: u32) {
    // SAFETY: the main queue lives for the process; the context is the fps, not a pointer
    unsafe { dispatch_async_f(&raw const _dispatch_main_q, fps as usize as *mut std::ffi::c_void, set_frame_rate) };
}

objc2::define_class!(
    /// The display link's target: nothing to do on its ticks, it's only there for the rate it asks for.
    #[unsafe(super(objc2::runtime::NSObject))]
    #[name = "HstFrameRate"]
    struct FrameRate;

    impl FrameRate {
        #[unsafe(method(tick:))]
        fn tick(&self, _link: &objc2::runtime::AnyObject) {}
    }
);

extern "C" fn set_frame_rate(fps: *mut std::ffi::c_void) {
    use objc2::ClassType;
    use objc2_quartz_core::{CADisplayLink, CAFrameRateRange};
    thread_local! {
        // main thread only: made on first use, kept
        static LINK: std::cell::OnceCell<objc2::rc::Retained<CADisplayLink>> = const { std::cell::OnceCell::new() };
    }
    let fps = fps as usize as f32;
    let range = if fps == 0.0 || fps > 60.0 { CAFrameRateRange { minimum: 80.0, maximum: 120.0, preferred: 120.0 } } else { CAFrameRateRange { minimum: fps, maximum: fps, preferred: fps } };
    LINK.with(|l| {
        // SAFETY: on the main thread (dispatched there); the selector names FrameRate's own `tick:` method
        let link = l.get_or_init(|| unsafe {
            let target: objc2::rc::Retained<FrameRate> = objc2::msg_send![FrameRate::class(), new];
            let link = CADisplayLink::displayLinkWithTarget_selector(&target, objc2::sel!(tick:));
            link.addToRunLoop_forMode(&objc2_foundation::NSRunLoop::mainRunLoop(), objc2_foundation::NSRunLoopCommonModes);
            link
        });
        link.setPreferredFrameRateRange(range);
    });
    eprintln!("display link: {}–{} Hz, preferred {}", range.minimum, range.maximum, range.preferred);
}

// CoreMotion's accelerometer for `orientation`'s gravity check
#[link(name = "CoreMotion", kind = "framework")]
unsafe extern "C" {}

/// CoreMotion's `CMAcceleration`, in g along the device's axes (x right, y up the screen held upright).
#[repr(C)]
#[derive(Clone, Copy)]
struct Acceleration {
    x: f64,
    y: f64,
    _z: f64,
}

/// What `orientation` holds the window's view controller to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Hold {
    Landscape,
    Portrait,
    /// B694: rotation lock on: landscape with the camera / Dynamic Island on the left only
    Locked,
}

/// `orientation`'s state: the orientation the game is held to, the device's last reported orientation and since
/// when, since when the device has said portrait while the accelerometer says landscape, and the accelerometer.
#[derive(Default)]
pub struct Orientation {
    held: Option<Hold>,
    device: isize,
    since: f32,
    across: Option<f32>,
    motion: usize,
}

/// M144: the game turns into portrait and back with the device, and holds landscape while rotation lock is on (B694).
/// Info.plist allows both; the window's view controller is kept to the orientation the game is in (landscape or
/// portrait, `ValidOrientations`), and moves to the other only when the device turns there. An iPhone's rotation lock
/// makes UIDevice report portrait however the phone is held, so a turn to portrait the accelerometer doesn't agree
/// with is the lock going on, not a turn: ignored, the game stays as it is, landscape included. Under the lock the
/// device's orientation doesn't change again, so the game stays put until it's off. An iPad's lock holds the
/// current orientation itself. Main thread (UIKit, winit's windows).
///
/// B694: once the device has said portrait for a second while the accelerometer says the phone is held sideways,
/// that's rotation lock, and the game turns to landscape with the camera / Dynamic Island on the left
/// (`ValidOrientations::LandscapeRight`) and stays there, held upright included, until the device reports a turn
/// the accelerometer agrees with (the lock off).
pub fn orientation(mut state: Local<Orientation>, time: Res<Time<Real>>, _main_thread: bevy::ecs::system::NonSendMarker) {
    use winit::platform::ios::{ValidOrientations, WindowExtIOS};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    type Id = *mut std::ffi::c_void;
    let now = time.elapsed_secs();
    // SAFETY: on the main thread (NonSendMarker); objc_msgSend cast to each message's signature as UIKit and
    // CoreMotion declare it; the motion manager is retained in `state` for the app's life
    let (device, gravity) = unsafe {
        let obj: extern "C" fn(Id, Id) -> Id = std::mem::transmute(objc_msgSend as *const ());
        let int: extern "C" fn(Id, Id) -> isize = std::mem::transmute(objc_msgSend as *const ());
        let accel: extern "C" fn(Id, Id) -> Acceleration = std::mem::transmute(objc_msgSend as *const ());
        let ui_device = obj(objc_getClass(c"UIDevice".as_ptr()), sel_registerName(c"currentDevice".as_ptr()));
        if state.motion == 0 {
            obj(ui_device, sel_registerName(c"beginGeneratingDeviceOrientationNotifications".as_ptr()));
            let class = objc_getClass(c"CMMotionManager".as_ptr());
            if !class.is_null() {
                let m = obj(obj(class, sel_registerName(c"alloc".as_ptr())), sel_registerName(c"init".as_ptr()));
                obj(m, sel_registerName(c"startAccelerometerUpdates".as_ptr()));
                state.motion = m as usize;
            }
        }
        let data = if state.motion == 0 { std::ptr::null_mut() } else { obj(state.motion as Id, sel_registerName(c"accelerometerData".as_ptr())) };
        // landscape when gravity runs across the screen more than along it; none while the device lies flat
        let gravity = (!data.is_null()).then(|| accel(data, sel_registerName(c"acceleration".as_ptr()))).filter(|a| a.x.abs().max(a.y.abs()) > 0.5).map(|a| a.x.abs() > a.y.abs());
        (int(ui_device, sel_registerName(c"orientation".as_ptr())), gravity)
    };
    bevy::winit::WINIT_WINDOWS.with_borrow(|w| {
        let Some(win) = w.windows.values().next() else { return };
        let size = win.inner_size();
        let window = size.width > size.height;
        if device != state.device {
            state.device = device;
            state.since = now;
        }
        // portrait, upside down, landscape left/right; face up/down and unknown say nothing
        let turned = (1..=4).contains(&device).then_some(device >= 3);
        let hold = |landscape: bool| if landscape { Hold::Landscape } else { Hold::Portrait };
        // rotation lock: portrait from the device with the phone held sideways, for longer than a turn takes to report
        if turned == Some(false) && gravity == Some(true) {
            state.across.get_or_insert(now);
        } else {
            state.across = None;
        }
        let locked = state.across.is_some_and(|t| now - t >= 1.0);
        // the accelerometer has a moment to agree with a new orientation (turning the phone past 45° fires it)
        let want = match (state.held, turned) {
            (held, _) if locked => (held != Some(Hold::Locked)).then_some(Hold::Locked),
            (None, _) => Some(hold(window)),
            (Some(held), Some(to)) if hold(to) != held && now - state.since < 0.5 && gravity.is_none_or(|g| g == to) => Some(hold(to)),
            _ => None,
        };
        let Some(held) = want else { return };
        state.held = Some(held);
        win.set_valid_orientations(match held {
            Hold::Landscape => ValidOrientations::Landscape,
            Hold::Portrait => ValidOrientations::Portrait,
            Hold::Locked => ValidOrientations::LandscapeRight,
        });
        // iOS 16 rotates on this rather than winit's attemptRotationToDeviceOrientation
        if let Ok(h) = win.window_handle()
            && let RawWindowHandle::UiKit(h) = h.as_raw()
            && let Some(vc) = h.ui_view_controller
        {
            // SAFETY: the view controller lives as long as the window; respondsToSelector: guards iOS 15
            unsafe {
                let obj: extern "C" fn(Id, Id) -> Id = std::mem::transmute(objc_msgSend as *const ());
                let responds: extern "C" fn(Id, Id, Id) -> bool = std::mem::transmute(objc_msgSend as *const ());
                let sel = sel_registerName(c"setNeedsUpdateOfSupportedInterfaceOrientations".as_ptr());
                if responds(vc.as_ptr(), sel_registerName(c"respondsToSelector:".as_ptr()), sel) {
                    obj(vc.as_ptr(), sel);
                }
            }
        }
        eprintln!("orientation: held {held:?}");
    });
}

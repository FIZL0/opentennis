//! B290: the Settings screen's pages (Display, Graphics, Sound, Controls, Online & Mods); every page but Controls is a
//! column of options, each a row with its value (or volume slider) between ◀ ▶ arrows.
//! B294 (user's mockup, 2026-10-14): the pages are tabs across the title band (L1 / R1), the rows compact in a list
//! panel, a side panel with the chosen option's name, description and choices; no description bar (the legend
//! strip's keys only); laid out on the whole view at any aspect (the select's [`Grid`] says how far it reaches).

use super::super::widescreen::{ASPECTS, aspect_choices};
use super::roster::{self, PX, PY, rounded};
use super::{legend, look, Draw, Effect, Grid, INK, MENU_SIZES, Menu, Press, RENDERERS, SAFE_AREAS, Screen, Settings, Tex, WHITE, controls, pointer, safe_area_name};
use crate::mods_online::Ask;
use crate::graphics::{FPS, Graphics, MESHES, MSAA, SCALES, SHADOWS};
use bevy::prelude::{KeyCode, default};

/// One option row.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Opt {
    Display,
    Aspect,
    /// The menus' size (`widescreen::MENU_SCALE`).
    MenuSize,
    /// B608: how far in from the screen's edges the HUD and menus keep (`widescreen::MARGIN`).
    SafeArea,
    ShowFps,
    /// B301: the match's keyboard hints.
    KeyHints,
    /// B362: the pads' button icons (`legend::LAYOUTS`).
    ButtonIcons,
    Preset,
    Scale,
    Shadows,
    Msaa,
    Fps,
    Vsync,
    Original,
    Upscaled,
    /// B345: leave off-screen characters and effects undrawn (`cull.rs`).
    Cull,
    /// `Volumes`' slider `i`, 0..=10.
    Volume(usize),
    MenuMusic,
    MatchMusic,
    /// B474: the emotes' voice lines.
    EmoteVoices,
    Downloads,
    /// B339: how many store downloads run at once.
    AtOnce,
    /// B289: what to do with an online lobby's mods.
    OnlineMods,
    KeepDownloads,
    /// M20: online chat shown.
    Chat,
    /// B273: the browser build's renderer.
    Renderer,
    /// B118: the on-screen pad buzzes on each press.
    Vibration,
    /// B343: the speed-up switch (`Graphics::fewer_shadows`).
    FewerShadows,
    /// B364: the characters' share of triangles (`Graphics::mesh`, `lod.rs`).
    Mesh,
    /// B374: look for a newer release at start (`crate::update`), desktop only.
    UpdateCheck,
    /// B411: Graphics' last row, ✕ opens the Advanced sub-page (`Screen::Advanced`).
    Advanced,
}
use Opt::*;

/// The Settings tabs, in order, and the pages they open; Mobile (B118) only on a touch screen (`tabs_shown`).
pub(super) const TABS: [(&str, Screen); 6] = [
    ("Display", Screen::Display),
    ("Graphics", Screen::Graphics),
    ("Sound", Screen::Sound),
    ("Controls", Screen::Controls),
    ("Online & Mods", Screen::ModSettings),
    ("Mobile", Screen::Mobile),
];
/// The tabs on show: Mobile's phone-only options on a touch screen.
pub(super) fn tabs_shown() -> &'static [(&'static str, Screen)] {
    if super::touch_page() { &TABS } else { &TABS[..5] }
}
/// `page`'s tab (B411: Advanced's is Graphics).
pub(super) fn tab(page: Screen) -> usize {
    let page = if page == Screen::Advanced { Screen::Graphics } else { page };
    TABS.iter().position(|(_, s)| *s == page).unwrap_or(0)
}

/// The browser build stays in its page (`graphics::KEEP_WINDOWED`), so it has no Display mode row; it has a
/// Renderer instead (B273).
const DISPLAY: &[Opt] = if cfg!(target_arch = "wasm32") { &[Renderer, Aspect, MenuSize, SafeArea, ShowFps, KeyHints, ButtonIcons] } else { &[Display, Aspect, MenuSize, SafeArea, ShowFps, KeyHints, ButtonIcons] };
/// What a preset sets, then the textures, then the way into Advanced.
const GRAPHICS: &[Opt] = &[Preset, Scale, Shadows, Msaa, Fps, Vsync, Original, Upscaled, Advanced];
/// B411: Graphics → Advanced, the speed-up switches for a slow device.
const ADVANCED: &[Opt] = &[Mesh, FewerShadows, Cull];
const SOUND: &[Opt] = &[Volume(0), Volume(1), Volume(2), Volume(3), Volume(4), MenuMusic, MatchMusic, EmoteVoices];
const MODS: &[Opt] = if cfg!(target_arch = "wasm32") { &[Downloads, AtOnce, OnlineMods, KeepDownloads, Chat] } else { &[Downloads, AtOnce, OnlineMods, KeepDownloads, Chat, UpdateCheck] };
/// B118: phone-only options (later: control size and opacity).
const MOBILE: &[Opt] = &[Vibration];
/// B289: Online Mod Downloads' values.
const ASK_VALUES: [(Ask, &str); 3] = [(Ask::Each, "Ask"), (Ask::Always, "Always"), (Ask::Never, "Never")];

/// Page `s`'s options (none on a screen that isn't an options page).
pub(super) fn page(s: Screen) -> &'static [Opt] {
    match s {
        Screen::Display => DISPLAY,
        Screen::Graphics => GRAPHICS,
        Screen::Advanced => ADVANCED,
        Screen::Sound => SOUND,
        Screen::ModSettings => MODS,
        Screen::Mobile => MOBILE,
        _ => &[],
    }
}

/// The options the browser build keeps off and locked (B227: Upscaled Downloads, `modstore::install`; P54:
/// Anti-aliasing on WebGL2, `graphics::webgl2`; M5a: Texture Replacements is open there, for the texture packs).
pub(super) fn locked(o: Opt) -> bool {
    cfg!(target_arch = "wasm32") && matches!(o, Downloads) || o == Msaa && crate::graphics::webgl2()
}

fn label(o: Opt) -> &'static str {
    match o {
        Display => "Display",
        Aspect => "Aspect Ratio",
        MenuSize => "Menu Size",
        SafeArea => "Safe Area",
        ShowFps => "Show FPS",
        KeyHints => "Key Hints",
        ButtonIcons => "Button Icons",
        Preset => "Preset",
        Scale => "Resolution",
        Shadows => "Shadows",
        Msaa => "Anti-aliasing",
        Fps => "Frame Limit",
        Vsync => "VSync",
        Original => "Original Mode",
        Upscaled => "Texture Replacements",
        Cull => "Skip Off-Screen",
        Volume(i) => ["Master", "Music", "Sound Effects", "Voices", "Ambience"][i],
        MenuMusic => "Menu Music",
        MatchMusic => "Match Music",
        EmoteVoices => "Emote Voices",
        Downloads => "Upscaled Downloads",
        AtOnce => "Downloads at Once",
        OnlineMods => "Online Mod Downloads",
        KeepDownloads => "Keep Online Downloads",
        Chat => "Chat",
        Renderer => "Renderer",
        Vibration => "Vibration",
        UpdateCheck => "Updates",
        FewerShadows => "Fewer Shadows",
        Mesh => "Mesh",
        Advanced => "Advanced",
    }
}

fn about(o: Opt) -> &'static str {
    match o {
        Display => "Borderless fullscreen on the current monitor, or a window.",
        Aspect => aspect_about(),
        MenuSize => "Makes every menu smaller or bigger. The whole layout scales together, never stretched; the extra room goes around it.",
        SafeArea => "Keeps the menus and the match's score clear of the screen's edges; the picture always fills the screen. Auto keeps them off a phone's notch, camera cutout, rounded corners and home bar; a margin keeps them that far in from every edge as well (for a TV that crops its edges); None lets them reach the edges.",
        ShowFps => "Show the frame rate in the bottom-left corner, in the menus and in a match. Detailed: where each frame's time goes (game, simulation, rendering, waiting on the GPU/browser), meshes drawn and the renderer.",
        KeyHints => "Show the keyboard's keys at the bottom during a match while you play on the keyboard.",
        ButtonIcons => "The controller buttons the hints show. Auto: each controller's own (PlayStation, Switch, Steam Deck, else Xbox); or the same set for every controller.",
        Preset => "High draws at full quality with every speed-up off; Default looks the same with Fewer Shadows and Skip Off-Screen on; Steam Deck looks like High with vsync and the speed-ups on; Low and Phone trade detail for frame rate; Lowest is the cheapest (half resolution, no shadows, simpler characters) for the weakest devices. Auto picks one for this device, then times the first seconds of each match and lowers the resolution and shadows until it holds 60 fps.",
        Scale => "Draw the court at this share of the window's resolution, scaled up to fill it (the HUD stays sharp). Above 100 % draws it larger and scales it down: smoother edges, more work for the graphics card.",
        Shadows => "Off, or the shadow map's detail (Low: one cascade, and only what shades the court casts).",
        Msaa if locked(o) => "Off on WebGL2: anti-aliasing makes the browser build every drawing setup a second time, which stalls the first match for seconds. WebGPU (Display → Renderer) or the desktop app has it.",
        Msaa => "Smooth the court's jagged edges (MSAA): off, 2x or 4x samples per pixel.",
        Fps => "The most frames drawn a second (the game itself always runs at 60 Hz).",
        Vsync => "Wait for the display between frames: no tearing, at most its refresh rate.",
        Mesh => "Draw the players, their rackets and the court's figures (umpire, ball kids, crowd) with this share of their triangles: lower runs faster on a weak GPU, most of all with detailed mod characters. Faces and the edges between parts keep their shape. Applies to characters loaded after a change (the next match).",
        Original => "Draw at the PS2's 640x448, 4:3, with the disc's textures, scaled up unfiltered.",
        Downloads if locked(o) => "Upscaled textures aren't available in the browser.",
        Upscaled if cfg!(target_arch = "wasm32") => "Draw the installed texture packs' textures in place of the disc's (from the next match).",
        Upscaled => "Draw the installed texture packs' textures and the ones in mods/texture-replacements in place of the disc's (from the next match), and mods' upscaled textures.",
        Cull => "Don't draw players, figures and effects that are out of view (or their shadows out of the light's). Looks the same; turn it off if it makes this device slower.",
        Volume(0) => "The volume of everything.",
        Volume(1) => "The menu and match music and the jingles.",
        Volume(2) => "The ball, the crowd, the weather and the menu sounds.",
        Volume(3) => "The players' and the umpire's voices.",
        Volume(_) => "The court's surroundings: birds, surf, chimes and court mods' ambience.",
        MenuMusic => "Play music in the menus.",
        MatchMusic => "Play each court's music during a match.",
        EmoteVoices => "Hear the players' voice lines when they emote. Off mutes them on this device only; the emote itself still plays.",
        Downloads => "Download mods' upscaled textures from the mod store (most of a mod's size).",
        AtOnce => "How many mods the mod store downloads side by side (Download all, or several installs). Auto: 3 on a computer, 2 in a browser, 1 on a phone; a browser keeps each one in memory until it's saved, so it allows fewer.",
        OnlineMods => "When an online lobby has a mod you don't have: ask each time, always download it, or never (a stand-in plays it).",
        KeepDownloads => "After an online match, the mods you downloaded for it: ask whether to add them to your mods, always add them, or never.",
        Chat => "Show the online lobby's and match's text chat (T, or R2/L2 on a pad), or hide it.",
        Renderer => "Auto: WebGPU when the browser has it, else WebGL2; the one in use shows in brackets, with Hardware (drawn by the graphics card) or Software (drawn by the CPU, much slower: turn on the browser's hardware acceleration). A change asks first: the page reloads into it.",
        Vibration => vibration_about(),
        UpdateCheck => "Look for a newer version while the game runs. Ask: the main menu asks Update? the first time it finds one. Always: it installs by itself where it can (a phone's installer still asks you to confirm). Never: no looking.",
        FewerShadows => "At Low shadows, only the players, the umpire and what shades the court cast: fewer draws. Off: Low casts as many as High.",
        Advanced => "Switches that trade a little detail for speed on a slow device: the characters' mesh detail, fewer shadows, skipping what's off-screen.",
    }
}

/// B608: Stretch is gone, Auto now fills the screen everywhere (the Safe Area setting keeps the menus off the notch).
fn aspect_about() -> &'static str {
    "Auto fills the screen; a fixed ratio shows the whole picture at that shape, black bars around it."
}

/// B329: the Vibration row's text, by what this browser can do (`web/touch.js`).
fn vibration_about() -> &'static str {
    #[cfg(target_arch = "wasm32")]
    let can = crate::web::vibration();
    #[cfg(target_arch = "wasm32")]
    let can = can.as_str();
    #[cfg(not(target_arch = "wasm32"))]
    let can = if cfg!(any(target_os = "ios", target_os = "android")) { "haptic" } else { "vibrate" };
    match can {
        "haptic" => "A light haptic tick on each press of the on-screen controls.",
        "vibrate" => "Buzz briefly on each press of the on-screen controls. Android: not while the phone's ringer is on Silent.",
        "firefox" => "Firefox for Android doesn't let web pages vibrate, so this does nothing here. Chrome on Android buzzes.",
        _ => "This browser can't vibrate, so this does nothing here.",
    }
}

/// A Menu Size value's word (`MENU_SIZES`: 0 Auto, else tenths).
fn menu_size_name(t: u8) -> String {
    if t == 0 { "Auto".into() } else { format!("{}%", t as u32 * 10) }
}

fn fps_name(fps: u32) -> String {
    if fps == 0 { "Uncapped".into() } else { format!("{fps} fps") }
}

/// B316: the words of the rows whose values are small indices, shared by [`choices`] and [`Settings::value`].
const SHOW_FPS_NAMES: [&str; 3] = ["Off", "On", "Detailed"];
const SHADOW_NAMES: [&str; 3] = ["Off", "Low", "High"];
const MSAA_NAMES: [&str; 3] = ["Off", "2x", "4x"];

/// B294: every value `o` can take, in order, as [`Settings::value`] words it (the side panel's choices).
fn choices(o: Opt) -> Vec<String> {
    let words = |v: &[&str]| v.iter().map(|w| w.to_string()).collect();
    match o {
        _ if locked(o) => words(&["Off"]),
        Display => words(&["Fullscreen", "Window"]),
        Aspect => words(&ASPECTS[..aspect_choices()]),
        MenuSize => MENU_SIZES.iter().map(|&t| menu_size_name(t)).collect(),
        SafeArea => SAFE_AREAS.iter().map(|&m| safe_area_name(m)).collect(),
        Preset => Graphics::PRESETS.iter().map(|(n, _)| n.to_string()).chain([AUTO.into(), "Custom".into()]).collect(),
        Scale => SCALES.iter().map(|s| format!("{s} %")).collect(),
        Mesh => MESHES.iter().map(|s| format!("{s} %")).collect(),
        Shadows => words(&SHADOW_NAMES),
        Msaa => words(&MSAA_NAMES),
        ShowFps => words(&SHOW_FPS_NAMES),
        Renderer => RENDERERS.iter().map(|(_, n)| n.to_string()).collect(),
        ButtonIcons => super::legend::LAYOUTS.iter().map(|l| l.0.to_string()).collect(),
        Fps => FPS.map(fps_name).to_vec(),
        Volume(_) => (0..=10).map(|v| v.to_string()).collect(),
        OnlineMods | KeepDownloads | UpdateCheck => ASK_VALUES.iter().map(|(_, n)| n.to_string()).collect(),
        AtOnce => (0..=super::max_downloads()).map(at_once_name).collect(),
        Advanced => Vec::new(),
        _ => words(&["Off", "On"]),
    }
}

/// B498: rows with this many choices or more open a dropdown list on ✕ (On/Off keeps the ‹ › toggle; user 2026-10-17:
/// every row with more than two choices is a dropdown).
const LIST: usize = 3;

/// B498: whether `o` opens a dropdown (a slider or a way in never does).
pub(super) fn listed(o: Opt) -> bool {
    !locked(o) && !matches!(o, Volume(_) | Advanced) && choices(o).len() >= LIST
}

/// B498: a dropdown choice's short hint (the presets'), so Low and Phone tell apart without the side panel.
fn hint(o: Opt, c: &str) -> &'static str {
    match (o, c) {
        (Preset, "High") => "full quality, no speed-ups",
        (Preset, "Default") => "High with speed-ups on",
        (Preset, "Steam Deck") => "High's look, vsync, speed-ups",
        (Preset, "Low") => "less detail, more fps",
        (Preset, "Lowest") => "half res, no shadows",
        (Preset, "Phone") => "for phones and tablets",
        (Preset, AUTO) => "picks and tunes itself",
        (Preset, "Custom") => "your own mix",
        _ => "",
    }
}

/// B339: a Downloads at Once value's word (0 Auto).
fn at_once_name(n: usize) -> String {
    if n == 0 { "Auto".into() } else { n.to_string() }
}

fn on_off(on: bool) -> String {
    if on { "On" } else { "Off" }.into()
}

/// The next (`d` = 1) or previous (−1) of `list` after `cur` (the first if `cur` isn't in it).
fn cycle<T: PartialEq + Copy>(list: &[T], cur: T, d: isize) -> T {
    let i = list.iter().position(|x| *x == cur).map_or(0, |i| (i as isize + d).rem_euclid(list.len() as isize) as usize);
    list[i]
}

/// P53: the Preset row's word for `Graphics::auto`.
const AUTO: &str = "Auto";

impl Settings {
    /// M140: the Updates row's choice: Never is Check for Updates off, else Ask or Always.
    fn updates(&self) -> Ask {
        match (self.update_check, self.updates_always) {
            (false, _) => Ask::Never,
            (true, true) => Ask::Always,
            (true, false) => Ask::Each,
        }
    }

    /// Change `o` one step in direction `d`.
    pub(super) fn step(&mut self, o: Opt, d: isize) {
        let g = &mut self.graphics;
        // P53: setting the graphics by hand ends Auto's timing
        g.auto &= !matches!(o, Scale | Shadows | Msaa | Fps | Vsync | Original | Cull | FewerShadows | Mesh);
        match o {
            Display => g.windowed = !g.windowed,
            Aspect => self.aspect = (self.aspect as isize + d).rem_euclid(aspect_choices() as isize) as u8,
            MenuSize => self.menu_size = cycle(&MENU_SIZES, self.menu_size, d),
            SafeArea => self.safe_area = cycle(&SAFE_AREAS, self.safe_area, d),
            ShowFps => g.show_fps = (g.show_fps as isize + d).rem_euclid(3) as u8,
            KeyHints => self.key_hints = !self.key_hints,
            ButtonIcons => self.button_icons = (self.button_icons as isize + d).rem_euclid(super::legend::LAYOUTS.len() as isize) as u8,
            // P53: Auto after the presets: the device's pick, timed in matches
            Preset => {
                let names: Vec<_> = Graphics::PRESETS.iter().map(|(n, _)| *n).chain([AUTO]).collect();
                let n = cycle(&names, if g.auto { AUTO } else { g.preset() }, d);
                self.preset(n);
            }
            Scale => g.scale = cycle(&SCALES, g.scale, d),
            Shadows => g.shadows = cycle(&SHADOWS, g.shadows, d),
            Msaa => g.msaa = cycle(&MSAA, g.msaa, d),
            Fps => g.fps = cycle(&FPS, g.fps, d),
            Vsync => g.vsync = !g.vsync,
            Original => g.original = !g.original,
            Cull => g.cull = !g.cull,
            // Upscaled Downloads follows Texture Replacements (B227)
            Upscaled => {
                self.upscaled_textures = !self.upscaled_textures;
                self.upscaled_downloads = self.upscaled_textures;
            }
            Volume(i) => {
                let v = &mut self.volumes.0[i];
                *v = (*v as isize + d).clamp(0, 10) as _;
            }
            MenuMusic => self.menu_music = !self.menu_music,
            MatchMusic => self.music = !self.music,
            EmoteVoices => {
                self.emote_voices = !self.emote_voices;
                crate::audio::EMOTE_VOICES.store(self.emote_voices, std::sync::atomic::Ordering::Relaxed);
            }
            Downloads => self.upscaled_downloads = !self.upscaled_downloads,
            AtOnce => self.downloads = (self.downloads as isize + d).rem_euclid(super::max_downloads() as isize + 1) as u8,
            KeepDownloads => self.keep_downloads = cycle(&ASK_VALUES.map(|(a, _)| a), self.keep_downloads, d),
            OnlineMods => self.online_mods = cycle(&ASK_VALUES.map(|(a, _)| a), self.online_mods, d),
            Chat => {
                self.chat = !self.chat;
                crate::chat::HIDDEN.store(!self.chat, std::sync::atomic::Ordering::Relaxed);
            }
            Renderer => self.renderer = (self.renderer as isize + d).rem_euclid(RENDERERS.len() as isize) as u8,
            Vibration => self.vibration = !self.vibration,
            // M140: Ask / Always / Never, Never being the older Check for Updates off
            UpdateCheck => {
                let a = cycle(&ASK_VALUES.map(|(a, _)| a), self.updates(), d);
                self.update_check = a != Ask::Never;
                crate::update::CHECK.store(self.update_check, std::sync::atomic::Ordering::Relaxed);
                self.updates_always = a == Ask::Always;
                crate::update::ALWAYS.store(self.updates_always, std::sync::atomic::Ordering::Relaxed);
            }
            FewerShadows => g.fewer_shadows = !g.fewer_shadows,
            Mesh => g.mesh = cycle(&MESHES, g.mesh, d),
            Advanced => {}
        }
    }

    /// Preset `n` (one of `Graphics::PRESETS`' names, or Auto: the device's pick); the window and Show FPS stay.
    fn preset(&mut self, n: &str) {
        let g = &mut self.graphics;
        *g = match Graphics::PRESETS.iter().find(|(m, _)| *m == n) {
            Some((_, p)) => Graphics { auto: false, ..g.keep(*p) },
            None => Graphics { windowed: g.windowed, show_fps: g.show_fps, ..super::auto_graphics() },
        };
    }

    /// B498: set `o` to its `j`th choice ([`choices`]), as stepping to it would (Preset's Custom changes nothing).
    pub(super) fn pick(&mut self, o: Opt, j: usize) {
        let all = choices(o);
        let Some(want) = all.get(j) else { return };
        if self.value(o) == *want || o == Preset && want == "Custom" {
            return;
        }
        if o == Preset {
            self.preset(want);
            return;
        }
        let was = *self;
        for _ in 0..all.len() {
            self.step(o, 1);
            if self.value(o) == *want {
                return;
            }
        }
        *self = was;
    }

    /// The value shown beside `o` (a slider's is its number).
    pub(super) fn value(&self, o: Opt) -> String {
        let g = &self.graphics;
        match o {
            _ if locked(o) => "Off".into(),
            Display => if g.windowed { "Window" } else { "Fullscreen" }.into(),
            Aspect => ASPECTS[self.aspect as usize].into(),
            MenuSize => menu_size_name(self.menu_size),
            SafeArea => safe_area_name(self.safe_area),
            ShowFps => SHOW_FPS_NAMES[g.show_fps.min(2) as usize].into(),
            KeyHints => on_off(self.key_hints),
            ButtonIcons => super::legend::LAYOUTS[self.button_icons as usize].0.into(),
            Preset if g.auto => AUTO.into(),
            Preset => g.preset().into(),
            Scale => format!("{} %", g.scale),
            Shadows => SHADOW_NAMES[g.shadows as usize].into(),
            Msaa => MSAA_NAMES[g.msaa.ilog2().min(2) as usize].into(),
            Fps => fps_name(g.fps),
            Vsync => on_off(g.vsync),
            Original => on_off(g.original),
            Cull => on_off(g.cull),
            Mesh => format!("{} %", g.mesh),
            Upscaled => on_off(self.upscaled_textures),
            Volume(i) => self.volumes.0[i].to_string(),
            MenuMusic => on_off(self.menu_music),
            MatchMusic => on_off(self.music),
            EmoteVoices => on_off(self.emote_voices),
            KeepDownloads => ASK_VALUES.iter().find(|(a, _)| *a == self.keep_downloads).unwrap().1.into(),
            Downloads => on_off(self.upscaled_downloads),
            AtOnce => at_once_name(self.downloads as usize),
            OnlineMods => ASK_VALUES.iter().find(|(a, _)| *a == self.online_mods).unwrap().1.into(),
            Chat => on_off(self.chat),
            Renderer => RENDERERS[self.renderer as usize].1.into(),
            Vibration => on_off(self.vibration),
            UpdateCheck => ASK_VALUES.iter().find(|(a, _)| *a == self.updates()).unwrap().1.into(),
            FewerShadows => on_off(g.fewer_shadows),
            Advanced => ">".into(),
        }
    }
}

impl Menu {
    /// An options page (B326, as the confirm screen's Sets and Games): ✕ on a row picks it, ◀ ▶ then change it (applied
    /// and saved at once), ✕ keeps it, ○ puts back what it was; ◀ ▶ on a row not picked do nothing; L1 / R1 and ○ as
    /// [`Self::step_tabs`].
    /// B498: a row with many choices ([`listed`]) opens a dropdown on ✕ instead ([`Self::step_list`]), and its ◀ ▶
    /// change it at once, unopened.
    pub(super) fn step_page(&mut self, k: Press, out: &mut Vec<Effect>) {
        let o = page(self.screen).get(self.sel).copied().filter(|&o| !locked(o));
        // B411: Advanced is never picked (it opens its page)
        if o == Some(Advanced) {
            self.editing = None;
        }
        if let Some(j) = self.listing {
            self.step_list(o, j, k, out);
            return;
        }
        if let Some(was) = self.editing {
            let Some(o) = o else {
                self.editing = None;
                return;
            };
            if k.left || k.right {
                let now = self.settings;
                self.settings.step(o, if k.left { -1 } else { 1 });
                if self.settings != now {
                    out.extend([Effect::Sound(0), Effect::SaveSettings]);
                }
            } else if k.ok {
                self.editing = None;
                out.push(Effect::Sound(2));
            } else if k.back {
                self.editing = None;
                if self.settings != was {
                    self.settings = was;
                    crate::chat::HIDDEN.store(!was.chat, std::sync::atomic::Ordering::Relaxed);
                    crate::audio::EMOTE_VOICES.store(was.emote_voices, std::sync::atomic::Ordering::Relaxed);
                    out.push(Effect::SaveSettings);
                }
                out.push(Effect::Sound(1));
            }
            return;
        }
        // B411: Advanced opens its page instead of being picked
        if k.ok && o == Some(Advanced) {
            self.go(Screen::Advanced, out);
            return;
        }
        if let Some(o) = o.filter(|&o| listed(o)) {
            if k.ok {
                self.editing = Some(self.settings);
                self.listing = Some(choices(o).iter().position(|c| *c == self.settings.value(o)).unwrap_or(0));
                out.push(Effect::Sound(2));
                return;
            }
            // (the Renderer changes only through its list and the popup: the page reloads)
            if (k.left || k.right) && o != Renderer {
                let now = self.settings;
                self.settings.step(o, if k.left { -1 } else { 1 });
                if self.settings != now {
                    out.extend([Effect::Sound(0), Effect::SaveSettings]);
                }
                return;
            }
        }
        if k.ok && o.is_some() {
            self.editing = Some(self.settings);
            out.push(Effect::Sound(2));
            return;
        }
        self.step_tabs(k, out);
    }

    /// B498: one press while row `o`'s dropdown is open on choice `j`: ↑ ↓ move (wrapping), ✕ picks it (applied and
    /// saved) and closes, ○ closes it unchanged; nothing else.
    fn step_list(&mut self, o: Option<Opt>, j: usize, k: Press, out: &mut Vec<Effect>) {
        let Some(o) = o.filter(|&o| listed(o)) else {
            (self.listing, self.editing) = (None, None);
            return;
        };
        let n = choices(o).len();
        if k.up || k.down {
            self.listing = Some((j + if k.down { 1 } else { n - 1 }) % n);
            out.push(Effect::Sound(0));
        } else if k.ok && o == Renderer {
            // B327: a different renderer asks first (the page reloads into it); the one in use just closes the list
            (self.listing, self.editing) = (None, None);
            if j as u8 != self.settings.renderer {
                self.switch = Some((j as u8, 0));
            }
            out.push(Effect::Sound(2));
        } else if k.ok {
            (self.listing, self.editing) = (None, None);
            let now = self.settings;
            self.settings.pick(o, j);
            out.push(Effect::Sound(2));
            if self.settings != now {
                out.push(Effect::SaveSettings);
            }
        } else if k.back {
            (self.listing, self.editing) = (None, None);
            out.push(Effect::Sound(1));
        }
    }

    /// B327: one press while the Renderer switch's popup is up (it takes them all; it opens on a renderer picked from
    /// the row's list): ↑ ↓ move between Switch and Cancel, ✕ chooses, ○ is Cancel. Switch saves the setting, which reloads
    /// the page into it (`web::renderer`); Cancel leaves it as it was. Returns whether the popup was up.
    pub(super) fn switch_step(&mut self, k: Press, out: &mut Vec<Effect>) -> bool {
        let Some((r, c)) = self.switch else { return false };
        if k.up || k.down {
            self.switch = Some((r, 1 - c));
            out.push(Effect::Sound(0));
        } else if k.ok && c == 0 {
            self.settings.renderer = r;
            self.switch = None;
            out.extend([Effect::Sound(2), Effect::SaveSettings]);
        } else if k.ok || k.back {
            self.switch = None;
            out.push(Effect::Sound(1));
        }
        true
    }

    /// Any Settings page: L1 / R1 open the tab before / after it (wrapping), ○ goes back to the main menu's
    /// Settings row (closes the pause overlay); B411: on Advanced, ○ goes back to Graphics' Advanced row.
    pub(super) fn step_tabs(&mut self, k: Press, out: &mut Vec<Effect>) {
        if k.back && self.screen == Screen::Advanced {
            self.screen = Screen::Graphics;
            self.sel = GRAPHICS.len() - 1;
            out.push(Effect::Sound(1));
        } else if k.prev || k.next {
            let tabs = tabs_shown();
            let n = tabs.len();
            self.go_tab(tabs[(tab(self.screen) + if k.next { 1 } else { n - 1 }) % n].1, out);
        } else if k.back && self.settings_back() {
            // B558: back to the select or lobby they were opened from
            out.push(Effect::Sound(1));
        } else if k.back {
            self.sel = self.main_at(super::home::Row::Settings);
            self.screen = Screen::Main;
            out.push(Effect::Sound(1));
        }
    }

    /// B442: open Settings tab `page` (L1 / R1, a tap on its pill) with the soft cursor tick, not the select cue.
    pub(super) fn go_tab(&mut self, page: Screen, out: &mut Vec<Effect>) {
        self.screen = page;
        self.sel = 0;
        // B291: Controls opens on the column of the device last used (a phone has no keyboard)
        self.keys_tab = !super::legend::MOBILE && super::legend::last_style() == super::legend::Style::Keys;
        out.push(Effect::Sound(0));
    }
}

/// B294: the Settings pages' list (of `n` rows) and side panel on the view `g` fits (the side panel under the list in a portrait
/// window), from under the top bar down to the legend strip. B448: as the Mods page's panels (`look`), on the view a
/// portrait window draws larger (`zoom`); B471: a portrait view's page tabs in a row of their own over them.
pub(super) fn boxes(g: &Grid, n: usize) -> ([f32; 4], [f32; 4]) {
    let (portrait, zoom) = zoom(g);
    let [l, t, r, b] = zoom.inner(g.view);
    let (x0, x1) = (l + look::MARGIN * PX, r - look::MARGIN * PX);
    let w = x1 - x0;
    let top = t + look::TOP_BAR * PY + if portrait { (look::PILL + look::ROW_GAP) * PY } else { 4.0 * PY };
    let h = b - look::LEGEND - top;
    let gap = look::GAP;
    if portrait {
        // the list as tall as the page's `n` rows want, the panel the rest
        let lh = (2.0 * look::RIM + look::STRIP + 2.5 + FOOT + 2.0 * look::PAD) * PY + PITCH * PY * n as f32;
        let lh = lh.min(h - 160.0 - gap * PY);
        ([x0, top, w, lh], [x0, top + lh + gap * PY, w, h - lh - gap * PY])
    } else {
        let dw = (340.0 * PX).min(w * 0.4);
        ([x0, top, w - dw - gap * PX, h], [x1 - dw, top, dw, h])
    }
}

/// B608: whether page `s`'s rows keep their full pitch on the view `g` (a bigger Menu Size on a short view would
/// squeeze them, `slot`).
pub(super) fn rows_fit(g: &Grid, s: Screen) -> bool {
    let n = if s == Screen::Controls { controls::action_names().count() + 1 } else { page(s).len() };
    let (list, _) = boxes(g, n);
    slot(list, n, if pinned(s) { 1.0 } else { 0.0 }, 0.0).1 >= PITCH * PY - 1e-3
}

/// B448: a list's foot (the footnote) and a row's pitch, mockup px.
const FOOT: f32 = 30.0;
const PITCH: f32 = 46.0;

/// B471: the header strip (mockup px) of a list of `n` rows: the main menu's, slimmer where the rows wouldn't keep
/// their full pitch under it (a very wide window's Controls).
fn strip(list: [f32; 4], n: usize) -> f32 {
    let full = (2.0 * look::RIM + look::STRIP + 2.5 + 2.0 * look::PAD + FOOT + PITCH * n as f32) * PY;
    if list[3] >= full { look::STRIP } else { 30.0 }
}

/// B471: the light green of the side panel's strip.
fn side_band() -> [f32; 3] {
    look::c(look::MINT)
}

/// Whether `g` is a portrait view, and the zoom its page draws with (`look::Zoom`).
fn zoom(g: &Grid) -> (bool, look::Zoom) {
    let portrait = g.detail[0] <= g.strip[0];
    (portrait, look::Zoom::of(g.view, portrait))
}

/// Row `i` of `n` in the list `list` (under its strip, over its footnote): pills 46 px apart, closer when they'd run
/// past the footnote.
pub(super) fn row(list: [f32; 4], n: usize, i: usize) -> [f32; 4] {
    slot(list, n, 0.0, i as f32).0
}

/// Row `at` (a fractional place) of `n` rows and `more` rows' room besides in the list `list`; and the pitch.
fn slot(list: [f32; 4], n: usize, more: f32, at: f32) -> ([f32; 4], f32) {
    let inset = look::RIM + look::PAD;
    let top = list[1] + (look::RIM + strip(list, n) + 2.5 + look::PAD) * PY;
    let room = list[1] + list[3] - (inset + FOOT) * PY - top;
    let pitch = (room / (n as f32 + more)).min(PITCH * PY);
    let gap = (pitch * 0.12).min(look::ROW_GAP * PY);
    ([list[0] + inset * PX, top + pitch * at, list[2] - 2.0 * inset * PX, pitch - gap], pitch)
}

/// B498: whether page `s`'s first row (Graphics' Preset) sits alone over the rest: its "Start here" label over it
/// and a divider under it, half a row's room each.
fn pinned(s: Screen) -> bool {
    s == Screen::Graphics
}

/// Row `i` of page `s`'s `n` in the list `list` (B498: past a pinned first row's label and divider).
pub(super) fn page_row(s: Screen, list: [f32; 4], n: usize, i: usize) -> [f32; 4] {
    if !pinned(s) {
        return row(list, n, i);
    }
    slot(list, n, 1.0, i as f32 + if i == 0 { 0.5 } else { 1.0 }).0
}

/// B498: a pinned first row's "START HERE" label over it and the dashed divider under it.
fn pin_marks(d: &mut Draw, list: [f32; 4], n: usize) {
    let ([x, y, w, h], pitch) = slot(list, n, 1.0, 0.5);
    let soft = look::c(look::MUTED);
    let hs = (11.0 * PY).min(pitch * 0.4);
    d.text("START HERE", x + 16.0 * PX, y - pitch * 0.25 - hs / 2.0 - 1.0 * PY, hs, soft);
    let ry = y + h + (pitch * 0.5 + pitch - h) / 2.0 - 1.0 * PY;
    let (dash, step) = (10.0 * PX, 18.0 * PX);
    let mut dx = x + 8.0 * PX;
    while dx + dash <= x + w - 8.0 * PX {
        d.q(Tex::Solid, [0.0; 4], [dx, ry, dash, 2.0 * PY], soft, 128.0);
        dx += step;
    }
}

/// B471: the page in the main menu's look: the wall, the blue title band with its "Settings" tab, the page tabs
/// between L1 / R1 (a tap opens one) in the top bar, the list panel with the page's name and count on its blue strip
/// over its rows and its footnote under them, and the side panel; the list box and the side box. The caller draws
/// the rows and the side panel's words.
fn chrome(d: &mut Draw, m: &Menu, n: usize, name: &str, foot: &str) -> ([f32; 4], [f32; 4]) {
    let (list, side) = boxes(&m.grid, n);
    settings_bar(d, m, side[0] + side[2], 0.0);
    let [sx, sy, sw, sh] = look::panel(d, list, strip(list, n), look::c(look::BLUE));
    // the page's name outlined white and its count of rows on the strip
    let (ix, iw) = (sx + look::PAD * PX, sw - 2.0 * look::PAD * PX);
    let hs = 20.0 * PY;
    // (B291: Controls' tabs take the strip after its name instead of a count)
    let count = if m.screen == Screen::Controls { String::new() } else { format!("{n} {}", if n == 1 { "option" } else { "options" }) };
    let cs = 13.0 * PY;
    let cw = d.width(&count, cs);
    let title = roster::ellipsis(d, name, hs, iw - cw - 12.0 * PX);
    look::bold(d, &title, [ix, sy + (sh - hs) / 2.0], hs, WHITE);
    look::bold(d, &count, [ix + iw - cw, sy + (sh - cs) / 2.0], cs, WHITE);
    footnote(d, m, list, foot);
    look::panel(d, side, strip(list, n), side_band());
    (list, side)
}

/// The wall's blue title band with its "Settings" tab and the page tabs between L1 / R1 (a tap opens one) in the top
/// bar (a portrait view: a row of their own under it), ending at `right` less `reserve` mockup px (B609: Controls'
/// Reset button there).
fn settings_bar(d: &mut Draw, m: &Menu, right: f32, reserve: f32) {
    let (portrait, z) = zoom(&m.grid);
    let view = z.inner(m.grid.view);
    let tab_end = look::top(d, m.grid.view, "Settings", look::c(look::BLUE));
    let tabs = tabs_shown();
    let words: Vec<String> = tabs.iter().map(|t| t.0.to_string()).collect();
    let on = tabs.iter().position(|t| t.1 == TABS[tab(m.screen)].1);
    let x = if portrait { view[0] + look::MARGIN * PX } else { tab_end };
    let ty = if portrait { view[1] + look::TOP_BAR * PY } else { look::bar_y(view[1]) };
    look::tabs(d, [x, ty, right - reserve * PX - x], &words, look::Tabs::Bar, on, None, Some((5, 6)), [false; 2], pointer::Act::Tab);
}

/// A panel `list`'s footnote under its rows, over a light rule (B558: over the select or the lobby, the host's
/// countdown in its place).
fn footnote(d: &mut Draw, m: &Menu, list: [f32; 4], foot: &str) {
    let inset = look::RIM + look::PAD;
    let (ix, iw) = (list[0] + inset * PX, list[2] - 2.0 * inset * PX);
    let fs = 12.5 * PY;
    let fy = list[1] + list[3] - (inset + FOOT) * PY;
    d.q(Tex::Solid, [0.0; 4], [ix, fy + 2.0 * PY, iw, 2.0 * PY], look::c(look::RULE), 128.0);
    let ty = fy + (FOOT * PY - fs) / 2.0 + 3.0 * PY;
    match super::over::countdown(m) {
        Some(words) => d.text(&words, ix, ty, fs, look::c(look::BLUE)),
        None => d.text(&roster::ellipsis(d, foot, fs, iw), ix, ty, fs, look::c(look::MUTED)),
    }
}

/// The side panel (its light green strip, `strip` mockup px high, drawn by `chrome`): the name outlined on the strip, what it does, a rule
/// headed `heading` (CHOICES) over its choices as pills (the current one blue with white words, the rest light) and a
/// footnote.
#[allow(clippy::too_many_arguments)]
fn side_panel(d: &mut Draw, side: [f32; 4], strip: f32, name: &str, about: &str, heading: &str, chips: &[String], now: Option<&str>, note: &str) {
    let [x, y, w, _] = side;
    let (x, w) = (x + (look::RIM + look::PAD) * PX, w - 2.0 * (look::RIM + look::PAD) * PX);
    let sh = strip * PY;
    let ns = (20.0 * PY * (w / d.width(name, 20.0 * PY).max(1.0)).min(1.0)).max(14.0 * PY);
    let shown = roster::ellipsis(d, name, ns, w);
    look::bold(d, &shown, [x, y + look::RIM * PY + (sh - ns) / 2.0], ns, WHITE);
    let (ink, soft) = (look::c(look::INK), look::c(look::MUTED));
    let mut y = y + (look::RIM + 2.5) * PY + sh + 14.0 * PY;
    y = look::para(d, about, [x, y], w, 14.0 * PY, 19.0 * PY, 8, ink) + 14.0 * PY;
    if !chips.is_empty() {
        // the rule: its heading in small capitals, a light line on to the edge
        let hs = 11.0 * PY;
        let head = heading.to_uppercase();
        d.text(&head, x, y, hs, soft);
        let hx = x + d.width(&head, hs) + 8.0 * PX;
        d.q(Tex::Solid, [0.0; 4], [hx, y + hs / 2.0 - 1.0 * PY, x + w - hx, 2.0 * PY], look::c(look::RULE), 128.0);
        y += hs + 10.0 * PY;
        let (ph, cs) = (26.0 * PY, 13.0 * PY);
        let mut cx = x;
        for c in chips {
            let cw = (d.width(c, cs) + 24.0 * PX).min(w);
            if cx + cw > x + w && cx > x {
                (cx, y) = (x, y + ph + 6.0 * PY);
            }
            let cur = now == Some(c.as_str());
            let b = [cx, y, cw, ph];
            look::outlined_pill(d, b, look::c(if cur { look::BLUE } else { look::TILE }), 2.0);
            look::pill_words(d, b, c, cs, if cur { WHITE } else { ink });
            cx += cw + 6.0 * PX;
        }
        y += ph + 14.0 * PY;
    }
    look::para(d, note, [x, y], w, 12.5 * PY, 17.0 * PY, 5, soft);
}

/// B471: a list row: the main menu's grey pill (the chosen one yellow), its `label` on the left (white, dark on
/// yellow; `dim` faded).
fn row_pill(d: &mut Draw, r @ [x, y, _, h]: [f32; 4], on: bool, label: &str, room: f32, dim: bool) -> f32 {
    look::button(d, r, on);
    let size = (h * 0.42).min(15.0 * PY);
    let n0 = d.out.len();
    d.text(&roster::ellipsis(d, label, size, room), x + 16.0 * PX, y + (h - size) / 2.0, size, if on { look::c(look::INK) } else { WHITE });
    if dim {
        d.out[n0..].iter_mut().for_each(|q| q.alpha = 70.0);
    }
    size
}

/// A page's name as its header says it (B411: Advanced's under Graphics).
fn page_name(s: Screen) -> &'static str {
    if s == Screen::Advanced { "Graphics > Advanced" } else { TABS[tab(s)].0 }
}

/// An options page: the rows in the list as tiles (the chosen one the selected colour, a ring inset), each label on the
/// left and its value in a "< value >" pill on the right (a tap picks the row; picked, the pill filled and its halves
/// step it, B326), a volume as a bar beside it; the chosen one's name, description and choices in the side column.
pub(super) fn page_layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    let opts = page(m.screen);
    let z = zoom(&m.grid).1;
    look::backdrop(d);
    let n0 = d.out.len();
    let (list, side) = chrome(d, m, opts.len(), page_name(m.screen), "Changes apply right away");
    if pinned(m.screen) {
        pin_marks(d, list, opts.len());
    }
    let mut edit = None;
    let mut open = None;
    for (i, &o) in opts.iter().enumerate() {
        let r @ [x, y, w, h] = page_row(m.screen, list, opts.len(), i);
        let on = i == m.sel;
        let vh = (h - 10.0 * PY).min(26.0 * PY);
        // (the Renderer's value carries the renderer running and Hardware/Software: room for "Auto (WebGPU, Hardware)")
        let vw = if o == Renderer { 200.0 } else { 130.0f32 }.min(w * 0.4);
        let vb = [x + w - vw - 5.0 * PX, y + (h - vh) / 2.0, vw, vh];
        let size = row_pill(d, r, on, label(o), vb[0] - x - 30.0 * PX, locked(o));
        if m.editing.is_none() {
            d.hit(r, pointer::Act::Row(i, pointer::OK));
        }
        let mut value = m.settings.value(o);
        // the renderer actually running beside the setting (Auto's pick, or WebGL2 where the browser lacked WebGPU),
        // and whether a graphics card draws it or the CPU (software: much slower)
        if o == Renderer {
            let running = if crate::graphics::webgl2() { "WebGL2" } else { "WebGPU" };
            let kind = crate::graphics::SOFTWARE.get().map(|&s| if s { "Software" } else { "Hardware" });
            value = match (value == running, kind) {
                (true, None) => value,
                (true, Some(k)) => format!("{value} ({k})"),
                (false, None) => format!("{value} ({running})"),
                (false, Some(k)) => format!("{value} ({running}, {k})"),
            };
        }
        let editing = on && m.editing.is_some() && !locked(o);
        if o == Advanced {
            // (a way in, not a value: no arrows)
            look::steps(d, vb, false, &value, size, false, [false; 2]);
        } else if listed(o) {
            // B498: a dropdown's pill: its value and a ▼ (▲ blue while its list is open)
            let shown = on && m.listing.is_some();
            drop_pill(d, vb, &value, size, shown);
            if shown {
                open = Some((o, r));
            }
        } else if !editing {
            look::steps(d, vb, false, &value, size, value == "Off" || locked(o), [true; 2]);
        }
        if let Volume(k) = o {
            // B471: a white track in a dark outline, filled blue
            let sw = 110.0f32.min(w * 0.22);
            let bar = [vb[0] - sw - 12.0 * PX, y + h / 2.0 - 5.0 * PY, sw, 10.0 * PY];
            rounded(d, bar, 5.0, look::c(look::INK), 128.0);
            let t = 2.0;
            let inner = [bar[0] + t * PX, bar[1] + t * PY, sw - 2.0 * t * PX, bar[3] - 2.0 * t * PY];
            rounded(d, inner, 3.0, WHITE, 128.0);
            let fill = (inner[2] * m.settings.volumes.0[k] as f32 / 10.0).max(6.0 * PX);
            rounded(d, [inner[0], inner[1], fill, inner[3]], 3.0, look::c(look::BLUE), 128.0);
        }
        if editing {
            edit = Some((i, o, r, vb, value, size));
        }
    }
    if let Some(&o) = opts.get(m.sel) {
        let preset = matches!(o, Scale | Shadows | Msaa | Fps | Vsync | Original | Mesh | FewerShadows | Cull);
        let note = if preset {
            "Set by Preset. Changing it switches Preset to Custom."
        } else if o == MenuSize {
            if super::touch_page() { "Auto picks a size that reads well at this screen's size. The match HUD keeps its size." } else { "Auto is 100%, 80% on screens taller than 1440 px. The match HUD keeps its size." }
        } else {
            ""
        };
        // B324: the WebGL2 build says what it reaches here under the frame rate's and the renderer's rows
        #[cfg(target_arch = "wasm32")]
        let reach = matches!(o, Renderer | ShowFps).then(crate::web::webgl2_reach).flatten().map(|(fps, play, hz)| webgl2_reach(fps, play, hz));
        #[cfg(not(target_arch = "wasm32"))]
        let reach: Option<String> = None;
        let note = reach.as_deref().unwrap_or(note);
        let now = m.settings.value(o);
        side_panel(d, side, strip(list, opts.len()), label(o), about(o), "Choices", &choices(o), Some(&now), note);
    }
    // B326: only the picked row takes < >: its value pill filled (an arrow's gone at a slider's end), its value's
    // halves step it, the rest of the row keeps it, a tap off puts it back (as ○)
    if let (Some((o, r)), Some(j)) = (open, m.listing) {
        let bottom = (list[1] + list[3]).max(side[1] + side[3]);
        drop_list(d, o, r, j, &m.settings.value(o), [list[1], bottom]);
    } else if let Some((i, o, r @ [_, y, _, h], vb, value, size)) = edit {
        d.popup(Some(pointer::press(|k| k.back = true)));
        let [vx, vy, vw, vh] = vb;
        // B471: picked, the value pill blue with white words
        look::outlined_pill(d, vb, look::c(look::BLUE), 2.0);
        let ink = WHITE;
        d.label(&value, vx + vw / 2.0, vy + (vh - size) / 2.0, size, vw - 40.0 * PX, ink);
        let ends = match o {
            Volume(k) => (m.settings.volumes.0[k] > 0, m.settings.volumes.0[k] < 10),
            _ => (true, true),
        };
        if ends.0 {
            d.text("<", vx + 9.0 * PX, vy + (vh - size) / 2.0, size, ink);
        }
        if ends.1 {
            d.text(">", vx + vw - 9.0 * PX - d.width(">", size), vy + (vh - size) / 2.0, size, ink);
        }
        d.hit(r, pointer::Act::Row(i, pointer::OK));
        d.hit([vx, y, vw / 2.0, h], pointer::Act::Row(i, Press { left: true, ..default() }));
        d.hit([vx + vw / 2.0, y, vw / 2.0, h], pointer::Act::Row(i, Press { right: true, ..default() }));
    }
    z.apply(d, n0);
    switch_layout(d, m, hand_x);
}

/// B498: a dropdown row's value pill: white (blue with white words while open), its value and a ▼ (▲ open).
fn drop_pill(d: &mut Draw, b @ [x, y, w, h]: [f32; 4], value: &str, size: f32, open: bool) {
    look::outlined_pill(d, b, look::c(if open { look::BLUE } else { look::PAPER }), 2.0);
    let ink = if open { WHITE } else { look::c(look::INK) };
    let (tw, th) = (size * 0.7, size * 0.45);
    super::roster::triangle(d, [x + w - 12.0 * PX - tw, y + (h - th) / 2.0, tw, th], !open, if open { WHITE } else { look::c(look::MUTED) });
    let room = w - tw - 34.0 * PX;
    let value = roster::ellipsis(d, value, size, room);
    d.centred(&value, x + 10.0 * PX + room / 2.0, y + (h - size) / 2.0, size, ink);
}

/// B498: row `r`'s open dropdown of `o`'s choices (`now` the current one, a dot beside it; `j` highlighted, yellow),
/// right under the row (over it when there's more room there), within `span` (top, bottom); choices closer together
/// where they'd run past it. Each takes the pointer (hover highlights, a tap picks); a tap off it closes it.
fn drop_list(d: &mut Draw, o: Opt, [x, y, w, h]: [f32; 4], j: usize, now: &str, [top, bottom]: [f32; 2]) {
    d.popup(Some(pointer::press(|k| k.back = true)));
    let all = choices(o);
    let n = all.len() as f32;
    // the Preset's Auto and Custom under a rule (not presets of their own)
    let rule = if o == Preset { Graphics::PRESETS.len() } else { all.len() };
    let (pad, sep) = (5.0 * PY, if rule < all.len() { 8.0 * PY } else { 0.0 });
    let (below, above) = (bottom - (y + h + 2.0 * PY) - 6.0 * PY, y - 2.0 * PY - top - 6.0 * PY);
    let room = below.max(above);
    let ip = ((room - 2.0 * pad - sep) / n).min(28.0 * PY);
    let bh = ip * n + 2.0 * pad + sep;
    let by = if below >= bh || below >= above { y + h + 2.0 * PY } else { y - 2.0 * PY - bh };
    let bw = if o == Preset { 330.0 } else { 200.0 } * PX;
    let bw = bw.min(w - 8.0 * PX);
    let bx = x + w - bw - 2.0 * PX;
    // a hard dark shadow, the dark outline, the white body
    rounded(d, [bx + 4.0 * PX, by + 5.0 * PY, bw, bh], 14.0, look::c(look::INK), 128.0);
    roster::ringed(d, [bx, by, bw, bh], 14.0, 3.0, look::c(look::INK), WHITE);
    let size = (ip * 0.55).min(15.0 * PY);
    let hs = size * 0.78;
    let mut iy = by + pad;
    for (k, c) in all.iter().enumerate() {
        if k == rule {
            d.q(Tex::Solid, [0.0; 4], [bx + 12.0 * PX, iy + sep / 2.0 - 1.0 * PY, bw - 24.0 * PX, 2.0 * PY], look::c(look::RULE), 128.0);
            iy += sep;
        }
        let b = [bx + 6.0 * PX, iy, bw - 12.0 * PX, ip];
        if k == j {
            rounded(d, b, 9.0, look::c(look::YELLOW), 128.0);
        }
        d.hit(b, pointer::Act::Row(k, pointer::OK));
        let ty = iy + (ip - size) / 2.0;
        let tx = b[0] + 10.0 * PX;
        let end = b[0] + b[2] - 22.0 * PX;
        let name = roster::ellipsis(d, c, size, end - tx);
        d.text(&name, tx, ty, size, look::c(look::INK));
        let hint = hint(o, c);
        let hx = tx + d.width(&name, size) + 8.0 * PX;
        if !hint.is_empty() && hx < end - 20.0 * PX {
            let hint = roster::ellipsis(d, hint, hs, end - hx);
            d.text(&hint, hx, ty + (size - hs) * 0.75, hs, look::c(look::MUTED));
        }
        if c == now {
            rounded(d, [end + 6.0 * PX, iy + (ip - 7.0 * PY) / 2.0, 7.0 * PX, 7.0 * PY], 3.5, look::c(look::BLUE), 128.0);
        }
        iy += ip;
    }
}

/// The Renderer switch's popup's pills (640×448): their tops, 40 px apart as the mods' Reload / Later popup's.
const SWITCH_TOP: f32 = 226.0;

/// B327: the Renderer switch's confirm popup over the dimmed page, as the mods' Reload / Later one
/// (`mods_screen::ask_layout`): the renderer picked, Switch and Cancel.
fn switch_layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    let Some((r, c)) = m.switch else { return };
    use super::super::menu::{CHOSEN, PANEL, UNCHOSEN};
    // the page's own hand goes; the popup has its own
    if let Some(i) = d.out.iter().rposition(|q| q.tex == Tex::Cursor) {
        d.out.remove(i);
    }
    // a tap off the popup is Cancel
    d.popup(Some(pointer::Act::Row(1, pointer::OK)));
    d.anchor = super::Anchor::Span;
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 640.0, 448.0], [0.0; 3], 80.0);
    d.anchor = super::Anchor::Centre;
    let top = SWITCH_TOP - 70.0;
    d.box3([160.0, top, 320.0, 70.0 + 72.0 + 16.0], 16.0, [Some(INK), Some(WHITE), Some(PANEL)], 128.0);
    let name = format!("Switch to {}?", RENDERERS[r as usize].1);
    d.outlined(|d| d.label(&name, 320.0, SWITCH_TOP - 56.0, 22.0, 224.0, WHITE));
    d.outlined(|d| d.label("The page reloads to change it.", 320.0, SWITCH_TOP - 32.0, 18.0, 288.0, WHITE));
    for (k, word) in ["Switch", "Cancel"].into_iter().enumerate() {
        let y = SWITCH_TOP + 40.0 * k as f32;
        let on = k == c;
        d.pill_at([176.0, y, 288.0, 32.0], [INK, WHITE, if on { CHOSEN } else { UNCHOSEN }], 128.0);
        d.hit([176.0, y, 288.0, 32.0], pointer::Act::Row(k, pointer::OK));
        d.label(word, 320.0, y + 6.0, 20.0, 160.0, if on { WHITE } else { UNCHOSEN.map(|c| c * 1.4) });
    }
    // no `said` line: Settings has no description bar, its ticker would run over the list's footnote
    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [116.0 + hand_x, SWITCH_TOP + 40.0 * c as f32, 64.0, 32.0], WHITE, 128.0);
}

/// B324: what the WebGL2 build reaches here, in words: `fps`, the best second's frame rate (`play`: in a match,
/// else in the menus), against the display's `refresh` rate (Hz, when the page could measure it). Short of the
/// display's rate by more than 10 % (a second's count wobbles by a frame, and the odd hitch), it's capped: say so and
/// where the full rate is to be had.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn webgl2_reach(fps: f64, play: bool, refresh: Option<f64>) -> String {
    let (fps, place) = (fps.round(), if play { "in a match" } else { "in the menus" });
    match refresh.map(f64::round) {
        Some(hz) if fps < hz * 0.9 => format!(
            "WebGL2 reaches up to {fps} fps {place} on this device, below the display's {hz} Hz. WebGPU (where the browser has it) or the desktop app (download the desktop build) gets the full refresh rate."
        ),
        Some(hz) => format!("WebGL2 reaches the display's full {hz} Hz {place} on this device."),
        None => format!("WebGL2 reaches up to {fps} fps {place} on this device."),
    }
}

/// B291: the Controller column's button pills' dark fill.
const PAD_WELL: u32 = 0x4a4560;

/// B609: the Controls table's column headings.
pub(super) const CONTROLS_HEAD: [&str; 3] = ["Action", "Controller", "Keyboard & mouse"];

/// B609: a Controls row's least pitch (mockup px): roomier than a Settings row's, so the button icons read on a phone.
const TABLE_PITCH: f32 = 52.0;

/// B609: the Controls table on the view `g`: its panel (the list's and side panel's room as one: the page's whole
/// width, a portrait view's whole height), the rows' area under its strip (the scroll bar's lane right of it, kept
/// even when every row fits), the pitch and how many rows show whole.
pub(super) struct Table {
    pub panel: [f32; 4],
    pub rows: [f32; 4],
    pub bar: [f32; 4],
    pub pitch: f32,
    pub shown: usize,
}

pub(super) fn table(g: &Grid) -> Table {
    let (list, side) = boxes(g, 1);
    let panel = [list[0], list[1], side[0] + side[2] - list[0], side[1] + side[3] - list[1]];
    let inset = look::RIM + look::PAD;
    let top = panel[1] + (look::RIM + look::STRIP + 2.5 + look::PAD) * PY;
    let h = panel[1] + panel[3] - (inset + FOOT) * PY - top;
    let (x, w) = (panel[0] + inset * PX, panel[2] - 2.0 * inset * PX);
    let lane = (look::BAR + look::PAD) * PX;
    // (the rows that fit whole, spread to fill the room)
    let shown = ((h / (TABLE_PITCH * PY) + 0.15) as usize).max(1);
    let pitch = (h / shown as f32).min(1.25 * TABLE_PITCH * PY);
    Table { panel, rows: [x, top, w - lane, h], bar: [x + w - look::BAR * PX, top, look::BAR * PX, h], pitch, shown }
}

impl Table {
    /// Place `j` on view's row box.
    pub(super) fn row(&self, j: usize) -> [f32; 4] {
        let [x, y, w, _] = self.rows;
        [x, y + self.pitch * j as f32, w, self.pitch - look::ROW_GAP * PY]
    }

    /// Row box `r`'s label room and its Controller and Keyboard & mouse cells.
    pub(super) fn cells(&self, [x, y, w, h]: [f32; 4]) -> (f32, [[f32; 4]; 2]) {
        let label = (w * 0.3).min(240.0 * PX);
        let gap = 8.0 * PX;
        let cw = (w - label - 3.0 * gap) / 2.0;
        let ch = h - 8.0 * PY;
        let cell = |k: f32| [x + label + gap + (cw + gap) * k, y + 4.0 * PY, cw, ch];
        (label, [cell(0.0), cell(1.0)])
    }
}

/// B609: the square Reset button at the right of the Settings bar (its tabs' row in a portrait view).
pub(super) fn reset_button(g: &Grid) -> [f32; 4] {
    let (portrait, z) = zoom(g);
    let view = z.inner(g.view);
    let right = view[2] - look::MARGIN * PX;
    let y = if portrait { view[1] + look::TOP_BAR * PY } else { look::bar_y(view[1]) };
    [right - look::PILL * PX, y, look::PILL * PX, look::PILL * PY]
}

/// B609: the Controls table's first action on view: where it was, moved just enough to show the chosen row.
pub(super) fn controls_top(m: &Menu) -> usize {
    let n = controls::action_names().count();
    let shown = table(&m.grid).shown.min(n);
    let top = m.controls_top.min(n - shown);
    if m.sel < n { top.min(m.sel).max((m.sel + 1).saturating_sub(shown)) } else { top }
}

/// B609: the pointer's row index of the Controls table's cell: action `a`'s Controller (`keys` false) or Keyboard &
/// mouse cell; past every action, the Reset button.
pub(super) fn controls_cell(a: usize, keys: bool) -> usize {
    let n = controls::action_names().count();
    if a < n { 2 * a + usize::from(keys) } else { 2 * n }
}

impl Menu {
    /// B609: the pointer on the Controls table's cell `i` ([`controls_cell`]): its row and column chosen; the
    /// cell that was chosen.
    pub(super) fn controls_choose(&mut self, i: usize) -> usize {
        let was = controls_cell(self.sel, self.keys_tab);
        self.sel = i / 2;
        if i < controls_cell(usize::MAX, false) {
            self.keys_tab = i % 2 == 1;
        }
        self.controls_top = controls_top(self);
        was
    }
}

/// The Controls page (B609: one table): the Settings bar with its square Reset button at the right end, then a panel
/// with the column headings on its strip (Action, Controller, Keyboard & mouse) over a row per action: its name, its
/// pad buttons as the pad's own button icons (the stick that always does it beside them) and its keys as key caps,
/// each cell rebindable (✕ on the chosen one: the next button or key pressed replaces the cell's). The rows scroll
/// with the cursor (pad, wheel, a finger's drag) when they don't all fit, a scroll bar beside them; the footnote
/// says what the chosen cell does.
pub(super) fn controls_layout(d: &mut Draw, m: &Menu, bind: &controls::Bindings, _hand_x: f32) {
    let names: Vec<String> = controls::action_names().map(|n| n[..1].to_uppercase() + &n[1..].replace('_', " ")).collect();
    let n = names.len();
    let z = zoom(&m.grid).1;
    look::backdrop(d);
    let n0 = d.out.len();
    let reset = reset_button(&m.grid);
    settings_bar(d, m, reset[0] + reset[2], look::PILL + 8.0);
    reset_square(d, reset, m.sel == n);
    d.hit(reset, pointer::Act::Row(controls_cell(n, false), pointer::OK));
    let t = table(&m.grid);
    let [_, sy, _, sh] = look::panel(d, t.panel, look::STRIP, look::c(look::BLUE));
    // the column headings on the strip, over the label and each cell
    let (label, cells) = t.cells(t.row(0));
    let hs = 16.0 * PY;
    let hy = sy + (sh - hs) / 2.0;
    let fit = |d: &Draw, s: &str, room: f32| hs.min(hs * room / d.width(s, hs).max(1.0));
    let s0 = fit(d, CONTROLS_HEAD[0], label - 20.0 * PX);
    look::bold(d, CONTROLS_HEAD[0], [t.rows[0] + 16.0 * PX, hy + (hs - s0) / 2.0], s0, WHITE);
    for (c, head) in cells.iter().zip(&CONTROLS_HEAD[1..]) {
        let s = fit(d, head, c[2] - 8.0 * PX);
        let x = c[0] + (c[2] - d.width(head, s)) / 2.0;
        look::bold(d, head, [x, hy + (hs - s) / 2.0], s, WHITE);
    }
    let waiting = m.rebinding.is_some();
    let style = legend::pad_style_shown();
    let top = controls_top(m);
    let shown = t.shown.min(n);
    for (j, i) in (top..top + shown).enumerate() {
        let r @ [x, y, _, h] = t.row(j);
        let on = i == m.sel;
        look::button(d, r, on);
        let (label, cells) = t.cells(r);
        let size = (h * 0.4).min(16.0 * PY);
        let ink = if on { look::c(look::INK) } else { WHITE };
        d.text(&roster::ellipsis(d, &names[i], size, label - 20.0 * PX), x + 16.0 * PX, y + (h - size) / 2.0, size, ink);
        for (k, b) in cells.into_iter().enumerate() {
            let keys = k == 1;
            let chosen = on && m.keys_tab == keys;
            let wait = chosen && waiting;
            if chosen {
                look::halo(d, b);
            }
            // B471: a white pill in a dark outline, the one waiting for a key or button blue (B291: the Controller
            // column's dark, so the pad art's white arms and arrows show)
            let fill = if wait { look::c(look::BLUE) } else if keys { WHITE } else { look::c(PAD_WELL) };
            look::outlined_pill(d, b, fill, if chosen { 3.0 } else { 2.0 });
            if wait {
                let say = if keys { "Press a key..." } else { "Press a button..." };
                d.label(say, b[0] + b[2] / 2.0, b[1] + (b[3] - size * 0.9) / 2.0, size * 0.9, b[2] - 16.0 * PX, WHITE);
            } else if keys {
                key_caps(d, b, &bind.keys_of(i).into_iter().map(key_name).collect::<Vec<_>>(), size * 0.9);
            } else {
                let mut icons: Vec<u8> = bind.pads_of(i).into_iter().filter_map(legend::button_glyph).collect();
                icons.extend(controls::fixed_stick(i).map(|(right, dir)| legend::stick_glyph(right, dir)));
                button_icons(d, b, style, &icons, size);
            }
            d.hit(b, pointer::Act::Row(controls_cell(i, keys), pointer::OK));
        }
        // the label: the row in the column already chosen
        d.hit([x, y, label, h], pointer::Act::Row(controls_cell(i, m.keys_tab), pointer::OK));
    }
    if n > shown {
        look::scroll_bar(d, t.bar, look::c(look::BLUE), shown, n, top, n - shown);
    }
    let foot = if m.sel >= n {
        "Reset: puts every key and button back as it was first."
    } else if waiting && m.keys_tab {
        "Press the new key for it (Escape cancels). It replaces the action's keys; its controller buttons stay."
    } else if waiting {
        "Press the new button on any controller (Escape or a tap cancels). It replaces the action's buttons; its keys stay."
    } else if m.keys_tab {
        "The mouse points, clicks and scrolls in the menus; Enter, Esc, Q / E and Tab always work there."
    } else {
        match controls::fixed_stick(m.sel) {
            Some((false, _)) => "The left stick always moves as well; it can't be rebound.",
            Some((true, _)) => "The right stick, flicked fully that way, always plays it as well; it can't be rebound.",
            None => "Saved to controls.txt, shared by every player.",
        }
    };
    footnote(d, m, t.panel, foot);
    z.apply(d, n0);
}

/// B609: the square Reset button: the main menu's button (grey, chosen yellow) with rounded corners and a
/// round-arrow mark.
fn reset_square(d: &mut Draw, b @ [x, y, w, h]: [f32; 4], on: bool) {
    let (r, t) = (7.0, 2.0);
    rounded(d, b, r, look::c(look::INK), 128.0);
    rounded(d, [x + t * PX, y + t * PY, w - 2.0 * t * PX, h - 2.0 * t * PY], r - t, look::c(if on { look::YELLOW } else { look::GREY }), 128.0);
    let ink = if on { look::c(look::INK) } else { WHITE };
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let (rx, ry) = (w * 0.27, h * 0.27);
    let stroke = |d: &mut Draw, [ax, ay]: [f32; 2], [bx, by]: [f32; 2]| {
        let (dx, dy) = ((bx - ax) / PX, (by - ay) / PY);
        let len = dx.hypot(dy) + 1.6;
        d.q(Tex::Solid, [0.0; 4], [(ax + bx) / 2.0 - len * PX / 2.0, (ay + by) / 2.0 - 1.0 * PY, len * PX, 2.0 * PY], ink, 128.0);
        if let Some(q) = d.out.last_mut() {
            q.angle = dy.atan2(dx);
        }
    };
    // the circle, open at the top right, then the arrowhead at its start pointing on round
    let at = |a: f32| [cx + rx * a.cos(), cy - ry * a.sin()];
    let (a0, a1, steps) = (0.35f32, 0.35 + std::f32::consts::TAU * 0.8, 12);
    for s in 0..steps {
        let (p, q) = (a0 + (a1 - a0) * s as f32 / steps as f32, a0 + (a1 - a0) * (s + 1) as f32 / steps as f32);
        stroke(d, at(p), at(q));
    }
    let tip = at(a0);
    stroke(d, tip, [tip[0] - w * 0.16, tip[1] - h * 0.02]);
    stroke(d, tip, [tip[0] + w * 0.02, tip[1] - h * 0.17]);
}

/// B291: key caps `names` in a row, centred in the pill `b` (narrower where they'd run past it): each a light cap in a
/// dark outline with its name; a dash for none.
fn key_caps(d: &mut Draw, [x, y, w, h]: [f32; 4], names: &[String], size: f32) {
    if names.is_empty() {
        d.label("-", x + w / 2.0, y + (h - size) / 2.0, size, w, look::c(look::MUTED));
        return;
    }
    let ch = h - 8.0 * PY;
    let gap = 6.0 * PX;
    let ws: Vec<f32> = names.iter().map(|s| (d.width(s, size) + 18.0 * PX).max(34.0 * PX)).collect();
    let total = ws.iter().sum::<f32>() + gap * (ws.len() - 1) as f32;
    let k = ((w - 16.0 * PX) / total).min(1.0);
    let mut cx = x + (w - total * k) / 2.0;
    for (s, cw) in names.iter().zip(ws) {
        let c = [cx, y + (h - ch) / 2.0, cw * k, ch];
        look::outlined_pill(d, c, look::c(look::TILE), 2.0);
        look::pill_words(d, c, s, size * k.max(0.7), look::c(look::INK));
        cx += (cw + gap) * k;
    }
}

/// B291: the button icons `icons` (`Tex::Glyph` indices of `style`'s art) in a row, centred in the pill `b`
/// (smaller where they'd run past it); a dash for none.
fn button_icons(d: &mut Draw, [x, y, w, h]: [f32; 4], style: legend::Style, icons: &[u8], size: f32) {
    if icons.is_empty() {
        d.label("-", x + w / 2.0, y + (h - size) / 2.0, size, w, WHITE);
        return;
    }
    let gh = h - 2.0 * PY;
    let gap = 6.0 * PX;
    let ws: Vec<f32> = icons.iter().map(|&i| gh * legend::glyph_aspect(style, i)).collect();
    let total = ws.iter().sum::<f32>() + gap * (ws.len() - 1) as f32;
    let k = ((w - 16.0 * PX) / total).min(1.0);
    let mut cx = x + (w - total * k) / 2.0;
    for (&i, gw) in icons.iter().zip(ws) {
        d.q(Tex::Glyph(style, i), [0.0; 4], [cx, y + (h - gh * k) / 2.0, gw * k, gh * k], WHITE, 128.0);
        cx += (gw + gap) * k;
    }
}

/// B291: a key's name as its cap shows it: `KeyW` W, `Digit1` 1, `ArrowLeft` Left, `Numpad4` Num 4, `ShiftLeft`
/// L Shift (controls.txt keeps Bevy's names).
fn key_name(k: KeyCode) -> String {
    let n = format!("{k:?}");
    if let Some(r) = n.strip_prefix("Numpad") {
        return format!("Num {r}");
    }
    for (side, short) in [("Left", "L"), ("Right", "R")] {
        for (m, word) in [("Shift", "Shift"), ("Control", "Ctrl"), ("Alt", "Alt")] {
            if n == format!("{m}{side}") {
                return format!("{short} {word}");
            }
        }
    }
    ["Key", "Digit", "Arrow"].iter().find_map(|p| n.strip_prefix(p).filter(|r| !r.is_empty())).unwrap_or(&n).to_string()
}



#[cfg(test)]
mod tests {
    use super::super::{Dev, Graphics, Volumes};
    use super::*;

    #[test]
    fn webgl2_reach_words() {
        let capped = webgl2_reach(59.6, true, Some(120.2));
        assert!(capped.starts_with("WebGL2 reaches up to 60 fps in a match on this device, below the display's 120 Hz.") && capped.contains("desktop build"), "{capped}");
        assert_eq!(webgl2_reach(116.0, false, Some(120.0)), "WebGL2 reaches the display's full 120 Hz in the menus on this device.");
        assert_eq!(webgl2_reach(60.0, true, Some(60.0)), "WebGL2 reaches the display's full 60 Hz in a match on this device.");
        assert_eq!(webgl2_reach(60.0, true, None), "WebGL2 reaches up to 60 fps in a match on this device.");
    }

    fn press(f: impl FnOnce(&mut Press) -> &mut bool) -> Press {
        let mut p = Press::default();
        *f(&mut p) = true;
        p
    }

    /// Every tab: ✕ on the main menu's Settings opens the first, R1 walks them in order and wraps, L1 back; ↓ walks each
    /// page's rows and wraps (changing nothing); ◀ ▶ on a row not picked change nothing (B326); ✕ picks it, then ◀ ▶
    /// change it and save (◀ undoing ▶ on a cycle), ↓ and L1 / R1 stay on it, ✕ keeps it, ○ puts it back; each value it
    /// takes one of its side panel's choices; ○ lands on the main menu's Settings row.
    #[test]
    fn every_page() {
        let mut m = Menu::default();
        m.sel = m.main_at(super::super::home::Row::Settings);
        m.step(Dev::Keys, press(|k| &mut k.ok));
        for &(_, s) in tabs_shown() {
            assert_eq!((m.screen, m.sel), (s, 0));
            let n = if s == Screen::Controls { controls::action_names().count() + 1 } else { page(s).len() };
            for want in (1..n).chain([0]) {
                m.step(Dev::Keys, press(|k| &mut k.down));
                assert_eq!(m.sel, want);
            }
            assert_eq!(m.settings, Settings::default());
            each_row(&mut m, s);
            m.sel = 0;
            m.step(Dev::Keys, press(|k| &mut k.next));
        }
        assert_eq!(m.screen, TABS[0].1);
        m.step(Dev::Keys, press(|k| &mut k.prev));
        assert_eq!(m.screen, tabs_shown().last().unwrap().1);
        m.step(Dev::Keys, press(|k| &mut k.back));
        assert_eq!((m.screen, m.sel), (Screen::Main, m.main_at(super::super::home::Row::Settings)));
    }

    /// B609: the Controls table: ←→ choose a row's Controller or Keyboard & mouse cell, ✕ rebinds it; ↑ from the
    /// first row is the banner's Reset; the rows scroll with the cursor where they don't all fit (and stay put while
    /// it moves on view); a tap on a cell chooses that cell.
    #[test]
    fn controls_table() {
        let n = controls::action_names().count();
        let mut m = Menu { screen: Screen::Controls, ..Menu::default() };
        assert!(!m.keys_tab);
        assert_eq!(m.step(Dev::Keys, press(|k| &mut k.right)), [Effect::Sound(0)]);
        assert!(m.keys_tab);
        assert!(m.step(Dev::Keys, press(|k| &mut k.right)).is_empty(), "already on it");
        m.step(Dev::Keys, press(|k| &mut k.left));
        assert!(!m.keys_tab);
        m.step(Dev::Keys, press(|k| &mut k.right));
        m.sel = 4;
        m.step(Dev::Keys, press(|k| &mut k.ok));
        assert_eq!((m.rebinding, m.keys_tab, m.screen), (Some(4), true, Screen::Controls));
        m.rebinding = None;
        // Reset over the first row; ←→ there do nothing
        m.sel = 0;
        m.step(Dev::Keys, press(|k| &mut k.up));
        assert_eq!(m.sel, n);
        assert!(m.step(Dev::Keys, press(|k| &mut k.left)).is_empty() && m.keys_tab);
        assert!(m.step(Dev::Keys, press(|k| &mut k.ok)).contains(&Effect::ResetControls));
        m.step(Dev::Keys, press(|k| &mut k.up));
        assert_eq!(m.sel, n - 1);
        // the last row on view at the foot, back up it stays till the cursor passes the top
        let t = table(&m.grid);
        assert!(t.shown < n, "{} rows on view", t.shown);
        assert_eq!(m.controls_top, n - t.shown);
        m.step(Dev::Keys, press(|k| &mut k.up));
        assert_eq!(m.controls_top, n - t.shown);
        for _ in 0..t.shown {
            m.step(Dev::Keys, press(|k| &mut k.up));
        }
        assert_eq!(m.controls_top, m.sel);
        // the wheel steps the cursor (and so the rows) too
        let top = m.controls_top;
        m.point(Dev::Keys, pointer::Act::Wheel(false), true);
        assert_eq!(m.controls_top, top - 1);
        // a tap on a row's Controller cell: that cell, waiting for a button
        let ([x, y, w, h], i) = cell_rect(&m, 0, false);
        tap_at(&mut m, x + w / 2.0, y + h / 2.0);
        assert_eq!((m.sel, m.keys_tab, m.rebinding), (i, false, Some(i)));
        m.rebinding = None;
        let ([x, y, w, h], i) = cell_rect(&m, 1, true);
        tap_at(&mut m, x + w / 2.0, y + h / 2.0);
        assert_eq!((m.sel, m.keys_tab, m.rebinding), (i, true, Some(i)));
        m.rebinding = None;
        let [x, y, w, h] = reset_button(&m.grid);
        assert!(tap_at(&mut m, x + w / 2.0, y + h / 2.0).contains(&Effect::ResetControls));
        assert_eq!(key_name(KeyCode::ArrowLeft), "Left");
        assert_eq!(key_name(KeyCode::Numpad4), "Num 4");
        assert_eq!(key_name(KeyCode::ShiftLeft), "L Shift");
        assert_eq!(key_name(KeyCode::KeyJ), "J");
    }

    /// A tap at the 640×448 point (x, y) on a 4:3 window.
    fn tap_at(m: &mut Menu, x: f32, y: f32) -> Vec<Effect> {
        let a = super::super::pointer::tests::act_at(m, x, y).unwrap_or_else(|| panic!("nothing at {x},{y}"));
        m.point(Dev::Keys, a, true)
    }

    /// The Controls table's row `j` on view's Controller (`keys` false) or Keyboard & mouse cell, and its action.
    fn cell_rect(m: &Menu, j: usize, keys: bool) -> ([f32; 4], usize) {
        let t = table(&m.grid);
        (t.cells(t.row(j)).1[usize::from(keys)], controls_top(m) + j)
    }

    #[test]
    fn a_tab_change_ticks() {
        let mut m = Menu { screen: TABS[0].1, ..Menu::default() };
        assert_eq!(m.step(Dev::Keys, press(|k| &mut k.next)), [Effect::Sound(0)]);
        assert_eq!(m.screen, TABS[1].1);
        assert_eq!(m.step(Dev::Keys, press(|k| &mut k.prev)), [Effect::Sound(0)]);
        assert_eq!(m.screen, TABS[0].1);
        assert_eq!(m.point(Dev::Keys, pointer::Act::Tab(2), true), [Effect::Sound(0)]);
        assert_eq!((m.screen, m.sel), (TABS[2].1, 0));
    }

    /// [`every_page`]'s checks of each of page `s`'s rows; B411: Graphics' Advanced row opens its page (✕, its rows
    /// checked the same, its tab Graphics', ○ back on the Advanced row).
    fn each_row(m: &mut Menu, s: Screen) {
        for (i, &o) in page(s).iter().enumerate() {
            m.sel = i;
            if o == Advanced {
                assert!(m.settings.value(o) == ">" && choices(o).is_empty());
                m.step(Dev::Keys, press(|k| &mut k.ok));
                assert_eq!((m.screen, m.sel, m.editing, tab(m.screen)), (Screen::Advanced, 0, None, tab(Screen::Graphics)));
                each_row(m, Screen::Advanced);
                m.step(Dev::Keys, press(|k| &mut k.back));
                assert_eq!((m.screen, m.sel), (s, i));
                continue;
            }
            let was = m.settings;
            if listed(o) {
                list_row(m, o);
                continue;
            }
            for k in [Press { right: true, ..default() }, Press { left: true, ..default() }] {
                assert!(m.step(Dev::Keys, k).is_empty() && m.settings == was, "{o:?}");
            }
            m.step(Dev::Keys, press(|k| &mut k.ok));
            assert_eq!(m.editing, Some(was), "{o:?}");
            let fx = m.step(Dev::Keys, press(|k| &mut k.right));
            if matches!(o, Volume(_)) {
                m.step(Dev::Keys, press(|k| &mut k.left));
                let fx = m.step(Dev::Keys, press(|k| &mut k.left));
                assert!(fx.contains(&Effect::SaveSettings) && m.settings != was, "{o:?}");
            } else {
                assert!(fx.contains(&Effect::SaveSettings) && m.settings != was, "{o:?}");
                m.step(Dev::Keys, press(|k| &mut k.left));
                assert_eq!(m.settings, was, "{o:?}");
                m.step(Dev::Keys, press(|k| &mut k.right));
            }
            // still on it: ↓ and R1 do nothing while it's picked
            m.step(Dev::Keys, press(|k| &mut k.down));
            m.step(Dev::Keys, press(|k| &mut k.next));
            assert_eq!((m.screen, m.sel), (s, i));
            let changed = m.settings;
            let fx = m.step(Dev::Keys, press(|k| &mut k.back));
            assert!(fx.contains(&Effect::SaveSettings) && m.settings == was && m.editing.is_none(), "{o:?}: ○ puts it back");
            m.step(Dev::Keys, press(|k| &mut k.ok));
            m.settings = changed;
            m.step(Dev::Keys, press(|k| &mut k.ok));
            assert!(m.settings == changed && m.editing.is_none(), "{o:?}: ✕ keeps it");
            for _ in 0..12 {
                assert!(choices(o).contains(&m.settings.value(o)), "{o:?} {}", m.settings.value(o));
                m.settings.step(o, -1);
            }
            m.settings = was;
        }
    }

    /// B498: [`each_row`]'s checks of a dropdown row `o` (row `m.sel`): ◀ ▶ change it unopened and save (◀ undoing
    /// ▶); ✕ opens its list on the current choice, ↑ ↓ move it (wrapping) and change nothing, L1 / R1 and ◀ ▶ do
    /// nothing, ○ closes it unchanged; ✕ on a choice sets it and saves; every choice is reachable this way.
    fn list_row(m: &mut Menu, o: Opt) {
        let (was, at) = (m.settings, (m.screen, m.sel));
        let fx = m.step(Dev::Keys, press(|k| &mut k.right));
        assert!(fx.contains(&Effect::SaveSettings) && m.settings != was && m.editing.is_none(), "{o:?}");
        m.step(Dev::Keys, press(|k| &mut k.left));
        assert_eq!(m.settings, was, "{o:?}");
        let all = choices(o);
        let cur = all.iter().position(|c| *c == m.settings.value(o)).unwrap();
        m.step(Dev::Keys, press(|k| &mut k.ok));
        assert_eq!((m.listing, m.editing.is_some()), (Some(cur), true), "{o:?}");
        m.step(Dev::Keys, press(|k| &mut k.down));
        for k in [press(|k| &mut k.left), press(|k| &mut k.next), press(|k| &mut k.right)] {
            assert!(m.step(Dev::Keys, k).is_empty(), "{o:?}");
        }
        assert_eq!((m.listing, (m.screen, m.sel), m.settings), (Some((cur + 1) % all.len()), at, was), "{o:?}");
        assert_eq!(m.step(Dev::Keys, press(|k| &mut k.back)), [Effect::Sound(1)]);
        assert_eq!((m.listing, m.editing, m.settings), (None, None, was), "{o:?}: ○ closes it unchanged");
        for (j, c) in all.iter().enumerate() {
            // (Preset's Auto is this device's pick; Custom leaves it as it is)
            if o == Preset && (c == AUTO || c == "Custom") {
                continue;
            }
            m.step(Dev::Keys, press(|k| &mut k.ok));
            for _ in 0..(j + all.len() - m.listing.unwrap()) % all.len() {
                m.step(Dev::Keys, press(|k| &mut k.down));
            }
            m.step(Dev::Keys, press(|k| &mut k.ok));
            assert_eq!((m.settings.value(o), m.listing, m.editing), (c.clone(), None, None), "{o:?}");
        }
        m.settings = was;
    }

    /// B498: a dropdown's pick saves only a change; Preset's Custom changes nothing; the list draws inside the view
    /// at any aspect (Preset's under its row, Frame Limit's over it where there's more room there) and a hover or a
    /// tap on a choice highlights / picks it, a tap off it closes it.
    #[test]
    fn dropdowns() {
        // every row with more than two choices (user 2026-10-17); On/Off rows, sliders and Advanced aren't
        assert!(GRAPHICS.iter().chain(DISPLAY).chain(ADVANCED).chain(MODS).all(|&o| listed(o) == (choices(o).len() > 2 && !locked(o) && o != Advanced)));
        assert!(listed(Shadows) && listed(Msaa) && listed(ShowFps) && listed(Renderer) && !listed(Vsync) && !listed(Volume(0)));
        assert!(choices(Preset).iter().all(|c| !hint(Preset, c).is_empty()));
        let mut m = Menu { screen: Screen::Graphics, ..default() };
        m.step(Dev::Keys, press(|k| &mut k.ok));
        assert_eq!(m.step(Dev::Keys, press(|k| &mut k.ok)), [Effect::Sound(2)], "the same: no save");
        m.step(Dev::Keys, press(|k| &mut k.ok));
        (0..2).for_each(|_| _ = m.step(Dev::Keys, press(|k| &mut k.up)));
        assert_eq!(m.listing, Some(choices(Preset).len() - 1));
        assert_eq!(m.step(Dev::Keys, press(|k| &mut k.ok)), [Effect::Sound(2)], "Custom: no save");
        assert_eq!(m.settings, Settings::default());
        let t = super::super::pointer::tests::text_with_mods(0);
        for a in [4.0, 21.0 / 9.0, 16.0 / 9.0, 4.0 / 3.0, 9.0 / 16.0, 0.4] {
            for sel in [0, 4] {
                let mut m = Menu { screen: Screen::Graphics, sel, grid: Grid::fit(a), ..default() };
                m.step(Dev::Keys, press(|k| &mut k.ok));
                let d = super::super::drawn(&m, &t, &controls::Bindings::default(), &[], 0, 0.0);
                assert_eq!(d.outside, Some(pointer::press(|k| k.back = true)));
                let n = choices(page(m.screen)[sel]).len();
                let hits: Vec<_> = d.hits.iter().filter(|h| matches!(h.act, pointer::Act::Row(..))).collect();
                assert_eq!(hits.len(), n, "{a} {sel}: only the choices take the pointer");
                let (list, side) = boxes(&m.grid, GRAPHICS.len());
                let r = page_row(m.screen, list, GRAPHICS.len(), sel);
                for h in &hits {
                    let [_, y, _, hh] = h.rect;
                    assert!(y >= list[1] && y + hh <= (list[1] + list[3]).max(side[1] + side[3]) && hh >= 10.0, "{a} {sel}: {:?}", h.rect);
                    assert!(y >= r[1] + r[3] || y + hh <= r[1], "{a} {sel}: off its row");
                }
                if sel == 0 {
                    assert!(hits[0].rect[1] > r[1], "{a}: Preset's list under it");
                }
                // the second choice hovered, then tapped: picked
                let want = choices(page(m.screen)[sel])[1].clone();
                m.point(Dev::Keys, hits[1].act, false);
                assert_eq!(m.listing, Some(1));
                m.point(Dev::Keys, hits[1].act, true);
                assert_eq!((m.listing, m.settings.value(page(m.screen)[sel])), (None, want));
                m.step(Dev::Keys, press(|k| &mut k.ok));
                m.point(Dev::Keys, pointer::press(|k| k.back = true), true);
                assert!(m.listing.is_none() && m.editing.is_none());
            }
        }
    }

    /// B411: Display keeps only display rows; the speed-up switches and Mesh are on Advanced, under Graphics, whose
    /// tab stays lit there and whose L1 / R1 go to Graphics' neighbours.
    #[test]
    fn advanced_under_graphics() {
        assert!(!DISPLAY.iter().any(|o| matches!(o, FewerShadows | Cull | Mesh)));
        assert!(ADVANCED.iter().all(|o| !GRAPHICS.contains(o) && !DISPLAY.contains(o)) && GRAPHICS.last() == Some(&Advanced));
        for (k, want) in [(press(|k| &mut k.next), Screen::Sound), (press(|k| &mut k.prev), Screen::Display)] {
            let mut m = Menu { screen: Screen::Graphics, sel: GRAPHICS.len() - 1, ..default() };
            m.step(Dev::Keys, press(|k| &mut k.ok));
            assert_eq!(m.screen, Screen::Advanced);
            m.step(Dev::Keys, k);
            assert_eq!(m.screen, want);
        }
    }

    /// B294: at any aspect every page's rows sit in its list over the footnote, the side panel has room, and the
    /// busiest page draws within the pause overlay's quad pool.
    #[test]
    fn pages_fit() {
        let t = super::super::pointer::tests::text_with_mods(0);
        for a in [4.0, 21.0 / 9.0, 16.0 / 9.0, 4.0 / 3.0, 5.0 / 4.0, 9.0 / 16.0, 0.4] {
            let grid = Grid::fit(a);
            // B609: the Controls table's rows on view, under its strip and over its footnote, a phone's icons readable
            let tb = table(&grid);
            let last = tb.row(tb.shown - 1);
            assert!(tb.shown >= 3 && last[1] + last[3] <= tb.panel[1] + tb.panel[3] - 24.0 * PY && last[3] >= 40.0 * PY, "{a} {last:?} {:?}", tb.panel);
            assert!(tb.cells(last).1[1][0] + tb.cells(last).1[1][2] <= last[0] + last[2], "{a}");
            for s in TABS.map(|t| t.1).into_iter().chain([Screen::Advanced]) {
                let n = if s == Screen::Controls { controls::action_names().count() + 1 } else { page(s).len() };
                let (list, side) = boxes(&grid, n);
                assert!(side[2] >= 150.0 && side[3] >= 150.0 && list[0] + list[2] <= side[0] + side[2], "{a}: {list:?} {side:?}");
                let r = page_row(s, list, n, n - 1);
                assert!(r[1] + r[3] <= list[1] + list[3] - 24.0 && r[3] >= 20.0, "{a} {s:?} {r:?} {list:?}");
                for sel in 0..n {
                    // B498: a dropdown row's list open too
                    let open = page(s).get(sel).is_some_and(|&o| listed(o));
                    for listing in [None, Some(0)].into_iter().take(1 + open as usize) {
                        let m = Menu { screen: s, sel, grid, listing, editing: listing.map(|_| Settings::default()), ..default() };
                        let q = super::super::drawn(&m, &t, &controls::Bindings::default(), &[], 0, 0.0).out.len();
                        assert!(q < super::super::pause::POOL_HERE, "{a} {s:?} {sel} {listing:?}: {q} quads");
                    }
                }
            }
        }
    }

    /// Menu Size: at 60% on a 16:9 view the 640×448 screen is 60% of its 100% size, centred, its own shape; a tap
    /// lands on the scaled rect, not where it was at 100%; Auto is 80% past a 1440-logical-px screen; it round-trips settings.txt.
    #[test]
    fn menu_size_scales_centred() {
        use super::super::super::widescreen::{menu_shares, place, vplace_in};
        let (s, v) = menu_shares(16.0 / 9.0, 0.6);
        let (left, width) = place(0.0, 640.0, s, super::super::Anchor::Centre);
        let (top, height) = vplace_in(0.0, 448.0, v, super::super::Anchor::Centre);
        let close = |a: f32, b: f32| (a - b).abs() < 1e-4;
        assert!(close(width, 45.0) && close(left, 27.5) && close(height, 60.0) && close(top, 20.0), "{left} {width} {top} {height}");
        // same shape: 45% of 16 wide by 60% of 9 high is 4:3
        assert!(close(width * 16.0 / (height * 9.0), 4.0 / 3.0));
        let hits = [pointer::Hit { rect: [0.0, 0.0, 64.0, 44.8], anchor: default(), act: pointer::Act::Tab(0) }];
        let at = |x, y, k| pointer::hit_at(&hits, bevy::math::Vec2::new(x, y), 16.0 / 9.0, k, 0.0);
        assert_eq!(at(0.3, 0.22, 0.6), Some(pointer::Act::Tab(0)));
        assert_eq!(at(0.14, 0.02, 0.6), None);
        assert_eq!(at(0.14, 0.02, 1.0), Some(pointer::Act::Tab(0)));
        let mut set = Settings::default();
        assert_eq!((set.menu_scale(1440.0), set.menu_scale(2160.0), set.value(MenuSize).as_str()), (1.0, 0.8, "Auto"));
        set.step(MenuSize, 1);
        assert_eq!((set.menu_scale(2160.0), set.value(MenuSize).as_str()), (0.5, "50%"));
        set.step(MenuSize, -1);
        set.step(MenuSize, -1);
        assert_eq!(set.value(MenuSize), "150%");
        assert!(set.text().contains("menu_size = 150\n") && Settings::parse(&set.text()) == set);
    }

    /// B316: every row's choices are its own values — stepping one round through any row (the web build's Renderer
    /// too, which no native page shows) lands on each of its choices once and on nothing else.
    #[test]
    fn choices_are_each_rows_own_values() {
        let all = [DISPLAY, &[Display, Renderer], GRAPHICS, ADVANCED, SOUND, MODS, MOBILE].concat();
        for o in all.into_iter().filter(|o| !matches!(o, Volume(_) | Preset | Advanced)) {
            let mut s = Settings::default();
            let want = choices(o);
            let seen: Vec<_> = (0..want.len()).map(|_| (s.value(o), s.step(o, 1)).0).collect();
            assert_eq!((s.value(o), seen.len()), (seen[0].clone(), want.len()), "{o:?}");
            assert!(want.iter().all(|c| seen.contains(c)), "{o:?}: {seen:?} vs {want:?}");
        }
        assert_eq!(choices(Renderer), ["Auto", "WebGPU", "WebGL2"]);
        let mut s = Settings::default();
        s.step(Renderer, -1);
        assert_eq!(s.value(Renderer), "WebGL2");
    }

    /// B235: the sliders clamp to 0..=10 and the volumes round-trip settings.txt.
    #[test]
    fn sound_sliders() {
        let mut m = Menu { screen: Screen::Sound, sel: 1, ..default() };
        assert_eq!(m.settings.volumes, Volumes([10, 5, 10, 10, 5]), "music and ambience default to half");
        m.settings.volumes = Volumes([10; 5]);
        m.step(Dev::Keys, press(|k| &mut k.ok));
        assert!(m.step(Dev::Keys, press(|k| &mut k.right)).is_empty(), "at its end: no sound, no save");
        assert_eq!(m.settings.volumes, Volumes([10, 10, 10, 10, 10]));
        (0..12).for_each(|_| _ = m.step(Dev::Keys, press(|k| &mut k.left)));
        assert_eq!(m.settings.volumes, Volumes([10, 0, 10, 10, 10]));
        m.step(Dev::Keys, press(|k| &mut k.ok));
        // M111: the ambience slider, under the voices'
        (0..3).for_each(|_| _ = m.step(Dev::Keys, press(|k| &mut k.down)));
        assert_eq!(page(m.screen)[m.sel], Volume(4));
        m.step(Dev::Keys, press(|k| &mut k.ok));
        (0..3).for_each(|_| _ = m.step(Dev::Keys, press(|k| &mut k.left)));
        assert_eq!(m.settings.volumes, Volumes([10, 0, 10, 10, 7]));
        m.step(Dev::Keys, press(|k| &mut k.ok));
        assert_eq!(Settings::parse(&m.settings.text()), m.settings);
    }

    /// B327: Renderer asks before it switches (the browser build's Display page; forced here, the desktop has no such
    /// row): its list picks a renderer, and only a different one opens the popup; ◀ ▶ on the row change nothing,
    /// Cancel / ○ / a tap off it change nothing, Switch saves; while it's up it takes the presses, the legend says so,
    /// and it draws within the pause overlay's pool at any aspect.
    #[test]
    fn renderer_asks_first() {
        let mut m = Menu { screen: Screen::Display, sel: 1, ..default() };
        // the row's list on choice `j`, picked
        let pick = |m: &mut Menu, j: usize| {
            (m.listing, m.editing) = (Some(j), Some(m.settings));
            let mut out = Vec::new();
            m.step_list(Some(Renderer), j, Press { ok: true, ..default() }, &mut out);
            out
        };
        assert!(listed(Renderer));
        // the one in use: no popup
        pick(&mut m, 0);
        assert_eq!((m.switch, m.listing), (None, None));
        pick(&mut m, 1);
        assert_eq!(m.switch, Some((1, 0)));
        assert_eq!(m.settings, Settings::default());
        let fx = m.step(Dev::Keys, press(|k| &mut k.down));
        assert_eq!((m.switch, m.sel, fx), (Some((1, 1)), 1, vec![Effect::Sound(0)]));
        let fx = m.step(Dev::Keys, press(|k| &mut k.ok));
        assert_eq!((m.switch, m.settings, fx), (None, Settings::default(), vec![Effect::Sound(1)]));
        pick(&mut m, 2);
        assert_eq!(m.switch, Some((2, 0)));
        let fx = m.step(Dev::Keys, press(|k| &mut k.back));
        assert_eq!((m.switch, m.screen, m.settings, fx), (None, Screen::Display, Settings::default(), vec![Effect::Sound(1)]));
        // a tap off the popup is Cancel; a tap on Switch switches
        pick(&mut m, 2);
        let t = super::super::pointer::tests::text_with_mods(0);
        let d = super::super::drawn(&m, &t, &controls::Bindings::default(), &[], 0, 0.0);
        assert_eq!(d.outside, Some(pointer::Act::Row(1, pointer::OK)));
        assert!(super::super::legend::acts(&m, Dev::Keys).iter().any(|(_, w)| w == "Cancel"));
        m.point(Dev::Keys, pointer::Act::Row(1, pointer::OK), true);
        assert_eq!((m.switch, m.settings.renderer), (None, 0));
        pick(&mut m, 2);
        let fx = m.point(Dev::Keys, pointer::Act::Row(0, pointer::OK), true);
        assert!(m.switch.is_none() && m.settings.renderer == 2 && fx.contains(&Effect::SaveSettings));
        assert!(m.settings.text().contains("renderer = webgl2") || !cfg!(target_arch = "wasm32"));
        assert_eq!(m.settings.value(Renderer), "WebGL2");
        // ◀ ▶ on the row neither change it nor ask
        m.step(Dev::Keys, press(|k| &mut k.right));
        assert_eq!((m.switch, m.settings.renderer), (None, 2));
        // the popup again, on every aspect
        pick(&mut m, 0);
        assert_eq!(m.switch, Some((0, 0)));
        for a in [4.0, 16.0 / 9.0, 4.0 / 3.0, 9.0 / 16.0, 0.4] {
            m.grid = Grid::fit(a);
            let q = super::super::drawn(&m, &t, &controls::Bindings::default(), &[], 0, 0.0).out.len();
            assert!(q < super::super::pause::POOL_HERE, "{a}: {q} quads");
        }
    }

    /// The preset steps through the presets (from Custom to the first, High); changing what it sets makes it Custom;
    /// Display and Show FPS survive a preset change; Upscaled Textures takes Upscaled Downloads with it.
    #[test]
    fn presets_and_switches() {
        let d = Graphics::default();
        let mut s = Settings::default();
        s.step(Preset, 1);
        assert_eq!(s.graphics, Graphics::STEAM_DECK);
        s.step(Preset, -1);
        assert_eq!(s.graphics, d);
        // B412: High before Default, and the speed-up switches are part of a preset
        s.step(Preset, -1);
        assert_eq!((s.graphics, s.value(Preset).as_str()), (Graphics::HIGH, "High"));
        s.step(FewerShadows, 1);
        assert_eq!((s.graphics.fewer_shadows, s.value(Preset).as_str()), (true, "Custom"));
        s.step(Cull, 1);
        assert_eq!(s.value(Preset), "Default");
        s.step(Mesh, -1);
        assert_eq!((s.graphics.mesh, s.value(Preset).as_str()), (75, "Custom"));
        s.step(Mesh, 1);
        assert_eq!(s.graphics, d);
        s.step(Scale, -1);
        assert_eq!((s.graphics.scale, s.value(Preset).as_str()), (85, "Custom"));
        s.step(Display, 1);
        s.step(ShowFps, 1);
        s.step(Preset, 1);
        assert_eq!((s.graphics.windowed, s.graphics.show_fps, s.value(Preset).as_str(), s.value(Display).as_str()), (true, 1, "High", "Window"));
        assert!(Settings::parse(&s.text()).graphics.show_fps == 1 && s.flags().contains(&"--show-fps".into()));
        // B414: Lowest sits between Low and Phone
        let mut l = Settings { graphics: Graphics::LOW, ..Settings::default() };
        l.step(Preset, 1);
        assert_eq!((l.graphics, l.value(Preset).as_str()), (Graphics::LOWEST, "Lowest"));
        l.step(Preset, 1);
        assert_eq!(l.graphics, Graphics::PHONE);
        // P53: Auto after Phone (this device's pick, saved), then round to High; a graphics row by hand ends it
        l.step(Preset, 1);
        assert!(l.graphics.auto && l.value(Preset) == AUTO && Settings::parse(&l.text()) == l && choices(Preset).contains(&l.value(Preset)));
        l.step(Scale, -1);
        assert!(!l.graphics.auto && l.value(Preset) != AUTO);
        l.step(Preset, 1);
        assert_eq!((l.graphics.auto, l.value(Preset).as_str()), (false, "High"));
        l.step(Preset, -1);
        l.step(ShowFps, 1);
        assert!(l.graphics.auto && l.value(Preset) == AUTO);
        s.step(Original, 1);
        assert!(s.graphics.original && Settings::parse(&s.text()) == s && s.flags().contains(&"--original".into()));
        // B301: Key Hints, on by default, saved
        assert_eq!(s.value(KeyHints), "On");
        s.step(KeyHints, 1);
        assert!(!s.key_hints && s.text().contains("key_hints = off") && Settings::parse(&s.text()) == s);
        // B362: Button Icons, Auto by default, saved
        assert_eq!(s.value(ButtonIcons), "Auto");
        s.step(ButtonIcons, -1);
        assert!(s.value(ButtonIcons) == "Generic" && s.text().contains("button_icons = generic") && Settings::parse(&s.text()) == s);
        assert_eq!(choices(ButtonIcons), ["Auto", "PlayStation", "Xbox", "Switch", "Generic"]);
        s.step(Upscaled, 1);
        assert!(!s.upscaled_textures && !s.upscaled_downloads);
        s.step(Downloads, 1);
        assert!(!s.upscaled_textures && s.upscaled_downloads);
        s.step(Aspect, -1);
        assert_eq!(s.value(Aspect), ASPECTS[aspect_choices() - 1]);
        // B608: Stretch is no longer offered; a settings.txt's Stretch reads as the default
        assert_eq!(choices(Aspect).len(), aspect_choices());
        let mut t = s;
        t.aspect = crate::play::widescreen::STRETCH;
        let stretch = aspect_choices() == ASPECTS.len();
        assert!(t.text().contains("aspect = Stretch") && (Settings::parse(&t.text()) == t) == stretch);
        // B339: Downloads at Once steps Auto, 1..=8 and wraps; its side panel lists the same
        assert_eq!((s.value(AtOnce), choices(AtOnce).len()), ("Auto".into(), 9));
        s.step(AtOnce, -1);
        assert_eq!((s.value(AtOnce).as_str(), s.downloads_at_once()), ("8", 8));
        s.step(AtOnce, 1);
        s.step(AtOnce, 1);
        assert_eq!((s.value(AtOnce).as_str(), s.downloads_at_once(), Settings::parse(&s.text()) == s), ("1", 1, true));
        // B411: Sway LOD is gone; an old settings.txt's key is ignored
        assert_eq!(Settings::parse(&format!("{}sway_lod = on\n", s.text())), s);
    }
}

//! Scene and HUD blended the way the PS2's GS blends: on the gamma-encoded values, not in linear light.
//!
//! The 3D cameras draw HDR (float, nothing encoded) into `scene` (raw `Rgba8Unorm`), and every 3D shader outputs
//! encoded colour: `gs.wgsl` as it computes it, Bevy's PBR shader (`StandardMaterial`) through a patched
//! `main_pass_post_lighting_processing` that encodes its result; the clear colour follows `ClearColor` encoded. So
//! translucent and `@add` draws blend on encoded values as the GS does (grass blue 0x33 + 0x4c is 0x7f, not the
//! 0x98 a linear-light add gives).
//! The UI draws on its own camera into `hud` (cleared transparent, so it holds the UI premultiplied: colour·a, a),
//! and a last camera (`out`, on its own render schedule) puts `enc(hud / a)·a + scene·(1 − a)` on the window. Bevy
//! blends the UI into an sRGB target in linear light, which makes translucent pieces read stronger (the serve panel's
//! 35 % strip ≈ 0.45 over the court instead of ≈ 0.3).
//!
//! While a match runs (`Persist`) the scene also keeps 25 % of the last frame shown, as the game's last draw does:
//! a full-screen sprite of the other frame buffer at FIX 0x20 over the new frame (its fields shifted by half a line
//! to line up, which a progressive frame needs not). Two images take turns as this frame's mix and last frame's.
//! Instead of it, while the camera looks within ~37° of the sun in clear or cloudy weather, the game blurs the frame
//! by half a texel twice (`glare`), more the closer to the sun and the higher the camera looks.
//!
//! ponytail: UI pieces overlapping each other still blend in linear among themselves; only their sum goes over the
//! court in gamma. Fine while translucent HUD pieces don't stack; else draw the UI with raw (non-sRGB) values.

use bevy::camera::visibility::RenderLayers;
use bevy::camera::{CameraProjection, ImageRenderTarget, RenderTarget};
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureFormat};
use bevy::window::PrimaryWindow;

mod out;

pub fn plugin(app: &mut App) {
    bevy::asset::embedded_asset!(app, "hud_gamma.wgsl");
    out::plugin(app);
    app.add_systems(Startup, setup)
        .add_systems(Update, (encode_pbr_output, two_channel_letters))
        .add_systems(PostUpdate, (retarget, clear, follow_window, persist).chain().before(bevy::camera::CameraUpdateSystems))
        .add_systems(PostUpdate, bleed.before(bevy::ui::UiSystems::Layout))
        .add_systems(PostUpdate, bleed_outside.before(bevy::ui::UiSystems::Layout))
        .add_systems(PostUpdate, pixel_aspect.after(bevy::camera::CameraUpdateSystems));
}

#[derive(Resource)]
struct Targets {
    scene: Handle<Image>,
    hud: Handle<Image>,
    /// The persistence's two images: frame parity `i` mixes the scene and `last[1 - i]` into `last[i]`.
    last: [Handle<Image>; 2],
    /// B608: the HUD's camera, drawing only in the safe part of its image (`widescreen::view`).
    ui: Entity,
    /// B623: the camera drawing the copies of `widescreen::Bleed` nodes on the whole HUD image, under the HUD's camera
    /// (on only while the view is smaller than the window), and the root they're on.
    bleed: Entity,
    bleed_root: Entity,
}

/// Present while a match runs (the game's match scene turns its frame persistence on, menus have none).
#[derive(Resource)]
pub struct Persist;

/// The composite camera's layer, so no sprite or mesh of the game's ends up on it.
const LAYER: usize = 31;

/// B629: how bright the bars beside a fixed Aspect Ratio's picture glow with its edges, at the picture's edge (it fades
/// out away from it). 0 (the original mode: a 4:3 TV's black bars) leaves them black.
const GLOW: f32 = 0.45;

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    // P57: no pixels on the CPU side: with Bevy's zero fill each window-sized target was uploaded at the first frame
    // and again on every change (four screens' worth, ~33 MB of a 958 ms first frame in the browser at 4K); the GPU
    // starts them cleared anyway. No copy on resize either (it needs COPY_SRC, and the zero-filled data came back
    // blank after a resize before too)
    let target = |f| Image { data: None, copy_on_resize: false, ..Image::new_target_texture(1, 1, f, None) };
    let (scene, hud) = (images.add(target(TextureFormat::Rgba8Unorm)), images.add(target(TextureFormat::Rgba8UnormSrgb)));
    let ui = commands.spawn((
        Camera2d,
        Camera { order: 50, clear_color: ClearColorConfig::Custom(Color::NONE), ..default() },
        RenderTarget::Image(ImageRenderTarget { handle: hud.clone(), scale_factor: 1.0 }),
        IsDefaultUiCamera,
        Msaa::Off,
        // UI only: no layer, so the per-view visibility check rejects every mesh at once (P41)
        RenderLayers::none(),
    )).id();
    // B623: before the HUD's camera, into the same image; it clears it then, the HUD's camera draws over it
    let bleed = commands.spawn((
        Camera2d,
        Camera { order: 49, is_active: false, clear_color: ClearColorConfig::Custom(Color::NONE), ..default() },
        RenderTarget::Image(ImageRenderTarget { handle: hud.clone(), scale_factor: 1.0 }),
        Msaa::Off,
        RenderLayers::none(),
    )).id();
    let full = Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
    let bleed_root = commands.spawn((full, UiTargetCamera(bleed), Pickable::IGNORE)).id();
    // raw like the scene image, so the mix lands in it unencoded
    let last = [(); 2].map(|_| images.add(target(TextureFormat::Rgba8Unorm)));
    // the camera putting the composite on the window (B629: all of it, a fixed aspect ratio's bars included)
    commands.spawn(out::camera());
    // B629: the bars' glow, averaged from the picture (`out`), filtered up smoothly
    let glow = images.add(Image { sampler: ImageSampler::linear(), ..Image { data: None, ..Image::new_target_texture(out::GLOW_SIZE, out::GLOW_SIZE, TextureFormat::Rgba8Unorm, None) } });
    commands.insert_resource(out::Frame { scene: scene.clone(), hud: hud.clone(), last: last.clone(), mix: None, picture: (Vec4::new(0.0, 0.0, 1.0, 1.0), 0.0), glow });
    commands.insert_resource(Targets { scene, hud, last, ui, bleed, bleed_root });
}

/// A 3D camera drawing into the scene image.
#[derive(Component)]
pub struct Scene;

/// Every 3D camera drawing to the window draws into the scene image instead. No `Hdr` (P34): the main texture is then
/// the scene image's `Rgba8Unorm`, half `Rgba16Float`'s colour bandwidth, and the upscale copies the encoded values
/// across unconverted; an LDR view would dither in Bevy's PBR shader, so that's off.
fn retarget(mut commands: Commands, t: Res<Targets>, mut q: Query<(Entity, &mut RenderTarget), Added<Camera3d>>) {
    for (e, mut target) in &mut q {
        if matches!(*target, RenderTarget::Window(_)) {
            *target = RenderTarget::Image(ImageRenderTarget { handle: t.scene.clone(), scale_factor: 1.0 });
            commands.entity(e).try_insert((Scene, bevy::core_pipeline::tonemapping::DebandDither::Disabled));
        }
    }
}

/// The scene cameras clear to `ClearColor`'s encoded values.
fn clear(colour: Res<ClearColor>, mut q: Query<&mut Camera, With<Scene>>, added: Query<(), Added<Scene>>) {
    if !colour.is_changed() && added.is_empty() {
        return;
    }
    let c = colour.0.to_srgba();
    // an overlay drawing over the court (`ClearColorConfig::None`) keeps not clearing
    for mut cam in q.iter_mut().filter(|c| !matches!(c.clear_color, ClearColorConfig::None)) {
        cam.clear_color = ClearColorConfig::Custom(Color::linear_rgba(c.red, c.green, c.blue, c.alpha));
    }
}

/// Bevy's PBR fragment (every `StandardMaterial`) ends with the encoded colour, as `gs.wgsl` does.
fn encode_pbr_output(mut events: MessageReader<AssetEvent<Shader>>, mut shaders: ResMut<Assets<Shader>>) {
    const AT: &str = "#ifdef PREMULTIPLY_ALPHA\n    output_color = premultiply_alpha(pbr_input.material.flags, output_color);\n#endif\n    return output_color;";
    const ENC: &str = "    output_color = vec4(select(1.055 * pow(output_color.rgb, vec3(1.0 / 2.4)) - 0.055, output_color.rgb * 12.92, output_color.rgb <= vec3(0.0031308)), output_color.a);\n";
    for e in events.read() {
        let AssetEvent::Added { id } = e else { continue };
        let Some(shader) = shaders.get(*id) else { continue };
        if !shader.path.ends_with("bevy_pbr/render/pbr_functions.wgsl") {
            continue;
        }
        let bevy::shader::Source::Wgsl(src) = &shader.source else { continue };
        assert_eq!(src.matches(AT).count(), 1, "Bevy's pbr_functions.wgsl changed: re-find where its forward output ends");
        let patched = src.replace(AT, &format!("{ENC}{AT}"));
        shaders.get_mut(*id).unwrap().source = bevy::shader::Source::Wgsl(patched.into());
    }
}

/// B646a: the menus' letter sheet (`glyph_sheet`) holds two channels, not four: R the white letter's coverage, G the
/// whole letter's (white, and its outline's ink round it), both linear and premultiplied, so the GPU's filtering of
/// them is the blend of the white-over-ink texels it stands for. A node drawing from it says so by its colour's alpha
/// ([`letters`]); Bevy's UI shader rebuilds the colour from the two there.
pub const LETTER_INK: u8 = 40;

/// What [`letters`] adds to a node's alpha: past any alpha a node is given, so the shader can tell.
const LETTER_FLAG: f32 = 16.0;

/// A node's colour (tint, alpha) for drawing a quad of the two-channel letter sheet.
pub fn letters(c: Color) -> Color {
    let a = c.alpha();
    if a > 0.0 { c.with_alpha(a + LETTER_FLAG) } else { c }
}

/// [`LETTER_INK`] in linear light.
pub fn letter_ink() -> f32 {
    let c = LETTER_INK as f32 / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// Bevy's UI shader taking a node flagged by [`letters`] as one drawing the two-channel sheet: colour
/// `tint · (white + ink · (cover − white)) / cover`, alpha `alpha · cover`.
fn two_channel_letters(mut shaders: ResMut<Assets<Shader>>, mut done: Local<bool>) {
    const AT: &str = "    let color = select(in.color, in.color * texture_color, enabled(in.flags, TEXTURED));";
    if *done {
        return;
    }
    let Some((id, has)) = shaders.iter().find(|(_, s)| s.path.ends_with("bevy_ui_render/ui.wgsl")).map(|(id, s)| (id, matches!(&s.source, bevy::shader::Source::Wgsl(w) if w.contains(AT)))) else { return };
    *done = true;
    if !has {
        error!("UI shader: no `{AT}`: the menus' letters won't draw");
        return;
    }
    if let Some(mut s) = shaders.get_mut(id)
        && let bevy::shader::Source::Wgsl(w) = &mut s.source
    {
        *w = w.replace(AT, &letters_wgsl(AT)).into();
    }
}

/// `at` (the UI shader's colour line) made mutable and followed by the two-channel letters' colour.
fn letters_wgsl(at: &str) -> String {
    let (flag, half, ink) = (LETTER_FLAG, LETTER_FLAG / 2.0, letter_ink());
    format!(
        "{}\n    if in.color.a >= {half:?} {{\n        let cover = texture_color.g;\n        let white = min(texture_color.r, cover);\n        color = vec4(in.color.rgb * (white + (cover - white) * {ink:?}) / max(cover, 1e-6), (in.color.a - {flag:?}) * cover);\n    }}",
        at.replacen("let color", "var color", 1)
    )
}

/// B547: the most device pixels per CSS px (logical px) the images get: a phone's DPR 3 drew and held 2.25× the pixels
/// of 2 for a picture that looks the same at its pixel density (the composite upscales). Phones and the browser only:
/// a desktop window always draws at its native pixels (a Steam Deck's X server can report a 2.25 scale factor, and
/// the cap drew its menus' text at 1138×711 and upscaled it to the 1280×800 panel).
const MAX_DENSITY: f32 = if cfg!(any(target_os = "ios", target_os = "android", target_arch = "wasm32")) { 2.0 } else { f32::INFINITY };

/// Both images at the window's physical size and scale factor, as the window target was; the scene's times the
/// render scale setting (the composite upscales it); neither past [`MAX_DENSITY`]. The persistence's images only
/// while a match keeps them (`Persist`), else 1 px (B547: two more screens' worth held in the menus). The original
/// mode (`Graphics::original`) draws the scene at 448 lines and 640 columns per 4:3 width (the PS2's non-square
/// pixels, `pixel_aspect`) and the HUD at 448 lines, both scaled up unfiltered.
fn follow_window(
    t: Res<Targets>,
    g: Res<crate::graphics::Graphics>,
    persist: Option<Res<Persist>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut targets: Query<&mut RenderTarget>,
    mut cams: Query<&mut Camera>,
    mut frame: ResMut<out::Frame>,
    device: Option<Res<bevy::render::renderer::RenderDevice>>,
) {
    let Ok(w) = window.single() else { return };
    // B117: the scene image is the picture's size (B608: the safe area doesn't shrink it); B629: the composite covers
    // the window, the scene in the picture's rect (`picture`), the bars beside it a glow of its edges
    let phys = Vec2::new(w.physical_width() as f32, w.physical_height() as f32);
    let v = crate::widescreen::scene_view(phys);
    let (pos, vs) = (v.min.round().as_uvec2(), v.size().round().as_uvec2().max(UVec2::ONE));
    let rect = (pos.as_vec2() / phys.max(Vec2::ONE)).extend(0.0).extend(0.0) + Vec4::new(0.0, 0.0, vs.x as f32 / phys.x.max(1.0), vs.y as f32 / phys.y.max(1.0));
    let picture = (rect, if g.original { 0.0 } else { GLOW });
    if frame.picture != picture {
        frame.picture = picture;
    }
    let key = |v: &Option<bevy::camera::Viewport>| v.as_ref().map(|v| (v.physical_position, v.physical_size));
    let (pw, ph) = (vs.x as f32, vs.y as f32);
    let (ww, wh) = (phys.x.max(1.0), phys.y.max(1.0));
    let size = |x: f32, y: f32| Extent3d { width: (x.round() as u32).max(1), height: (y.round() as u32).max(1), depth_or_array_layers: 1 };
    let ((scene, ks), (hud, kh)) = if g.original {
        let k = 448.0 / ph.max(1.0);
        ((size(480.0 * pw / ph.max(1.0), 448.0), k), (size(ww * k, wh * k), k))
    } else {
        let cap = densest(w.scale_factor());
        let side = device.map_or(8192, |d| d.limits().max_texture_dimension_2d) as f32;
        let k = scene_scale(g.scale, cap, pw.max(ph), side);
        ((size((pw * k).floor(), (ph * k).floor()), k), (size((ww * cap).floor(), (wh * cap).floor()), cap))
    };
    // B608: the HUD's camera draws in the view's part of its image (the whole image is still cleared each frame);
    // B629: the image covers the window
    let ui = crate::widescreen::view(phys);
    let f = Vec2::new(hud.width as f32 / ww, hud.height as f32 / wh);
    let (at, inside) = ((ui.min * f).round().as_uvec2(), (ui.size() * f).round().as_uvec2().max(UVec2::ONE));
    let full = UVec2::new(hud.width, hud.height);
    let ui_viewport = (inside != full).then(|| bevy::camera::Viewport { physical_position: at, physical_size: inside.min(full - at.min(full)).max(UVec2::ONE), ..default() });
    // B623: a view inside the window: the bleed camera clears the image and draws past the view, the HUD's over it
    let bleeds = ui_viewport.is_some();
    if let Ok(mut cam) = cams.get_mut(t.bleed)
        && cam.is_active != bleeds
    {
        cam.is_active = bleeds;
    }
    if let Ok(mut cam) = cams.get_mut(t.ui) {
        let clear = if bleeds { ClearColorConfig::None } else { ClearColorConfig::Custom(Color::NONE) };
        if key(&cam.viewport) != key(&ui_viewport) {
            cam.viewport = ui_viewport;
        }
        if std::mem::discriminant(&cam.clear_color) != std::mem::discriminant(&clear) {
            cam.clear_color = clear;
        }
        // B657: over the bleed camera's picture it blends its own, which is premultiplied (Bevy's default for a second
        // camera on a target treats it as straight alpha: a see-through piece came out at about half its strength
        // whenever the view was inset); alone it writes it as is
        let blend = bleeds.then_some(bevy::render::render_resource::BlendState::PREMULTIPLIED_ALPHA_BLENDING);
        if matches!(cam.output_mode, bevy::camera::CameraOutputMode::Write { blend_state, .. } if blend_state != blend)
            && let bevy::camera::CameraOutputMode::Write { blend_state, .. } = &mut cam.output_mode
        {
            *blend_state = blend;
        }
    }
    let sampler = if g.original { ImageSampler::nearest() } else { ImageSampler::Default };
    let last = if persist.is_some() { scene } else { size(1.0, 1.0) };
    for (h, size) in [(&t.scene, scene), (&t.hud, hud), (&t.last[0], last), (&t.last[1], last)] {
        if images.get(h).is_some_and(|i| i.texture_descriptor.size != size) {
            images.get_mut(h).unwrap().resize(size);
        }
    }
    // the composite samples whichever of these it shows
    for h in [&t.scene, &t.last[0], &t.last[1]] {
        if images.get(h).is_some_and(|i| i.sampler != sampler) {
            images.get_mut(h).unwrap().sampler = sampler.clone();
        }
    }
    for mut target in &mut targets {
        let Some(scale) = (match &*target {
            RenderTarget::Image(i) if i.handle == t.scene => Some(w.scale_factor() * ks),
            RenderTarget::Image(i) if i.handle == t.hud => Some(w.scale_factor() * kh),
            _ => None,
        }) else {
            continue;
        };
        if let RenderTarget::Image(i) = &*target
            && i.scale_factor != scale
        {
            let handle = i.handle.clone();
            *target = RenderTarget::Image(ImageRenderTarget { handle, scale_factor: scale });
        }
    }
}

/// B623: a `widescreen::Bleed` node's copy, on the bleed camera's root.
#[derive(Component)]
struct BleedCopy;

/// B623: keeps a copy of each `widescreen::Bleed` node on the whole picture while its edges reach past the view: same
/// image, tint and corners, its rect carried out to the window's edges (`widescreen::bleed_out`). The copies go when
/// their nodes do.
fn bleed(
    mut commands: Commands,
    t: Res<Targets>,
    window: Query<&Window, With<PrimaryWindow>>,
    cams: Query<&Camera>,
    nodes: Query<(Entity, &crate::widescreen::Bleed, &Node, Option<&ImageNode>, Option<&BackgroundColor>, &InheritedVisibility), Without<crate::widescreen::BleedOutside>>,
    mut copies: Query<(&mut Node, &mut ImageNode, &mut BackgroundColor, &mut Visibility, &mut ZIndex), (With<BleedCopy>, Without<crate::widescreen::Bleed>)>,
    mut of: Local<bevy::platform::collections::HashMap<Entity, Entity>>,
) {
    of.retain(|node, copy| {
        // B630: a copy gone with its node's mode (the mode switch takes every entity it didn't start with, the copies
        // too, while the loading screen's cover stays) is made again
        let alive = nodes.contains(*node) && copies.contains(*copy);
        if !alive && let Ok(mut e) = commands.get_entity(*copy) {
            e.despawn();
        }
        alive
    });
    let on = cams.get(t.bleed).is_ok_and(|c| c.is_active);
    let size = window.single().map_or(Vec2::ONE, |w| Vec2::new(w.width(), w.height()));
    for (e, b, node, img, bg, vis) in &nodes {
        let pct = |v: Val| if let Val::Percent(p) = v { Some(p) } else { None };
        let rect = (|| Some([pct(node.left)?, pct(node.top)?, pct(node.width)?, pct(node.height)?]))();
        let out = rect.filter(|_| on && b.on && vis.get()).map(|r| crate::widescreen::bleed_out(r, size)).filter(|&[l, t, w, h]| l < 0.0 || t < 0.0 || l + w > 100.0 || t + h > 100.0);
        let Some(&copy) = of.get(&e) else {
            if let Some(r) = out {
                // B630: drawn from its first frame (a loading screen's cover made again under a new mode)
                let [left, top, width, height] = crate::widescreen::view_to_window(r, size).map(Val::Percent);
                let n = Node { position_type: PositionType::Absolute, left, top, width, height, right: Val::Auto, bottom: Val::Auto, margin: UiRect::ZERO, ..node.clone() };
                let img = img.cloned().unwrap_or_else(|| ImageNode { color: Color::NONE, ..default() });
                let bg = BackgroundColor(bg.map_or(Color::NONE, |b| b.0));
                let copy = commands.spawn((BleedCopy, img, bg, n, Visibility::Inherited, ZIndex(b.order), Pickable::IGNORE, ChildOf(t.bleed_root))).id();
                of.insert(e, copy);
            }
            continue;
        };
        let Ok((mut n, mut i, mut c, mut v, mut z)) = copies.get_mut(copy) else { continue };
        let Some(r) = out else {
            if *v != Visibility::Hidden {
                *v = Visibility::Hidden;
            }
            continue;
        };
        let [left, top, width, height] = crate::widescreen::view_to_window(r, size).map(Val::Percent);
        let want = Node { position_type: PositionType::Absolute, left, top, width, height, right: Val::Auto, bottom: Val::Auto, margin: UiRect::ZERO, ..node.clone() };
        if *n != want {
            *n = want;
        }
        let img = img.cloned().unwrap_or_else(|| ImageNode { color: Color::NONE, ..default() });
        if i.image != img.image || i.color != img.color || i.rect != img.rect || i.image_mode != img.image_mode || i.flip_x != img.flip_x || i.flip_y != img.flip_y {
            *i = img;
        }
        let bg = bg.map_or(Color::NONE, |b| b.0);
        if c.0 != bg {
            c.0 = bg;
        }
        if *v != Visibility::Inherited {
            *v = Visibility::Inherited;
        }
        if z.0 != b.order {
            z.0 = b.order;
        }
    }
}

/// B657: a `widescreen::BleedOutside` node's copy: only its pieces outside the view (`widescreen::outside_pieces`), each
/// showing its share of the node's image, so a see-through node isn't drawn twice over the view. Made, moved and
/// hidden as `bleed`'s copies.
#[allow(clippy::type_complexity)]
fn bleed_outside(
    mut commands: Commands,
    t: Res<Targets>,
    window: Query<&Window, With<PrimaryWindow>>,
    cams: Query<&Camera>,
    images: Res<Assets<Image>>,
    nodes: Query<(Entity, &crate::widescreen::Bleed, &Node, Option<&ImageNode>, Option<&BackgroundColor>, &InheritedVisibility), With<crate::widescreen::BleedOutside>>,
    mut copies: Query<(&mut Node, &mut ImageNode, &mut BackgroundColor, &mut Visibility, &mut ZIndex), (With<BleedCopy>, Without<crate::widescreen::Bleed>)>,
    mut of: Local<bevy::platform::collections::HashMap<Entity, [Entity; 4]>>,
) {
    of.retain(|node, pieces| {
        let alive = nodes.contains(*node) && pieces.iter().all(|p| copies.contains(*p));
        if !alive {
            for p in pieces.iter() {
                if let Ok(mut e) = commands.get_entity(*p) {
                    e.despawn();
                }
            }
        }
        alive
    });
    let on = cams.get(t.bleed).is_ok_and(|c| c.is_active);
    let size = window.single().map_or(Vec2::ONE, |w| Vec2::new(w.width(), w.height()));
    // the HUD camera's viewport (whole pixels of the HUD image) in window px: the pieces meet it with no seam
    let drawn = (|| {
        let vp = cams.get(t.ui).ok()?.viewport.as_ref()?;
        let k = size / images.get(&t.hud)?.size_f32().max(Vec2::ONE);
        Some(Rect::from_corners(vp.physical_position.as_vec2() * k, (vp.physical_position + vp.physical_size).as_vec2() * k))
    })();
    for (e, b, node, img, bg, vis) in &nodes {
        let pct = |v: Val| if let Val::Percent(p) = v { Some(p) } else { None };
        let rect = (|| Some([pct(node.left)?, pct(node.top)?, pct(node.width)?, pct(node.height)?]))();
        let pieces = rect.filter(|_| on && b.on && vis.get()).map(|r| crate::widescreen::outside_pieces(r, size, drawn)).unwrap_or_default();
        if pieces.is_empty() && !of.contains_key(&e) {
            continue;
        }
        // the part of the image the node shows (all of it without a rect), in texels
        let whole = img.and_then(|i| i.rect.or_else(|| images.get(&i.image).map(|im| Rect::from_corners(Vec2::ZERO, im.size_f32()))));
        let want = |k: usize| {
            let ([left, top, width, height], [u0, v0, u1, v1]) = *pieces.get(k)?;
            let n = Node { position_type: PositionType::Absolute, left: Val::Percent(left), top: Val::Percent(top), width: Val::Percent(width), height: Val::Percent(height), right: Val::Auto, bottom: Val::Auto, margin: UiRect::ZERO, ..node.clone() };
            let mut i = img.cloned().unwrap_or_else(|| ImageNode { color: Color::NONE, ..default() });
            if let (Some(img), Some(w)) = (img, whole) {
                let (u0, u1) = if img.flip_x { (1.0 - u1, 1.0 - u0) } else { (u0, u1) };
                let (v0, v1) = if img.flip_y { (1.0 - v1, 1.0 - v0) } else { (v0, v1) };
                let at = |u: f32, v: f32| w.min + Vec2::new(u, v) * w.size();
                i.rect = Some(Rect::from_corners(at(u0, v0), at(u1, v1)));
            }
            Some((n, i, BackgroundColor(bg.map_or(Color::NONE, |b| b.0))))
        };
        let copy = *of.entry(e).or_insert_with(|| {
            std::array::from_fn(|_| commands.spawn((BleedCopy, ImageNode::default(), BackgroundColor(Color::NONE), Node::default(), Visibility::Hidden, ZIndex(b.order), Pickable::IGNORE, ChildOf(t.bleed_root))).id())
        });
        for (k, p) in copy.into_iter().enumerate() {
            match (want(k), copies.get_mut(p)) {
                (Some((n, i, c)), Ok((mut pn, mut pi, mut pc, mut pv, mut pz))) => {
                    pn.set_if_neq(n);
                    if pi.image != i.image || pi.color != i.color || pi.rect != i.rect || pi.image_mode != i.image_mode || pi.flip_x != i.flip_x || pi.flip_y != i.flip_y {
                        *pi = i;
                    }
                    pc.set_if_neq(c);
                    pv.set_if_neq(Visibility::Inherited);
                    pz.set_if_neq(ZIndex(b.order));
                }
                (Some((n, i, c)), Err(_)) => {
                    // B630's case: spawned this frame (commands not applied yet): drawn from its first frame
                    commands.entity(p).insert((n, i, c, Visibility::Inherited));
                }
                (None, Ok((_, _, _, mut pv, _))) => {
                    pv.set_if_neq(Visibility::Hidden);
                }
                (None, Err(_)) => {}
            }
        }
    }
}

/// The scene image's size in window pixels at render scale `scale` % with the density cap `cap` ([`densest`]): up to
/// 100 % the scale, at most the cap; above it (P63: supersampling) the capped size times the scale, the composite
/// filtering it down, but no side past the GPU's largest texture (`side` px; the window's longer side is `long`).
fn scene_scale(scale: u32, cap: f32, long: f32, side: f32) -> f32 {
    let s = scale as f32 / 100.0;
    if s <= 1.0 { s.min(cap) } else { (cap * s).min(side / long.max(1.0)).max(cap) }
}

/// The share of a window's physical pixels the images may have at `scale_factor` device px per logical px
/// ([`MAX_DENSITY`]).
fn densest(scale_factor: f32) -> f32 {
    (MAX_DENSITY / scale_factor.max(1e-3)).min(1.0)
}

/// Each frame in a match: this parity's mix of the last frame into the scene, shown by the composite (`out`).
/// The game mixes 25 % per 60 Hz frame; at a higher frame rate more per frame, so the trail fades as fast.
fn persist(
    on: Option<Res<Persist>>,
    time: Res<Time<Real>>,
    t: Res<Targets>,
    sun: Option<Res<crate::shadow::Sun>>,
    view: Query<(&Projection, &Transform), With<crate::Orbit>>,
    images: Res<Assets<Image>>,
    mut frame: ResMut<out::Frame>,
    mut parity: Local<usize>,
) {
    *parity ^= 1;
    let i = *parity;
    let mix = on.map(|_| {
        let fix = match (sun, view.single()) {
            (Some(sun), Ok((Projection::Perspective(p), tf))) if crate::weather::now() < 2 => glare(sun.light, p, tf.rotation),
            _ => 0.0,
        };
        // the game's field texel: 640 columns per 4:3 width, 224 lines
        let size = images.get(&t.scene).map_or(UVec2::ONE, |i| i.size()).as_vec2();
        let texel = Vec2::new(size.y * 4.0 / 3.0 / 640.0 / size.x, 1.0 / 224.0);
        let w = if fix > 0.0 { 0.0 } else { 0.25f32.powf((time.delta_secs() * 60.0).clamp(0.25, 1.0)) };
        (i, Vec4::new(w, fix, texel.x, texel.y))
    });
    if frame.mix != mix {
        frame.mix = mix;
    }
}

/// The sun glare's blend weight (FIX / 128, 0 = off) for the match camera turned by `rot` (Bevy space) with
/// projection `p` and the sun lighting along `light` (game space, y down, sun → ground): on while the view is within
/// acos 0.8 of the sun, ((cos − 0.8)/0.2·0.5 + seen·0.2) · how far it looks up (clamped to 0..0.8)/0.8 · 128/0.7.
/// ponytail: `seen` is the lens flare's read-back of how much of the sun shows (0..1); taken as 1 while the sun is in
/// the picture, as `play/flare.rs` does, so a player in front of the sun doesn't weaken it.
fn glare(light: Vec3, p: &PerspectiveProjection, rot: Quat) -> f32 {
    let s = -light.normalize();
    let sun = Vec3::new(s.x, -s.y, -s.z);
    let fwd = rot * Vec3::NEG_Z;
    let c = fwd.dot(sun);
    if c <= 0.8 {
        return 0.0;
    }
    let sv = rot.inverse() * sun;
    let tan = (p.fov / 2.0).tan();
    let seen = (sv.x / -sv.z).abs() <= tan * p.aspect_ratio && (sv.y / -sv.z).abs() <= tan;
    let fix = ((c - 0.8) / 0.2 * 0.5 + if seen { 0.2 } else { 0.0 }) * (fwd.y.clamp(0.0, 0.8) / 0.8) * 128.0 / 0.7;
    (fix as u8) as f32 / 128.0
}

/// In the original mode the scene image's pixels aren't square (640 per 4:3 width): project for the window's aspect,
/// not the image's. Bevy recomputes `clip_from_view` from the image's aspect whenever the projection changes, so this
/// runs after it every frame. The frustum stays the image's, a little wider than the view: it only culls less.
fn pixel_aspect(g: Res<crate::graphics::Graphics>, window: Query<&Window, With<PrimaryWindow>>, mut q: Query<(&mut Camera, &Projection), With<Scene>>) {
    let Ok(w) = window.single() else { return };
    if !g.original {
        return;
    }
    let aspect = crate::widescreen::scene_aspect(w);
    for (mut cam, proj) in &mut q {
        if let Projection::Perspective(p) = proj {
            let clip = PerspectiveProjection { aspect_ratio: aspect, ..p.clone() }.get_clip_from_view();
            if cam.computed.clip_from_view != clip {
                cam.computed.clip_from_view = clip;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn density_capped_at_two() {
        // a DPR-3 phone: two thirds of its pixels; DPR 2 and below, and any desktop window: all
        if MAX_DENSITY.is_finite() {
            assert_eq!(densest(3.0), 2.0 / 3.0);
        } else {
            assert_eq!(densest(2.25), 1.0);
        }
        assert_eq!((densest(2.0), densest(1.25), densest(1.0)), (1.0, 1.0, 1.0));
    }

    #[test]
    fn supersampling_scale() {
        // up to 100 % as before; above it the capped size times the scale, within the largest texture
        assert_eq!((scene_scale(67, 1.0, 1920.0, 8192.0), scene_scale(100, 2.0 / 3.0, 1920.0, 8192.0)), (0.67, 2.0 / 3.0));
        assert_eq!((scene_scale(150, 1.0, 1280.0, 8192.0), scene_scale(200, 2.0 / 3.0, 1280.0, 8192.0)), (1.5, 4.0 / 3.0));
        assert_eq!((scene_scale(200, 1.0, 5120.0, 8192.0), scene_scale(200, 1.0, 9000.0, 8192.0)), (1.6, 1.0));
    }

    #[test]
    fn bleed_copy_made_again_after_a_mode_switch() {
        let mut world = World::new();
        world.spawn((Window { resolution: bevy::window::WindowResolution::new(1280, 720), ..default() }, PrimaryWindow));
        let bleed_cam = world.spawn(Camera { is_active: true, ..default() }).id();
        let bleed_root = world.spawn(Node::default()).id();
        let [scene, hud] = [Handle::default(), Handle::default()];
        world.insert_resource(Targets { scene, hud, last: [Handle::default(), Handle::default()], ui: bleed_cam, bleed: bleed_cam, bleed_root });
        // a backdrop past the view's sides (the loading screen's court)
        let node = Node { left: Val::Percent(-10.0), top: Val::Percent(0.0), width: Val::Percent(120.0), height: Val::Percent(100.0), ..default() };
        world.spawn((node, crate::widescreen::Bleed { order: 7, on: true }, InheritedVisibility::VISIBLE));
        let copies = |w: &mut World| w.query_filtered::<(&Visibility, &ZIndex), With<BleedCopy>>().iter(w).map(|(v, z)| (*v, z.0)).collect::<Vec<_>>();
        let run = world.register_system(bleed);
        world.run_system(run).unwrap();
        // shown from its first frame
        assert_eq!(copies(&mut world), [(Visibility::Inherited, 7)]);
        // the mode switch takes the copy; the next frame has it back
        let copy = world.query_filtered::<Entity, With<BleedCopy>>().single(&world).unwrap();
        world.despawn(copy);
        world.run_system(run).unwrap();
        assert_eq!(copies(&mut world), [(Visibility::Inherited, 7)]);
    }

    /// B657: a `BleedOutside` node's copy is only what's outside the view, its share of the image; it hides with the node.
    #[test]
    fn bleed_outside_copies_only_the_outside() {
        let mut world = World::new();
        world.spawn((Window { resolution: bevy::window::WindowResolution::new(1280, 720), ..default() }, PrimaryWindow));
        world.init_resource::<Assets<Image>>();
        let bleed_cam = world.spawn(Camera { is_active: true, ..default() }).id();
        let bleed_root = world.spawn(Node::default()).id();
        let [scene, hud] = [Handle::default(), Handle::default()];
        world.insert_resource(Targets { scene, hud, last: [Handle::default(), Handle::default()], ui: bleed_cam, bleed: bleed_cam, bleed_root });
        // a strip 20 % of the view tall, from 10 % past its left edge to 30 % in, showing texels 0..40 × 0..16
        let node = Node { left: Val::Percent(-10.0), top: Val::Percent(40.0), width: Val::Percent(40.0), height: Val::Percent(20.0), ..default() };
        let img = ImageNode { rect: Some(Rect::new(0.0, 0.0, 40.0, 16.0)), ..default() };
        let e = world.spawn((node, img, crate::widescreen::Bleed { order: 3, on: true }, crate::widescreen::BleedOutside, InheritedVisibility::VISIBLE)).id();
        let run = world.register_system(bleed_outside);
        world.run_system(run).unwrap();
        let shown = |w: &mut World| {
            w.query_filtered::<(&Node, &ImageNode, &Visibility, &ZIndex), With<BleedCopy>>().iter(w).filter(|c| *c.2 == Visibility::Inherited).map(|(n, i, _, z)| (n.left, n.width, i.rect, z.0)).collect::<Vec<_>>()
        };
        // the window is the view (no safe area): the quarter left of it, texels 0..10
        assert_eq!(shown(&mut world), [(Val::Percent(-10.0), Val::Percent(10.0), Some(Rect::new(0.0, 0.0, 10.0, 16.0)), 3)]);
        world.entity_mut(e).insert(crate::widescreen::Bleed { order: 3, on: false });
        world.run_system(run).unwrap();
        assert!(shown(&mut world).is_empty());
    }

    #[test]
    fn glare_strength() {
        let p = PerspectiveProjection { fov: 0.8, aspect_ratio: 4.0 / 3.0, ..default() };
        // sun 40° up, straight ahead (game space: −z ahead of a camera looking along Bevy −z, y down)
        let (e, look) = (40f32.to_radians(), |pitch: f32| Quat::from_rotation_x(pitch.to_radians()));
        let light = -Vec3::new(0.0, -e.sin(), e.cos());
        // looking straight at it: cos 1, sun in the picture, looking up sin 40° → ((0.5 + 0.2)·0.643/0.8·128/0.7) = 102
        assert_eq!(glare(light, &p, look(40.0)), 102.0 / 128.0);
        // looking level: 40° off (cos 0.766 < 0.8) → off; looking down → off
        assert_eq!(glare(light, &p, look(0.0)), 0.0);
        assert_eq!(glare(light, &p, look(-20.0)), 0.0);
    }

    /// B646a: a letter node's alpha carries the flag, a hidden one stays hidden; the shader line is made mutable and
    /// followed by the decode, which reads the flag back off.
    #[test]
    fn two_channel_letters_flagged() {
        let c = letters(Color::srgba(0.5, 0.25, 1.0, 0.75));
        assert_eq!((c.alpha(), c.to_srgba().red), (16.75, 0.5));
        assert_eq!(letters(Color::NONE).alpha(), 0.0);
        let at = "    let color = select(in.color, in.color * texture_color, enabled(in.flags, TEXTURED));";
        let w = letters_wgsl(at);
        assert!(w.starts_with("    var color = select(") && w.contains("in.color.a >= 8.0") && w.contains("(in.color.a - 16.0) * cover"), "{w}");
        assert!((letter_ink() - 0.0212).abs() < 1e-4);
    }
}

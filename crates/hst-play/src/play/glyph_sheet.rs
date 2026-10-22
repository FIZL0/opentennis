//! The menus' letters, set from our own font (M PLUS Rounded 1c Bold, OFL) at `K`×: every letter `word_label::FONT`
//! has, twice: plain, and again with its dark outline round it, so outlined words are one quad a letter with a
//! clean round ring (B646a: kept as two channels, `hud_gamma::letters`; draw it with that colour). Units are the disc sheet's: a line is 32 high, the baseline 28 down, a capital 22 tall.

use super::menu_art::{BOLD, K};
#[cfg(test)]
use super::menu_art::{outline, text_path, unpremultiply};
use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::collections::HashMap;
#[cfg(test)]
use tiny_skia::{Color as SkColor, Paint, Pixmap, Transform};

/// Room round each letter's line box (units): its outline and the odd accent above a capital.
pub const PAD: f32 = 8.0;
/// The outline's width (units): 2 px at a 16-px line, 4 on a 32-px title, thick as the original's.
#[cfg(test)]
const RING: f32 = 4.0;
/// The outline's colour (the menus' near-black).
#[cfg(test)]
const INK: u8 = crate::hud_gamma::LETTER_INK;
#[cfg(test)]
const BASELINE: f32 = 28.0;
const CAP: f32 = 22.0;
/// The sheet's width (units).
const WIDE: f32 = 1024.0;

/// Each letter's pen origin on the plain half (x, line top) and its advance, and how far down the outlined half is.
pub struct Glyphs {
    pub map: HashMap<char, (f32, f32, f32)>,
    pub outlined: f32,
}

/// Where each letter goes, without drawing it.
pub fn glyphs() -> &'static Glyphs {
    static G: std::sync::OnceLock<Glyphs> = std::sync::OnceLock::new();
    G.get_or_init(|| layout().0)
}

fn layout() -> (Glyphs, f32, f32) {
    use ab_glyph::Font;
    let font = ab_glyph::FontRef::try_from_slice(BOLD).expect("menu font");
    let em = em(&font);
    let unit = em / font.units_per_em().unwrap_or(1000.0);
    let (mut x, mut y, mut map) = (0.0, 0.0, HashMap::new());
    let line = 32.0 + 2.0 * PAD;
    for ch in super::word_label::FONT.concat().chars() {
        let id = font.glyph_id(ch);
        if id.0 == 0 || map.contains_key(&ch) {
            continue;
        }
        let adv = (font.h_advance_unscaled(id) * unit).round();
        if x + adv + 2.0 * PAD > WIDE {
            (x, y) = (0.0, y + line);
        }
        map.insert(ch, (x + PAD, y + PAD, adv));
        x += adv + 2.0 * PAD;
    }
    let half = y + line;
    (Glyphs { map, outlined: half }, em, half)
}

/// The font's size (units an em) that makes a capital `CAP` tall.
fn em(font: &ab_glyph::FontRef) -> f32 {
    use ab_glyph::Font;
    let h = font.outline(font.glyph_id('H')).map_or(0.7 * font.units_per_em().unwrap_or(1000.0), |o| (o.bounds.max.y - o.bounds.min.y).abs());
    CAP * font.units_per_em().unwrap_or(1000.0) / h
}

/// The sheet ([`glyphs`] says where each letter is), its levels down to a quarter, `textures::ours` scaled.
/// B645: built once and kept: the menus and the match's pause screen each want it, and drawing it again on every
/// switch between them (a 4096-wide sheet and its levels) was the biggest part of a start's memory peak in the browser,
/// where the heap never shrinks.
pub fn image(images: &mut Assets<Image>) -> Handle<Image> {
    static KEPT: std::sync::Mutex<Option<Handle<Image>>> = std::sync::Mutex::new(None);
    let mut kept = KEPT.lock().unwrap();
    let k = scale();
    // the sheet's own size, so a handle another app's store gave out (tests) isn't taken for it
    let size = ((WIDE * k as f32) as u32, (2.0 * glyphs().outlined * k as f32) as u32);
    if let Some(h) = kept.as_ref().filter(|h| images.get(h.id()).is_some_and(|i| (i.width(), i.height()) == size)) {
        return h.clone();
    }
    let handle = images.add(baked(k));
    crate::textures::ours(handle.id(), Vec2::splat(k as f32));
    *kept = Some(handle.clone());
    handle
}

/// The sheet's scale: `K`, or half that where memory is short (B644: 2048 px wide, ~12 MB with its levels instead of
/// ~50; the menus' text is at most 2× the disc's on a phone's screen, so it looks the same there).
fn scale() -> u32 {
    if crate::textures::low_memory() { K / 2 } else { K }
}

/// B646b: the sheet as baked into the game ([`BAKED`]), every level read straight into one buffer sized for them
/// all: no drawing or filtering at start, and nothing held beside the sheet but the PNG decoder's own state.
fn baked(k: u32) -> Image {
    use image::ImageDecoder;
    let (w, h) = ((WIDE * k as f32) as u32, (2.0 * glyphs().outlined * k as f32) as u32);
    let levels = crate::mips::levels_to_1x(k as f32 / 0.25);
    let file = BAKED.iter().find(|b| b.0 == k).expect("a baked letter sheet at this scale").1;
    let mut data = Vec::with_capacity(crate::mips::room(w, h, levels) / 2);
    let (mut rest, mut size) = (file, (w, h));
    for _ in 0..levels {
        let n = u32::from_le_bytes(rest[..4].try_into().expect("baked letter sheet")) as usize;
        let png = image::codecs::png::PngDecoder::new(std::io::Cursor::new(&rest[4..4 + n])).expect("baked letter sheet");
        assert_eq!((png.dimensions(), png.color_type()), (size, image::ColorType::La8), "baked letter sheet is stale: {STALE}");
        let at = data.len();
        data.resize(at + size.0 as usize * size.1 as usize * 2, 0);
        png.read_image(&mut data[at..]).expect("baked letter sheet");
        (rest, size) = (&rest[4 + n..], ((size.0 / 2).max(1), (size.1 / 2).max(1)));
    }
    let mut img = Image::new_uninit(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, TextureFormat::Rg8Unorm, RenderAssetUsages::RENDER_WORLD);
    img.data = Some(data);
    img.texture_descriptor.mip_level_count = levels;
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor { mipmap_filter: ImageFilterMode::Linear, ..ImageSamplerDescriptor::linear() });
    img
}

/// The sheet at each scale ([`scale`]) as `sheet` and `two_channels` make it, baked by `bake` (tests): in git, since
/// it's drawn from our own font. `baked_sheets_are_current` fails when they no longer match what the code draws.
static BAKED: [(u32, &[u8]); 2] = [
    (K, include_bytes!("../../../hst/assets/fonts/letters-k4.bin")),
    (K / 2, include_bytes!("../../../hst/assets/fonts/letters-k2.bin")),
];
const STALE: &str = "redraw it with HST_BAKE_LETTERS=1 tools/check.sh -p hst-play baked_sheets_are_current";

#[cfg(test)]
fn sheet(k: u32) -> Image {
    use ab_glyph::Font;
    let font = ab_glyph::FontRef::try_from_slice(BOLD).expect("menu font");
    let (g, em, half) = layout();
    let k = k as f32;
    let (w, h) = ((WIDE * k) as u32, (2.0 * half * k) as u32);
    let levels = crate::mips::levels_to_1x(k / 0.25);
    // B646: its buffer has room for the levels from the start, so they go in without it being copied to a bigger one
    let mut px = Vec::with_capacity(crate::mips::room(w, h, levels));
    px.resize(w as usize * h as usize * 4, 0);
    let mut out = Pixmap::from_vec(px, tiny_skia::IntSize::from_wh(w, h).expect("glyph sheet")).expect("glyph sheet");
    let s = em / font.units_per_em().unwrap_or(1000.0) * k;
    let mut white = Paint::default();
    white.set_color(SkColor::WHITE);
    let ink = SkColor::from_rgba8(INK, INK, INK, 255);
    for (&ch, &(x, y, _)) in &g.map {
        let Some(path) = text_path(&font, &ch.to_string()) else { continue };
        for (top, ring) in [(y, false), (y + half, true)] {
            let at = Transform::from_scale(s, s).post_translate(x * k, (top + BASELINE) * k);
            if ring && let Some(p) = path.clone().transform(at) {
                outline(&mut out, &p, ink, RING * k);
            }
            out.fill_path(&path, &white, tiny_skia::FillRule::Winding, at, None);
        }
    }
    let img = Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, unpremultiply(out), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
    crate::mips::MipSteps::new(img, levels, None, 1.5).finish()
}

#[cfg(test)]
/// B646a: the sheet (`Rgba8UnormSrgb`, straight, all its levels) as `Rg8Unorm` in the same buffer: R the white's
/// coverage, G the letter's, linear and premultiplied (`hud_gamma::letters`): half the bytes, here and on the GPU.
/// Each texel is white over the ink, so its colour and alpha say how much of each it holds.
fn two_channels(mut img: Image) -> Image {
    let ink = crate::hud_gamma::letter_ink();
    let lin: [f32; 256] = std::array::from_fn(|i| {
        let c = i as f32 / 255.0;
        if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    });
    if let Some(data) = img.data.as_mut() {
        let n = data.len() / 4;
        // texel i's two bytes land at or before its own four, so it's read before anything writes over it
        for i in 0..n {
            let (c, a) = (data[i * 4], data[i * 4 + 3]);
            let cover = a as f32 / 255.0;
            let white = ((lin[c as usize] - ink) / (1.0 - ink) * cover).clamp(0.0, cover);
            (data[i * 2], data[i * 2 + 1]) = ((white * 255.0 + 0.5) as u8, a);
        }
        data.truncate(n * 2);
        data.shrink_to_fit();
    }
    img.texture_descriptor.format = TextureFormat::Rg8Unorm;
    img
}

#[cfg(test)]
/// A sheet's levels as baked: each one a grey-and-alpha PNG (grey the white's coverage, alpha the letter's: the
/// `Rg8Unorm` texels as they are) after its length (`u32`, little-endian).
fn bake(img: &Image) -> Vec<u8> {
    use image::ImageEncoder;
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    let (mut w, mut h) = (img.width(), img.height());
    let data = img.data.as_deref().unwrap_or_default();
    let (mut at, mut out) = (0, Vec::new());
    for _ in 0..img.texture_descriptor.mip_level_count {
        let n = w as usize * h as usize * 2;
        let mut png = Vec::new();
        PngEncoder::new_with_quality(&mut png, CompressionType::Best, FilterType::Adaptive)
            .write_image(&data[at..at + n], w, h, image::ExtendedColorType::La8)
            .expect("baked letter sheet");
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&png);
        (at, w, h) = (at + n, (w / 2).max(1), (h / 2).max(1));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// B646b: the baked sheets are what the code draws now (font, `FONT`, layout, outline, levels): exact but for the
    /// odd texel a step off (another machine's float maths in the filter). With `HST_BAKE_LETTERS=1` it rewrites them.
    #[test]
    fn baked_sheets_are_current() {
        let bake_them = std::env::var("HST_BAKE_LETTERS").is_ok_and(|v| v == "1");
        for (k, _) in BAKED {
            let drawn = two_channels(sheet(k));
            if bake_them {
                let path = format!("{}/../hst/assets/fonts/letters-k{k}.bin", env!("CARGO_MANIFEST_DIR"));
                std::fs::write(&path, bake(&drawn)).unwrap();
                continue;
            }
            let read = baked(k);
            let dims = |i: &Image| (i.width(), i.height(), i.texture_descriptor.mip_level_count, i.texture_descriptor.format);
            assert_eq!(dims(&read), dims(&drawn), "{STALE}");
            let (a, b) = (read.data.as_ref().unwrap(), drawn.data.as_ref().unwrap());
            assert_eq!(a.len(), b.len(), "{STALE}");
            let off: Vec<u8> = a.iter().zip(b).map(|(x, y)| x.abs_diff(*y)).filter(|&d| d > 0).collect();
            assert!(off.iter().all(|&d| d <= 2) && off.len() * 1000 <= a.len(), "{} of {} bytes differ (most {}): {STALE}", off.len(), a.len(), off.iter().max().unwrap_or(&0));
            assert!(matches!(&read.sampler, ImageSampler::Descriptor(d) if d.mipmap_filter == ImageFilterMode::Linear));
        }
    }

    /// Every letter a name can hold is on the sheet, none overlapping the next or running off it.
    #[test]
    fn every_letter_has_its_own_cell() {
        let g = glyphs();
        let missing: String = super::super::word_label::FONT.concat().chars().filter(|c| !g.map.contains_key(c)).collect();
        assert!(missing.chars().all(|c| "˜".contains(c)), "missing {missing}");
        let mut cells: Vec<_> = g.map.values().map(|&(x, y, a)| (y, x - PAD, x + a + PAD)).collect();
        cells.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for w in cells.windows(2) {
            assert!(w[0].0 != w[1].0 || w[0].2 <= w[1].1, "{w:?} overlap");
        }
        assert!(cells.iter().all(|c| c.2 <= WIDE));
    }

    /// B644: the low-memory sheet is the same layout at half the scale: a quarter of the pixels, `H` still drawn where
    /// [`glyphs`] puts it.
    #[test]
    fn the_half_scale_sheet_keeps_the_layout() {
        let k = K / 2;
        let img = sheet(k);
        let half = glyphs().outlined;
        assert_eq!((img.width(), img.height()), ((WIDE * k as f32) as u32, (2.0 * half * k as f32) as u32));
        let (x, y, adv) = glyphs().map[&'H'];
        let kf = k as f32;
        let px = |u: f32, v: f32| img.data.as_ref().unwrap()[(((v * kf) as u32 * img.width() + (u * kf) as u32) * 4 + 3) as usize];
        assert!(px(x + adv / 2.0 - 0.0, y + BASELINE - CAP / 2.0) > 0 || px(x + 3.0, y + BASELINE - CAP / 2.0) > 0, "H's stem or bar");
        assert_eq!(px(x - PAD / 2.0, y + 2.0), 0, "the pad round it is clear");
    }

    /// B645: the sheet is drawn once and handed out again while its store holds it; a store that doesn't gets its own.
    #[test]
    fn the_sheet_is_drawn_once() {
        let mut images = Assets::<Image>::default();
        let a = image(&mut images);
        assert_eq!(image(&mut images), a);
        assert_eq!(images.len(), 1);
        let mut other = Assets::<Image>::default();
        let b = image(&mut other);
        assert!(other.get(&b).is_some_and(|i| i.data == images.get(&a).unwrap().data));
    }

    /// B645: straight alpha in the pixmap's own buffer, as each pixel's own `demultiply`.
    #[test]
    fn unpremultiplied_in_place() {
        let mut p = Pixmap::new(37, 23).unwrap();
        let path = text_path(&ab_glyph::FontRef::try_from_slice(BOLD).unwrap(), "Ag").unwrap();
        let mut paint = Paint::default();
        paint.set_color(SkColor::from_rgba8(200, 90, 30, 170));
        p.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::from_scale(0.02, 0.02).post_translate(2.0, 20.0), None);
        let want: Vec<u8> = p.pixels().iter().map(|c| c.demultiply()).flat_map(|c| [c.red(), c.green(), c.blue(), c.alpha()]).collect();
        assert!(want.chunks(4).any(|c| c[3] > 0 && c[3] < 255), "no part-covered pixel to check");
        assert_eq!(unpremultiply(p), want);
    }

    /// B646a: white is all white, the ink none, the half-covered keep their coverage; the sheet comes back with its
    /// levels at half the bytes, a texel's colour rebuilt from the two (as the UI shader does) within a step of it.
    #[test]
    fn two_channels_halve_the_sheet() {
        let ink = crate::hud_gamma::letter_ink();
        let lin = |c: u8| {
            let c = c as f32 / 255.0;
            if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        let px = [[255, 255, 255, 200], [INK, INK, INK, 255], [0; 4], [180, 180, 180, 128]];
        let img = Image::new(Extent3d { width: 2, height: 2, depth_or_array_layers: 1 }, TextureDimension::D2, px.concat(), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
        let rg = two_channels(img);
        let d = rg.data.as_ref().unwrap();
        assert_eq!(rg.texture_descriptor.format, TextureFormat::Rg8Unorm);
        assert_eq!(&d[..6], &[200, 200, 0, 255, 0, 0]);
        let (white, cover) = (d[6] as f32 / 255.0, d[7] as f32 / 255.0);
        assert_eq!(d[7], 128);
        assert!(((white + (cover - white) * ink) / cover - lin(180)).abs() < 2e-3);
        let k = K / 2;
        let rgba = sheet(k);
        let two = two_channels(rgba.clone());
        assert_eq!(two.texture_descriptor.mip_level_count, rgba.texture_descriptor.mip_level_count);
        assert_eq!(two.data.as_ref().unwrap().len() * 2, rgba.data.as_ref().unwrap().len());
    }
}

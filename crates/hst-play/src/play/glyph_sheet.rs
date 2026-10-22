//! The menus' letters, set from our own font (M PLUS Rounded 1c Bold, OFL) at `K`×: every letter `word_label::FONT`
//! has, twice: plain, and again with its dark outline round it, so outlined words are one quad a letter with a
//! clean round ring (B646a: kept as two channels, `hud_gamma::letters`; draw it with that colour). Units are the disc sheet's: a line is 32 high, the baseline 28 down, a capital 22 tall.

use super::menu_art::{BOLD, K, outline, text_path, unpremultiply};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::collections::HashMap;
use tiny_skia::{Color as SkColor, Paint, Pixmap, Transform};

/// Room round each letter's line box (units): its outline and the odd accent above a capital.
pub const PAD: f32 = 8.0;
/// The outline's width (units): 2 px at a 16-px line, 4 on a 32-px title, thick as the original's.
const RING: f32 = 4.0;
/// The outline's colour (the menus' near-black).
const INK: u8 = crate::hud_gamma::LETTER_INK;
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
    let handle = images.add(two_channels(sheet(k)));
    crate::textures::ours(handle.id(), Vec2::splat(k as f32));
    *kept = Some(handle.clone());
    handle
}

/// The sheet's scale: `K`, or half that where memory is short (B644: 2048 px wide, ~12 MB with its levels instead of
/// ~50; the menus' text is at most 2× the disc's on a phone's screen, so it looks the same there).
fn scale() -> u32 {
    if crate::textures::low_memory() { K / 2 } else { K }
}

fn sheet(k: u32) -> Image {
    use ab_glyph::Font;
    let font = ab_glyph::FontRef::try_from_slice(BOLD).expect("menu font");
    let (g, em, half) = layout();
    let k = k as f32;
    let (w, h) = ((WIDE * k) as u32, (2.0 * half * k) as u32);
    let mut out = Pixmap::new(w, h).expect("glyph sheet");
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
    crate::mips::MipSteps::new(img, crate::mips::levels_to_1x(k / 0.25), None, 1.5).finish()
}

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
mod tests {
    use super::*;

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

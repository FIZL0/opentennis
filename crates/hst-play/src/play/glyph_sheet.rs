//! The menus' letters, set from our own font (M PLUS Rounded 1c Bold, OFL) at `K`×: every letter `word_label::FONT`
//! has, twice: plain, and again with its dark outline round it, so outlined words are one quad a letter with a
//! clean round ring. Units are the disc sheet's: a line is 32 high, the baseline 28 down, a capital 22 tall.

use super::menu_art::{BOLD, K, text_path, unpremultiply};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::collections::HashMap;
use tiny_skia::{Color as SkColor, LineJoin, Paint, Pixmap, Stroke, Transform};

/// Room round each letter's line box (units): its outline and the odd accent above a capital.
pub const PAD: f32 = 5.0;
/// The outline's width (units): 1.5 px at a 16-px line, thicker on a title, thinner on small print.
const RING: f32 = 3.0;
/// The outline's colour (the menus' near-black).
const INK: u8 = 40;
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
pub fn image(images: &mut Assets<Image>) -> Handle<Image> {
    let handle = images.add(sheet());
    crate::textures::ours(handle.id(), Vec2::splat(K as f32));
    handle
}

fn sheet() -> Image {
    use ab_glyph::Font;
    let font = ab_glyph::FontRef::try_from_slice(BOLD).expect("menu font");
    let (g, em, half) = layout();
    let k = K as f32;
    let (w, h) = ((WIDE * k) as u32, (2.0 * half * k) as u32);
    let mut out = Pixmap::new(w, h).expect("glyph sheet");
    let s = em / font.units_per_em().unwrap_or(1000.0) * k;
    let mut white = Paint::default();
    white.set_color(SkColor::WHITE);
    let mut ink = Paint::default();
    ink.set_color_rgba8(INK, INK, INK, 255);
    let ring = Stroke { width: 2.0 * RING * k, line_join: LineJoin::Round, ..default() };
    for (&ch, &(x, y, _)) in &g.map {
        let Some(path) = text_path(&font, &ch.to_string()) else { continue };
        for (top, outline) in [(y, false), (y + half, true)] {
            let at = Transform::from_scale(s, s).post_translate(x * k, (top + BASELINE) * k);
            if outline {
                out.stroke_path(&path, &ink, &ring, at, None);
            }
            out.fill_path(&path, &white, tiny_skia::FillRule::Winding, at, None);
        }
    }
    let img = Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, unpremultiply(out), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
    crate::mips::MipSteps::new(img, crate::mips::levels_to_1x(k / 0.25), None, 1.5).finish()
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
}

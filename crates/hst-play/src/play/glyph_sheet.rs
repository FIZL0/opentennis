//! The menus' letters, set from our own font (M PLUS Rounded 1c Bold, OFL) at `K`×: every letter `word_label::FONT`
//! has, twice: plain, and again with its dark outline round it, so outlined words are one quad a letter with a
//! clean round ring. Units are the disc sheet's: a line is 32 high, the baseline 28 down, a capital 22 tall.

use super::menu_art::{BOLD, K, text_path, unpremultiply};
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
    for (&ch, &(x, y, _)) in &g.map {
        let Some(path) = text_path(&font, &ch.to_string()) else { continue };
        for top in [y, y + half] {
            let at = Transform::from_scale(s, s).post_translate(x * k, (top + BASELINE) * k);
            out.fill_path(&path, &white, tiny_skia::FillRule::Winding, at, None);
        }
    }
    // the ring grown round each outlined letter by distance, not stroked: a thick stroke round a small counter (an
    // `a`'s, an `e`'s) leaves holes where its inner edges cross
    let (w, h0) = (w as usize, (half * k) as usize);
    let ring = ring(&out.data()[w * h0 * 4..].iter().skip(3).step_by(4).copied().collect::<Vec<_>>(), w, RING * k);
    for (p, r) in out.pixels_mut()[w * h0..].iter_mut().zip(ring) {
        let (a, ri) = (p.alpha() as f32 / 255.0, r * (1.0 - p.alpha() as f32 / 255.0));
        let c = |v: u8| (v as f32 + INK as f32 * ri).round() as u8;
        *p = tiny_skia::PremultipliedColorU8::from_rgba(c(p.red()), c(p.green()), c(p.blue()), ((a + ri) * 255.0).round() as u8).unwrap_or(*p);
    }
    let img = Image::new(Extent3d { width: w as u32, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, unpremultiply(out), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
    crate::mips::MipSteps::new(img, crate::mips::levels_to_1x(k / 0.25), None, 1.5).finish()
}

/// How much of a ring `r` px wide round the inked (alpha ≥ half) pixels of `alpha` (`w` wide) covers each pixel.
fn ring(alpha: &[u8], w: usize, r: f32) -> Vec<f32> {
    let h = alpha.len() / w;
    let mut d: Vec<f32> = alpha.iter().map(|&a| if a >= 128 { 0.0 } else { 1e20 }).collect();
    let mut line = Vec::new();
    for y in 0..h {
        line.clear();
        line.extend_from_slice(&d[y * w..(y + 1) * w]);
        edt(&line, &mut d[y * w..(y + 1) * w]);
    }
    let mut out = vec![0.0; h];
    for x in 0..w {
        line.clear();
        line.extend((0..h).map(|y| d[y * w + x]));
        edt(&line, &mut out);
        (0..h).for_each(|y| d[y * w + x] = out[y]);
    }
    d.into_iter().map(|d2| (r + 0.5 - d2.sqrt()).clamp(0.0, 1.0)).collect()
}

/// Felzenszwalb–Huttenlocher's squared distance transform of one line `f` into `d`.
fn edt(f: &[f32], d: &mut [f32]) {
    let n = f.len();
    let (mut v, mut z, mut k) = (vec![0usize; n], vec![0f32; n + 1], 0);
    z[0] = f32::NEG_INFINITY;
    z[1] = f32::INFINITY;
    let cross = |q: usize, p: usize| ((f[q] + (q * q) as f32) - (f[p] + (p * p) as f32)) / (2.0 * (q as f32 - p as f32));
    for q in 1..n {
        let mut s = cross(q, v[k]);
        while s <= z[k] {
            k -= 1;
            s = cross(q, v[k]);
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = f32::INFINITY;
    }
    k = 0;
    for (q, d) in d.iter_mut().enumerate() {
        while z[k + 1] < q as f32 {
            k += 1;
        }
        *d = (q as f32 - v[k] as f32).powi(2) + f[v[k]];
    }
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

    /// The ring fills a gap narrower than itself whole and reaches its width round a dot.
    #[test]
    fn ring_fills_small_counters() {
        let w = 40;
        let mut a = vec![0u8; w * w];
        // a box with a 3-px hole, and nothing past it
        for y in 10..20 {
            for x in 10..20 {
                a[y * w + x] = if (14..17).contains(&x) && (14..17).contains(&y) { 0 } else { 255 };
            }
        }
        let r = ring(&a, w, 4.0);
        assert_eq!(r[15 * w + 15], 1.0);
        assert_eq!(r[15 * w + 22], 1.0);
        assert_eq!(r[15 * w + 23], 0.5);
        assert_eq!(r[15 * w + 25], 0.0);
    }
}

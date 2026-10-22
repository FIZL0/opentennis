//! B570: mip levels for the menus' own art. Their sheets and glyphs are drawn several times their on-screen size
//! (`menu_art::K`× the disc's, the key glyphs 64 px tall) so they stay sharp on a big window; on a small one (a low-res
//! screen, a phone) the GPU shrinks them 3–6× and, with only the one level, picks a texel here and there: jagged,
//! shimmering text. With a chain of halved levels, each filtered, and a sampler that blends between them, the text is
//! drawn from the level nearest its size on screen, at any window size, resolution or DPI, without being rebuilt.

use bevy::image::{Image, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::shader::{Shader, Source};

/// The UI shader's sampling, as Bevy has it, and as [`plugin`] puts it: biased toward the sharper level.
const SAMPLE: &str = "textureSample(sprite_texture, sprite_sampler, in.uv)";

/// The level bias: the GPU takes the level from the size on screen and blends it with the next smaller one, which
/// leaves small words a little soft; a quarter-to-half level sharper reads like a text rasteriser's output.
const BIAS: f32 = -0.75;

/// B570: Bevy's UI shader sampling its images `BIAS` levels sharper (images with one level are drawn as before).
pub fn plugin(app: &mut App) {
    app.add_systems(Update, sharpen_ui);
}

/// Once Bevy's UI shader is in, its one texture sample biased by [`BIAS`] (`textureSampleBias`, which wgpu and WebGL2
/// both have); the pipelines using it recompile on their own. If a Bevy update changes the line, nothing changes.
fn sharpen_ui(mut shaders: ResMut<Assets<Shader>>, mut done: Local<bool>) {
    if *done {
        return;
    }
    let found = shaders.iter().find(|(_, s)| s.path.ends_with("bevy_ui_render/ui.wgsl")).map(|(id, s)| (id, matches!(&s.source, Source::Wgsl(w) if w.contains(SAMPLE))));
    let Some((id, has)) = found else { return };
    *done = true;
    if !has {
        warn!("UI shader: no `{SAMPLE}` to bias");
        return;
    }
    if let Some(mut s) = shaders.get_mut(id)
        && let Source::Wgsl(w) = &mut s.source
    {
        *w = w.replace(SAMPLE, &format!("textureSampleBias(sprite_texture, sprite_sampler, in.uv, {BIAS:?})")).into();
    }
}

/// `img` (`Rgba8UnormSrgb`, straight alpha, one level) with up to `levels` levels in all, each half the last (rounded
/// down, at least 1 px), and a trilinear sampler. A level is the last one halved through a Lanczos filter in linear
/// light weighted by alpha (sharper than a 2×2 average, and a glyph's edge and outline keep their colours as they
/// shrink). Anything else comes back as it is.
pub fn with_mips(img: Image, levels: u32) -> Image {
    MipSteps::new(img, levels, None, 1.0).finish()
}

/// [`with_mips`] for a sheet of glyphs in `cell`-px square cells: each cell shrunk on its own (no level blends a glyph
/// into the next cell's), and each smaller level's coverage times `gain` (capped at full), as a text rasteriser darkens
/// its coverage: a stroke thinner than a pixel shrinks to a part-covered one, and the UI blends in linear light, which
/// shows dark words on a light pill paler than their ink. A gain rather than a curve, so a filter's faint ripple
/// round a glyph stays faint.
pub fn with_glyph_mips(img: Image, levels: u32, cell: u32, gain: f32) -> Image {
    MipSteps::new(img, levels, Some(cell as usize), gain).finish()
}

/// [`with_mips`] / [`with_glyph_mips`] a band of rows at a time ([`MipSteps::step`], a few ms each), for a sheet built
/// in steps so the browser draws frames in between (B409).
pub struct MipSteps {
    img: Image,
    levels: u32,
    /// The cell size at the level being read (halving with it), or `None` for the sheet as one.
    cell: Option<usize>,
    gain: f32,
    /// Levels done (1: just the image's own).
    n: u32,
    /// The level being read: its size and texels (premultiplied linear; `None` for the image's own bytes).
    w: usize,
    h: usize,
    src: Option<Vec<[f32; 4]>>,
    /// The level being made, a band of rows at a time.
    next: Vec<[f32; 4]>,
}

/// Output texels a [`MipSteps::step`] makes.
const STEP: usize = 1 << 15;

impl MipSteps {
    pub fn new(mut img: Image, levels: u32, cell: Option<usize>, gain: f32) -> MipSteps {
        let d = img.texture_descriptor.size;
        let fits = img.texture_descriptor.format == TextureFormat::Rgba8UnormSrgb
            && img.texture_descriptor.mip_level_count == 1
            && d.depth_or_array_layers == 1
            && img.data.as_ref().is_some_and(|p| p.len() == d.width as usize * d.height as usize * 4);
        let levels = if fits { levels } else { 1 };
        // B645: room for every level up front, so the sheet isn't copied into a bigger buffer (twice its size) as
        // they're added: in the browser a heap never gives back what a start peaked at
        let (mut lw, mut lh, mut rest) = (d.width as usize, d.height as usize, 0);
        for _ in 1..levels {
            if lw <= 1 && lh <= 1 {
                break;
            }
            (lw, lh) = ((lw / 2).max(1), (lh / 2).max(1));
            rest += lw * lh * 4;
        }
        if let Some(data) = img.data.as_mut() {
            data.reserve_exact(rest);
        }
        MipSteps { img, levels, cell, gain, n: 1, w: d.width as usize, h: d.height as usize, src: None, next: Vec::new() }
    }

    /// All the steps at once.
    pub fn finish(mut self) -> Image {
        loop {
            if self.step() {
                return self.img;
            }
        }
    }

    /// The image, once [`Self::step`] has said it's done.
    pub fn image(self) -> Image {
        self.img
    }

    /// Makes the next band of the level being made; true once every level is in.
    pub fn step(&mut self) -> bool {
        if self.n >= self.levels || (self.w <= 1 && self.h <= 1) {
            if self.n > 1 {
                self.img.texture_descriptor.mip_level_count = self.n;
                self.img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor { mipmap_filter: ImageFilterMode::Linear, ..ImageSamplerDescriptor::linear() });
            }
            self.levels = self.n;
            return true;
        }
        let (w, h) = (self.w, self.h);
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        // cells on whole texels at this level and the next, or the sheet as one
        self.cell = self.cell.filter(|&c| c >= 2 && c % 2 == 0 && w % c == 0 && h % c == 0);
        let (cx, cy) = self.cell.map_or((w, h), |c| (c, c));
        if self.next.is_empty() {
            self.next.reserve_exact(nw * nh);
        }
        let r0 = self.next.len() / nw;
        let r1 = (r0 + (STEP / nw).max(1)).min(nh);
        let data = self.img.data.as_ref().expect("checked in new");
        let lut = srgb_lut();
        let k = taps();
        // the input rows the band's taps reach (clamped to each one's cell), across first
        let row_of = |r: usize, t: usize| if h < 2 { 0 } else { let lo = (2 * r) / cy * cy; (2 * r + t).saturating_sub(3).clamp(lo, (lo + cy).min(h) - 1) };
        let (lo, hi) = (row_of(r0, 0), row_of(r1 - 1, 7));
        let mut across = vec![[0f32; 4]; (hi - lo + 1) * nw];
        let mut line = vec![[0f32; 4]; w];
        for y in lo..=hi {
            let row: &[[f32; 4]] = match &self.src {
                Some(s) => &s[y * w..][..w],
                None => {
                    data[y * w * 4..][..w * 4].as_chunks::<4>().0.iter().zip(line.iter_mut()).for_each(|(p, o)| *o = premultiplied(p, lut));
                    &line
                }
            };
            for i in 0..nw {
                let o = &mut across[(y - lo) * nw + i];
                if w < 2 {
                    *o = row[0];
                    continue;
                }
                let left = (2 * i) / cx * cx;
                let (first, last) = (left, (left + cx).min(w) - 1);
                if 2 * i >= first + 3 && 2 * i + 4 <= last {
                    let taps = &row[2 * i - 3..][..8];
                    for (p, &kt) in taps.iter().zip(&k) {
                        (0..4).for_each(|c| o[c] += p[c] * kt);
                    }
                } else {
                    for (t, &kt) in k.iter().enumerate() {
                        let p = row[(2 * i + t).saturating_sub(3).clamp(first, last)];
                        (0..4).for_each(|c| o[c] += p[c] * kt);
                    }
                }
            }
        }
        for r in r0..r1 {
            for i in 0..nw {
                let mut s = [0f32; 4];
                if h < 2 {
                    s = across[i];
                } else {
                    for (t, &kt) in k.iter().enumerate() {
                        let p = across[(row_of(r, t) - lo) * nw + i];
                        (0..4).for_each(|c| s[c] += p[c] * kt);
                    }
                }
                // the lobes ring a little past 0 and full
                let a = s[3].clamp(0.0, 1.0);
                self.next.push([s[0].clamp(0.0, a), s[1].clamp(0.0, a), s[2].clamp(0.0, a), a]);
            }
        }
        if r1 == nh {
            let enc = encode_lut();
            let gain = self.gain;
            let next = std::mem::take(&mut self.next);
            self.img.data.as_mut().expect("checked in new").extend(next.iter().flat_map(|&p| straight(p, (p[3] * gain).min(1.0), enc)));
            (self.src, self.w, self.h, self.n) = (Some(next), nw, nh, self.n + 1);
            self.cell = self.cell.map(|c| c / 2);
        }
        false
    }
}

/// The number of levels down to where `scale`× art is drawn 1:1 (`scale` 4 → 3: 4×, 2×, 1×); an atlas whose pieces
/// sit a texel or two apart at 1× stops there, so no level blends one piece into the next more than the disc's own
/// sheet would.
pub fn levels_to_1x(scale: f32) -> u32 {
    1 + scale.max(1.0).log2().floor() as u32
}

/// Lanczos-2 taps for halving: the 8 texels round an output texel, at -3.5…3.5 input texels from its centre.
fn taps() -> [f32; 8] {
    let sinc = |x: f32| if x == 0.0 { 1.0 } else { (std::f32::consts::PI * x).sin() / (std::f32::consts::PI * x) };
    let t: [f32; 8] = std::array::from_fn(|i| {
        let d = (i as f32 - 3.5) / 2.0;
        sinc(d) * sinc(d / 2.0)
    });
    let sum: f32 = t.iter().sum();
    t.map(|v| v / sum)
}

/// sRGB byte → linear light.
fn srgb_lut() -> &'static [f32; 256] {
    static LUT: std::sync::OnceLock<[f32; 256]> = std::sync::OnceLock::new();
    LUT.get_or_init(|| {
        std::array::from_fn(|i| {
            let c = i as f32 / 255.0;
            if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        })
    })
}

/// Linear light (0..1 in [`ENC`] steps) → sRGB byte.
fn encode_lut() -> &'static [u8] {
    static LUT: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    LUT.get_or_init(|| {
        (0..=ENC)
            .map(|i| {
                let l = i as f32 / ENC as f32;
                let c = if l <= 0.0031308 { l * 12.92 } else { 1.055 * l.powf(1.0 / 2.4) - 0.055 };
                (c * 255.0 + 0.5) as u8
            })
            .collect()
    })
}

const ENC: usize = 16384;

fn premultiplied(p: &[u8], lut: &[f32; 256]) -> [f32; 4] {
    let a = p[3] as f32 / 255.0;
    [lut[p[0] as usize] * a, lut[p[1] as usize] * a, lut[p[2] as usize] * a, a]
}

/// A premultiplied texel as straight sRGB bytes, its alpha shown as `shown`.
fn straight(p: [f32; 4], shown: f32, enc: &[u8]) -> [u8; 4] {
    let a = p[3];
    let e = |l: f32| enc[if a > 0.0 { ((l / a).clamp(0.0, 1.0) * ENC as f32 + 0.5) as usize } else { 0 }];
    [e(p[0]), e(p[1]), e(p[2]), (shown * 255.0 + 0.5) as u8]
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension};

    fn sheet(w: u32, h: u32, px: impl Fn(u32, u32) -> [u8; 4]) -> Image {
        let px = &px;
        let data = (0..h).flat_map(|y| (0..w).flat_map(move |x| px(x, y))).collect();
        Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD)
    }

    /// The level at `n` (sizes halving from `w`×`h`) of a mipped image's bytes.
    fn level(img: &Image, n: usize) -> &[u8] {
        let (mut w, mut h, mut at) = (img.width() as usize, img.height() as usize, 0);
        for _ in 0..n {
            at += w * h * 4;
            (w, h) = ((w / 2).max(1), (h / 2).max(1));
        }
        &img.data.as_ref().unwrap()[at..at + w * h * 4]
    }

    /// Levels halve down to the count asked (or 1×1); a white glyph on clear keeps white wherever it shows (no dark
    /// fringe from the clear texels' black) and a flat colour stays that colour; the sampler blends levels.
    #[test]
    fn levels_keep_colours() {
        let img = with_mips(sheet(16, 8, |x, _| if x < 8 { [255, 255, 255, 255] } else { [0; 4] }), 9);
        assert_eq!(img.texture_descriptor.mip_level_count, 5);
        assert_eq!(img.data.as_ref().unwrap().len(), (128 + 32 + 8 + 2 + 1) * 4);
        for n in 1..5 {
            assert!(level(&img, n).chunks(4).all(|p| p[3] == 0 || p[..3] == [255; 3]), "level {n}");
        }
        assert_eq!(&level(&img, 1)[..4], &[255; 4]);
        let flat = with_mips(sheet(8, 8, |_, _| [200, 40, 90, 255]), 3);
        assert!(level(&flat, 2).chunks(4).all(|p| p == [200, 40, 90, 255]));
        assert!(matches!(&img.sampler, ImageSampler::Descriptor(d) if d.mipmap_filter == ImageFilterMode::Linear));
        assert_eq!((levels_to_1x(4.0), levels_to_1x(1.0), levels_to_1x(3.0), levels_to_1x(16.0)), (3, 1, 2, 5));
    }

    /// Glyph cells shrink apart: ink against one cell's edge never reaches the next cell's texels at any level, where
    /// a whole-sheet filter does; and the gain fills a half-covered texel fuller.
    #[test]
    fn glyph_cells_stay_apart() {
        // two 8-px cells across; the left one inked in its last two columns
        let ink = |x: u32, _| if (6..8).contains(&x) { [255, 255, 255, 255] } else { [0; 4] };
        let cells = with_glyph_mips(sheet(16, 8, ink), 4, 8, 1.0);
        let whole = with_mips(sheet(16, 8, ink), 4);
        for n in 1..4 {
            let w = 16 >> n;
            let right = |img: &Image| level(img, n).chunks(4).enumerate().filter(|(i, _)| i % w >= w / 2).all(|(_, p)| p[3] == 0);
            assert!(right(&cells), "level {n}");
            assert!(n > 1 || !right(&whole));
        }
        let half = |gain| level(&with_glyph_mips(sheet(4, 4, |x, _| if x % 2 == 0 { [255; 4] } else { [0; 4] }), 2, 4, gain), 1)[3];
        assert!(half(1.5) > half(1.0) && half(1.0) >= 120, "{} {}", half(1.0), half(1.5));
    }
}

//! A disc word card traced sharp: its white rim, dark edge and fill each cut out of the disc's pixels, the outline of
//! each upscaled as a smooth contour (a distance field sampled between pixels) and coloured from the disc's own
//! pixels of that layer, so the letters keep the disc's exact shapes (the 1's curved stem, the 4's notch) and shading.

use tiny_skia::Pixmap;

/// Out, rim, edge, fill: each layer lies inside the one before it.
const LAYERS: usize = 4;
/// How far (disc px) a layer's colour spreads past its own pixels.
const REACH: i32 = 3;
/// A level's slope per disc px where the disc anti-aliases an edge (a pixel's width from in to out), and more: crisp.
const GAIN: f32 = 1.5;

/// The picture `w`×`h` (straight RGBA) at `k`× as a premultiplied pixmap.
pub(crate) fn trace(w: usize, h: usize, rgba: &[u8], k: u32) -> Pixmap {
    let px = |i: usize| [rgba[i * 4], rgba[i * 4 + 1], rgba[i * 4 + 2], rgba[i * 4 + 3]];
    let thr = (0..w * h).map(|i| px(i)[3]).max().unwrap_or(0) / 2;
    let layer = classify(w, h, &px, thr);
    let a_max = thr as f32 * 2.0;
    // each layer's boundary as a level (0.5) of a smooth value: the disc's own anti-aliasing places it between
    // pixels. The outline by alpha; the rim's inside by how dark (the rim is white, the edge dark); the fill's by how
    // bright (the edge is dark, the fill coloured)
    let fields: Vec<Vec<f32>> = (1..LAYERS as u8).map(|l| (0..w * h).map(|i| {
        let c = px(i);
        let (lo, hi) = (*c[..3].iter().min().unwrap() as f32, *c[..3].iter().max().unwrap() as f32);
        match (l, layer[i]) {
            (1, _) => c[3] as f32 / a_max,
            (2, 0) => 0.0,
            (2, 1) => 1.0 - lo / 255.0,
            (2, _) => 1.0,
            (3, 3) => ((hi - 40.0) / 160.0).clamp(0.0, 1.0),
            (3, 2) => ((hi - 40.0) / 160.0).clamp(0.0, 0.45),
            _ => 0.0,
        }
    }).collect::<Vec<f32>>()).map(|f| blur(w, h, &blur(w, h, &f))).collect();
    // the rim and the edge one colour each (the disc's in-between pixels are greys), the fill
    // shaded as the disc's
    let colours: Vec<Vec<[f32; 4]>> = (1..LAYERS).map(|l| match l {
        3 => colours(w, h, &layer, &px, 3),
        _ => vec![pick(&layer, &px, l as u8, a_max); w * h],
    }).collect();
    let (ow, oh) = (w as u32 * k, h as u32 * k);
    let mut out = Pixmap::new(ow, oh).expect("traced size");
    let kf = k as f32;
    for (o, p) in out.pixels_mut().iter_mut().enumerate() {
        // pixel centres: disc pixel i's centre is at i
        let u = ((o as u32 % ow) as f32 + 0.5) / kf - 0.5;
        let v = ((o as u32 / ow) as f32 + 0.5) / kf - 0.5;
        let cover = |f: &[f32]| (0.5 + (cubic(w, h, f, u, v) - 0.5) * kf * GAIN).clamp(0.0, 1.0);
        let outer = cover(&fields[0]);
        if outer == 0.0 {
            continue;
        }
        // the rim's colour, the edge's over it, the fill's over that, each as far as its layer covers
        let mut c = bilinear4(w, h, &colours[0], u, v);
        for (f, col) in fields.iter().zip(&colours).skip(1) {
            let t = cover(f);
            if t > 0.0 {
                let lc = bilinear4(w, h, col, u, v);
                c = std::array::from_fn(|i| c[i] + (lc[i] - c[i]) * t);
            }
        }
        let a = outer * c[3] / 255.0;
        *p = tiny_skia::ColorU8::from_rgba(c[0] as u8, c[1] as u8, c[2] as u8, (a * 255.0) as u8).premultiply();
    }
    out
}

/// Each pixel's layer: 0 clear, 1 the light rim (light pixels reached from outside without crossing the dark edge,
/// so the fill's light highlights stay fill), 2 the dark edge, 3 the fill.
fn classify(w: usize, h: usize, px: &impl Fn(usize) -> [u8; 4], thr: u8) -> Vec<u8> {
    let light = |c: [u8; 4]| c[..3].iter().min().unwrap() >= &185 && c[..3].iter().max().unwrap() - c[..3].iter().min().unwrap() <= 45;
    let dark = |c: [u8; 4]| *c[..3].iter().max().unwrap() < 120;
    let mut layer: Vec<u8> = (0..w * h).map(|i| match px(i) {
        c if c[3] < thr => 0,
        c if dark(c) => 2,
        _ => 3,
    }).collect();
    let mut todo: Vec<usize> = (0..w * h).filter(|&i| layer[i] == 0).collect();
    while let Some(i) = todo.pop() {
        let (x, y) = (i % w, i / w);
        for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                continue;
            }
            let n = ny as usize * w + nx as usize;
            if layer[n] == 3 && light(px(n)) {
                layer[n] = 1;
                todo.push(n);
            }
        }
    }
    // a fill pixel against the rim or clear is the edge's anti-aliasing (the edge always lies between)
    let touch = |i: usize| {
        let (x, y) = (i % w, i / w);
        [(x > 0, i.wrapping_sub(1)), (x + 1 < w, i + 1), (y > 0, i.wrapping_sub(w)), (y + 1 < h, i + w)]
            .into_iter().any(|(ok, n)| ok && layer[n] < 2)
    };
    let edge: Vec<usize> = (0..w * h).filter(|&i| layer[i] == 3 && touch(i)).collect();
    for i in edge {
        layer[i] = 2;
    }
    layer
}

/// A [1 2 1]² blur: the disc's pixel noise along a contour smoothed, a straight edge's level left where it was.
fn blur(w: usize, h: usize, f: &[f32]) -> Vec<f32> {
    let at = |x: i32, y: i32| f[y.clamp(0, h as i32 - 1) as usize * w + x.clamp(0, w as i32 - 1) as usize];
    (0..w * h).map(|i| {
        let (x, y) = ((i % w) as i32, (i / w) as i32);
        (-1..=1).flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
            .map(|(dx, dy)| at(x + dx, y + dy) * [1.0, 2.0, 1.0][(dx + 1) as usize] * [1.0, 2.0, 1.0][(dy + 1) as usize])
            .sum::<f32>() / 16.0
    }).collect()
}

/// Layer `l`'s colour: the rim its light ones' (the disc's rim pixels are greyed by the outline's anti-aliasing),
/// the edge its middle one by brightness, each solid (white for a sheet with no such layer, as an all-white flash is).
fn pick(layer: &[u8], px: &impl Fn(usize) -> [u8; 4], l: u8, a: f32) -> [f32; 4] {
    let mut all: Vec<[u8; 4]> = (0..layer.len()).filter(|&i| layer[i] == l).map(px).collect();
    all.sort_by_key(|c| c[0] as u32 + c[1] as u32 + c[2] as u32);
    all.get(all.len() * if l == 1 { 7 } else { 4 } / 8).map_or([255.0, 255.0, 255.0, a], |c| [c[0] as f32, c[1] as f32, c[2] as f32, a])
}

/// Layer `l`'s colours (straight RGBA), softened among its own pixels and spread out past them, so sampling between
/// pixels near its edge never takes another layer's colour.
fn colours(w: usize, h: usize, layer: &[u8], px: &impl Fn(usize) -> [u8; 4], l: u8) -> Vec<[f32; 4]> {
    let mine = |i: usize| layer[i] == l;
    let mut known: Vec<Option<[f32; 4]>> = (0..w * h).map(|i| mine(i).then(|| px(i).map(f32::from))).collect();
    // a 3×3 average among the layer's own pixels: the disc's dithering smoothed
    let soft: Vec<Option<[f32; 4]>> = (0..w * h).map(|i| known[i].map(|_| average(w, h, &known, i).unwrap())).collect();
    known = soft;
    for _ in 0..REACH + 1 {
        known = (0..w * h).map(|i| known[i].or_else(|| average(w, h, &known, i))).collect();
    }
    known.into_iter().map(|c| c.unwrap_or([0.0; 4])).collect()
}

fn average(w: usize, h: usize, known: &[Option<[f32; 4]>], i: usize) -> Option<[f32; 4]> {
    let (x, y) = ((i % w) as i32, (i / w) as i32);
    let (mut sum, mut n) = ([0.0f32; 4], 0.0);
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                continue;
            }
            if let Some(c) = known[ny as usize * w + nx as usize] {
                sum = std::array::from_fn(|j| sum[j] + c[j]);
                n += 1.0;
            }
        }
    }
    (n > 0.0).then(|| sum.map(|s| s / n))
}

fn corners(w: usize, h: usize, u: f32, v: f32) -> ([usize; 4], [f32; 4]) {
    let (x0, y0) = (u.floor().clamp(0.0, w as f32 - 1.0), v.floor().clamp(0.0, h as f32 - 1.0));
    let (fx, fy) = ((u - x0).clamp(0.0, 1.0), (v - y0).clamp(0.0, 1.0));
    let (x0, y0) = (x0 as usize, y0 as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    ([y0 * w + x0, y0 * w + x1, y1 * w + x0, y1 * w + x1], [(1.0 - fx) * (1.0 - fy), fx * (1.0 - fy), (1.0 - fx) * fy, fx * fy])
}

/// Catmull-Rom: smooth between pixels, so a level's contour curves instead of stepping.
fn cubic(w: usize, h: usize, f: &[f32], u: f32, v: f32) -> f32 {
    let weights = |t: f32| [((-0.5 * t + 1.0) * t - 0.5) * t, (1.5 * t - 2.5) * t * t + 1.0, ((-1.5 * t + 2.0) * t + 0.5) * t, (0.5 * t - 0.5) * t * t];
    let (x0, y0) = (u.floor(), v.floor());
    let (wx, wy) = (weights(u - x0), weights(v - y0));
    let at = |x: i32, y: i32| f[y.clamp(0, h as i32 - 1) as usize * w + x.clamp(0, w as i32 - 1) as usize];
    (0..4).map(|j| wy[j] * (0..4).map(|i| wx[i] * at(x0 as i32 - 1 + i as i32, y0 as i32 - 1 + j as i32)).sum::<f32>()).sum()
}

fn bilinear4(w: usize, h: usize, f: &[[f32; 4]], u: f32, v: f32) -> [f32; 4] {
    let (i, t) = corners(w, h, u, v);
    std::array::from_fn(|c| (0..4).map(|j| f[i[j]][c] * t[j]).sum())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A white-rimmed, dark-edged red square keeps its square corners and its layers' colours.
    #[test]
    fn keeps_layers_and_corners() {
        let (w, h) = (16, 16);
        let mut rgba = vec![0u8; w * h * 4];
        for y in 1..14 {
            for x in 1..14 {
                let ring = (x as i32 - 7).abs().max((y as i32 - 7).abs());
                let c = match ring {
                    6 => [255, 255, 255, 255],
                    5 => [40, 20, 20, 255],
                    _ => [200, 40, 40, 255],
                };
                rgba[(y * w + x) * 4..][..4].copy_from_slice(&c);
            }
        }
        let out = trace(w, h, &rgba, 4);
        let at = |x: u32, y: u32| out.pixel(x, y).unwrap().demultiply();
        let mid = at(30, 30);
        assert_eq!((mid.red(), mid.green(), mid.alpha()), (200, 40, 255));
        let rim = at(6, 30);
        assert!(rim.red() > 240 && rim.green() > 240, "rim {rim:?}");
        let edge = at(10, 30);
        assert!(edge.red() < 80, "edge {edge:?}");
        // the square's corner is still filled, not rounded off
        assert!(at(8, 8).alpha() > 200);
        assert_eq!(at(1, 30).alpha(), 0);
    }
}

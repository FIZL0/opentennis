//! A disc word card traced sharp: its white rim, dark edge and fill each cut out of the disc's pixels, the outline of
//! each traced as an outline, straightened into clean runs (corners and chips kept, the low-res wobble gone) and filled
//! sharp, coloured from the disc's own pixels of that layer: the disc's shapes (the 1's curved stem, the 4's notch)
//! and shading, without its blur.

use std::collections::HashMap;
use tiny_skia::{FillRule, LineJoin, Mask, PathBuilder, Pixmap, Stroke, Transform};

/// Out, rim, edge, fill: each layer lies inside the one before it.
const LAYERS: usize = 4;
/// How far (disc px) a layer's colour spreads past its own pixels.
const REACH: i32 = 3;
/// How far (disc px) an outline may stray from the disc's to run straight: the low-res wobble goes, the chips stay.
const STRAY: f32 = 0.35;
/// A turn sharper than this (radians) is a corner and stays sharp; gentler ones are rounded off as curves.
const CORNER: f32 = 0.6;

/// The picture `w`×`h` (straight RGBA) at `k`× as a premultiplied pixmap.
pub(crate) fn trace(w: usize, h: usize, rgba: &[u8], k: u32) -> Pixmap {
    let px = |i: usize| [rgba[i * 4], rgba[i * 4 + 1], rgba[i * 4 + 2], rgba[i * 4 + 3]];
    let thr = (0..w * h).map(|i| px(i)[3]).max().unwrap_or(0) / 2;
    let layer = classify(w, h, &px, thr);
    let a_max = thr as f32 * 2.0;
    // each layer's boundary as a level (0.5) of a softened value: the disc's own anti-aliasing places it between
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
            _ => 0.0,
        }
    }).collect::<Vec<f32>>()).map(|f| blur(w, h, &f)).collect();
    // the rim and the edge one colour each (the disc's in-between pixels are greys), the fill
    // shaded as the disc's
    let colours: Vec<Vec<[f32; 4]>> = (1..LAYERS).map(|l| match l {
        3 => colours(w, h, &layer, &px, 3),
        _ => vec![pick(&layer, &px, l as u8, a_max); w * h],
    }).collect();
    let (ow, oh) = (w as u32 * k, h as u32 * k);
    let mut out = Pixmap::new(ow, oh).expect("traced size");
    let kf = k as f32;
    // each layer's outline as clean paths, filled sharp at k×: disc pixel i's centre lands on output (i + 0.5)·k
    let at = Transform::from_row(kf, 0.0, 0.0, kf, kf / 2.0, kf / 2.0);
    let mut masks: Vec<Mask> = (0..fields.len()).map(|_| Mask::new(ow, oh).expect("traced size")).collect();
    // word by word (a word's letters share one rim): its fill traced, its edge and rim the disc's widths for that word
    // laid evenly round the fill, so thin outlines come out clean; a word with no fill (the white flash) by its outline
    let part = parts(w, h, &layer);
    // the bevel (output px): + a light lip inside the fill's top-left edge, − a dark one inside its bottom-right, as
    // wide as 0.4 of the word's edge
    let mut bevel = vec![0.0f32; (ow * oh) as usize];
    for n in 1..=part.iter().copied().max().unwrap_or(0) {
        let mine: Vec<Vec<f32>> = fields.iter().map(|f| (0..w * h).map(|i| if part[i] == n { f[i] } else { 0.0 }).collect()).collect();
        let area = |f: &[f32]| f.iter().filter(|&&v| v > 0.5).count() as f32;
        let Some((fill, len)) = outline(w, h, &mine[2]).filter(|_| area(&mine[2]) >= 4.0) else {
            for (m, f) in masks.iter_mut().zip(&mine) {
                if let Some((path, _)) = outline(w, h, f) {
                    m.fill_path(&path, FillRule::EvenOdd, true, at);
                }
            }
            continue;
        };
        // the anti-aliased pixels either side count whole: a third of a pixel off
        let edge = (area(&mine[1]) - area(&mine[2])) / len - 0.3;
        let rim = (area(&mine[0]) - area(&mine[1])) / len;
        let mut word = Mask::new(ow, oh).expect("traced size");
        word.fill_path(&fill, FillRule::EvenOdd, true, at);
        let d = (edge * kf * 0.4).round() as i32;
        let f = |x: i32, y: i32| if x < 0 || y < 0 || x >= ow as i32 || y >= oh as i32 { 0.0 } else { word.data()[(y * ow as i32 + x) as usize] as f32 / 255.0 };
        for (o, b) in bevel.iter_mut().enumerate().filter(|(o, _)| word.data()[*o] > 0) {
            let (x, y) = ((o as u32 % ow) as i32, (o as u32 / ow) as i32);
            *b = f(x, y) * ((1.0 - f(x - d, y - d)) * 0.6 - (1.0 - f(x + d, y + d)) * 0.2);
        }
        masks[2].fill_path(&fill, FillRule::EvenOdd, true, at);
        for (l, wd, join) in [(1, edge, LineJoin::Miter), (0, edge + rim, LineJoin::Round)] {
            let m = &mut masks[l];
            m.fill_path(&fill, FillRule::EvenOdd, true, at);
            let stroke = Stroke { width: 2.0 * wd.max(0.0), line_join: join, miter_limit: 2.0, ..Stroke::default() };
            if let Some(ring) = fill.stroke(&stroke, kf) {
                m.fill_path(&ring, FillRule::Winding, true, at);
            }
        }
    }
    for (o, p) in out.pixels_mut().iter_mut().enumerate() {
        let outer = masks[0].data()[o] as f32 / 255.0;
        if outer == 0.0 {
            continue;
        }
        let u = ((o as u32 % ow) as f32 + 0.5) / kf - 0.5;
        let v = ((o as u32 / ow) as f32 + 0.5) / kf - 0.5;
        // the rim's colour, the edge's over it, the fill's over that, each as far as its layer covers
        let mut c = bilinear4(w, h, &colours[0], u, v);
        for (m, col) in masks.iter().zip(&colours).skip(1) {
            let t = m.data()[o] as f32 / 255.0;
            if t > 0.0 {
                let lc = bilinear4(w, h, col, u, v);
                c = std::array::from_fn(|i| c[i] + (lc[i] - c[i]) * t);
            }
        }
        let b = bevel[o];
        c = std::array::from_fn(|i| if i == 3 { c[3] } else if b > 0.0 { c[i] + (255.0 - c[i]) * b } else { c[i] * (1.0 + b) });
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
    // its inner pixels only: those against another layer are the disc's anti-aliasing, half the neighbour's colour
    let mine = |i: usize| {
        let (x, y) = (i % w, i / w);
        layer[i] == l && x > 0 && y > 0 && x + 1 < w && y + 1 < h && [i - 1, i + 1, i - w, i + w].iter().all(|&n| layer[n] == l)
    };
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

/// The level-0.5 contour of `f` (disc px, pixel centres at integers) as closed loops, straightened and rounded.
fn outline(w: usize, h: usize, f: &[f32]) -> Option<(tiny_skia::Path, f32)> {
    // marching squares over the field padded with a clear border, so every loop closes; a crossing is keyed by its
    // grid edge (0 across, 1 down, at its top-left corner) and shared by the two cells either side
    let v = |x: i32, y: i32| if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 { 0.0 } else { f[y as usize * w + x as usize] };
    let mut segs: Vec<[(u8, i32, i32); 2]> = vec![];
    for y in -1..h as i32 {
        for x in -1..w as i32 {
            let c = [v(x, y), v(x + 1, y), v(x + 1, y + 1), v(x, y + 1)];
            let inside = c.map(|c| c > 0.5);
            let (t, r, b, l) = ((0, x, y), (1, x + 1, y), (0, x, y + 1), (1, x, y));
            let middle = c.iter().sum::<f32>() / 4.0 > 0.5;
            match inside {
                // saddles: the middle says which diagonal joins
                [true, false, true, false] if middle => segs.extend([[t, r], [b, l]]),
                [true, false, true, false] => segs.extend([[t, l], [r, b]]),
                [false, true, false, true] if middle => segs.extend([[t, l], [r, b]]),
                [false, true, false, true] => segs.extend([[t, r], [b, l]]),
                _ => {
                    let cross: Vec<_> = [(t, 0, 1), (r, 1, 2), (b, 3, 2), (l, 0, 3)].into_iter().filter(|e| inside[e.1] != inside[e.2]).map(|e| e.0).collect();
                    if let [a, b] = cross[..] {
                        segs.push([a, b]);
                    }
                }
            }
        }
    }
    let point = |(d, x, y): (u8, i32, i32)| {
        let (a, b) = (v(x, y), if d == 0 { v(x + 1, y) } else { v(x, y + 1) });
        let t = (0.5 - a) / (b - a);
        if d == 0 { (x as f32 + t, y as f32) } else { (x as f32, y as f32 + t) }
    };
    let mut ends: HashMap<(u8, i32, i32), Vec<usize>> = HashMap::new();
    for (i, s) in segs.iter().enumerate() {
        for e in s {
            ends.entry(*e).or_default().push(i);
        }
    }
    let mut used = vec![false; segs.len()];
    let (mut pb, mut len) = (PathBuilder::new(), 0.0);
    for start in 0..segs.len() {
        if used[start] {
            continue;
        }
        let (mut loop_, mut i, mut at) = (vec![], start, segs[start][1]);
        while !used[i] {
            used[i] = true;
            loop_.push(point(at));
            let Some(&n) = ends[&at].iter().find(|&&n| n != i) else { break };
            at = if segs[n][0] == at { segs[n][1] } else { segs[n][0] };
            i = n;
        }
        let pts = round(&straighten(&loop_));
        if pts.len() < 3 {
            continue;
        }
        len += (0..pts.len()).map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
        }).sum::<f32>();
        pb.move_to(pts[0].0, pts[0].1);
        for p in &pts[1..] {
            pb.line_to(p.0, p.1);
        }
        pb.close();
    }
    pb.finish().map(|p| (p, len))
}

/// Each pixel's word (1, 2, …; 0 clear): the touching (8 ways) pixels that aren't clear.
fn parts(w: usize, h: usize, layer: &[u8]) -> Vec<u32> {
    let mut part = vec![0u32; w * h];
    let mut n = 0;
    for start in 0..w * h {
        if layer[start] == 0 || part[start] != 0 {
            continue;
        }
        n += 1;
        part[start] = n;
        let mut todo = vec![start];
        while let Some(i) = todo.pop() {
            let (x, y) = ((i % w) as i32, (i / w) as i32);
            for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && nx < w as i32 && ny < h as i32 {
                    let j = ny as usize * w + nx as usize;
                    if layer[j] != 0 && part[j] == 0 {
                        part[j] = n;
                        todo.push(j);
                    }
                }
            }
        }
    }
    part
}

/// A closed loop's points cut to straight runs (Douglas–Peucker): none strays more than [`STRAY`] from them.
fn straighten(pts: &[(f32, f32)]) -> Vec<(f32, f32)> {
    fn run(p: &[(f32, f32)], out: &mut Vec<(f32, f32)>) {
        let (a, b) = (p[0], p[p.len() - 1]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let off = |q: &(f32, f32)| ((q.0 - a.0) * dy - (q.1 - a.1) * dx).abs() / len;
        match (1..p.len() - 1).max_by(|&i, &j| off(&p[i]).total_cmp(&off(&p[j]))) {
            Some(i) if off(&p[i]) > STRAY => {
                run(&p[..=i], out);
                run(&p[i..], out);
            }
            _ => out.push(a),
        }
    }
    if pts.len() < 4 {
        return pts.to_vec();
    }
    // split at the point farthest from the first, so both halves are open runs
    let d = |q: &(f32, f32)| (q.0 - pts[0].0).powi(2) + (q.1 - pts[0].1).powi(2);
    let far = (0..pts.len()).max_by(|&i, &j| d(&pts[i]).total_cmp(&d(&pts[j]))).unwrap();
    let mut closed = pts.to_vec();
    closed.push(pts[0]);
    let mut out = vec![];
    run(&closed[..=far], &mut out);
    run(&closed[far..], &mut out);
    out
}

/// Gentle turns rounded off (Chaikin, three passes), corners sharper than [`CORNER`] kept.
fn round(pts: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut pts = pts.to_vec();
    for _ in 0..3 {
        let n = pts.len();
        if n < 3 {
            break;
        }
        let mut out = Vec::with_capacity(n * 2);
        for i in 0..n {
            let (a, p, b) = (pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]);
            let turn = ((p.1 - a.1).atan2(p.0 - a.0) - (b.1 - p.1).atan2(b.0 - p.0)).abs();
            let turn = turn.min(std::f32::consts::TAU - turn);
            let lerp = |q: (f32, f32), t: f32| (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t);
            if turn > CORNER {
                out.push(p);
            } else {
                out.extend([lerp(a, 0.25), lerp(b, 0.25)]);
            }
        }
        pts = out;
    }
    pts
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

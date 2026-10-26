//! The disc's 8-bit palettised Windows BMPs (the loading screen's court pictures, `MENU/WALL/SUBnn.XB`).

use twox_hash::XxHash3_64;

/// A BMP's width, height and RGBA8 (opaque, top row first), or `None` if it isn't an 8-bit palettised one.
pub fn decode(d: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let u32_ = |o: usize| Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?));
    let (off, w, h) = (u32_(10)? as usize, u32_(18)?, u32_(22)?);
    if d.get(28) != Some(&8) {
        return None;
    }
    let pal = d.get(54..54 + 1024)?;
    let stride = (w as usize + 3) & !3;
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in (0..h as usize).rev() {
        for &i in d.get(off + y * stride..off + y * stride + w as usize)? {
            let c = &pal[i as usize * 4..];
            rgba.extend([c[2], c[1], c[0], 255]);
        }
    }
    Some((w, h, rgba))
}

/// A BMP's texture key (a replacement's `<name>-<key>.png`): it never goes through the GS, so it has no
/// [`GsTex::key`](crate::texhash::GsTex::key); the hash of the whole file, tagged so it can't meet one.
pub fn key(d: &[u8]) -> String {
    let mut b = b"BMP".to_vec();
    b.extend_from_slice(d);
    format!("{:016x}", XxHash3_64::oneshot(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_by_two() {
        // header, a 4-entry palette (B, G, R, 0) padded to 256, rows bottom first padded to 4 bytes
        let mut d = vec![0u8; 54 + 1024];
        d[..2].copy_from_slice(b"BM");
        d[10..14].copy_from_slice(&(54u32 + 1024).to_le_bytes());
        d[18..22].copy_from_slice(&2u32.to_le_bytes());
        d[22..26].copy_from_slice(&2u32.to_le_bytes());
        d[28] = 8;
        d[54..62].copy_from_slice(&[255, 0, 0, 0, 0, 255, 0, 0]);
        d.extend([0, 1, 0, 0, 1, 1, 0, 0]);
        let (w, h, rgba) = decode(&d).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(rgba, [0, 255, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 0, 255, 0, 255]);
        assert_eq!(key(&d).len(), 16);
        d[28] = 24;
        assert!(decode(&d).is_none());
    }
}

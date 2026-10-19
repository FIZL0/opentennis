//! The disc readers on any bytes (a damaged or wrong image must fail, not crash): first byte picks the reader.
#![no_main]
use hst_data::{ani, mdl, mtl, tim2, xb};

libfuzzer_sys::fuzz_target!(|d: &[u8]| {
    let Some((&pick, b)) = d.split_first() else { return };
    match pick % 6 {
        0 => drop(tim2::decode(b)),
        1 => drop(ani::parse(b)),
        2 => drop(ani::parse_cam(b)),
        3 => drop(mdl::parse(b)),
        4 => drop(mtl::parse(b, None)),
        _ => {
            if let Ok(x) = xb::Archive::parse(b) {
                for e in &x.entries {
                    let _ = x.read(e);
                }
            }
        }
    }
});

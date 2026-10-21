//! The disc readers on any bytes (a damaged or wrong image must fail, not crash): first byte picks the reader.
#![no_main]
use hst_data::{ani, iso, layout, mdl, mor, mtl, noi, snd, texhash, tim2, xb};

libfuzzer_sys::fuzz_target!(|d: &[u8]| {
    let Some((&pick, b)) = d.split_first() else { return };
    match pick % 14 {
        0 => drop(tim2::decode(b)),
        1 => drop(ani::parse(b)),
        2 => drop(ani::parse_cam(b)),
        3 => drop(mdl::parse(b)),
        4 => drop(mtl::parse(b, None)),
        5 => {
            if let Ok(x) = xb::Archive::parse(b) {
                for e in &x.entries {
                    let _ = x.read(e);
                }
            }
        }
        6 => {
            // an ISO or a pack: walk it, read every file, and pull the textures out of its archives
            if let Ok(mut x) = iso::Iso::from_source(Box::new(std::io::Cursor::new(b.to_vec()))) {
                let names: Vec<String> = x.files.keys().cloned().collect();
                for n in names {
                    let _ = x.read(&n);
                }
                texhash::disc_textures(&mut x, drop);
            }
        }
        7 => drop(mor::parse(b, 1)),
        8 => drop(mor::parse(b, 4)),
        9 => {
            if let Ok(bank) = snd::Bank::parse(b) {
                for p in 0..4 {
                    for k in 0..4 {
                        let _ = bank.key_ons(p, k);
                        let _ = bank.program(p, k as u8 * 40);
                    }
                    let _ = bank.set_volume(p);
                    let _ = bank.tone(p, 60);
                }
            }
        }
        10 => {
            if let Some(mut s) = snd::Sequencer::new(b) {
                for _ in 0..256 {
                    if s.done() {
                        break;
                    }
                    s.frame();
                }
            }
        }
        11 => {
            drop(layout::plants(b));
            drop(layout::entries(&String::from_utf8_lossy(b)));
            let _ = layout::cloud_count(b, b.get(b.len() / 2..).unwrap_or_default(), b.first().map_or(0, |&e| e as usize % 4));
        }
        12 => drop(noi::parse(b)),
        _ => {
            // texture names of whatever pictures decode
            for p in tim2::decode(b).into_iter().flatten() {
                drop(texhash::tim2_names(&p));
            }
        }
    }
});

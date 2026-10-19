//! A well-formed mod zip around one fuzzed file, so the fuzzer reaches the per-file checks (character model,
//! voice, texture-pack image) without first having to build a zip.
#![no_main]
use hst_base::modfs::{install, remove, zip};
use std::path::Path;

const CHARACTER: &[u8] = br#"{"standard":1,"id":"x","name":"X","costumes":["c.glb"],"donor":3,"hand":"right","params":{"base":3},"ai_row":3,"face":"none"}"#;
const PACK: &[u8] = br#"{"standard":1,"kind":"textures","id":"p","name":"P"}"#;

libfuzzer_sys::fuzz_target!(|d: &[u8]| {
    let Some((&pick, b)) = d.split_first() else { return };
    let glb: &[u8] = b"glTF";
    let files: Vec<(&str, &[u8])> = match pick % 4 {
        0 => vec![("mod.json", CHARACTER), ("c.glb", b)],
        1 => vec![("mod.json", CHARACTER), ("c.glb", glb), ("voice/0_0.wav", b)],
        2 => vec![("mod.json", PACK), ("textures/x.0-00000000000000aa.png", b)],
        _ => vec![("mod.json", b), ("c.glb", glb)],
    };
    let root = Path::new("fuzz/mods");
    let _ = install(&zip(&files, false, 0o644), root);
    remove(root);
});

//! Any bytes as the mod pack a peer sends in an online lobby (with the hash it announced, so the parser runs).
#![no_main]
use hst_base::mods_online::{hash, unpack};

libfuzzer_sys::fuzz_target!(|pack: &[u8]| {
    let cache = std::env::temp_dir().join(format!("hst-fuzz-pack-{}", std::process::id()));
    let _ = unpack(pack, hash(pack), &cache);
});

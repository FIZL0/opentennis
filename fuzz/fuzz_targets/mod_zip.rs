//! Any bytes as a downloaded or dropped-in mod zip: unpacked and every file checked, as the game installs one.
#![no_main]
use std::path::Path;

libfuzzer_sys::fuzz_target!(|zip: &[u8]| {
    let root = Path::new("fuzz/mods");
    let _ = hst_base::modfs::install(zip, root);
    hst_base::modfs::remove(root);
});

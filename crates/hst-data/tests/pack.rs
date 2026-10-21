//! A disc pack (`Iso::write_pack`) opens like the ISO and reads every packed file byte for byte, its hash is the same
//! on every import, and it is far smaller than the image. Skips without the ISO.

use hst_data::fixture;
use hst_data::iso::Iso;

#[test]
fn pack_reads_like_the_disc() {
    let Ok(mut iso) = fixture::iso() else { return };
    let dir = std::env::temp_dir().join(format!("hst-pack-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("disc.pack");
    let hash = iso.write_pack(&out, |_, _| {}).unwrap();
    // B612: our copy's hash is the known-good one the import checks against
    assert!(hst_data::iso::KNOWN_PACKS.iter().any(|&(k, _)| k == hash), "{hash:016x}");
    let mut pack = Iso::open(&out).unwrap();
    assert_eq!(pack.pack_hash, Some(hash));
    assert!(pack.files.contains_key("ZZBIN/GAME.BIN") && !pack.files.contains_key("A/OPEN.PSS"));
    let names: Vec<String> = pack.files.keys().cloned().collect();
    for n in &names {
        assert_eq!(pack.read(&n.to_lowercase()).unwrap(), iso.read(n).unwrap(), "{n}");
    }
    let (p, i) = (std::fs::metadata(&out).unwrap().len(), std::fs::metadata(fixture::path(fixture::ISO)).unwrap().len());
    assert!(p * 5 < i, "pack {p} bytes vs ISO {i}");
    // the same disc always gives the same pack (M2e: peers compare this hash)
    assert_eq!(iso.write_pack(&out, |_, _| {}).unwrap(), hash);
    // the browser's path: written into any writer, mounted from memory, opened by its mount path
    let mut mem = std::io::Cursor::new(Vec::new());
    assert_eq!(iso.write_pack_to(&mut mem, |_, _| {}).unwrap(), hash);
    assert_eq!(mem.get_ref(), &std::fs::read(&out).unwrap());
    hst_data::iso::mount("mounted.pack", mem.into_inner().into());
    let mut m = Iso::open("mounted.pack").unwrap();
    assert_eq!((m.pack_hash, m.read("zzbin/game.bin").unwrap()), (Some(hash), iso.read("ZZBIN/GAME.BIN").unwrap()));
    std::fs::remove_file(&out).unwrap();
    std::fs::remove_dir(&dir).unwrap();
}

#[test]
fn rejects_other_files() {
    let dir = std::env::temp_dir().join(format!("hst-pack-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("not.iso");
    std::fs::write(&f, vec![0u8; 40 * 2048]).unwrap();
    assert!(Iso::open(&f).is_err());
    std::fs::remove_file(&f).unwrap();
    std::fs::remove_dir(&dir).unwrap();
}

/// One byte flipped in a packed file (a bad rip that still parses) is refused with its hash, and no pack is left behind.
#[test]
fn rejects_unknown_contents() {
    use std::io::{Read, Seek, SeekFrom};
    struct Flip(std::fs::File, u64);
    impl Read for Flip {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let pos = self.0.stream_position()?;
            let n = self.0.read(buf)?;
            if (pos..pos + n as u64).contains(&self.1) {
                buf[(self.1 - pos) as usize] ^= 1;
            }
            Ok(n)
        }
    }
    impl Seek for Flip {
        fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
            self.0.seek(to)
        }
    }
    let Ok(iso) = fixture::iso() else { return };
    let (off, size) = iso.files.iter().find(|(n, _)| n.starts_with("ZZDATA/") || n.ends_with(".XB")).map(|(_, &e)| e).unwrap();
    let file = std::fs::File::open(fixture::path(fixture::ISO)).unwrap();
    let mut bad = Iso::from_source(Box::new(Flip(file, off + size as u64 / 2))).unwrap();
    let dir = std::env::temp_dir().join(format!("hst-pack-flip-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("disc.pack");
    let e = bad.write_pack(&out, |_, _| {}).unwrap_err();
    assert_eq!(e.kind(), std::io::ErrorKind::InvalidData);
    assert!(e.to_string().starts_with("unrecognised disc contents (bad rip or different version), pack hash "), "{e}");
    assert!(!out.exists() && !out.with_extension("part").exists());
    std::fs::remove_dir(&dir).unwrap();
}

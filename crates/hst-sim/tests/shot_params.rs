//! The shot parameter table built from the disc must equal the game's own runtime table (captured from a
//! save state's RAM: `fixtures/shot_params_ram.bin`). Needs the extracted disc under `fixtures/iso`.

use hst_data::fixture;
use hst_data::exe::Game;
use hst_sim::params::ShotParams;

#[test]
fn built_table_equals_the_games_runtime_table() {
    let ctx = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");
    let (Ok(cnf), Ok(bin), Ok(ram)) = (
        fixture::read(format!("{ctx}/iso/SYSTEM.CNF")),
        fixture::read(format!("{ctx}/iso/ZZBIN/GAME.BIN")),
        fixture::read(format!("{ctx}/shot_params_ram.bin")),
    ) else {
        fixture::skip(format_args!("fixture missing, skipped"));
        return;
    };
    let src = Game::new(&cnf, &bin).unwrap().shot_params();
    let built = ShotParams::build(&src.base, &src.kinds, &src.weights, src.middle_mix);
    let want: Vec<f32> = ram.as_chunks::<4>().0.iter().map(|&c| f32::from_le_bytes(c)).collect();
    let mut checked = 0;
    for class in 0..4 {
        for kind in 0..src.kinds[class] as usize {
            for r in 0..hst_sim::params::RECORDS {
                let got = built.record(class, kind, r);
                let o = class * 0x451 + kind * 0xdd + r * 0xd;
                for f in 0..13 {
                    assert_eq!(got[f].to_bits(), want[o + f].to_bits(), "class {class} kind {kind} record {r} field {f}: {} vs {}", got[f], want[o + f]);
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, (4 + 5 + 5 + 2) * 17);
}

/// The variant records (filled at start-up) equal the slot-5 save state's (`fixtures/slot5_ee.bin`).
#[test]
fn variant_records_equal_the_games() {
    let ctx = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");
    let (Ok(cnf), Ok(bin), Ok(ram)) = (
        fixture::read(format!("{ctx}/iso/SYSTEM.CNF")),
        fixture::read(format!("{ctx}/iso/ZZBIN/GAME.BIN")),
        fixture::read(format!("{ctx}/slot5_ee.bin")),
    ) else {
        fixture::skip(format_args!("fixture missing, skipped"));
        return;
    };
    let game = Game::new(&cnf, &bin).unwrap();
    let src = game.shot_params();
    let built = ShotParams::build(&src.base, &src.kinds, &src.weights, src.middle_mix);
    let mut checked = 0;
    for ch in 0..14 {
        for (n, v) in game.shot_variants(ch).iter().enumerate() {
            let got = built.variant(v.class, v.kind, hst_sim::params::record_of(ch), v.weight);
            let o = 0x40_8e60 + ch * 0x8b8 + n * 0x48 + 0x10;
            for f in 0..13 {
                let want = f32::from_le_bytes(ram[o + f * 4..o + f * 4 + 4].try_into().unwrap());
                assert_eq!(got[f].to_bits(), want.to_bits(), "character {ch} variant {n} field {f}: {} vs {want}", got[f]);
            }
            checked += 1;
        }
    }
    assert!(checked >= 14 * 18, "{checked}");
}

/// P117: a smash with a non-zero launch blend mixes its spin group (launch and first-bounce spin, side angle,
/// restitution pair) toward the character's `dw1` smash record, the base record when none is listed. On the
/// disc every such variant's spin group is the base record's, so the launch's base `smash_spins` is exact.
#[test]
fn smash_down_variants_keep_the_base_spin_group() {
    let ctx = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");
    let (Ok(cnf), Ok(bin)) = (fixture::read(format!("{ctx}/iso/SYSTEM.CNF")), fixture::read(format!("{ctx}/iso/ZZBIN/GAME.BIN"))) else {
        return fixture::skip(format_args!("the extracted disc absent, skipped"));
    };
    let game = Game::new(&cnf, &bin).unwrap();
    let src = game.shot_params();
    let built = ShotParams::build(&src.base, &src.kinds, &src.weights, src.middle_mix);
    let spin = hst_sim::params::field::SPIN_GROUP;
    let mut listed = 0;
    for ch in 0..14 {
        let r = hst_sim::params::record_of(ch);
        for v in game.shot_variants(ch).iter().filter(|v| v.class == 3 && v.uses[1] != 0) {
            let (base, dw1) = (built.record(3, v.kind, r), built.variant(3, v.kind, r, v.weight));
            assert_eq!(base[spin.clone()].iter().map(|x| x.to_bits()).collect::<Vec<_>>(), dw1[spin.clone()].iter().map(|x| x.to_bits()).collect::<Vec<_>>(), "character {ch} smash kind {}", v.kind);
            listed += 1;
        }
    }
    assert!(listed >= 14, "{listed} dw1 smash variants");
}

//! Every known release reads the same way (M163a): the Japanese disc's tables come out equal to the US disc's
//! except where its own tuning differs and its files answer to the US names; the European disc's come out equal.
//! Needs the US disc and `fixtures/discs/SCPS-15113.iso` / `SCES-54535.iso` (skipped without them).

use hst_data::exe::{Game, Menu, Release, bank_volumes, court_plates, sound_tables, stereo_tables};
use hst_data::fixture;
use hst_data::iso::Iso;

const JAPAN: &str = "fixtures/discs/SCPS-15113.iso";
const EUROPE: &str = "fixtures/discs/SCES-54535.iso";

fn japan() -> Option<Iso> {
    if !fixture::path(JAPAN).exists() {
        // not the user's disc: a second release is optional even under HST_REQUIRE_DISC
        eprintln!("skipped: no {JAPAN}");
        return None;
    }
    Some(Iso::open(fixture::path(JAPAN)).unwrap())
}

/// Every table `Game` reads, as text, keyed by what it is.
fn tables(g: &Game) -> Vec<(String, String)> {
    let mut t: Vec<(String, String)> = Vec::new();
    let mut put = |k: String, v: String| t.push((k, v));
    let p = g.shot_params();
    put("shot base".into(), format!("{:?}", (p.base, p.kinds, p.middle_mix)));
    for (r, w) in p.weights.chunks(31).enumerate() {
        put(format!("shot weights {r}"), format!("{w:?}"));
    }
    for c in 0..14 {
        put(format!("shot_variants {c}"), format!("{:?}", g.shot_variants(c)));
        put(format!("shot_effects {c}"), format!("{:?}", g.shot_effects(c)));
        put(format!("umpire_side {c}"), format!("{:?}", g.umpire_side(c)));
    }
    // material ids past 121 are no material: whatever program bytes follow the table
    put("misc".into(), format!("{:?}", (g.line_margin(), &g.surfaces()[..122], g.bounce_looks(), g.foot(), g.bounce_debris(), g.tornado_fade(), g.tornado_floor())));
    put("court_margins".into(), format!("{:?}", g.court_margins()));
    put("second_toss".into(), format!("{:?}", g.second_toss()));
    put("serve_miss".into(), format!("{:?}", g.serve_miss()));
    // a camera shot's bytes 64..68 are a pointer into the program (which sits elsewhere in each release), unused
    let shots: Vec<_> = g.camera_shots().into_iter().map(|mut s| (s.record.splice(64..68, [0; 4]).count(), s)).collect();
    put("camera".into(), format!("{:?}", (shots, g.cutaway_lists(), g.replay_cameras())));
    for l in [0u8, 1, 2, 3, 4, 0xb] {
        for u in 0..4 {
            for s in 0..2 {
                put(format!("umpire_words {l} {u} {s}"), format!("{:?}", g.umpire_words(l, u, s)));
            }
        }
    }
    for c in 0..13u32 {
        put(format!("court {c}"), format!("{:?}", (g.npc_roster(c), g.walkers(c), g.player_light(c))));
    }
    for c in 1..13u32 {
        put(format!("wind {c}"), format!("{:?}", (g.wind(c), g.gusts(c))));
        put(format!("weather_odds {c}"), format!("{:?}", g.weather_odds(c)));
    }
    put("looks".into(), format!("{:?}", (g.rain_clouds(), g.weather_looks(), g.surf_bearings(), g.cheer_sprites(), g.ball_routes(), g.ball_sounds())));
    put("voices".into(), format!("{:?}", (g.deciding_voices(), g.hud_colours())));
    for ty in 0..53u8 {
        put(format!("trigger {ty}"), format!("{:?}", (g.emitter(ty), g.trigger_model(ty), g.trigger(ty))));
    }
    for v in 0..2u8 {
        for u in 0..2u8 {
            put(format!("scoreboard {v} {u}"), format!("{:?}", g.scoreboard_timing(v, u)));
        }
    }
    t
}

/// Where the Japanese release's own data differs from the US one (M163a's journal lists each).
fn japan_differs(key: &str) -> bool {
    let n = |k: &str| key.strip_prefix(k).map(|r| r.trim().to_string());
    // its rain is likelier on these courts
    if let Some(c) = n("weather_odds") {
        return ["1", "2", "4", "5", "6", "10"].contains(&c.as_str());
    }
    // its umpire speaks only Japanese, whatever language is asked for
    if let Some(r) = n("umpire_words") {
        return !r.starts_with("11 ") && !r.starts_with("5 ");
    }
    // its characters' shot blend weights, all but character 12's (rows 3.. are characters 0..)
    if let Some(r) = n("shot weights") {
        return (3..17).contains(&r.parse::<usize>().unwrap()) && r != "15";
    }
    // no missed-serve table
    key == "serve_miss"
}

#[test]
fn japanese_tables_match_the_us_ones_but_for_its_tuning() {
    let Some(mut jp) = japan() else { return };
    let Ok(mut us) = fixture::iso() else { return };
    let (us_cnf, us_bin) = (us.read("SYSTEM.CNF").unwrap(), us.read("ZZBIN/GAME.BIN").unwrap());
    let (jp_cnf, jp_bin) = (jp.read("SYSTEM.CNF").unwrap(), jp.read("ZZBIN/GAME.BIN").unwrap());
    let (us_game, jp_game) = (Game::new(&us_cnf, &us_bin).unwrap(), Game::new(&jp_cnf, &jp_bin).unwrap());
    assert_eq!((us_game.release(), jp_game.release()), (Release::Us, Release::Japan));
    // both play, the Japanese one by its own rules (M163a1a)
    assert!(us.playable().is_ok());
    assert!(jp.playable().is_ok());
    let mut wrong = Vec::new();
    for ((k, a), (_, b)) in tables(&us_game).into_iter().zip(tables(&jp_game)) {
        if (a == b) == japan_differs(&k) {
            wrong.push(k);
        }
    }
    assert!(wrong.is_empty(), "unexpected (in)equality: {wrong:?}");
    assert_eq!(jp_game.serve_miss(), [[0; 3]; 3]);
    // the Japanese umpire's word timing is the US table's Japanese row
    assert_eq!(jp_game.umpire_words(0, 0, 0), us_game.umpire_words(0xb, 0, 0));

    // the boot program's sound and court tables
    let (us_elf, jp_elf) = (us.read(Release::Us.boot()).unwrap(), jp.read(Release::Japan.boot()).unwrap());
    assert_eq!(sound_tables(&us_elf).unwrap(), sound_tables(&jp_elf).unwrap());
    assert_eq!(stereo_tables(&us_elf).unwrap(), stereo_tables(&jp_elf).unwrap());
    assert_eq!(bank_volumes(&us_elf).unwrap(), bank_volumes(&jp_elf).unwrap());
    assert_eq!(court_plates(&us_elf).unwrap(), court_plates(&jp_elf).unwrap());

    // the character select's grades
    let (us_menu, jp_menu) = (us.read("ZZBIN/MENU.BIN").unwrap(), jp.read("ZZBIN/MENU.BIN").unwrap());
    assert_eq!(Menu::new(&us_cnf, &us_menu).unwrap().grades(), Menu::new(&jp_cnf, &jp_menu).unwrap().grades());
}

#[test]
fn japanese_files_answer_to_the_us_names() {
    let Some(mut jp) = japan() else { return };
    // the boot program, a language archive, motions and a voice set (both in the Japanese motions archive)
    let boot = jp.read(Release::Us.boot()).unwrap();
    assert_eq!(&boot[..4], b"\x7fELF");
    assert!(jp.read("AZUMA/INPANE/INPANE.XB0").is_ok());
    let motions = jp.read("PCANI/PC00ANI.XB").unwrap();
    assert_eq!(motions, jp.read("PCANI/PC00ANI0.XB").unwrap());
    let voices = jp.read("SND/VOICE/PC/PC03VCE5.XB0").unwrap();
    assert_eq!(voices, jp.read("PCANI/PC03ANI5.XB").unwrap());
    assert!(jp.read("NOT/THERE.XB0").is_err());
}

fn europe() -> Option<Iso> {
    if !fixture::path(EUROPE).exists() {
        // not the user's disc: optional even under HST_REQUIRE_DISC
        eprintln!("skipped: no {EUROPE}");
        return None;
    }
    Some(Iso::open(fixture::path(EUROPE)).unwrap())
}

/// The European disc's program data reads as the US disc's, every table but two camera script times: its shot records (151 per-frame amounts
/// stored ×1.2 for 50 Hz) come back as the US values to the bit, and so do the boot program's tables and the
/// select's grades. It plays (at 60 Hz on regenerated trajectory tables, offline).
#[test]
fn european_tables_read_as_the_us_ones() {
    let Some(mut eu) = europe() else { return };
    let Ok(mut us) = fixture::iso() else { return };
    let (us_cnf, us_bin) = (us.read("SYSTEM.CNF").unwrap(), us.read("ZZBIN/GAME.BIN").unwrap());
    let (eu_cnf, eu_bin) = (eu.read("SYSTEM.CNF").unwrap(), eu.read("ZZBIN/GAME.BIN").unwrap());
    let (us_game, eu_game) = (Game::new(&us_cnf, &us_bin).unwrap(), Game::new(&eu_cnf, &eu_bin).unwrap());
    assert_eq!(eu_game.release(), Release::Europe);
    assert!(eu_game.release().regenerated_shots() && !us_game.release().regenerated_shots());
    assert!(eu.playable().is_ok());
    let bits = |b: &[f32]| b.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
    assert_eq!(bits(&us_game.shot_params().base), bits(&eu_game.shot_params().base));
    // two camera script times were retuned for 50 Hz (a few hundredths of a second shorter): presentation only, kept
    let wrong: Vec<String> = tables(&us_game).into_iter().zip(tables(&eu_game)).filter(|((k, a), (_, b))| a != b && k != "camera").map(|((k, _), _)| k).collect();
    assert!(wrong.is_empty(), "differ: {wrong:?}");

    let (us_elf, eu_elf) = (us.read(Release::Us.boot()).unwrap(), eu.read(Release::Us.boot()).unwrap());
    assert_eq!(sound_tables(&us_elf).unwrap(), sound_tables(&eu_elf).unwrap());
    assert_eq!(stereo_tables(&us_elf).unwrap(), stereo_tables(&eu_elf).unwrap());
    assert_eq!(bank_volumes(&us_elf).unwrap(), bank_volumes(&eu_elf).unwrap());
    assert_eq!(court_plates(&us_elf).unwrap(), court_plates(&eu_elf).unwrap());

    let (us_menu, eu_menu) = (us.read("ZZBIN/MENU.BIN").unwrap(), eu.read("ZZBIN/MENU.BIN").unwrap());
    assert_eq!(Menu::new(&us_cnf, &us_menu).unwrap().grades(), Menu::new(&eu_cnf, &eu_menu).unwrap().grades());
}

/// Imports the Japanese disc into a pack and checks it's a known copy. ~170 MB hashed: slow in a debug build, so
/// ignored; run it after touching `write_pack`, `packed` or `KNOWN_PACKS`:
/// `tools/check.sh -p hst-data --test releases -- --ignored`.
#[test]
#[ignore]
fn the_japanese_disc_imports() {
    let Some(mut jp) = japan() else { return };
    let hash = jp.write_pack_to(&mut std::io::Cursor::new(Vec::new()), |_, _| {}).unwrap();
    assert_eq!(hst_data::iso::release_name(hash), Some("SCPS-15113 Japan 1.00"));
}


/// Archive `data`'s entries (lower-case names), decoded, sorted.
fn entries(data: &[u8]) -> Vec<(String, Vec<u8>)> {
    let arc = hst_data::xb::Archive::parse(data).unwrap();
    let mut v: Vec<_> = arc.entries.iter().map(|e| (e.name.to_ascii_lowercase(), arc.read(e).unwrap())).collect();
    v.sort();
    v
}

/// M174: the European disc with the US disc lent to it (`crossplay::lend`, a player with both) plays the US game: it
/// digests as the US disc; its program, shot tables (both copies), character data and courts read as the US ones (court
/// 3's plant layout and its two European-only sign models included); its menus, text, voices, mode pictures and court 5's
/// drawn models stay its own. It plays, is still the European disc, and reads as itself again once nothing is lent.
#[test]
fn the_european_disc_plays_the_us_game_with_the_us_disc_lent() {
    let Some(mut bare) = europe() else { return };
    let Ok(mut us) = fixture::iso() else { return };
    if std::env::var_os("HST_DISC_EU").is_some() {
        return eprintln!("skipped: HST_DISC_EU already lends the US disc");
    }
    // a link of its own, so the other tests read the European disc alone
    let link = std::env::temp_dir().join(format!("hst-lend-{}.iso", std::process::id()));
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(fixture::path(EUROPE).canonicalize().unwrap(), &link).unwrap();
    struct Gone(std::path::PathBuf);
    impl Drop for Gone {
        fn drop(&mut self) {
            hst_data::crossplay::unlend(&self.0);
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _gone = Gone(link.clone());
    let us_digest = hst_data::digest::of(&mut us).unwrap();
    assert_eq!(hst_data::crossplay::lend(&link, fixture::path(fixture::ISO)), Ok(us_digest));
    assert!(hst_data::crossplay::lent(&link));
    let mut eu = Iso::open(&link).unwrap();
    assert_eq!(hst_data::digest::of(&mut Iso::open_own(&link).unwrap()), Ok(us_digest));

    // the program: the US one (shot records as stored, the two camera script times, every table)
    let (us_cnf, us_bin) = (us.read("SYSTEM.CNF").unwrap(), us.read("ZZBIN/GAME.BIN").unwrap());
    let (cnf, bin) = (eu.read("SYSTEM.CNF").unwrap(), eu.read("ZZBIN/GAME.BIN").unwrap());
    assert!(cnf == us_cnf && bin == us_bin);
    let game = Game::new(&cnf, &bin).unwrap();
    assert_eq!(game.release(), Release::Us);
    assert_eq!(tables(&game), tables(&Game::new(&us_cnf, &us_bin).unwrap()));
    assert!(eu.playable().is_ok());
    assert!(eu.read("ZZBIN/MENU.BIN").unwrap() == us.read("ZZBIN/MENU.BIN").unwrap());
    // still the European disc: its own serial, boot program (whose tables are the US ones) and edition
    assert_eq!(eu.read_disc("SYSTEM.CNF").unwrap(), bare.read("SYSTEM.CNF").unwrap());
    assert!(eu.read(Release::Us.boot()).unwrap() == bare.read(Release::Us.boot()).unwrap());
    assert_eq!(hst_data::discs::Edition::of(&mut eu), Some(hst_data::discs::Edition::Europe));

    // shot tables, both copies, every character: the US ones
    for c in 0..14 {
        for ab in ["A", "B"] {
            let p = format!("TRAJ/TRAJ{c:02}{ab}.XB");
            assert!(entries(&eu.read(&p).unwrap()) == entries(&us.read(&p).unwrap()), "{p}");
        }
        let p = format!("SAITOU/TRAJ/TRAJPC{c:02}.XB");
        if us.files.contains_key(&p) {
            assert!(eu.read(&p).unwrap() == us.read(&p).unwrap(), "{p}");
            assert!(bare.read(&p).unwrap() != us.read(&p).unwrap(), "{p}: the European copy is timed for 50 Hz");
        }
    }
    // court 3: the US plant layout; the European-only signs it places are gone, its pictures stay the disc's own
    for p in ["COURT/03/HOL01.XB", "COURT/03/SSN1/HOL01.XB"] {
        let (got, want, own) = (entries(&eu.read(p).unwrap()), entries(&us.read(p).unwrap()), entries(&bare.read(p).unwrap()));
        let data = |v: &[(String, Vec<u8>)]| v.iter().filter(|(n, _)| !n.ends_with(".mti")).cloned().collect::<Vec<_>>();
        assert!(data(&got) == data(&want), "{p}");
        assert!(own.iter().any(|(n, _)| n.contains("banktitle")) && !got.iter().any(|(n, _)| n.contains("banktitle.mdl")), "{p}");
    }
    // presentation stays European: menus and their text, voices, the mode screens' pictures, court 5's drawn figures
    for p in ["MENU/MENU00B.XB0", "SND/VOICE/PC/PC03VCE0.XB0", "SAITOU/HSMODE/HSM.XB0", "COURTSET/05/HOLE01.XB"] {
        let own = bare.read(p).unwrap();
        assert!(eu.read(p).unwrap() == own, "{p}");
    }
    for p in ["SAITOU/HSMODE/HSM.XB0", "COURTSET/05/HOLE01.XB"] {
        assert!(us.read(p).unwrap() != bare.read(p).unwrap(), "{p}: the US disc's differs");
    }

    drop(_gone);
    assert!(!hst_data::crossplay::lent(&link));
}

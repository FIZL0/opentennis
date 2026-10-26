//! Every hit-effect model in `AZUMA/C_EFF/EFFCT.XB0` parses: models, materials, hierarchical `.ANI` byte-exact
//! to the end, `.MOR`/`.MTA`/`.UVA` key tracks. Skips without the ISO.

use hst_data::fixture;
use hst_data::{ani, mdl, mor, mtl, xb::Archive};

#[test]
fn every_effect_parses() {
    let Ok(mut iso) = fixture::iso() else { return };
    let data = iso.read("AZUMA/C_EFF/EFFCT.XB0").unwrap();
    let arc = Archive::parse(&data).unwrap();
    let get = |name: &str| arc.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with(&name.to_ascii_lowercase())).map(|e| arc.read(e).unwrap());
    let mut n = 0;
    for e in &arc.entries {
        let lower = e.name.to_ascii_lowercase();
        let d = arc.read(e).unwrap();
        let ok = |r: Result<(), hst_data::xb::Error>| r.unwrap_or_else(|err| panic!("{}: {err}", e.name));
        match lower.rsplit_once('.').map(|x| x.1) {
            Some("ani") => {
                let a = ani::parse(&d).unwrap_or_else(|err| panic!("{}: {err}", e.name));
                assert_eq!(a.ticks_per_frame, 80, "{}", e.name);
            }
            Some("mor") => ok(mor::parse(&d, 1).map(drop)),
            Some("mta") => ok(mor::parse(&d, 1).map(drop)),
            Some("uva") => ok(mor::parse(&d, 4).map(drop)),
            Some("mdl") => {
                ok(mdl::parse(&d).map(drop));
                let stem = e.name[..e.name.len() - 4].replace('\\', "/");
                let m = mtl::parse(&get(&format!("{stem}.MTL")).unwrap(), get(&format!("{stem}.MTI")).as_deref()).unwrap();
                assert!(m.materials.iter().all(|m| !m.name.is_empty()), "{}", e.name);
            }
            _ => continue,
        }
        n += 1;
    }
    eprintln!("{n} effect files");
    assert!(n > 90);
    let top = ani::parse(&get("yumoto/impact_top_a.ANI").unwrap()).unwrap();
    assert_eq!(top.tracks.iter().filter(|t| t.parent == Some(0)).count(), 8);
    let m = mtl::parse(&get("yumoto/impact_top_a.MTL").unwrap(), get("yumoto/impact_top_a.MTI").as_deref()).unwrap();
    assert!(m.materials.iter().any(|m| m.blend() == mtl::Blend::Add));
}

/// Court 10's wind (slot 5 blew from 315° at 2) and cloud count (9 in doubles as in RAM, 11 in singles).
#[test]
fn court10_wind_and_clouds() {
    let Ok(mut iso) = fixture::iso() else { return };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let game = hst_data::exe::Game::new(&cnf, &bin).unwrap();
    assert_eq!(game.wind(10), (vec![315.0, 0.0, 45.0], 2.0));
    // weathers (P17i): court 10 gusts 2% kind 2, cloudy 5% for 2..3 games, no rain; court 11 rains heavy; court 7 calm
    assert_eq!(game.gusts(10), (2, 2));
    assert_eq!(game.weather_odds(10), [5, 2, 3, 0, 1, 3, 0, 1, 0]);
    assert_eq!(game.weather_odds(11), [20, 2, 3, 30, 2, 4, 0, 0, 1]);
    // B480: the courts that rain, one spell a match lasting min..=max games (rain chance, min, max, heavy)
    let rain = |c| (game.weather_odds(c)[3], game.weather_odds(c)[4], game.weather_odds(c)[5], game.weather_odds(c)[8]);
    assert_eq!([1, 8, 9, 11].map(rain), [(20, 2, 3, 0), (40, 1, 2, 1), (15, 1, 2, 0), (30, 2, 4, 1)]);
    assert!((1..=11).filter(|c| ![1, 8, 9, 11].contains(c)).all(|c| game.weather_odds(c)[3] == 0));
    assert_eq!(game.wind(7), (vec![0.0], 0.0));
    assert_eq!(game.rain_clouds(), (200, [50.0, -2.0, -5.0]));
    assert_eq!(game.weather_looks()[3], [1.0, 0.7, 0.6, 40.0, 150.0, 0.65]);
    let mut find = |xb: &str, suffix: &str| {
        let data = iso.read(xb).unwrap();
        let arc = Archive::parse(&data).unwrap();
        arc.read(arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(suffix)).unwrap()).unwrap()
    };
    let (envir, hole) = (find("COURT/10/CMN.XB", "envir_c10.dat"), find("COURT/10/GRD01.XB", "envir_c10_h01.dat"));
    assert_eq!(hst_data::layout::cloud_count(&envir, &hole, 0), Some(9));
    assert_eq!(hst_data::layout::cloud_count(&envir, &hole, 1), Some(11));
}

/// The ceremony's camera clips parse to the end: track `Camera01`, 229/240/319 frames, the FOV in radians.
#[test]
fn ceremony_cameras() {
    let Ok(mut iso) = fixture::iso() else { return };
    let data = iso.read("AZUMA/C_EFF/EFFCT.XB0").unwrap();
    let arc = Archive::parse(&data).unwrap();
    for (name, frames) in [("win_99", 229), ("lost_99", 240), ("hand_99", 319)] {
        let e = arc.entries.iter().find(|e| e.name.replace('\\', "/").ends_with(&format!("prize/CAM/{name}.CAM"))).unwrap();
        let (cam, fov) = ani::parse_cam(&arc.read(e).unwrap()).unwrap();
        assert_eq!((cam.tracks[0].name.as_str(), cam.end_tick().max(fov.end_tick()) / cam.ticks_per_frame), ("Camera01", frames), "{name}");
        assert!(fov.tracks[0].position.iter().all(|k| (0.3..1.4).contains(&k.1[0])), "{name}");
    }
}

/// B680: the bounce debris table: 30 clods and 30 blades, their rows (clods smaller and not slowing next to the
/// dive's, blades smaller), the blades' sink; the dive debris' clod cells and least fall the smash burst shares.
#[test]
fn bounce_debris_rows() {
    let Ok(mut iso) = fixture::iso() else { return };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let game = hst_data::exe::Game::new(&cnf, &bin).unwrap();
    let t = game.bounce_debris();
    assert_eq!(t.counts, [30, 30]);
    let [c, b] = t.rows;
    assert_eq!((c.reach, c.spread, c.speed, c.power, c.decay, c.size, c.shrink, c.fall), (2.0, 3.5, [0.1, 0.5], 1.1, 1.0, [0.05, 0.1], 0.95, 0.8));
    assert_eq!((b.reach, b.spread, b.speed, b.power, b.decay, b.size, b.shrink, b.fall, b.push), (2.0, 3.5, [0.0, 0.0], 0.0, 1.0, [0.2, 0.3], 0.95, 0.0, 0.1));
    assert_eq!((b.offset, b.spin), ([[-0.45, 0.45], [-0.4, -0.05]], [[-15.0, 15.0], [-10.0, 10.0], [-10.0, 10.0]]));
    assert_eq!(t.sink, 0.0005);
    let d = game.foot().debris;
    assert_eq!((d.cells[0], d.lift), ([[0, 0], [32, 0], [0, 32], [32, 32]], -0.03));
}

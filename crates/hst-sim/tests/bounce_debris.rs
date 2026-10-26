//! The smash burst against a recorded bot doubles match on court 10 (`fixtures/b680/smash_c10.bin`): a smash lands
//! at its first bounce, and from the ball and its contact record the port throws its 30 clods with the same draws
//! of the sound manager's generator, leaving it in the game's state, and every clod's position, speed, size and cell
//! match the game's bit for bit over the burst's life.

use hst_data::{exe, fixture};
use hst_sim::effect::{BounceBits, Contact, DebrisCourt};
use hst_sim::weather::Mt;

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn u(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn v4(b: &[u8], o: usize) -> [f32; 4] {
    std::array::from_fn(|k| f(b, o + 4 * k))
}

// Per frame: vsync 4, ball 0x290, sound generator 0x9c8, first contact records 0xf0 (zeros while it has none), the
// smash clods 0x1120 (30 clods of 0x90 from +0x20: alive +0, position +0x40, speed +0x50, size +0x60, cell +0x64).
const BALL: usize = 4;
const MT: usize = BALL + 0x290;
const CONTACTS: usize = MT + 0x9c8;
const CLODS: usize = CONTACTS + 0xf0;
const SIZE: usize = CLODS + 0x1120;

#[test]
fn a_smash_landing_throws_the_games_clods() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(data), Ok(cnf), Ok(bin)) = (
        fixture::read(format!("{root}/fixtures/b680/smash_c10.bin")),
        fixture::read(format!("{root}/fixtures/iso/SYSTEM.CNF")),
        fixture::read(format!("{root}/fixtures/iso/ZZBIN/GAME.BIN")),
    ) else {
        return fixture::skip("b680/smash_c10.bin or disc absent, skipped");
    };
    let game = exe::Game::new(&cnf, &bin).unwrap();
    let (table, dive) = (game.bounce_debris(), game.foot().debris);
    let materials = hst_sim::court::materials(&game);
    let court = u(&data, 0) as usize;
    let fc = &game.foot().courts[court];
    let ground = DebrisCourt { grass: fc.grass, clay: fc.clay, court };
    let frames: Vec<&[u8]> = data[4..].as_chunks::<SIZE>().0.iter().map(|s| s.as_slice()).collect();
    let mt = |b: &[u8]| Mt::from_ram(&b[MT..]);
    let mut bits = BounceBits::default();
    let mut thrown = 0;
    for w in frames.windows(2) {
        let (prev, b) = (w[0], w[1]);
        let ball = &b[BALL..];
        let bounces = u(ball, 0x224) as i32;
        let c = &b[CONTACTS..];
        let first = (c.iter().any(|&x| x != 0)).then(|| Contact {
            point: v4(c, 0x10),
            vel: v4(c, 0x20),
            normal: v4(c, 0x30),
            court: materials[c[0x40] as usize].court,
        });
        let pos = v4(ball, 0xe0);
        let mut g = mt(prev);
        let mut draws = 0;
        // the recording is of a smash rally: its first bounce is the smash landing
        bits.tick(&table, &dive, ground, bounces, first.as_ref(), [pos[0], pos[1], pos[2]], false, true, &mut || {
            draws += 1;
            g.next()
        });
        if draws > 0 {
            thrown += 1;
            assert_eq!(draws, 120, "vsync {}", u(b, 0));
            let want = mt(b);
            assert_eq!((g.0.0, g.0.1), (want.0.0, want.0.1), "generator after the throw, vsync {}", u(b, 0));
        }
        let pool = &b[CLODS..];
        let v = u(b, 0);
        for (k, s) in bits.smash.iter().enumerate() {
            let e = &pool[0x20 + k * 0x90..];
            assert_eq!(s.alive, e[0] != 0, "clod {k} alive, vsync {v}");
            if thrown == 0 {
                continue;
            }
            let got = [s.m[3], s.vel].concat().into_iter().chain([s.size]).map(f32::to_bits).collect::<Vec<_>>();
            let want = [v4(e, 0x40), v4(e, 0x50)].concat().into_iter().chain([f(e, 0x60)]).map(f32::to_bits).collect::<Vec<_>>();
            assert_eq!(got, want, "clod {k} position, speed, size, vsync {v}");
            assert_eq!(s.cell, [u(e, 0x64) as i32, u(e, 0x68) as i32], "clod {k} cell, vsync {v}");
        }
    }
    assert_eq!(thrown, 1);
    assert!(!bits.smash_live, "the clods have all landed by the end of the recording");
}

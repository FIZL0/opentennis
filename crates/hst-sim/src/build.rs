//! Which build of the game's rules a match plays by: the US release's (the one every replay test proves), or the
//! Japanese release's (Minna no Tennis, SCPS-15113), an earlier build of the same game with a few rules missing or
//! different. Every difference is a switch here that is off for the US; the code that plays it asks.
//!
//! What the Japanese build does differently (in a match):
//! - the serve: every toss is graded on the rally timing table (its data files carry no serve timing counts of their
//!   own: `hst_data::columns`), a mistimed strong toss throws in no miss (no table, no random side), and no serve is
//!   ever a special serve;
//! - strikes: no counter (every shot launches from the hitter's own record), no "dull" hit flag (same draws);
//! - the aim: a plain smash off its timing narrows its widest angle by depth over 6.4..11.885 m by up to 0.3 (the US
//!   3..11.885 m by up to 0.4), and no shot gets the US's 90° widest angle (ground slices and lobs, rally lobs);
//! - the CPUs: no extra reaction (and no draw for it) for a lob or a special serve; the smash search takes a stand spot
//!   behind the baseline too;
//! - presentation: the red landing marker shows on the serve too, and km/h is ×60 ×60 ×60 ÷ 1000.
//!
//! The European release (Everybody's Tennis, SCES-54535) has the US rules retimed for 50 Hz television: no rule
//! changes, only numbers tied to time, each typed again by hand (so its own bit patterns, never `us × 1.2`): frame
//! counts about 5/6, per-frame amounts about 6/5, per-frame² amounts about 36/25, km/h ×50. The motions keep the US
//! files but play 96 ticks a frame instead of 80, except the swings and serve swings ([`Build::motion_rate`]), and a
//! clip length that isn't whole is nudged up a hair ([`Build::clip_length`]). Some counts were left alone (the
//! post-point wait table, the ball's 0.003 per-frame term): those run 20 % slower in real time, as they do there.
#![deny(clippy::float_arithmetic)]

use crate::ps2::{F, fs};

/// The game build a match plays by (`Us` unless the disc is another release).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Build {
    #[default]
    Us,
    /// The Japanese release (Minna no Tennis, SCPS-15113).
    Japan,
    /// The European release at 50 Hz (Everybody's Tennis, SCES-54535): the US rules, retimed.
    Europe,
}

const TOSS_50: u32 = 0x3b67_3604;

/// The motion files' own ticks per frame (every stock file).
pub const MOTION_TICKS: i32 = 80;

impl Build {
    /// The US rules (the European build has them too; only its timing differs).
    fn us(self) -> bool {
        self != Build::Japan
    }

    /// The European program's 50 Hz timing.
    pub fn fifty(self) -> bool {
        self == Build::Europe
    }

    /// Ticks (frames) a second.
    pub fn hz(self) -> u32 {
        if self.fifty() { 50 } else { 60 }
    }

    /// The ball's gravity per frame² (9.8 / 3600; 9.8 / 2500 at 50 Hz).
    pub fn gravity(self) -> f32 {
        if self.fifty() { f32::from_bits(0x3b80_7358) } else { f32::from_bits(0x3b32_6750) }
    }

    /// The serve toss's gravity per frame² (the ball's 0.9 of [`Build::gravity`]).
    pub fn toss_gravity(self) -> f32 {
        if self.fifty() { f32::from_bits(TOSS_50) } else { f32::from_bits(0x3b20_902e) }
    }

    /// A player's base run speed per frame.
    pub fn run(self) -> f32 {
        if self.fifty() { f32::from_bits(0x3db0_3740) } else { f32::from_bits(0x3d92_d8b5) }
    }

    /// The squared ball speed below which a ground stroke's follow-through turns soft.
    pub fn soft_speed2(self) -> f32 {
        if self.fifty() { f32::from_bits(0x3e9e_0653) } else { f32::from_bits(0x3e5b_7a8e) }
    }

    /// The server's walk along the baseline per frame of stick (and the CPU server's stop tolerance).
    pub fn serve_walk(self) -> f32 {
        if self.fifty() { f32::from_bits(0x3d23_d69f) } else { f32::from_bits(0x3d08_882f) }
    }

    /// The toss motion's frame where the ball leaves the hand (the toss motion is retimed at 50 Hz).
    pub fn toss_release(self) -> f32 {
        if self.fifty() { 37.5 } else { 45.0 }
    }

    /// Steps after its launch at which a live ball still in the rally phase comes to rest.
    pub fn rest_timeout(self) -> i32 {
        if self.fifty() { 1500 } else { 1800 }
    }

    /// A CPU server's random wait from a draw of the AI generator: 60..119 frames (50..99 at 50 Hz).
    pub fn serve_wait(self, draw: u32) -> i32 {
        let n = if self.fifty() { 50 } else { 60 };
        (draw >> 16 & 0x7fff) as i32 % n + n
    }

    /// `frames` counted at 60 Hz as the European program retimed it: the AI waits 30 → 25 and 60 → 50, its
    /// "more than 29" → 24, the point gates 900 → 750 and 1800 → 1500. Only counts the European program changed
    /// by exactly 5/6 come through here; at 60 Hz every count is itself.
    pub fn frames(self, frames: i32) -> i32 {
        if !self.fifty() {
            return frames;
        }
        match frames {
            29 => 24,
            n => {
                debug_assert!(n % 6 == 0, "{n} frames: not a whole 5/6 retime");
                n / 6 * 5
            }
        }
    }

    /// The ticks per frame a motion file's `ticks` (80 in every stock file) plays at, for a player's motion
    /// `index` (`None` for a motion that isn't one of a player's 0..47 or its dummies). At 50 Hz every motion plays
    /// 60/50 as many ticks a frame (80 → 96, integer arithmetic), so it keeps its real-time speed, except a
    /// player's swings (16..=31) and serve swings (37, 38), which keep their frame counts (and contact frames), and
    /// the dummies after 47 but the ball one (index 50).
    pub fn motion_rate(self, ticks: i32, index: Option<usize>) -> i32 {
        let rescaled = match index {
            _ if !self.fifty() => false,
            Some(16..=31 | 37 | 38 | 48 | 49 | 51) => false,
            _ => true,
        };
        if rescaled { ticks * 60 / 50 } else { ticks }
    }

    /// A clip's length in frames from its last key's tick and its ticks per frame: at 50 Hz a length that isn't a
    /// whole number is nudged up ×1.000001 (so a test that truncates lands on the next frame).
    pub fn clip_length(self, last_tick: f32, ticks: f32) -> f32 {
        let len = F(last_tick) / ticks;
        if self.fifty() && len.0 as i32 as f32 != len.0 { (len * f32::from_bits(0x3f80_0008)).0 } else { len.0 }
    }

    /// Metres per frame to km/h the US way (×60 × 3600 ÷ 1000), with 50 for 60 at 50 Hz.
    fn per_second(self) -> f32 {
        if self.fifty() { 50.0 } else { 60.0 }
    }

    /// A mistimed strong toss throws in a miss off the aim (the missed-serve table, or a random side).
    pub fn strong_toss_miss(self) -> bool {
        self.us()
    }

    /// A clean strong-toss serve by the right characters can be a special serve.
    pub fn special_serve(self) -> bool {
        self.us()
    }

    /// A clean flat/topspin answer to a stronger hitter's flat/topspin shot is a counter.
    pub fn counter(self) -> bool {
        self.us()
    }

    /// A shot whose random power multiplier was chosen is flagged dull (its sound only).
    pub fn dull_flag(self) -> bool {
        self.us()
    }

    /// Ground slices and lobs and rally lobs aim with a 90° widest angle off straight.
    pub fn wide_lobs(self) -> bool {
        self.us()
    }

    /// A plain smash off its timing narrows its widest angle by its depth: (nearest depth, depth span, most narrowing).
    pub fn smash_narrowing(self) -> (f32, f32, f32) {
        match self {
            Build::Us | Build::Europe => (3.0, 8.885, 0.4),
            Build::Japan => (6.4, 5.485, 0.3),
        }
    }

    /// A CPU adds (and draws) the extra reaction to a lob or a special serve.
    pub fn lob_reaction(self) -> bool {
        self.us()
    }

    /// The CPU's smash search skips a stand spot at or beyond the baseline.
    pub fn smash_baseline(self) -> bool {
        self.us()
    }

    /// The red landing marker shows on the serve too (with more than one player; it does with one in both).
    pub fn marker_on_serve(self) -> bool {
        !self.us()
    }

    /// The ball's speed in km/h as the strike pop-up, the CONK sound, the smash bounce and the statistics read it
    /// (`crate::sound::kmh` in the US build).
    pub fn kmh(self, vel: [f32; 3]) -> f32 {
        match self {
            Build::Us => crate::sound::kmh(vel),
            Build::Europe => {
                let v = fs(vel);
                ((v[2] * v[2]).madd(v[0], v[0]).madd(v[1], v[1]).sqrt() * self.per_second() * 3600.0 / 1000.0).0
            }
            Build::Japan => {
                let v = fs(vel);
                japan_kmh((v[2] * v[2]).madd(v[0], v[0]).madd(v[1], v[1]).sqrt())
            }
        }
    }

    /// The ball's speed in km/h from its last move, as the background figures read it (`crate::npc::ball_kmh` in the
    /// US build).
    pub fn ball_kmh(self, [x, y, z]: [f32; 3]) -> f32 {
        match self {
            Build::Us => crate::npc::ball_kmh([x, y, z]),
            Build::Europe => (F(3600.0) * (F(self.per_second()) * (F(y) * y).madd(x, x).madd(z, z).sqrt()) / 1000.0).0,
            Build::Japan => japan_kmh((F(y) * y).madd(x, x).madd(z, z).sqrt()),
        }
    }
}

/// Metres per frame to km/h the Japanese way: three chopped multiplies by 60, then ÷ 1000.
fn japan_kmh(per_frame: F) -> f32 {
    (per_frame * 60.0 * 60.0 * 60.0 / 1000.0).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn us_kmh_is_the_shared_one() {
        let v = [0.37663022, 0.14124857, -0.57089436];
        assert_eq!(Build::Us.kmh(v).to_bits(), crate::sound::kmh(v).to_bits());
        assert_eq!(Build::Us.ball_kmh(v).to_bits(), crate::npc::ball_kmh(v).to_bits());
    }

    #[test]
    fn japan_kmh_multiplies_by_sixty_three_times() {
        // 1 m/frame is 216 km/h either way; the steps differ in their rounding only
        assert_eq!(Build::Japan.kmh([0.0, 0.0, 1.0]), 216.0);
        let v = [0.37663022, 0.14124857, -0.57089436];
        let len = (F(v[2]) * v[2]).madd(v[0], v[0]).madd(v[1], v[1]).sqrt();
        let want = ((((len * 60.0) * 60.0) * 60.0) / 1000.0).0;
        assert_eq!(Build::Japan.kmh(v).to_bits(), want.to_bits());
        assert!((Build::Japan.kmh(v) - Build::Us.kmh(v)).abs() < 0.001);
    }
}

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
#![deny(clippy::float_arithmetic)]

use crate::ps2::{F, fs};

/// The game build a match plays by (`Us` unless the disc is another release).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Build {
    #[default]
    Us,
    /// The Japanese release (Minna no Tennis, SCPS-15113).
    Japan,
}

impl Build {
    fn us(self) -> bool {
        self == Build::Us
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
            Build::Us => (3.0, 8.885, 0.4),
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

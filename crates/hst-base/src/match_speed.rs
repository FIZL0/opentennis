//! M176: Settings → Gameplay → Match Speed: how many ticks a second a match runs. 60 (the US and Japanese game, the
//! default) or 50 (the European original, Everybody's Tennis, which was retimed for 50 Hz television).
//!
//! The match's rate is one number, [`current`], fixed when a match starts (`--match-hz`, which the menus and an
//! online lobby pass) and read by everything that counts match time in real seconds: the fixed clock (`Time<Fixed>`,
//! also held and given back by the court intro and the pause screen), the rollback session's clock and input delay,
//! the match statistics' match time, and the drawing, which blends the last two ticks by `Time<Fixed>`'s overstep at
//! any rate. The menus stay at 60.
//!
//! 50 Hz is offered only where the European disc's data is here (the main disc, or a disc beside it: `releases`) and
//! the game's own 50 Hz timing is in the sim ([`FIFTY_HZ_SIM`]); until then the row has only 60 Hz and is hidden.
//! Online the lobby plays its host's speed (`net::Room::hz`); a guest that can't play it is turned away.

use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

/// The rates, in the row's order: 60 Hz (the default), 50 Hz.
pub const SPEEDS: [u32; 2] = [60, 50];

/// Whether the sim plays the European program's 50 Hz timing (its retimed counts, per-tick amounts and motion
/// rescale: M169a1), as the recorded European bot match shows (M183a1). Off, a match only runs at 60.
pub const FIFTY_HZ_SIM: bool = true;

/// The saved choice (60 or 50).
static CHOSEN: AtomicU32 = AtomicU32::new(60);
/// The running (or next) match's rate, set from `--match-hz` each time a match's flags are read.
static MATCH: AtomicU32 = AtomicU32::new(60);

/// `hz` as one of [`SPEEDS`], 60 for anything else.
pub fn valid(hz: u32) -> u32 {
    if SPEEDS.contains(&hz) { hz } else { 60 }
}

/// Whether the European disc's data is here: the main disc or one beside it.
fn european_here() -> bool {
    crate::releases::has(hst_data::discs::Edition::Europe) || crate::rules_choice::own() == 3
}

/// Whether a match can run at `hz` here (60 always; 50 with the European data and the sim's 50 Hz timing).
pub fn playable(hz: u32) -> bool {
    hz == 60 || hz == 50 && FIFTY_HZ_SIM && european_here()
}

/// The rates a match can run at here, in [`SPEEDS`] order.
pub fn available() -> Vec<u32> {
    SPEEDS.into_iter().filter(|&hz| playable(hz)).collect()
}

/// The rate an offline match (or a lobby this app hosts) runs at: the choice where it is playable, else 60.
pub fn chosen() -> u32 {
    let c = CHOSEN.load(Relaxed);
    if playable(c) { c } else { 60 }
}

/// The setting as saved; kept when it isn't playable here, so it comes back with the European disc.
pub fn setting() -> u32 {
    CHOSEN.load(Relaxed)
}

/// Settings → Match Speed picks `hz`.
pub fn choose(hz: u32) {
    CHOSEN.store(valid(hz), Relaxed);
}

/// The rate of the match being started or played: what its `--match-hz` said (60 without one).
pub fn current() -> u32 {
    MATCH.load(Relaxed)
}

/// A match's flags were read: it runs at `hz` (`--match-hz`; 60 without one).
pub fn set_current(hz: u32) {
    MATCH.store(valid(hz), Relaxed);
    hst_data::iso::set_own_rate(sim_hz() == 50);
}

/// M183: the rate the sim's timing plays a test at before [`FIFTY_HZ_SIM`] is on (0: none, [`sim_hz`]).
static SIM_TEST: AtomicU32 = AtomicU32::new(0);

/// The tick rate whose timing the sim plays the match by (`hst_sim::build::Build::Europe` at 50 on the European
/// disc): the match's own rate once [`FIFTY_HZ_SIM`] is on, 60 until then (but under [`test_sim_hz`]). At 50 the
/// disc's data is read at its own rate too (`hst_data::iso::set_own_rate`: the European disc's raw tables, no lending).
pub fn sim_hz() -> u32 {
    match SIM_TEST.load(Relaxed) {
        0 if FIFTY_HZ_SIM => current(),
        0 => 60,
        hz => hz,
    }
}

/// Tests: the next matches play the sim's timing at `hz` (and run their clock at it), `FIFTY_HZ_SIM` or not; 0 ends it.
pub fn test_sim_hz(hz: u32) {
    SIM_TEST.store(if hz == 0 { 0 } else { valid(hz) }, Relaxed);
    set_current(if hz == 0 { 60 } else { hz });
}

/// The flags that start a match at `hz` (none at the default 60).
pub fn flags(hz: u32) -> Vec<String> {
    if valid(hz) == 60 { Vec::new() } else { vec!["--match-hz".into(), valid(hz).to_string()] }
}

/// A rate in the player's words. 60 Hz on the European disc alone is the US game on re-solved shot tables (M170):
/// close to the original, not exact, so the row says so.
pub fn name(hz: u32) -> &'static str {
    match valid(hz) {
        50 => "50 Hz (European original)",
        _ if crate::rules_choice::own() == 3 && !crate::rules_choice::laid() => "60 Hz (close, not the original)",
        _ => "60 Hz",
    }
}

/// The settings.txt line (`match_speed = 50`).
pub fn setting_line(hz: u32) -> String {
    format!("match_speed = {}\n", valid(hz))
}

/// The rate a settings.txt names (its last `match_speed` line), else 60.
pub fn parse_setting(text: &str) -> u32 {
    text.lines()
        .rev()
        .filter_map(|l| l.split('#').next()?.split_once('='))
        .find(|(k, _)| k.trim() == "match_speed")
        .map_or(60, |(_, v)| valid(v.trim().trim_end_matches("hz").trim_end_matches("Hz").trim().parse().unwrap_or(60)))
}

/// Why this app can't join a lobby that runs at `hz`, `None` when it can.
pub fn refused(hz: u32) -> Option<String> {
    let hz = if hz == 0 { 60 } else { hz };
    (!playable(hz)).then(|| {
        if !SPEEDS.contains(&hz) {
            format!("The host's match runs at {hz} Hz, which this version doesn't play.")
        } else if !FIFTY_HZ_SIM {
            "The host plays at 50 Hz (European original), which this version doesn't play yet.".into()
        } else {
            "The host plays at 50 Hz (European original): it needs the European disc (Everybody's Tennis) here.".into()
        }
    })
}

/// The match's rate as a resource (`play::plugin` inserts it with `Time<Fixed>`): ticks a second, and a tick's length.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchHz(pub u32);

impl Default for MatchHz {
    fn default() -> Self {
        MatchHz(60)
    }
}

impl MatchHz {
    /// The running match's ([`current`]).
    pub fn now() -> Self {
        MatchHz(current())
    }

    pub fn hz(self) -> f64 {
        self.0 as f64
    }

    /// One tick, in seconds.
    pub fn tick(self) -> f32 {
        1.0 / self.0 as f32
    }

    /// The fixed clock for it.
    pub fn fixed(self) -> bevy::time::Time<bevy::time::Fixed> {
        bevy::time::Time::<bevy::time::Fixed>::from_hz(self.hz())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting_words() {
        assert_eq!(parse_setting(""), 60);
        assert_eq!(parse_setting("match_speed = 50\n"), 50);
        assert_eq!(parse_setting("match_speed = 50 Hz\n"), 50);
        assert_eq!(parse_setting("match_speed = 50\nmatch_speed = 60\n"), 60);
        assert_eq!(parse_setting("match_speed = 30\n"), 60);
        for hz in SPEEDS {
            assert_eq!(parse_setting(&setting_line(hz)), hz);
        }
        assert_eq!(flags(60), Vec::<String>::new());
        assert_eq!(flags(50), ["--match-hz", "50"]);
        assert_eq!(flags(42), Vec::<String>::new());
        assert_eq!(name(50), "50 Hz (European original)");
    }

    /// 60 Hz is always playable; 50 only with the sim's 50 Hz timing (and the European data), so until then a saved
    /// 50 plays at 60 and a 50 Hz lobby turns this app away; the 60 Hz clock is the one the match always had.
    #[test]
    fn fifty_waits_for_the_sim() {
        assert!(playable(60) && available()[0] == 60);
        assert_eq!(refused(60), None);
        assert_eq!(refused(0), None);
        assert!(refused(30).is_some());
        if !FIFTY_HZ_SIM {
            assert_eq!(available(), [60]);
            choose(50);
            assert_eq!((setting(), chosen()), (50, 60));
            choose(60);
            assert!(refused(50).is_some());
        }
        assert_eq!(MatchHz(60).fixed().timestep(), bevy::time::Time::<bevy::time::Fixed>::from_hz(60.0).timestep());
        assert_eq!(MatchHz(50).fixed().timestep(), std::time::Duration::from_millis(20));
    }
}

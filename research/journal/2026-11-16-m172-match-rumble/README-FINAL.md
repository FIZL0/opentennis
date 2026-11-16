# M172 — match rumble, ours

The original never rumbles in a match (M138); M139's version (82ea0062) was reverted. This one is narrower, by the
user's three events, and off by default: Settings → Gameplay → Match Rumble (`match_rumble` in settings.txt).

## How it should work (decisions)
- **Mistimed / framed hit → big rumble** (both motors, heavy 1.0 / light 0.6, 0.3 s). Mistimed = the early or late
  balloon (grade off/slow); framed = the death lob (also drops its balloon, so the framed flag is read too).
- **Sweet-spot hit → short crisp tick** (light motor only 0.75, 0.06 s). Exactly the sweet balloon.
- **Clean hit off the sweet spot (note balloon), dives: nothing.** The contrast is the point; a buzz on every hit
  (M139) blurred it.
- **Hard serve hits the ground → thump**: the serve's first bounce, when its speed (the km/h the readout shows) is
  ≥ 125 — a well-struck power serve. A bot match's strong tosses run 108..136 km/h, weak ones 59..118, so placed,
  mistimed and second serves stay quiet. **Whose pad**: the receiver's at full (0.8 heavy, 0.12 s: it lands at
  them, the feel of facing a big serve) and the server's softer (0.35, 0.08 s: their own ace landing). In single
  player that means you feel both your own and the CPU's big serves.
- **Body hit → thump** (0.7 / 0.4, 0.2 s): kept from M139, it is a physical event.
- **Point / game / match won: no.** Not ball contact; it fired on every point in M139 and was noise.
- **Phones: pads only.** A phone with a Bluetooth pad rumbles that pad. The on-screen touch pad keeps only its own
  press buzz (Gameplay → Vibration); match haptics through it would muddle that and the request path is different.
- Strengths/lengths by feel (task says so).

## Code
- `crates/hst-play/src/play/rumble.rs`: `Watch::due` reads `Game` in FixedUpdate between `age_balloons` and the
  pop-ups' `surprise::tick` (that one takes the framed flag and can drop the balloon; made `pub(super)` for the
  ordering). Read-only: no sim state, RNG or timing touched. A balloon is the hit's own only at age ≤ 1.
  Each event keyed by (match clock frame, what, who): a rollback that replays ticks doesn't rumble twice.
  Sent only when `rumble::ON`, not in an instant replay or a played-back match, for players whose seat is this
  machine's (`Pads::slot_of`, online `Netplay::lock.mine`) on a real pad (not `emote::is_touch`). Watched even
  while off, so switching on mid-match fires nothing stale.
- Menus: `Settings::match_rumble` (a switch, default off), `Opt::MatchRumble` first on the Gameplay page,
  `rumble::ON` stored on load, toggle and ○-revert.
- M167's Gameplay tab isn't on main yet: its code changes are brought in unchanged (first commit on the branch) so
  this row has its page; identical hunks merge cleanly once task/M167 lands.

## Tests
`tools/check.sh -p hst-play --lib rumble`: the five rumbles stay in range and feel apart; a 150 s bot match:
every early/late balloon rumbles big for its hitter, every sweet one crisp, every clean one nothing; every hard
serve's landing thumps receiver + server exactly once, soft serves nothing; the same state twice and a rollback
replaying a thump rumble nothing (25 mistimed, 14 sweet, 29 quiet hits). `-p hst main_menu`: the settings pages.
Screenshot of the page: `context/m172/gameplay.png`.

## Not verified / not 1:1
- Not 1:1 by design: the original has no match rumble (M138); everything here is ours, strengths by feel.
- Not felt on a real controller: unattended run with no pad to hold. The request path is the
  `GamepadRumbleRequest::Add` the Assign screen's press buzz already uses.
- The seat → pad routing (local humans, online `mine`, touch excluded) has no automated test: a bot match has no
  humans. It reuses `slot_of`/`PadSlots` as M139 and `read_input` do.
- The 125 km/h threshold is from one bot pairing (chars 0 v 1); mod characters with power tables may serve harder
  more often — fine, they're hard serves.
- The rollback key uses the match clock, which caps at 99:59:59; past that, repeats could be swallowed.

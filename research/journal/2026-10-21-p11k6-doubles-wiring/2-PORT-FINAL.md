# P11k6: doubles AI glue: port phase

Port session (clean room, decompile locked), from `SPEC.md` and `context/fixtures/ai_glue_s05.bin`.

## What was done

- `hst_sim::rally::Rally::heard_hit_doubles`: the hit message's own work. Rallying past the return (the shot count
  the message sees > 1, not swinging) it goes back to its wait substate (Return roll, put-none record, net repick).
  Then its own team's hit re-picks the formation spot: cue Me with the partner's position when it hit itself, else
  cue Other with where the partner hit from. It uses the ball's depth then and "our hit", and a move rolls a new
  Return. Rallying, it clears held. `Rally::heard_strike`: the strike and new-path messages, at the message
  (receive → its wait, no draw; rally → set_sub(WAIT)). `repick` now goes through `repick_as`, which takes the depth
  and our-hit as arguments.
- App (`play/doubles_ai.rs`):
  - `frame` (the Body/World build, lifted out of `step` unchanged).
  - `at_message`: runs a routine at a message with the body as it is now. The app's net pick goes in and comes back,
    and the shot records come back.
  - Doubles `heard_hit` now does the re-entry and the repick at the message. Singles keeps its early-roll path.
  - `struck`: strike and new path at the message, from `heard` and from the figures' strike in `npcs.rs`.
  - `step`: doubles no longer re-enters on a new shot or a strike, and goes over to the rally in the same frame when
    the receive routine says so.
  - `play.rs` `ai_update`: the shots ≥ 2 / no swing rule now applies to singles only.
- `crates/hst-sim/tests/ai_rally.rs` `glue_matches_the_game` replays every call in the recording: 5135 updates and
  messages (4 kinds, 40+ hits), with the serve routine's frames skipped. Each call is bit-exact against the exit
  record: the AI's 0x280 bytes (except the shot memory), the stick, the button, the shot records and the draw count.
  The test found and pins down:
  - the guess's "from" is the whole position quad;
  - the rally's entry writes its flags whole;
  - the serve routine owns one countdown and one substate byte.

## Not verified / not 1:1

- The recorded points have no strike (0x14) or new-path (0x16) message (CPU-only doubles). `heard_strike` and its
  wiring follow the spec only. A recording with a let, a net cord or a ball striking a figure would cover them.
- The acceptance test drives the sim's glue (`Rally` + `Mind` + the timing draws, as the app strings them together),
  not `doubles_ai::step` on a `Game`: the app's `Game` can't be set from a record. The app wiring is checked by the
  hst-play doubles tests (they play) only, not against the recording.
- The shot memory (opponents', own team's, serve history) is the app's own (`ai_seen`/`ai_serves`), so the test
  takes it from the exit record instead of comparing bytes.
- The reaction message's winner comes from the score words in the record.
- One update (v8455, player 2) reads the NET/BASE stand scratch, which the update record doesn't capture: tag 5 takes
  the scratch below the update's own frame. The game's value there is what this AI's stand left the frame before, and
  the test uses that value for this one call. Carrying it on every call breaks a dozen calls whose slot was
  overwritten in between. The app carries `Rally::stash` (its last stand target) as before, so in the app this
  uninitialised read can still differ from the game's.
- The serve state's hand-over draws are unchanged: the app draws the net repick before the Return roll, the game
  draws them the other way round. The replay tests already cover this hand-over, and the spec leaves it as is.
- The strike from a figure is now handled after the figures' loop of that tick, not in the middle of it. Nothing
  else in that loop draws from the AI's generator.

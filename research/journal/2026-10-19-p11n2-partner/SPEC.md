# P11n2 spec: doubles partner logic (who of a team plays the ball)

This is the clean-room spec. It describes the original's behaviour in words. The port session works from this and the
recordings.

## Summary

The task line says the app picks the teammate that is nearer by x. That stand-in is no longer what decides.
- In a two- or four-player match, a computer player's receive and rally frames go through the ported doubles routines
  (`doubles_ai::step`, P11k3). `step` returns "handled" on every receive or rally frame outside a change of ends.
- On every other frame, `bot` either returns before the stand-in block (new point, serve) or has no interception plan
  (`intercept` runs only in the rally phase). So the `mine` filter, its "yours" call-out and the stand-in's
  sweet-frame press can't be reached in a real match.

The original's partner logic was mostly ported in P11k2 (`Rally::give_way`, follow, middle pick, voice), P11k7 (the
claim, the human shot record) and B479 (no partner gate on the press). The fixtures check it bit for bit.

Three inputs to that logic are still missing in the app. Without them the hand-over differs from the original:
1. **The partner's movement state** gates the give-way voice. The app passes "standing" every frame.
2. **The player's own miss mark** tells the partner "I swung and missed, the ball is yours". The app never sets it.
3. **The claim's exemption from keeping apart:** while a player strokes, it is kept 1 m from its partner only when it
   doesn't hold the claim. The app keeps every stroking player apart.

The port phase wires these three in and deletes the unreachable stand-in. The rest of this spec describes the whole
partner behaviour, so that the port can be checked against it and nothing already ported gets changed by mistake.

## Terms

- **Team:** players 0 and 2 are one team, 1 and 3 the other. A player's partner is the other player of its team.
- **Shot record:** one per player, shared by everyone. It holds:
  - the frame (tick) on which this player would make contact with the coming ball;
  - the kind of contact;
  - where the player stood when it posted the record;
  - the spot it would play from (its stand spot).

  Special frame values: "cleared" (−1), "none / no play" (−2) and "missed" (−3). A record is *valid* when its frame
  is ≥ 0.
- **Run of a record:** the horizontal (x, z) distance from where the player stood when it posted the record to its
  stand spot. It is the distance the player would have to run to play the ball.
- **Claim:** one value for the whole match, kept with the ball's predicted path. It names the player whose swing last
  found its contact, together with the tick on which it did.
- **Movement state:** a per-player small number:
  - 0 standing;
  - 1 running;
  - 2 in a stroke (a press still searching, a locked swing through its follow-through, a whiff or miss, a dive);
  - 3 standing after being hit by the ball.

  The app already derives it for footsteps (`foot_fx::state`, P17o).

## Who plays the ball

### The serve
- Only the receiver plays the serve, through its receive routine.
- The other three are in the rally routine. Its wait step does nothing until the shot count reaches 2, so the server's
  partner and the receiver's partner don't go for the serve.

### The rally: two players decide on their own, through the records

Every frame, each computer teammate runs its own rally routine (net style or baseline style) on the same predicted
path.

**The wait step**
- It does nothing:
  - while the line-call hold is on;
  - while the reaction delay after the opponents' hit counts down;
  - on the point-over phase's first shot;
  - while the shot count is under 2.
- After its own team's hit, it walks back to its formation spot. Every 30 frames it re-picks that spot, cued by who
  hit.
- Following the partner:
  - It starts to follow when the partner's record is valid and either:
    - this player isn't "fresh" (it already searched this ball); or
    - it plays in front (the net player) and the ball is on its half (z has the same sign).

    Starting to follow re-picks its formation spot from the partner's stand spot (cue "other").
  - While it follows, it re-picks from the partner's stand spot on every frame the partner's record is valid.
  - The follow is checked for its end when any of these holds:
    - the partner's record is no longer valid;
    - the partner's contact frame has passed;
    - the contact is less than 8 frames away, the ball is on the partner's half, and the ball is farther from the
      net than the partner;
    - the ball is on the partner's half and more than 2 m farther from the net than the partner.
  - At that check, the follow ends and the player becomes "fresh" again (it searches the ball itself) when it isn't
    on a middle pick and one of these holds:
    - the ball is on the other half from it;
    - the ball is no farther from the net than it is;
    - the ball is within 3 m of it.

    Otherwise it holds: it keeps its depth and stands at x = 2.7425 on the side away from the ball (−2.7425 when
    the ball's x ≥ 0, else +2.7425).
- When not following, it searches the path. A find posts its record (frame, kind, where it stands, stand spot) and
  goes to the run step on the same frame.
- With no find, the no-find tail runs:
  1. If the path's entry after its last one-bounce entry lies on the other half and the partner's record is valid,
     it follows the partner and may speak.
  2. Else, with two computer players, if the partner is nearer to that point (and still nearer to the no-bounce
     version), it takes the middle pick and follows.

     "Nearer" compares squared distances. The partner must be strictly nearer, so on a tie this player stays.
  3. Else it may dive, or it chases toward the after-bounce, fallback or last entry.

**The run step: give way or keep it**

Each frame, the player compares its own record with its partner's.

- Two computer teammates. It gives way when the partner's record is valid and:
  - the partner's contact is earlier and the partner's run is shorter: give way;
  - else, its own contact is earlier and its own run is shorter: keep it;
  - else, either run is under 2.5 m: the shorter run plays (give way when the partner's run is strictly shorter);
  - else the earlier contact plays (give way when the partner's frame is strictly earlier).

  Full ties keep the ball, on both sides.
- Beside a human. The partner's record comes from the human's record keeper (ported in P11k7). With a valid human
  record:
  - when its own run is under 2.5 m and it is more than 1 m nearer the net than the human (|own z| + 1 < |human
    z|): keep it;
  - else, when the human's run is under 2.0 m: give way;
  - else, when its own run is under 2.5 m: keep it;
  - else, when the two contact frames are fewer than 30 frames apart (a wrapping absolute difference): give way if
    the human's run + 1 m is shorter than its own;
  - else the earlier contact plays (give way when the human's frame is strictly earlier).

  After a "keep it" beside a human, when the human's record is valid or "none", it **defers**: it won't give way
  again until it next enters the wait step.
- Giving way does four things:
  1. re-picks its spot from the partner's stand spot;
  2. starts following;
  3. may speak (see *Voice*);
  4. goes back to the wait step, keeping its fresh, follow and middle state.
- Keeping the ball continues the run.
  - On frame 1, the ready check runs.
  - When the run's frames are up, it copies the path again and computes the swing frame, then goes to the ready
    step.
  - The ready step:
    - clears its own record to "none" once that record's contact frame has passed;
    - when the swing frame plus the timing lead falls under 9, it either holds (the line-call hold) or presses with
      its aim and goes to the swing step.

**Entering the wait step** writes its own record as "none". Unless the step keeps its state, it also:
- resets fresh, chase, middle, follow and own-team-hit;
- draws a new walk-back.

It re-rolls the net pick and clears the defer either way.

**The swing step**
- At swing frame 1 it outputs its aim stick.
- **When its own miss mark is set, it writes its own record as "missed".**
- When the stroke is over, it goes back to the wait step.
- While the opponents have the ball and the swing frame has passed, it counts its reaction delay down.

### The press, the claim and double hits
- A press's contact search never looks at the partner (B479, ported). Both teammates can lock the same ball. The
  first strike, hit or net touch cancels every other lock, and a cancelled lock plays its miss at its contact
  frame.
- The claim is set when a player's smash, volley or ground search finds its contact (not a dive), and when a
  serve swing starts:
  - it is taken when no one holds it, or when it was taken on an earlier tick;
  - on the same tick, the first finder keeps it.

  So a later tick's find takes the claim from an earlier one.
- The claim is dropped on every strike, hit, net-cord new path, serve phase start, change of ends and point set-up
  message (not on the players' reaction message).
- The claim is read in two places:
  - the human record keeper skips the claimant (ported);
  - keeping apart (below; not yet in the app).

### Keeping apart
- A doubles player's position is pushed out to 1 m from its partner when it ends up closer, staying on court, with
  the ported fallback.
- This applies on every move: running, standing, dives, approach runs, the receive's step and the other movers.
- Three movers apply it only when the player does **not** hold the claim:
  - **the stroke state's per-frame position check**, which runs with no step every frame the movement state is 2;
  - **the contact step that moves the body into the shot**, on the frame the step is taken and on the frames it
    blends;
  - **the serve swing's per-frame check.**

  The claimant isn't pushed away from its partner while it swings. The partner is pushed away from the claimant.
  For the server, its own serve-swing claim lasts until the serve's hit message drops it. After that it is kept
  apart again.

  This replaces the app's ponytail at the contact step ("the mover's partner check is skipped for the server in the
  original; kept here").
- In the stroke-state check, a non-claimant also has its "the contact step moved freely" flag cleared before the step
  runs. The contact step sets that flag again from its own move on the frames it runs.
- The push only happens in a four-player match, when the partner exists, and only when the mover asks for it. For
  these three movers that means: not the claimant. The claimant is the player whose index the claim holds.
- While the instant replay has a player's real state put aside (the replay's save and restore), the stroke-state check
  doesn't run at all, and any push measures against the partner's put-aside position instead of where it is drawn.
  This is replay playback only and doesn't change a match. The app's replay doesn't step the movers, so nothing to
  port.
- **What the app has today** (checked in `play.rs`):
  - The run and stand moves (`locomote` through the player's mover), the approach search, the dive's step and the
    contact step's IK tick pass the partner. The contact step passes it always, even for the claimant.
  - **There is no stroke-state per-frame check.** On stroke frames with no IK step the app doesn't call the mover at
    all. So a stroking player that its partner has walked into (closer than 1 m) is never pushed out. The port adds a
    zero-step mover call on every frame the movement state is 2, before the countdown, with the partner only when the
    player isn't the claimant. For a non-claimant it also clears the "moved freely" flag.
  - The serve's IK tick passes no partner at all. The original's serve-swing check exempts only the claimant. The
    server holds the claim from its swing start until the serve's hit message. So on the serve swing's frames after
    the hit, a server closer than 1 m to its partner is pushed out. In practice the server's partner stands at the
    net, so this almost never triggers, but the rule is: partner passed when not the claimant.

### The miss mark
- A player's miss mark is set on the frame its swing turns into its miss motion. That happens in two cases:
  - a locked swing reaches its contact frame after a cancel;
  - a press that found nothing reaches its miss frame (the search ended without a contact).
- It is set only when the swing's motion has a miss version: ground strokes, volleys and smashes have one, the dive
  doesn't.
- It is cleared:
  - at the start of every stroke-state update (after that frame's AI step has run);
  - when a new contact search starts;
  - when a serve swing starts;
  - when the point is set up.
- So the AI sees the mark on exactly one frame, the frame after the miss began.
- Only the doubles net and baseline rally routines read it. The receive and singles routines don't.
- Effect: the "missed" record makes a following partner end its follow on that frame, and a partner in its run step
  won't give way to it. The ball passes to the partner if the partner can still reach it.

### Voice
- When a player gives way, or follows from the no-find tail, it marks "speak". The mark is used on that frame:
  - if the partner's movement state is 2 or more (stroking or hit by the ball), the mark is dropped and nothing is
    drawn;
  - otherwise there is a 25% chance (one AI-generator draw), and when it speaks, a second draw picks between the two
    call lines (bit 16 of that draw).
- The partner's movement state is whatever the partner had when this player's AI step ran. The players update in
  index order, each running its AI step and then its own movement update before the next player starts (confirmed
  from the recordings, see *Order within a frame*). So players 0 and 1 see their partner's state as the previous
  frame left it, and players 2 and 3 see it after their partner's update on this frame.
- The app passes 0 (standing) every frame. So it draws the 25% roll, and sometimes the line, while the partner is
  mid-swing. That shifts the AI generator from the original's.

## Order within a frame (per player, in player order)
1. The AI step (receive or rally routine). It reads:
   - the partner's movement state as it is at that moment;
   - its own miss mark from its last update;
   - the claim and the records as left by the players before it.
2. The player's movement and hit update:
   - in a stroke: the keep-apart check with the claim exemption, the miss mark cleared, the countdown, a miss setting
     the mark again;
   - otherwise the run or stand moves, always kept apart.
3. The contact step (claim exemption).

The claim is taken inside the press or contact search during step 2. Messages (strike, hit, new path) are sent where
the ball event happens, and they drop the claim and the records for everyone.

**This order is confirmed from the recordings.** `ai_net_s05.bin` records, at each AI call, the player's own position
and movement state, the partner's position, and the partner's movement state as the routine read it. The scan below
used a scratch script, not in git.
- Within a tick, the AI calls always come in ascending player order.
- Partner position, on ticks where the partner moved: player 2 and 3 calls see the partner's position after this
  tick's move (1 012 of 1 026; 2 see the old position, 12 match neither). Player 0 and 1 calls see the partner's
  position from before its move on this tick (774 of 774).
- Partner movement state, on ticks where the partner's state changed: players 2 and 3 read the new state (74 of 75).
  Players 0 and 1 read the old one (77 of 77).
- The one exception (a player 2 call that read its partner's old state) wasn't looked into. It doesn't change the order.

The app's `control` runs `bot` per player in index order, and `bot` runs the AI step (`doubles_ai::step`) and then
that player's move and stroke update. So the order matches. The port reads the partner's movement state at the AI
step, the same way `foot_fx::state` derives it from the partner's fields at that moment. The routine side is
checked against the state field the `ai_net_s05.bin` calls carry.

## Constants
- Keep-apart distance: 1 m. Court bounds for the push: |x| ≤ 8.685 and 1.5 ≤ |z| ≤ 17.885 (already in the mover).
- Give way:
  - short run 2.5 m;
  - human short run 2.0 m;
  - human margin 1 m (depth and run);
  - near-frames window 30.
- Follow end:
  - 8 frames;
  - 2 m behind the partner;
  - 3 m near-ball radius;
  - hold x 2.7425.
- Wait walk-back re-pick: every 30 frames.
- Voice: 25%, then one bit, on the AI generator. Silent when the partner's movement state is ≥ 2.
- Miss mark: one frame. Record frame value "missed" = −3, "none" = −2, "cleared" = −1.

## Quirks (behaviour, match them)
- Ties in the two-CPU give-way keep the ball on both sides. Both then run and may both press. The first strike
  cancels the other, and the cancelled one plays its miss and marks "missed".
- A later tick's find takes the claim from an earlier one. The claim isn't "first to find".
- The voice is decided on the partner's state as it was when this player's AI ran, not on a state that is the same
  for both teammates on a frame.
- The miss mark is only one frame wide. If the AI isn't in its swing step on that frame, the mark has no effect.

## Verification

**`context/fixtures/match_s05.bin`** (slot 5, four CPUs, court 10, 26 400 frames, live layout) records each
player's block every frame. It holds:
- the contact countdown and branch;
- the miss mark;
- the movement state;
- the position.

It doesn't record the AI objects, the records or the claim. Scanned with a local script (not in git; it reads only
the fixture):
- After the first point's set-up, the two players of a team are **never locked at the same time** (0 frames). Both
  pressing at once (one searching, one searching or locked) happens 3 times: frames 3671–3687, 9145–9146 and
  23555–23556.
- **6 miss marks, each exactly 1 frame long.** All six come from a press that found nothing: the countdown goes from
  "searching" to "none" on the mark's frame, while the partner was running (4), standing (1) or stroking (1).
- Teammates never get closer than 1.52 m, so this fixture never exercises keeping apart.

**`context/fixtures/1p3goodcpus.bin`** (human + 3 CPUs, 3 910 frames):
- no double locks; 1 miss mark of 1 frame;
- teammates touch the 1 m keep-apart while both run or stand (49 frames), never while either strokes.

What the port can check:
1. **Miss mark glue.** A hst-play test forces a press with nothing to find, and a cancelled lock. The mark must be
   seen by the AI on exactly the frame after the miss starts, and the own record must become "missed" on that frame.
   With two bots, the partner that was following must end its follow on that frame. Against match_s05, the
   timing rule (mark one frame, set on the frame the countdown leaves "searching" without a contact) holds on all 6
   cases. Replaying those frames needs the countdown, motion and miss mark per frame, which match_s05 has.
2. **Voice gate.** A unit test sets the partner in a stroke on the give-way frame: no AI draw. The routine side is
   already bit-exact on the `ai_rally_s05*` fixtures, which carry the recorded partner state.
3. **Claim exemption.** A hst-play test (like `play/partner_ball.rs`) puts a claimant's partner within 1 m during
   the claimant's stroke. The claimant stays where it is and the partner is pushed out. A recording of the original
   needs a b479-style scripted run with P1 driven next to its locking CPU partner, logging both positions and the
   movement state per frame. match_s05 never gets teammates that close.
4. **One player per ball.** Over an app four-bot doubles run, count the frames where teammates are both locked
   after set-up. There should be (near) zero, and both-pressing should be rare, as in match_s05. A frame-exact,
   ball-for-ball comparison against match_s05 needs the same-state doubles replay (P11k6 after P11n1). Until then,
   this check is statistical.
5. **Stand-in removal.** Before deleting the `mine` / `intercept` / call-out block in `bot`, check (with a debug
   assert or a test over a four-bot match) that it never runs with a plan.

## Answered questions (spec pass 2)
- **Player update order:** index order, AI then movement per player. Confirmed from `ai_net_s05.bin` (*Order within a
  frame*).
- **Is the stroke-state check in the app?** No, only at the contact step. The port adds it (*Keeping apart*, *What
  the app has today*).
- **Proof of the keep-apart exemption:**
  - The decompile shows one per-call flag on the already bit-exact mover push. The three movers pass "not the
    claimant", and every other mover passes "always".
  - The acceptance test is a hst-play rule test (*Verification* point 3). It checks that the claimant isn't moved and
    a non-claimant is pushed, using the existing bit-exact `mover`.
  - No recording of the original shows it: match_s05 never brings teammates within 1 m during a stroke. A scripted
    capture isn't required. The port lists it under *Not verified / not 1:1* in its journal.

## Port findings (from the recordings, 2026-10-20)
- The voice gate's "may call" flag is armed at the point's set-up and again on every hit by the opponents, not on a
  hit by its own team. It is cleared after a give-way or follow call, spoken or not. Scan: 11 doubles fixtures;
  the only counterexamples cross a point boundary.

## Open questions
none

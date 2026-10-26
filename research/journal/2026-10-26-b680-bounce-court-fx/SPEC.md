# B680 — what a ball bounce shows on each court (spec)

Behaviour only, in our own words. Two separate effect objects react to the ball's bounces:

1. **The bounce effect** (already ported as `effect::Bounce`): a mark, puffs, the `ballbound` ring and the smash crater.
2. **The bounce debris** (not ported): flying particles. Clods (`par/tuti.tm2`) fly on clay courts and grass blades
   (`par/siba.tm2`) on grass courts, but only during instant replays. A burst of clods flies on every court when a
   smash lands, in live play too.

The material effect kinds 1..15 (`Material::effect`) drive the word pop-ups only (`bodyhit` `BouncePop`, already
ported). Neither object looks at them. Both look only at whether the contact's material is a **court surface** (the
material's court flag, `Material::court`).

## Scope of the port (user 2026-10-26)

The remaster has no instant replays yet (they are shelved under B72), so **this task ports the live-play parts only**:
- §2 except the marks save/restore across a replay (§2.1 last bullet group);
- §3.1's trigger with only the smash burst reachable (the clod and blade bursts are gated on the replay flag, so in
  live play they do nothing but store the kind);
- §3.4 the smash burst, with the cut always 0 (it only applies during a replay);
- §3.5 aging of the smash pool; §3.6 drawing of the smash pool.

The parts marked **(deferred to B72)** below stay here as the record of the original's behaviour; B72's line points
here, so they are ported when replays are. In live play they change nothing: the replay-gated bursts spend no
random draws (the gate is checked before any draw).

## 1. Per court: the answer to "wrong mark on grass?"

In live play every court shows the same `ballkon_00` mark, the same `kemuri` puff and the same ring. Only the
colours differ, from the court look table. **So the port's grass mark is right.** What the port is missing is listed
below.

| court flags (look table)           | courts       | live play                | instant replay, first bounce   |
|------------------------------------|--------------|--------------------------|--------------------------------|
| dusty + clay                       | 1, 2, 3, 9   | mark, puff, ring         | + 30 clods (clod colour)       |
| dusty + grass                      | 5, 6, 8, 10  | mark, puff, ring         | + 30 blades (two blade colours)|
| dusty only (hard)                  | 4, 11        | mark, puff, ring         | nothing more                   |
| none (indoor 7; rows 0 and 12)     | 7            | mark and ring, no puffs  | nothing more                   |

A **smash landing** (rule in §3.1) on any court throws the smash clod burst (§3.4) in live play and in replays,
alongside the crater. On a wet court the dust puffs become spray (§2).

## 2. The bounce effect: parts the port lacks

The port's `effect::Bounce` has the dry-weather behaviour. The original also has the following.

### 2.1 Point set-up (new point, and the start of an instant replay)
- **Wet** = weather state 2 or 3 (the two rain states).
- **Dusty** = not wet AND the court's puffs flag. The port's `puffing` flag only takes the court's puffs flag.
- Puffs animate (age, grow, fade) only when dusty or wet. Otherwise there are none.
- Puff texture: dry `run/kemuri_01.tm2`, wet `run/spray.tm2`.
- **Marks across a replay (deferred to B72):**
  - At an ordinary point set-up, the marks list is saved.
  - When an instant replay starts, the marks list is restored from that copy. The replay therefore starts with the
    marks as they were when the replayed point began: marks made during the point vanish and then appear again as
    the replay's bounces make them.
- Pause / resume (a save and a restore message) copy the puffs, ring and crater state away and back. This doesn't
  matter for a port that freezes the match while paused.

### 2.2 First bounce (court material)

| | dry | wet |
|-|-----|-----|
| puff start size | 0.15 | 0.25 |
| second-bounce puff start size | 0.1 | 0.2 |

- Growth is unchanged: 0.3 over 20 frames (the second bounce: 0.2 over 20 frames).
- On a wet court every puff also **rises 0.01 per frame**.
- **Smash landing, wet court:** no dust cloud. Instead an ordinary puff is added with **double** the start size and
  double the growth (0.5 and 0.6/20).
- **Smash landing, dry court:** the existing dust cloud. Its size is twice the dry puff size (0.3), as the port has
  it.
- **Ring model:** on a wet court, when the material is court and the point had no body hit, the ring is the
  `run/spray` model instead of `bnd/ballbound_00`. Its placement is the same: turned to the contact normal, at the
  contact.
- **Ring scale:** **1.5** when a body hit (the ball struck a player) has happened this point, else 1.0. The P9f
  journal's "replay scale 1.5" note is wrong: the scale follows the body hit, not the replay.
- **Crater lighting:** the crater is lit by the scene's global light only when not wet. Rendering only, so match by
  eye.

### 2.3 Unchanged
The mark (3-segment strip, width 0.05, length |v_xz|, colour = the mark rgb, alpha life·128/1800), the puff colour
(the dust rgb) and the crater all stay as ported.

## 3. The bounce debris (new)

### 3.1 Trigger (checked every frame, before aging)
- When the ball's bounce count is 0, the "seen first bounce" latch is cleared.
- The trigger fires once per flight: on the frame the count is 1 and the latch is not yet set. The frame sets the
  latch and goes on only if:
  - the ball has a contact record;
  - **no body hit** has happened this point;
  - the contact material is a court surface.
- **Kind:**
  - grass flag set → blades;
  - otherwise clay flag set → clods;
  - otherwise none.
  - Grass wins if a row had both flags; none does.
- **Smash landing:** the last shot was a smash (the hitter's shot kind is the smash kind, the same test as the port's
  `g.smashed`), there is more than one player (the game's player count > 1), and the ball's speed is 85 km/h or
  more (the same speed function as the bounce sound and crater). When all three hold, the smash burst (§3.4) is
  thrown **first**, whatever the kind.
- Then, for kind:
  - **clods, and no smash burst** was thrown → the clod burst (§3.2);
  - **blades** → the blade burst (§3.3), even after a smash burst;
  - **none** → the main pool goes inactive.
- **Gates inside the bursts** (as for the dive debris):
  - the clod and blade bursts do nothing unless the **instant-replay flag** is set;
  - the blade burst also does nothing on a **wet** court, unless a smash burst is live;
  - the smash burst has **no replay gate**: it fires in live play.
- A burst that does nothing still leaves the kind stored. If the pool is active from an earlier burst, aging goes on.
- A burst marks its pool active. On the trigger frame, aging (§3.5) runs right after the burst, so every particle
  ages once on the frame it is thrown.

### 3.2 Clod burst (replay only, clay courts; 30 particles) (deferred to B72)
Same algorithm as the dive debris clods (`foot.rs` `throw`, clods), with these inputs:
- **origin** = the ball's position on this frame;
- **direction** = the contact's velocity (the velocity stored with the first bounce's contact record);
- the speed magnitude is |contact velocity xz|, raised to the power, times the speed lerp.

Steps:
- basis = the impact basis of the direction (`effect::impact_matrix`); aim = origin + basis forward · reach.
- Per particle (four sound-manager draws, in this order):
  1. turn: the basis is turned about its forward axis by draw·2⁻³²·2π. The turns **accumulate** from particle to
     particle.
  2. r: direction = normalize(aim + side·r·spread − origin).
  3. speed lerp: velocity = direction · (mag^power · (lo + (hi − lo)·u)). The power is applied only when it is
     greater than 0.
  4. cell: one of four 32×32 cells, chosen by (draw >> 16) & 3.
- Then the vertical velocity is made non-positive (pointing up; the game's y is down) and clamped to no less than
  the lift.
- size = (lo + (1 − r)·(hi − lo)) / 2.

Constants (clod row):

| constant | value |
|----------|-------|
| count | 30 |
| reach | 2.0 |
| spread | 3.5 |
| speed | 0.1 .. 0.5 |
| power | 1.1 |
| decay | 1.0 |
| size | 0.05 .. 0.1 |
| shrink | 0.95 |
| fall | 0.8 |
| lift | −0.03 |

Cells (u, v) by index: (0,0), (32,0), (0,32), (32,32).

### 3.3 Blade burst (replay only, grass courts, not when wet; 30 particles) (deferred to B72)
Same algorithm as the dive debris blades (`foot.rs` `throw`, blades). Origin and direction are as in §3.2, and the
basis is built but not used.

Per particle:
1. the turn draw is spent;
2. size = lerp(size lo, hi, draw);
3. cell from a draw;
4. **three C-library `rand()` calls**, in the order spin z, spin y, spin x. Each is
   lo + (hi − lo)·(rand with its low 11 bits cleared, times 2⁻³¹), in degrees converted to radians, exactly as the dive blades do;
5. the matrix = euler(spin) applied to identity;
6. velocity = contact velocity · push, with vertical 0;
7. position = origin + x offset lerp (a draw) + y offset lerp (a draw).

The sound-manager draw order per particle is: turn, size, cell, x offset, y offset. The three `rand()` calls fall
between the cell and the x offset.

Colour: the first half of the particles (index ≤ count/2, so 0..15) take blade rgb 1; the rest take blade rgb 2.

Constants (blade row):

| constant | value |
|----------|-------|
| count | 30 |
| reach | 2.0 |
| spread | 3.5 |
| speed | 0 .. 0 |
| power | 0 |
| decay | 1.0 |
| size | 0.2 .. 0.3 |
| shrink | 0.95 |
| fall | 0 |
| x offset | −0.45 .. 0.45 |
| y offset | −0.4 .. −0.05 |
| spin x | −15 .. 15° |
| spin y | −10 .. 10° |
| spin z | −10 .. 10° |
| push | 0.1 |
| sink | 0.0005 per frame |

Compare the dive debris rows:
- dive clods: decay 0.9 and size 0.1..0.2;
- dive blades: size 0.3..0.5 and shrink 0.9.

The rest is the same. The two tables are separate in the game program. The bounce one is a 30-particle count
followed by two rows in the same layout as the dive rows. The port reads it from GAME.BIN through `hst-data::exe`
like the dive table: locate it by these values.

### 3.4 Smash burst (any court, live and replay; 30 particles, its own pool)
- Uses the clod row's constants and the clod algorithm (four sound-manager draws per particle, accumulating turns),
  with these differences:
  - **direction** = (contact velocity x, **−1**, contact velocity z), i.e. pointing up and along the ball. The basis
    comes from that. The speed magnitude is still |contact velocity xz|.
  - **origin** = the ball's position on this frame. Every particle starts there and is alive.
  - **size scale:** both size bounds are multiplied by s = base − cut:
    - base = 2.5 on courts 4, 7 and 11 (the hard and indoor courts), else 3.0;
    - cut = 0.5 when the instant-replay flag is set and the replay camera's mode is not one of the five close-up
      modes (camera mode 6..10), else 0.
    - So sizes are (0.05..0.1)·s, halved as in §3.2.
  - Its cells come from a list whose (u, v) by index are (0,0), (32,0), (0,32), (32,32). These are the same four
    cells.
  - Texture `par/tuti.tm2`, colour = the court's clod rgb (indoor court 7 has one too: 65,48,19).
- While its pool is live, the clod burst of §3.1 is skipped. The blade burst isn't.
- Its "live" flag is cleared at each point set-up, but its particles are not. They are only redrawn once a new
  smash burst refills them.

### 3.5 Aging (every frame, after the trigger)

**Main pool, clods (deferred to B72):**
- pos += vel;
- alive = pos.y < 0;
- vel ·= decay;
- vel.y += fall·9.8/3600 (the constant 0.0027222224);
- size ·= shrink.

**Main pool, blades (deferred to B72):**
- pos = pos + (vel + wind·0.5), in that order;
- pos.y += sink;
- vel ·= decay;
- size ·= shrink;
- matrix = euler(spin) · matrix;
- alive = not (y ≥ 0 or size < 0). The threshold is 0, unlike the dive blades' 0.01 (`foot.rs`): the two
  tables age with different tests. With shrink 0.95 the size never reaches 0, so blades die only by reaching the
  ground plane (y ≥ 0);
- the wind is the weather's wind vector, the one the puffs drift with.

**Smash pool:** aged like clods with the clod row's decay, fall and shrink.

A pool with no live particle left goes inactive. Aging runs in live play and in replays alike: only throwing is
gated.

In slow motion (an offbeat-rules mode, `offbeat`), every step above is scaled by the slow-motion rate:
- each move is multiplied by the rate;
- each decay/shrink becomes x − (x − x·k)·rate.

Port it only if the port's slow motion already scales the dive debris; otherwise leave it.

### 3.6 Draw
As the dive debris draw (`foot_fx`): uv = (cell + 0.5)/W .. (cell + 31.5)/W of the texture.
- **Clods** (main and smash pools): two quads each.
  - A camera-facing billboard of half-size `size` in the court's clod rgb, alpha 128.
  - A ground shadow quad at y −0.01 in (32,32,32), alpha 90.
- **Blades:** one quad, pos ± matrix row 0·size ± matrix row 2·size, in the particle's blade colour.
- Both use `par/tuti.tm2` for clods and `par/siba.tm2` for blades. Rendering only, so match by eye.

### 3.7 Pause
The save / restore messages (pause) copy each live pool away and back. This doesn't matter for our freeze-on-pause.

## 4. Randomness (this is gameplay-relevant)

**Frame order.** The effects are children of one effects manager, updated once a frame in the order they were
created: the dive/foot effects first, then the hit sparks, then the bounce debris. So on a frame where both act,
the hit sparks' end-of-burst re-roll (a quarter of their table from `rng.sound`, `effect::Sparks::tick`) draws
**before** the bounce debris' smash burst. The port runs the debris throw after `sparks.frame` in the same frame,
after the ball's flight for the frame has run (it reads this frame's bounce count, contact and position). Nothing
else in the port draws from `rng.sound` on a bounce frame (the shot's random bit is drawn at the strike).

- **Sound-manager generator** (`rng.sound`, MT19937, the one sparks and dive debris draw on):
  - a smash burst takes **120 draws in live play** (4 per particle);
  - a replay clod burst takes 120;
  - a replay blade burst takes 150 (5 per particle).
- **C-library `rand()` (deferred to B72):** a replay blade burst takes **90 calls** (3 per blade). `rand()` seeds the next point's
  shared and court generators. A grass-court instant replay therefore shifts every later point's randomness, so the
  port must spend these calls exactly as the dive blades do (`Feet::crand`).
- The smash burst's 120 live draws matter wherever `rng.sound` is part of the replay-test state or seeds anything
  later. The port must spend them on the bounce frame even with effects off: the sim's draw count can't depend on
  rendering settings. Keep the throw in `hst-sim` (alongside `effect::Bounce`) and the drawing in `hst-base`.

## 5. Verification plan (Port phase)

- **Bit-exact:** the smash burst. Find a live smash landing at ≥ 85 km/h in a doubles match (slot 5 is doubles; or
  force the shot kind / speed through PINE) and compare each particle's position, velocity, size and cell for the
  burst's life, plus `rng.sound`'s state after the throw frame. Prefer a frame where the hit sparks also re-roll, to
  prove the order above (or check both draws land in the right order from the generator state alone).
- **By eye:** `tools/screenshot.sh` of the original at a live smash landing on a hard, clay and grass court, and a
  wet-court bounce (spray ring, rising spray puffs, wet smash puff); against each, a `--shot` at the same moment.
- The replay bursts' checks go with B72.

## Port notes
- The port's `effect::Bounce` needs:
  - the wet flag (weather state);
  - the body-hit flag (ring scale, and the debris gate).
- The smash burst needs:
  - the contact velocity of the first bounce (`Contact::vel`, already recorded);
  - the ball position;
  - the court number (size base) and its clod colour (`exe::FootCourt`);
  - `rng.sound`, spent in sim order whatever the render settings (keep the throw in `hst-sim`).
- The bounce debris table (count + clod row + blade row) is read from GAME.BIN via `hst-data::exe`; the port only
  needs the clod row now, but reading the whole table keeps B72's side simple.
- No replay flag yet: the replay-gated bursts are simply not called (they would draw nothing in live play).

## Open questions

none

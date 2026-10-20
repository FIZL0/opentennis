# P11m2 spec: the AI generator's draws between match setup and the first point

Behaviour of the original, in our own words, from reading the game and checking it on the running game
(probes in `research/p11m2_intro_probe.py`, captures in `context/p11m2/`). Terms below use the app's names:
"AI generator" = `Rngs::ai`, "shared generator" = `Rngs::shared`, "reset" = the AI's per-point reset as the app
already does it (`Mind::reset` net pick, then `timing` with no shots, then `picks` with dive allowed, then `count`).

## What has an AI object

A player gets an AI object (and so a reseed of the AI generator from the shared generator) when it is a CPU, or a
human whose doubles partner is a CPU. The app already does this (P11m1). Singles humans and doubles teams of two
humans have none.

## The reset and its net pick

A reset draws, in order: the net pick (one draw, 50%) only when it's a *full* reset, then the eight timing-error
draws, the eight picks, and the one 2-4 hit-count draw. So a full reset is 18 draws, a plain one 17.

A reset is full when the match-start flag is set **and** either the match is singles, or the player's doubles partner
already exists and has an AI object. The match-start flag is on from setup until the first point is over; it is off
for every later point. So only the setup, the intro, and the first point use full resets.

Differences from the app today:

- The game's partner test is "the partner has an AI object", not "the partner is a CPU". When a team has one
  CPU and one human, both have AI objects, so the CPU's reset at the first point is full (18). The app's
  `partner_bot` gives 17.
- A human's AI object (a human whose doubles partner is a CPU) is reset like the others, every time the players
  are reset: 18 at the intro and the first point, 17 at every point after that. It is never updated (it never takes
  a role or draws for one). The app never resets it, so in a doubles match with a human and a CPU on one team, the
  app falls 18 behind at the first point and 17 behind at each later one.

## Setup: making the players

The players are made in order (player 0, 1, then 2 and 3 in doubles). For each player that gets an AI object:

1. The AI generator is reseeded from the shared generator, and the AI object gets one reset straight away.
2. The player's own placement then runs: one shared-generator draw (already in the app), then a second reset.

Both are full or plain by the rule above, using who exists *so far*. In doubles the first-made AI object of each
team sees no partner yet, so it draws 17+17. The second-made one sees its partner, so it draws 18+18.

Each reseed wipes the draws before it, so only the last-made AI object's two resets count. That one is always full:
in singles the net pick is always drawn, and in doubles the last AI object is player 2 or 3, whose partner was made
earlier and has an AI object. **The AI generator's index after setup is 36 whenever any AI object exists.**

Captured: singles human vs CPU: 0 -> 18 -> 36. Singles CPU vs CPU: (P0) 18 -> 36, reseed, (P1) 18 -> 36. Doubles human P0 +
CPU P2 vs two CPUs: 17/34, 17/34, 18/36, 18/36 for P0, P1, P2, P3.

## The intro (before the first point)

When the match's opening phase starts (after the court/players intro, before any point), every player is reset
once more. Then the CPUs take their opening roles before the first point's own reset:

1. **Every AI object gets a full reset (18 draws), CPU or human.** By now every partner exists, and on any team with an
   AI object both players have one, so all of these resets are full.
2. **Each CPU player's AI then enters its opening role**, with the same draws as when it enters that role at the
   start of a point:
   - server (Serve): 17 (timing with no shots 8 + picks 8 + count 1). The serve's own per-frame draws (spot pick,
     wait timer) don't happen yet: they only run once a point is under way.
   - receiver (Receive): 0.
   - doubles players in the rally role (Rally entry): 1 each (the return-to-centre roll).
   These are the same roles the app gives each CPU at the first point's start. A human's AI object takes no role.

Nothing else draws from the AI generator between setup and the first point. The first point's reset then overwrites
everything the intro drew, so only the **number** of intro draws matters, not their order or values.

Index at the first point (before its own reset pass), = 36 + 18 per AI object + 17 if the server is a CPU + 1 per
doubles CPU in the rally role:

| match (captured) | after setup | after intro resets | after roles = first point starts at |
|---|---|---|---|
| singles, human P0 serves vs CPU | 36 | 54 | 54 |
| singles, CPU P0 serves vs CPU | 36 | 72 | 89 |
| doubles, human P0 + CPU P2 vs CPU + CPU | 36 | 108 | 110 |

(P11m1's 54 singles / 108 doubles are the "after intro resets" column.)

## The first point and after (for reference: the app already does most of this)

At the first point every AI object is reset again, full (18 each, humans' AI objects included). Then each CPU
enters its role, as in the intro. The serve's per-frame draws only start then. Captured:

- singles CPU vs CPU: 89 -> 107 -> 125 (two resets) -> 142 (Serve entry 17) -> 144 (serve routine's first draws).
- doubles: 110 -> 182 (4 x 18) -> 184 (two Rally entries).
- doubles second point: 4 x 17 plain resets, the human's AI object included (505 -> 573), then 575 (two Rally entries).

The player-0-serves-first rule (score.rs) decides the server role in the intro, as at the first point.

## What the port must do

- At setup: 36 draws after the last reseed (the last-made AI object's two full resets). The app draws none today.
  Earlier AI objects' setup draws can be left out, since the next reseed wipes them. Only the count matters: the
  intro and the first point overwrite what they set.
- Before the first point's reset: 18 per AI object, then the role-entry draws (17 / 0 / 1) for each CPU.
- At every point: reset humans' AI objects too. Use "the partner has an AI object" for the net-pick test. Don't
  touch the shared, court or sound generators (P11m1 already matches those).

## Open questions

none

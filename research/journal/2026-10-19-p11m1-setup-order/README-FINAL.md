# P11m1 — the match setup's draw order (AI reseeds)

Live probe from the menus into a new match, on PCSX2 copy 4 (HST_REALTIME, 0.25 speed).

## How
- Menus by hand with `tools/vpad.py` from boot: Fun Time Tennis → Singles/Doubles → 1-A + COMs → characters → Start,
  saved on the Confirm Settings screen ("Start the match!" under the cursor) to scratch slots 8 (singles: Ashley vs
  COM Cody, North Academy clay) and 9 (doubles: Ashley + COM Jun vs COM Cody + COM, Queen's grass). On this copy ✕
  confirms and ○ goes back (REFERENCE says ○ confirms: not on copy 4's pad mapping).
- `research/journal/2026-10-19-p11m1-setup-order/scripts/p11m1_setup_probe.py <name> <slot> [follow s]` loads the slot, presses ✕ and polls (PINE, as fast as it
  goes) the AI generator and the shared generator (pointer at the match object +0x80: it's a pointer, not the
  object — reading +0x80 as the generator gave nonsense on the first try) at every change, until player 0 can serve.
- `research/journal/2026-10-19-p11m1-setup-order/scripts/p11m1_setup_order.py <name>` replays the shared generator from its seed (word 0 when first seen fresh)
  and names the shared draw each AI reseed took. Captures: `context/p11m1/{singles2,singles3,doubles1}.json`.

## Result (shared draws, 0-based, after the seed)
Singles (P0 human, P1 COM), same in two runs:
- #0–121 weather schedule (122)
- P0: #122 voice bank, #123 placement
- P1: #124 voice bank, **#125 AI reseed**, #126 placement
- #127–226 hit-spark table (100), #227 gallery pick
- intro: 2 + 141 more shared draws, then the first point reseeds the shared generator (they're lost), then 3 draws
  (2 placement + the server's first-serve voice).

Doubles (P0 human, P1–P3 COM; every player gets an AI object, P0 because its partner is a CPU):
- #0–121 weather
- P0: #122 voice, **#123 AI**, #124 placement · P1: #125, **#126**, #127 · P2: #128, **#129**, #130 ·
  P3: #131, **#132** (poll missed the moment; the final AI state is Mt(draw #132) exactly), #133
- #134–233 sparks, #234 gallery.

The final AI generator is Mt(shared draw of the last AI object made) in both: earlier reseeds are overwritten.

## Compared with the app
`play.rs` setup per player: voice draw (`r15`), `new_ai` for a CPU or a human beside a CPU partner, placement draw;
then `setup_gallery` (100 + 1). `main.rs` draws the weather schedule from the fresh shared generator first. Same
order, same counts: **matches**. Nothing to change in play.rs.

## Not order, but found (→ P11m2)
The AI generator is drawn from right after each reseed: 34–36 draws as the AI object is made (17+17 or 18+18 by
character), and more during the intro (singles 18, doubles 72), so at the first point it is at index 54 (singles)
/ 108 (doubles), then the bots' first point draws follow. The app draws nothing from it at setup or in the intro.
Since only the last reseed survives, what counts is the last AI object's creation draws plus the intro's.
New task P11m2.

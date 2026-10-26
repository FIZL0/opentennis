# B671a: Pause screen, Offbeat rules tab on the score panel

User's board D ("all 8 on, wide: short names, the tab wraps to a second row", AllWrap / AllWrapTall). Our own design,
built on B671's panel B. B671 was still in the runner's gate when this started, so its commit (f6e96c77) is merged into
this branch as the base.

## What changed
- `hst-play` `name_tags.rs`: `rules_tab` draws the tab as one image. It's a navy tab in the frame's gold rim
  (#c9c79a), with round top corners and no bottom edge. "OFFBEAT RULES" is in yellow (#f6d64a), followed by one orange
  (#f07a2c) chip per rule, each with a white ring. `tab_flow` lays the label and chips out in rows no wider than the
  tab may be: a chip is never split, dropped or shown as "+N", and new rows grow the tab upward (its foot stays on
  the frame).
- `main_menu/offbeat.rs`: `chips(&Offbeat)` gives the Confirm strip's short names (Bounce, Slow-mo, Turbo, Ghost,
  Tiny/Big, Fast/Slow, Sweet spot, All F–A). The strip's `rules_on` now calls it.
- `menu/rules_tab.rs` (new):
  - The tab is rebuilt while paused whenever the match's rules change. No rule on, no tab.
  - It is placed 20 px in from the frame's left edge, at most 424 px wide (20 px in from the right too), and its foot
    sits 3 px onto the frame's top.
  - `Fit` keeps the dialog off the tab. It moves the dialog up, but no higher than the top bar's rim (y 58). If that
    isn't enough (a 5-set match with "Controllers": the dialog already touches the frame), it also shrinks the
    dialog evenly, about the middle of its top edge. The confirm box (Yes/No) follows the same rule; its full-screen
    dimmer doesn't move.
  - Mouse and touch on the pills use the same `Fit`.
- `menu.rs`: a `Tex::Rules` quad, a `tab` field on `View`, and the dialog's quads passed through the fit.
- Tests:
  - `rules_tab_wraps` (hst-play): rows, wrapping, a chip wider than the tab still shown, all 8 in ≤2 rows.
  - `rules_tab_clears_the_dialog` (hst): every sets / local / confirm case with tabs of 1–5 rows.
    - The tab is on the frame.
    - The dialog is clear of the tab and below the bar.
    - The pointer rows match the drawn rows.
    - With room to spare, nothing moves.
- Release-notes line.

## Shots
In `context/B671a/` (`--play --pads k,-,-,- --offbeat …`, `HST_KEYS="8:Escape"`, `--shot-at 10`), at 1280×720 and
720×1080:
- `all8-5set-doubles-*`: all 8 on, 5 sets, doubles. The dialog moves up and shrinks a little.
- `all8-1set-singles-*`
- `two-3set-singles-*`: 2 on. Nothing moves.
- `none-singles-*`: no tab.
- Sheets: `sheet-wide.png`, `sheet-wide2.png`, `sheet-tall.png`.

## Not verified / not 1:1
- This is our own design; the original's pause screen shows no rules. It was checked by `--shot` against the board,
  not against the original.
- With the stock short names, all 8 rules fit in one row: the tab may be 424 of the frame's 464 px wide, wider than
  the board's 88 %. So the second row the board shows only appears if a row runs out of room. The wrap is covered by
  the test, not by a shot.
- A 5-set match with every rule on shrinks the dialog slightly (about 0.9×) instead of covering the tab: in that
  layout there's no room between the top bar and the frame. The board's mockup has more room than our layout.
- The colours are the board's hex values. The gold is the mockup's frame colour, not sampled from the disc's frame
  art, so it may differ slightly in tone.
- Online matches weren't tried; the tab reads the same `Game::offbeat` the match plays with.

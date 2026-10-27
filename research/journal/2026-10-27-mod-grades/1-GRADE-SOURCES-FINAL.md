# What the select screen's grades follow (data only)

Question: the five hand-set letters per stock character (Serve, Stroke, Volley, Impact, Footwork; 0 F … 5 A) —
which per-character data explains them, why do characters 3, 6, 7, 12 and 13 split C/D/D/D/C on Impact with
identical Impact columns, and what grading function would reproduce all 14 from data.

Method: a throwaway ignored test (removed again) dumped, through the repo's own loaders, every character's
TParam.csv row, the shot records of every class and kind, the executable's per-character blend-weight rows,
shot effects, shot-variant list and volley down-2 thresholds, plus every trajectory table in its TRAJ archives
(decoded to elevation / speed / frames; mean and max per table, and which characters share a table). Each
feature was ranked against each category's letters (Spearman ρ; "discordant" = pairs graded one way and
valued the other; "ties" = equal value, different letter). The dump stays in `context/mod-grades/`; the
analysis scripts were scratch.

## Structural findings

- The executable's blend-weight row for each character is exactly (TParam value − 8) / 8 of Serv POW, Strk
  POW, Voley POW, Lob POW, Lob POW2, Top / Slice / Drop SPIN. So every shot record is a pure function of
  TParam; the records add nothing independent.
- The trajectory tables are authored per character but track the same knobs: base volley table mean speed
  ranks exactly with Voley POW (ρ 1.00), stroke kind 0 with Strk POW (0.996), serve kind 1 with Serv POW (0.98).
  The lob tables (stroke and volley kind 3) are shared by 10 of 14, the second smash table by all.
- The stroke late-hit variant weight splits the roster into three groups that line up with Impact exactly:
  full drop (0, 1, 2, 5) = Impact A/B; 0.6 (3, 4, 6, 7, 12, 13) = C/D; 0.5 (8–11) = E/F.
- Curves: only characters 3 and 8 bend the rally slice (opposite signs), 11 and 12 bend the topspin serve,
  10 has the up1 slice serve, 6 and 13 a stronger lob curve. None of these lines up with Stroke or Impact
  (3 is Stroke D, 8 Stroke B); they are not needed to explain any letter.

## Per category (best single feature, then the refinement that orders all 14)

| Grade | Primary (ρ, discordant, cross-grade ties) | Exact with a tie-break |
| --- | --- | --- |
| Serve | Serv POW (0.983, 0, 2: 8 vs 9/12). Serve tables' mean speed 0.973, 1 discordant (8 below 9) | Serv POW + 0.1 Serv CON |
| Stroke | Strk POW (0.978, 0, 1: 4 vs 12, both 8). Base stroke table 0/2 mean speed alone is exact (0.981) | Strk POW + 0.1 Top SPIN |
| Volley | Voley POW (0.914, 4, 2): 9 (Voley POW 9) is a D, 2 (6) a C | Voley POW − 0.25 Strk POW + 0.1 Voley CON |
| Impact | early rally-hit frames (ウサギ GI, 0.969, 0, 6: the 3/4/6/7/12/13 block) | ウサギ GI − 0.1 Serv POW |
| Footwork | SPE (0.995, 0, 3) | SPE − 0.01 Reach (cm) |

Volley is the only category whose letters don't follow its own column closely: the fit says net players are
graded up against their ground power (a baseliner with high Voley POW, 9, gets a D). With 14 rows and two free
weights an exact fit is easy to find, so read the Volley and the tie-break terms as "consistent with", not proof.

## The 3 / 6 / 7 / 12 / 13 Impact split

Nothing timing-related differs (all six Impact columns, rising adjust, the stroke and volley variant weights
within the group). What separates them is power: Serv POW 7 and 6 for the two Cs (3, 13) against 8, 14 and 9
for the Ds (6, 7, 12); also Voley POW + Top SPIN (14, 14 for C; 16, 18, 18 for D, and 15 for 4's C). The
designer seems to grade Impact as "window size, and harder to time the stronger you hit". Both tie-breaks
also keep 2 and 5 (B) above 4 (C) and 11 (F) below 8–10 (E).

## Proposed grading function

Per category a numeric score from named fields, then the letter by cut points halfway between the stock
characters' neighbouring grade groups (monotone, since each score has no discordant pair and no cross-grade
tie):

- Serve: Serv POW + 0.1 · Serv CON — cuts 5.5 / 7.65 / 10 / 12 / 14.5
- Stroke: Strk POW + 0.1 · Top SPIN — cuts 5.1 / 6.25 / 8.9 / 11.45 / 15.9
- Volley: Voley POW − 0.25 · Strk POW + 0.1 · Voley CON — cuts 4.08 / 5.62 / 7.15 / 9.85 / 13.6
- Impact: early rally-hit good frames − 0.1 · Serv POW — cuts 0.3 / 2.35 / 3.25 / 5.0 / 7.15
- Footwork: SPE − 0.01 · Reach (cm) — cuts 8.05 / 8.6 / 9.6 / 10.7 / 11.35

All five reproduce all 14 stock letters (70/70). Leave-one-out (cuts rebuilt without the character, then it is
graded): exact 12, 10, 9, 11, 12 of 14; within one letter 14/14 in every category (the misses are characters
alone at the top or bottom grade, or groups only one wide). The current nearest-row method only gets Impact
right for the five-way tie through its base tie rule.

For a mod: take Serv POW from the serve source (its tables are authored from it), Strk / Voley POW from the
stroke / volley sources, and the timing frames, SPE and reach from the mod's own row (those drive its play).
Scores beyond the roster grade past the end cuts (A, or the S above A).

## Open questions

- Whether the 0.1 tie-break weights are the designer's intent or a fit; more characters would tell.
- What the shot records' middle group does in play (its volley kind-2 field also orders Impact exactly, but
  it is the same Voley POW + Top SPIN mix).

## Landed

`grades::scores` / `grades_of` grade a mod's card this way (Footwork's reach is TParam column 57; Impact's frames the
first part of column 28), cuts rebuilt from the disc's rows each run; S is a score as far past the roster's best as
that is past A's cut, so only a row's own cells (Impact, Footwork overrides) can reach it. The mods repo's
`gag_stats.pick_shots` mirrors the formula and picks each rebalanced mod's `shots` so its card reads its `stats`
(77 of 77 agree in the game's code; the all-S joke character reads AAAAC).

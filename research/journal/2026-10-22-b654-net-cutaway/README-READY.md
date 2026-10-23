# B654 — net in post-point cut-aways

**Result: the cut-away case does fire, and the port agrees with the original frame by frame.** The net the user saw
is most likely the cut-away from beside a net post. The original draws the net in that shot too. No code changed,
apart from a regression check. The user needs to decide whether to go beyond the original here (see *Your move*).

## What I checked
- **Wiring in the port.** I added a temporary log to `net_hide::hide` and ran all-CPU `--play` matches:
  - Courts 1 and 8, doubles, 120–300 s. The net is tagged (`HideNearCamera`, one entity per court). On singles,
    courts 1, 5 and 10 each have their own `znet_s0100` plant, also tagged.
  - In a cut-away, `Director::showing()` reaches the test.
  - Every cut-away that passes all three conditions hides the net for its whole length: 239/239 frames on court 1,
    832/832 and 118/118 on court 8. At the end of the frame its meshes have `InheritedVisibility` and
    `ViewVisibility` false.
  - Split view (off during cut-aways), the alternate camera (`alt` cleared while a cut-away is on screen) and own
    camera all hand the cut-away's view to the test. Nothing else resets the net's visibility (I checked
    `mode::warm`/`unwarm`).
  - Pitfall: `hst --play` without `--stage` loads no court at all, so there is no net to hide. Matches started from
    the menus pass `--stage`.
- **Against the original.** `scripts/b654_gates.py 5 0 60000 1940890` (slot 5, court 10, PINE) logs every vsync
  where the eye passes the two distance gates, with the game's own "not drawn" byte:
  - The cut-aways close in front of the net (8538–8656 and 13908–14154): the game hides the net, and the rule says
    hide.
  - The cut-away from beside the net post (eye x = ±6.285, 1.19 m up, 0.22 m from the net plane, 60° fov; at 17983
    and again at 29135): both distance gates pass, but the third condition (net top above the view's lower line)
    fails. **The game draws the net** (byte 0), and so does the port.
  - Screenshots at that shot (`scripts/b654_grab.sh`, `context/b654/orig_post.png`) vs the port's `HST_EYE` shot of
    the same view (`context/b654/port_post.png`): the net tape and mesh cut across the lower-right corner in both.
  - The port's logs show the same post shot on court 8 (x = ±6.29), not hidden. This is the "camera close to the net,
    net still shows" case.
- I added both of the game's post views to `slot5_cutaway` as "drawn" cases.

## Your move
The original keeps the net in the post shot. To hide it anyway (or crop it), that would be our own addition outside
the original's rule. Unblock with the choice:
- keep it as the original (close this);
- hide the net in post-point cut-aways whenever the eye is within 2 m of the net plane, dropping the pitch condition
  in a cut-away only. This would change presentation only; the sim and replays are untouched.

## Not verified / not 1:1
- Only slot 5 (court 10, doubles) was checked against the game's byte. Other courts and singles were checked only
  in the port. Their nets use the same rule.
- If the user saw a different shot (not the net-post one), I didn't reproduce it. Every cut-away in the port's runs
  (courts 1, 3, 8 and 10; singles and doubles) behaved as the rule says.
- Court mods' own nets aren't tagged for the hide test (mod content, outside the original).

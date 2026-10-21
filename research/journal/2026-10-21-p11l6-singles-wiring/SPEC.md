# P11l6 spec: the singles AI's wiring (shot target, the hit's shot memory, message order, under_min)

This is the clean-room spec. It describes the original's behaviour in words. The port session works from this and the
recordings.

## Scope

In a singles match the computer player's receive and rally frames go through `doubles_ai::step`. The routines it
calls (the singles receive, NET and BASE) are ported and replay bit for bit from recorded inputs (P11l1–P11l5). This
task fixes four inputs `step` feeds them, or the order it feeds them in:

1. the player's **own shot target**;
2. the **spot** in the opponent's shot memory;
3. what an opponent's **hit** does to the AI (the app also restarts the routine's state then, and draws for it);
4. the **under_min** height in the AI's reach.

The doubles gaps that also apply in singles belong to P11k6 and are ported there, once: the contact depth, the smash
stand offset, the path's mode and line gap, the serve's hold flag, and the stick quad left from the last frame.
The done check (a recorded singles point replayed through `step`, stick and button frame-exact) needs those as well.
So the port phase either lands after P11k6, or it checks only what this spec covers and leaves the rest of the
replay to P11k6.

## 1. The player's own shot target

Each player keeps one **shot target**: a point on the court (x, y, z, w). The singles routines read their own
player's target:
- The receive: when its stroke is over and it is dashing, it takes the target's x as its stand x.
- NET: when its stroke is over and it is dashing, same as the receive.
- NET's after-hit spot: which zone the target is in (the shot down the middle or at the opponent's zone).
- NET's dash spot: the target's x, clamped to the dash width.

They never read the opponent's target.

**What it holds:** the **aimed** point of the player's latest shot, as the shot's aim step computes it, before
anything that moves the ball off it. It is set by:
- **A rally shot** (ground stroke, volley, smash, dive): the rally aim from the stick, the base point, the
  character's angle limit, the straight-line depth and the 3 m (drop: 2 m) minimum past the net. In the app this is
  `shot::aim`'s `target`. These are **not** in it: the court-margin pull inside the lines, the timing error, the
  smash scatter, the centred-stick nudge (that is a separate output added to the timing error's side) and the
  launch itself. y and w are 0.
- **A serve**: the serve aim, i.e. the service box centre plus the stick, with a mistimed weak or underhand toss's
  smaller box. In the app this is `serve::target`'s first value. These are **not** in it: the mistimed strong toss's
  throw (`Miss`) and the serve scatter.
- (The toss also writes the target while the ball is in the air, but the serve's aim overwrites it before any AI
  routine can see it: during its own serve the AI is in its serve state, not the receive or rally. The port
  doesn't need it.)

It keeps that value until the player's next aim. Only the two aims and the toss write it; nothing clears it when a
point ends or starts. It belongs to the player, so every player keeps their own. The app's `Game::aim` is
shared (the last aim made by anyone), so it can't stand in for it.

**When it changes:** the aim is made on the shot's own frames, before the ball is launched. The routines read the
target only once the stroke is over, so the exact frame of the aim within the stroke doesn't matter to them.

**What the app does now:** it passes where the app's current flight will next land. That includes the margins and
every scatter, and it changes with the ball as it flies (so after the opponent's return it describes the opponent's
shot, not the AI's own).

**Quirk to keep:** after a serve the server's target is the serve aim. A NET or ALL server that dashes after its
serve takes its dash line from that point until it plays a rally shot. The app's serve dash (`ai_serve_dash`, from
the singles serve aim) already sets the dash's stand spot when the serve is aimed. The rally routines' later
reads of the target are what this changes.

## 2. The hit message's shot memory spot

Every shot (serve, ground stroke, volley, smash, dive) writes a **shot record** for the ball, at the moment the
hitter strikes it (the hitter's own update, when its stroke launches the ball):
- the hitter's player number;
- the kind (0 serve, 1 ground stroke, 2 volley, 3 dive, 4 smash: the app's branch codes, as `ai_heard_hit`
  passes them);
- the timing result, and two flags the timing draw uses (already in the app's timing draw);
- **the spot: the hitter's position at that moment**, as a full point (x, y, z, w = 1). y is the hitter's height
  as it is then. It is 0 on the ground. It is not 0 when the strike comes during a jump: a serve's spot has y of
  about 0.31–0.33 up, and a smash about 0.13 up. The game stores y down, as it stores every position;
- the hitter's shot target (section 1);
- the ball's velocity (already in the app's `Seen`).

An AI that hears the hit from the other side (an opponent's shot) keeps that record as "the opponent's last shot".
The singles rally aim reads its spot (x and z) as the mark (`Look::mark`, `Rally::last`). The return lane reads it
too, through the blurred zone. The routines also read the record's kind (`Rally::opp_kind`).

**What the app does now:** it uses the opponent's position at the AI's first `step` after the shot, with y flipped
up and w = 0. In the recordings that is the same x and z every time:
- the hitter doesn't move between its strike and the AI's next look;
- when the hitter updates earlier in the frame than the AI, the AI's next look is that same frame.

Even so, the faithful source is the hitter's position captured at the strike. Use it, with y and w as above, so the
AI object matches byte for byte.

## 3. What an opponent's hit does, and in which order

### The game's messages

Three messages are involved. They are sent synchronously, where the original sends them. The app already sends them
at those places (P11n1 and the existing `heard`):
- **strike**: the ball hit a background figure (a bystander), not a player's stroke;
- **new path**: the live ball touched the net or another special surface, so its path restarts;
- **hit**: a player launched a shot (any shot, the serve included).

A player's shot sends **only hit**. Strike and new path don't come with a shot.

### The singles handler, per message

- **Strike, new path:**
  - In the rally state with its substate not the swing: back to its wait substate (`set_sub(0)`). That rolls the
    return to the centre and maybe the net repick, as the wait substate's entry does.
  - In the receive state with the receive not in its swing: the receive's entry (`set_state(0)`).
  - New path on the point's first shot (the serve) sets the hold flag. That is P11k6's, and already in `heard`.
  - New path in the rally phase also clears the let-go hold (`held`).
- **Hit** (the singles one does less than the doubles one). In this order, at the moment of the hit:
  1. If the hitter is on the other side, the shot memory:
     - If the shot is a serve, the last serve moves to "serve before" and the new one becomes "last serve".
     - Then the opponent's last shot moves to "the one before" and the new record becomes the opponent's last
       shot (kind and spot as in section 2).
     - If the hitter is on its own side, the record becomes its own last shot and the "first shot" flag (the one
       that halves the stroke error) clears. The app's `ai_hit` / `first` already covers this.
     - Before anyone has hit (hitter "none"), neither branch runs.
  2. If the opponent's new shot is a **dive** and the AI isn't dashing already: a NET or ALL style dashes. It sets
     its dash flag, stand x = 0 and stand z = −(net reach) × its side (the app's `Rally::heard_hit`). This draws
     nothing.
  3. The timing draw for a hit (the app's `ai_draw` + `ai_draw_guess`, unchanged). In the timing draw, an
     own-side hit takes one off the hits-left count of the net pick (`Mind::own_hit`). An opponent's hit doesn't.

  That is all the singles hit does. In particular:
  - **no** return to the wait substate, and no receive entry, in either state, at any shot count;
  - **no** return-to-centre roll and **no** net repick (the doubles hit draws both);
  - **no** clear of the let-go hold (`held`); the doubles hit clears it. In singles only new path in the rally
    phase and the point's set-up clear it.

### Checked in the recordings

In the three main singles fixtures (`ai_rally_singles`, `_net`, `_base`), about 150 opponent hits fall between two
recorded calls of one AI. The wait substate's markers were the same on both sides of the hit every time: the fresh
flag, the chase flag and the "next" entry, including values that a re-entry would reset to −1 (21, 30, 32, 98). So
the singles hit never re-enters.

### What the app does now (to change in singles)

- `doubles_ai::heard_hit` returns "again" in singles (rally, substate not swing). It draws the early
  return-to-centre roll. Then `ai_heard_shot` draws the net repick (`Mind::rally`). In singles both must go: the
  hit makes neither draw.
- `step` treats a change in the shot count as a new path. In singles it re-enters the rally (`enter_rally`) and the
  receive (`enter_receive`) on it. In singles only strike and new path (`struck`) and a change of state (`entered`)
  may re-enter.
- `step` copies the opponent's kind and spot and runs the dive dash when it next sees the shot count change. These
  belong to the hit itself (section 2's spot, captured at the strike). Doing them at the next `step` gives the same
  result only while nothing reads them in between, and nothing does. A port may keep them in `step`, as long as the
  values are the ones from the hit. Doing them in the hit is the faithful shape.
- `heard` clears `held` on every hit. In singles it must not.

The doubles routines keep the doubles behaviour (re-entry from the second shot on, its draws, its `held` clear).
This section changes singles only.

## 4. The reach's under_min

The singles rally aim compares the contact height with three of the player's heights: the lowest scoop contact
(under_min), the ideal ground-stroke height and the ideal volley height (`Look::heights`). under_min is the
character's "lowest underhand contact" value, read from the same character table row as the other reach heights:
the TParam row's cell after the collision cell, centimetres ÷ 100 (`ReachStats::from_tparam`'s `under_min`). It
is fixed for the match. The recordings show 0.40 and 0.44 m for the two recorded
characters. The app passes 0, because `swing::Reach` doesn't carry it. The port feeds the character's value (a mod
character: its own row's value, through the same reader) into `Body::reach.under_min`.

## Verification the recordings already allow

- The `ai_rally_singles*` fixtures hold, at every receive, NET and BASE call:
  - the AI object, with the opponent's shot memory (kind and spot);
  - the player's own shot target;
  - the opponent's position;
  - the character record's reach heights, under_min included;
  - the AI generator before and after.
- A port can check, call by call:
  - that the target it keeps per player equals the recorded target. In the recordings y and w are 0, and the
    target keeps its value from one shot to the next (a drop shot's z of exactly ±2.0 shows the minimum past the
    net, before the margins);
  - that the spot equals the hitter's position at its strike, with w = 1 and the jump's y;
  - that the generator's draws between the call before an opponent's hit and the call after it are only the
    timing draw's.
- The frame-exact replay through `step` is the task's done check. See *Scope* for its dependency on P11k6.

## Open questions

none

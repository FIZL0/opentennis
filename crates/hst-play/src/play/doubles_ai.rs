//! The AI's dispatcher (`hst_sim::rally`): a computer player runs the ported receive while it receives and the NET
//! or BASE rally routine (by its style, an ALL-style player by its net pick) while it rallies, in place of the
//! stand-in intercept; doubles and singles have their own routines. The routines think y-up; the app is y-down.

use super::*;
use hst_sim::ai::{style, Guess, Phase as Ai, Play};
use hst_sim::rally::{guess, msg, phase, record, step, Body, Human, Out, CPU, PATH_MAX, PathCopy, Rally, SEEN, Shot, World};

/// The doubles AI state the players share (the path copy and the shot records) and each one's own.
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(super) struct Shared {
    /// Bumped once a tick (when a player steps a second time): the path copy's stamp; who stepped since.
    frame: i32,
    stepped: u8,
    path: PathCopy,
    seen: Vec<bool>,
    records: [Shot; 4],
    me: [Mine; 4],
    /// The human players' record keepers.
    humans: [Human; 4],
    /// The path object's claim: the player whose swing last found its contact (−1 none), and the tick it did.
    claim: (i32, i32),
    /// The match's predicted ball path (P11n1): what the AI copies, its line call and gap (B57).
    pub(super) record: hst_sim::path::Path,
    /// Each player's smash stand offset sideways (`smash_side`); worked out once, again after a snapshot load.
    #[serde(skip)]
    smash_x: Unsaved<Vec<f32>>,
    /// Each player's shot target: the aimed point of its latest shot (`aimed`), kept until its next aim.
    #[serde(default)]
    targets: [[f32; 4]; 4],
}

/// Worked out again after a snapshot load, so left out of the state text (`snapshot::state_text`) too.
#[derive(Default, Clone)]
struct Unsaved<T>(T);

impl<T> std::fmt::Debug for Unsaved<T> {
    fn fmt(&self, _: &mut std::fmt::Formatter) -> std::fmt::Result {
        Ok(())
    }
}

/// Player `i`'s smash stand offset sideways: as the original measures it at load, the smash motion's racket point
/// at its contact frame (x; its z is the reach's `smash_ahead`). A player without the motion keeps the measured
/// value of the stock smash.
fn smash_side(g: &mut Game, i: usize) -> f32 {
    if g.doubles_ai.smash_x.0.len() != g.players.len() {
        let x = |k: usize| {
            let d = g.data.get(k)?;
            let clip = d.motions.get(&(hst_sim::motion::id::SMASH as usize))?;
            Some(hst_sim::serve::serve_arm(&d.skeleton, clip, None, false).point[0])
        };
        g.doubles_ai.smash_x.0 = (0..g.players.len()).map(|k| x(k).unwrap_or(-0.067)).collect();
    }
    g.doubles_ai.smash_x.0[i]
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Mine {
    rally: Rally,
    lost: i32,
    lost_log: Vec<i32>,
    /// The AI phase and shot count it last stepped in.
    phase: Option<Ai>,
    shots: i32,
    /// The re-entry's return-to-centre draw, made in the hit message (`heard_hit`).
    early: Option<u32>,
    /// A creature struck the ball or a new path began (`msg::STRIKE`, `msg::NEW_PATH`) since its last step: back to the start of
    /// its state.
    struck: bool,
    /// Singles: the point set-up ran (cleared by the serve message).
    #[serde(default)]
    set_up: bool,
    /// Its swing turned into its miss motion since its last step (`missed`): the next step tells the partner.
    #[serde(default)]
    mark: bool,
}

/// The ball as the AI reads it from the ball object: y down, as the app (unlike the path, which is stored y up).
fn ball(g: &Game) -> [f32; 4] {
    let [x, y, z] = g.flight.ball.pos;
    [x, y, z, 1.0]
}

fn up(v: V3) -> [f32; 4] {
    [v[0], -v[1], v[2], 0.0]
}

/// Computer player `i`'s body and world as its AI reads them this frame (`mark`: its miss mark), its ball-lost log
/// taken out of `Shared` (the caller puts it back).
fn frame(g: &mut Game, i: usize, phase: u8, mark: bool) -> (Body, World) {
    let singles = g.players.len() == 2;
    let smash_x = smash_side(g, i);
    let p = g.players[i];
    let busy = p.contact.is_some() || p.pending.is_some() || p.swing.is_some() || p.whiff.is_some() || p.dive.is_some();
    // singles: no partner (its own spot), one opponent
    let mate = if singles { i } else { i ^ 2 };
    let opp = if singles { [i ^ 1; 2] } else { [(i & 1) ^ 1, ((i & 1) ^ 1) + 2] };
    let r = &g.reaches[i];
    let b = Body {
        pos: up(p.pos),
        facing: [p.body.face.dir[0], -p.body.face.dir[1], p.body.face.dir[2], p.body.face.dir[3]],
        vel: [p.body.vel[0], p.body.vel[2]],
        side: p.end,
        hand: p.hand,
        team: i as i32,
        size: p.body.size(),
        stats: p.stats,
        stamina: p.body.stamina,
        tick: p.body.stamina_tick,
        run: p.body.run,
        moving: p.body.running as u8,
        depth: r.ahead,
        smash_off: [smash_x, r.smash_ahead],
        control: CPU,
        formation: p.formation,
        swing: p.contact.map_or(-1, |c| c.frames as i32),
        stroke: busy as u8,
        lost: g.doubles_ai.me[i].lost,
        lost_log: std::mem::take(&mut g.doubles_ai.me[i].lost_log),
        reach: loco::ReachStats {
            base: r.base,
            reach: r.reach,
            stroke_height: r.stroke_height,
            volley_height: r.volley_height,
            smash: [r.smash_top, g.smash_heights[i][1], r.smash_bottom],
            under_min: g.under_min.get(i).copied().unwrap_or_default(),
            ..Default::default()
        },
        strong: if g.strong_hand[i] >= 0.0 { hst_sim::ai::strong::MINUS } else { hst_sim::ai::strong::PLUS },
        mate: up(g.players[mate].pos),
        opp: opp.map(|j| up(g.players[j].pos)),
        // the partner's movement state as it is now: its own update this frame has run only if it steps first
        mate_voice: if singles { 0 } else { foot_fx::state(g, mate) },
        mark: mark as u8,
        target: if singles { g.doubles_ai.targets[i] } else { [0.0; 4] },
        opp_lefty: g.players[opp[0]].hand < 0.0,
    };
    let mut w = World {
        frame: g.doubles_ai.frame,
        phase,
        players: 4,
        court: g.score.side,
        receiver: g.score.receiver,
        hitter: g.last_hitter,
        shots: g.shots,
        ball: ball(g),
        hits: [g.flight.bounces, g.flight.contacts],
        gravity: hst_sim::ball::Params::default().gravity,
        drag: hst_sim::ball::Params::default().drag,
        floor: 0,
        path: Vec::new(),
        path_from: None,
        records: g.doubles_ai.records,
        path_mode: 0,
        path_gap: 0.0,
    };
    // B57: a CPU lets a ball that will land out by its row's margin go by (the NET and BASE routines' hold)
    let r = &g.doubles_ai.record;
    (w.path_mode, w.path_gap) = (r.mode as u8, r.gap);
    if g.doubles_ai.path.stamp != w.frame {
        w.path = r.window().to_vec();
    }
    (b, w)
}

/// One frame of computer player `i` in the receive or rally: false (nothing done) outside them and between ends.
pub(super) fn step(g: &mut Game, i: usize, mind: &hst_sim::ai::Mind) -> bool {
    let phase = match g.phase {
        Phase::Serve => phase::SERVE,
        Phase::Rally => phase::RALLY,
        Phase::Post => phase::POINT_OVER,
        Phase::ChangeEnds(_) => return false,
    };
    let singles = g.players.len() == 2;
    if !matches!(g.players.len(), 2 | 4) || !matches!(mind.phase, Ai::Receive | Ai::Rally) {
        return false;
    }
    let d = &mut g.doubles_ai;
    if d.stepped & 1 << i != 0 {
        (d.frame, d.stepped) = (d.frame + 1, 0);
    }
    d.stepped |= 1 << i;
    // the miss mark lasts until the stroke update after this step: a miss from here on is seen next frame
    let mark = std::mem::take(&mut d.me[i].mark);
    // the stand-in's preamble: the recovery, the whiff, the press's approach run
    follow_through(&mut g.players[i], true);
    whiff_frame(g, i, true);
    if let Some(dir) = g.players[i].approach {
        locomote(g, i, dir);
    }
    let (mut b, mut w) = frame(g, i, phase, mark);
    let p = g.players[i];
    let busy = p.contact.is_some() || p.pending.is_some() || p.swing.is_some() || p.whiff.is_some() || p.dive.is_some();
    let mate = if singles { i } else { i ^ 2 };
    let human_mate = !singles && g.humans.get(mate) == Some(&true);
    let row = p.ai;
    let me = &mut g.doubles_ai.me[i];
    if singles && !std::mem::replace(&mut me.set_up, true) {
        // B15: the singles AI's point set-up: the spot it walks back to (the middle, 10 back for the NET style, else
        // 11), no dash, no short ball (hold and held clear on the serve message, `heard`)
        let z = if row.style == style::NET { -10.0 } else { -11.0 };
        (me.rally.spot[0], me.rally.spot[2]) = (0.0, hst_sim::ps2::mul(p.end, z));
        (me.rally.dash, me.rally.short) = (false, false);
    }
    let x = &mut me.rally;
    (x.level, x.mate, x.first, x.window, x.singles) = (3, human_mate as i32, 2, PATH_MAX as i32, singles);
    // B15: the net reach (the dashes' depth) its level gives it; unset it was 0 and every dash ran into the net
    x.reach = hst_sim::ai::Choice::new(0, 0, false).reach;
    (x.rate, x.radius) = if singles {
        (row.singles_center_rate, row.singles_center_radius)
    } else {
        (row.doubles_center_rate, row.doubles_center_radius)
    };
    x.high_level = p.ai_picks.high_level;
    x.lean = hst_sim::position::Team::lean(b.formation, !human_mate, row.style, g.players[mate].ai.style);
    let t = p.ai_timing;
    x.leads = [t.stroke, t.volley, t.smash];
    (x.dive, x.body, x.low, x.volley_level) =
        (p.ai_picks.dive == Some(true), p.ai_picks.body, p.ai_picks.low, p.ai_picks.volley_level);
    // the app's Mind keeps the net pick (its point result, its repick on each hit); the routines' own repicks write back
    (x.mind.net, x.mind.net_rate, x.mind.net_left) = (mind.net, mind.net_rate, mind.net_left);
    let (entered, new_shot) = (me.phase != Some(mind.phase), me.shots != g.shots);
    (me.phase, me.shots) = (Some(mind.phase), g.shots);
    let struck = std::mem::take(&mut me.struck);
    if new_shot {
        // the opponents' hit lets it call again on giving way (as the point's set-up does)
        x.voice |= g.last_hitter >= 0 && g.last_hitter as usize & 1 != i & 1;
        // the hit message's timing draw: the reaction (or the guess walk) and the guess
        x.wait = p.ai_hold;
        (x.guess, x.from) = match p.ai_guess {
            Some((Guess::Wide, at)) => (guess::WIDE, at),
            Some((Guess::Other, at)) => (guess::OTHER, at),
            None => (guess::NONE, x.from),
        };
    }
    let mut early = me.early.take();
    let rng = &mut g.rng.ai;
    let mut roll = || early.take().unwrap_or_else(|| rng.next());
    // the dispatcher's state setter on entering a state, and the strike/hit messages: back to the start of its
    // state (the app already drew the net repick those messages make, so theirs is dropped)
    let kept = x.mind;
    match mind.phase {
        // doubles re-enters at the strike, hit and new-path messages themselves (`heard_hit`, `heard`); singles
        // re-enters only on the strike and the new path, never on the hit
        Ai::Receive if entered || singles && struck && x.state != step::SWING => x.enter_receive(&mut roll),
        Ai::Rally if entered || singles && struck && x.sub != step::SWING => {
            x.enter_rally(&b, &mut w, &mut roll);
            x.mind = kept;
        }
        _ => {}
    }
    // ponytail: the caller's stick quad starts zeroed each frame
    let mut out = Out::default();
    let seen = &mut g.doubles_ai.seen;
    seen.resize(SEEN, false);
    let c = &mut g.doubles_ai.path;
    let mut received = false;
    if mind.phase == Ai::Receive {
        received = x.receive(&row, &mut b, &w, c, seen, &mut out, &mut roll);
        if received && !singles {
            // P11k6: doubles goes over to the rally in the same frame, on the routine's own word
            x.enter_rally(&b, &mut w, &mut roll);
            me.phase = Some(Ai::Rally);
        }
    } else {
        let net = x.mind.play(row.style) == Play::Net;
        if singles {
            x.singles_rally(net, &row, &mut b, &w, c, seen, &mut out, &mut roll);
        } else {
            x.rally(net, &row, &mut b, &mut w, c, seen, &mut out, &mut roll);
        }
    }
    if singles {
        // the singles dispatcher's kind lock on the routine's button (its stick lock waits for the contact,
        // `ai_contact_stick`)
        out.button = out.button.map(|bt| row.lock_button(bt as u8, &mut roll) as u32);
    }
    if let Some(bit) = x.said.take() {
        g.whooshes.push((0, i, hst_sim::sound::call_out(i, bit)));
    }
    let x_kind = x.kind;
    let (net, net_rate, net_left, aim) = (x.mind.net, x.mind.net_rate, x.mind.net_left, x.stick);
    (me.lost, me.lost_log) = (b.lost, b.lost_log);
    g.doubles_ai.records = w.records;
    let mut m = *mind;
    (m.net, m.net_rate, m.net_left) = (net, net_rate, net_left);
    if received && singles {
        m.rally(&mut ai_roll(&mut g.rng));
    } else if received {
        m.phase = Ai::Rally;
    }
    g.players[i].ai_mind = Some(m);
    // the routine's dive press lets go of the stick: the dive starts from the run it was judged on (the stroke state
    // takes over from the run), not from a body this frame's stand already stopped
    let dives = out.button.is_some() && x_kind == hst_sim::aim::contact::DIVE;
    if !busy && !dives {
        let dir = if mind.active { [out.stick[0], out.stick[2]] } else { [0.0; 2] };
        let [x, z] = loco::stick_dir(loco::bot_stick(dir), phase);
        // past its wait the stick breaks off a serve's follow-through (`ai_serve_step`)
        if ai_serve_step(g, i, Vec2::new(x, z)) {
            return true;
        }
        locomote(g, i, Vec2::new(x, z));
    }
    if let Some(button) = out.button {
        g.players[i].ai_press = Some((Vec2::new(aim[0], aim[2]), button as u8));
        press(g, i, button_kind(button as u8));
    }
    advance_stroke(g, i, |g| ai_contact_stick(g, i));
    true
}

/// The hit message to computer player `i` from hitter `who`. Doubles (P11k6): the re-entry and its own team's
/// formation repick happen here (`Rally::heard_hit_doubles`). Singles doesn't re-enter on the hit (nor draw the roll
/// or the net repick). Whether `step` re-enters: always false (the net repick is the doubles re-entry's own).
pub(super) fn heard_hit(g: &mut Game, i: usize, who: usize) -> bool {
    let mind = g.players[i].ai_mind;
    let rally = mind.is_some_and(|m| m.phase == Ai::Rally);
    if g.players.len() == 4 {
        // the hit message reaches the AI only while it plays the point
        if mind.is_some_and(|m| matches!(m.phase, Ai::Receive | Ai::Rally)) {
            let (shots, at) = (g.shots, up(g.players[who].pos));
            at_message(g, i, |x, b, w, roll| x.heard_hit_doubles(rally, shots, b, w, Some((who as i32, at)), &mut || roll()));
        }
    }
    false
}

/// Doubles: a message's work on computer player `i`'s AI routine (`f`), at the message, with its body and world as
/// they are now and the app's net pick in; the net pick and the shot records it writes go back.
fn at_message(g: &mut Game, i: usize, f: impl FnOnce(&mut Rally, &Body, &mut World, &mut dyn FnMut() -> u32)) {
    let phase = match g.phase {
        Phase::Serve => phase::SERVE,
        Phase::Rally => phase::RALLY,
        Phase::Post => phase::POINT_OVER,
        Phase::ChangeEnds(_) => return,
    };
    let Some(mut m) = g.players[i].ai_mind else { return };
    let mark = g.doubles_ai.me[i].mark;
    let (b, mut w) = frame(g, i, phase, mark);
    let x = &mut g.doubles_ai.me[i].rally;
    (x.mind.net, x.mind.net_rate, x.mind.net_left) = (m.net, m.net_rate, m.net_left);
    let rng = &mut g.rng.ai;
    f(x, &b, &mut w, &mut || rng.next());
    (m.net, m.net_rate, m.net_left) = (x.mind.net, x.mind.net_rate, x.mind.net_left);
    g.players[i].ai_mind = Some(m);
    g.doubles_ai.me[i].lost_log = b.lost_log;
    g.doubles_ai.records = w.records;
}

/// Player `i` aimed its shot (the rally aim's or the serve aim's point, before the margins, the scatter and the
/// launch): its shot target from now until its next aim, as the singles routines read it (`Body::target`).
pub(super) fn aimed(g: &mut Game, i: usize, target: V3) {
    if let Some(t) = g.doubles_ai.targets.get_mut(i) {
        *t = [target[0], target[1], target[2], 0.0];
    }
}

/// The singles hit message to each computer player on an opponent's shot, at the strike: the shot's kind and spot
/// (the hitter's position now, y down, w 1) become its opponent's last shot, and a dive sends a NET or ALL style
/// that isn't dashing to the net.
fn singles_hit(g: &mut Game) {
    let Ok(who) = usize::try_from(g.last_hitter) else { return };
    if g.players.len() != 2 || who >= 2 {
        return;
    }
    let [x, y, z] = g.players[who].pos;
    for i in 0..2 {
        let p = &g.players[i];
        if i & 1 == who & 1 || g.humans.get(i) == Some(&true) {
            continue;
        }
        let Some(seen) = p.ai_seen[0] else { continue };
        let b = Body { side: p.end, ..Default::default() };
        let style = p.ai.style;
        let r = &mut g.doubles_ai.me[i].rally;
        (r.opp_kind, r.last) = (seen.kind, [x, y, z, 1.0]);
        r.heard_hit(seen.kind, style, &b);
    }
}

/// A match message to the AI side (`msg`: `rally::msg::*`), sent where the original sends it, in its order within
/// the tick.
pub(super) fn heard(g: &mut Game, msg: u8) {
    let r = &mut g.doubles_ai.record;
    match msg {
        msg::NEW_PATH => r.new_path(),
        msg::HIT => {
            // the predictor takes the ball just launched; from the serve on, the path is built now
            r.hit(&g.flight, &g.shot, &COURTS[g.court], g.phase == Phase::Rally, !matches!(g.phase, Phase::ChangeEnds(_)));
            singles_hit(g);
        }
        msg::STRIKE | msg::REACTION => r.stop(),
        msg::SERVE | msg::CHANGE_ENDS => r.point(),
        _ => {}
    }
    let records = msg != msg::HIT || !matches!(g.phase, Phase::ChangeEnds(_));
    g.doubles_ai.hear(&g.players, msg, records);
    for m in &mut g.doubles_ai.me {
        let x = &mut m.rally;
        match msg {
            // a serve that touched the net: the AI stands still for the rest of it
            msg::NEW_PATH if g.shots == 1 => x.hold = true,
            // ponytail: the original clears both as it sets the point up; the serve message comes there
            msg::SERVE => (x.hold, x.held, x.voice, m.set_up, m.mark) = (false, false, true, false, false),
            _ => {}
        }
        // the singles hit leaves the let-go hold alone
        if msg == msg::HIT && g.players.len() == 4 || msg == msg::NEW_PATH && g.phase == Phase::Rally {
            x.held = false;
        }
    }
    if matches!(msg, msg::STRIKE | msg::NEW_PATH) {
        struck(g);
    }
}

/// Doubles (P11k6): the strike and new-path messages send each computer player in its receive or rally back to the
/// start of its state at the message, unless it swings (`Rally::heard_strike`).
pub(super) fn struck(g: &mut Game) {
    if g.players.len() != 4 {
        return;
    }
    for i in 0..4 {
        let Some(phase) = g.players[i].ai_mind.map(|m| m.phase) else { continue };
        if g.humans.get(i) != Some(&true) && matches!(phase, Ai::Receive | Ai::Rally) {
            at_message(g, i, |x, b, w, roll| x.heard_strike(phase == Ai::Receive, b, w, &mut || roll()));
        }
    }
}

impl Shared {
    /// Player `i`'s swing turned into its miss motion (a cancelled lock at its contact, a press that found nothing
    /// at its pose): its next step writes its own shot record as missed, so a partner following it takes over.
    pub(super) fn missed(&mut self, i: usize) {
        self.me[i].mark = true;
    }

    /// A new contact search or serve swing starts for player `i`: no miss to tell.
    pub(super) fn unmark(&mut self, i: usize) {
        self.me[i].mark = false;
    }

    /// `heard`: the path object drops its claim (not on the reaction and the message after it) and, when `records`
    /// (not on a hit before the serve), every shot record (`record::CLEARED`); on the strike, hit, new path and serve
    /// each AI object's base handler sends its human search back to the path's start (by `players`' z) and marks the
    /// shared path copy stale.
    pub(super) fn hear(&mut self, players: &[Player], msg: u8, records: bool) {
        if msg != msg::REACTION && msg != msg::REACTION + 1 {
            self.claim.0 = -1;
        }
        if records {
            self.records.iter_mut().for_each(|r| r.n = record::CLEARED);
        }
        if matches!(msg, msg::STRIKE | msg::NEW_PATH) {
            // only an AI in its receive or rally hears it
            for (m, p) in self.me.iter_mut().zip(players) {
                // doubles re-enters at the message itself (`struck`)
                m.struck = players.len() == 2 && p.ai_mind.is_some_and(|x| matches!(x.phase, Ai::Receive | Ai::Rally));
            }
        }
        if matches!(msg, msg::STRIKE | msg::HIT | msg::NEW_PATH | msg::SERVE) {
            self.path = PathCopy { stamp: -1, balls: Vec::new() };
            for (h, p) in self.humans.iter_mut().zip(players) {
                h.reset(p.pos[2]);
            }
        }
    }
}

/// The path's once-a-tick update (P11n1), after the ball's step and before anyone thinks, as the original's.
pub(super) fn path_frame(g: &mut Game) {
    let swinging = (g.players.len() == 4).then(|| g.players.iter().any(|p| p.swing.is_some()));
    g.doubles_ai.record.frame(&g.shot, &COURTS[g.court], g.phase == Phase::Rally, swinging);
}

/// The live ball touched the net (or another special material) on its step: the path starts again from it, keeping
/// the old path's counts.
pub(super) fn net_touch(g: &mut Game) {
    g.doubles_ai.record.net(&g.flight);
    heard(g, msg::NEW_PATH);
}

/// Player `i`'s swing found its contact (the stroke, volley or smash search; not a dive): it claims the ball unless
/// another player claimed it earlier this tick.
pub(super) fn claim(g: &mut Game, i: usize) {
    let d = &mut g.doubles_ai;
    if d.claim.0 < 0 || d.claim.1 < d.frame {
        d.claim = (i as i32, d.frame);
    }
}

/// The partner a stroking player `i` is kept 1 m from: its own, in a four-player match, unless `i` holds the claim
/// (the claimant swings where it stands; the partner is the one pushed away).
pub(super) fn stroke_mate(g: &Game, i: usize) -> Option<V3> {
    (g.players.len() == 4 && g.doubles_ai.claim.0 != i as i32).then(|| g.players[i ^ 2].pos)
}

/// The stroke state's own per-frame position check, ahead of its countdown: a stroking player's position goes
/// through the mover with no step (kept on court, 1 m from the partner unless it holds the claim). A non-claimant
/// also loses the contact step's free move.
pub(super) fn stroke_check(g: &mut Game, i: usize) {
    if foot_fx::state(g, i) != 2 {
        return;
    }
    let mate = stroke_mate(g, i);
    let claimant = g.doubles_ai.claim.0 == i as i32;
    let p = &mut g.players[i];
    if !claimant && let Some(ik) = p.ik.as_mut() {
        ik.free = false;
    }
    p.pos = loco::mover(p.pos, [0.0; 3], p.end, mate, false);
}

/// A human player `i` in doubles: its AI object keeps its shot record (where it will play the opponents' shot), so
/// a computer partner gives way to it or follows it.
pub(super) fn human(g: &mut Game, i: usize) {
    let phase = match g.phase {
        Phase::Serve => phase::SERVE,
        Phase::Rally => phase::RALLY,
        Phase::Post => phase::POINT_OVER,
        Phase::ChangeEnds(_) => return,
    };
    // not while it serves (to its serve's end) or reacts to the point
    let serving = i as i32 == g.score.server && (g.phase == Phase::Serve || g.players[i].served.is_some());
    if g.players.len() != 4 || serving || input_off(g) {
        return;
    }
    let d = &mut g.doubles_ai;
    if d.stepped & 1 << i != 0 {
        (d.frame, d.stepped) = (d.frame + 1, 0);
    }
    d.stepped |= 1 << i;
    let smash_x = smash_side(g, i);
    let p = g.players[i];
    let r = &g.reaches[i];
    let b = Body {
        pos: up(p.pos),
        facing: [p.body.face.dir[0], -p.body.face.dir[1], p.body.face.dir[2], p.body.face.dir[3]],
        vel: [p.body.vel[0], p.body.vel[2]],
        side: p.end,
        hand: p.hand,
        team: i as i32,
        size: p.body.size(),
        stats: p.stats,
        stamina: p.body.stamina,
        tick: p.body.stamina_tick,
        run: p.body.run,
        moving: p.body.running as u8,
        depth: r.ahead,
        smash_off: [smash_x, r.smash_ahead],
        control: CPU,
        reach: loco::ReachStats {
            base: r.base,
            reach: r.reach,
            stroke_height: r.stroke_height,
            volley_height: r.volley_height,
            smash: [r.smash_top, g.smash_heights[i][1], r.smash_bottom],
            ..Default::default()
        },
        strong: if g.strong_hand[i] >= 0.0 { hst_sim::ai::strong::MINUS } else { hst_sim::ai::strong::PLUS },
        mate: up(g.players[i ^ 2].pos),
        ..Default::default()
    };
    let mut w = World {
        frame: g.doubles_ai.frame,
        phase,
        players: 4,
        court: g.score.side,
        receiver: g.score.receiver,
        hitter: g.last_hitter,
        shots: g.shots,
        ball: ball(g),
        hits: [g.flight.bounces, g.flight.contacts],
        gravity: hst_sim::ball::Params::default().gravity,
        drag: hst_sim::ball::Params::default().drag,
        records: g.doubles_ai.records,
        ..Default::default()
    };
    if g.doubles_ai.path.stamp != w.frame {
        w.path = g.doubles_ai.record.window().to_vec();
    }
    let row = p.ai;
    let d = &mut g.doubles_ai;
    d.seen.resize(SEEN, false);
    let x = &mut d.me[i].rally;
    // its AI object beside a human: level 3, `mate` 2
    (x.level, x.mate, x.first, x.window, x.singles) = (3, 2, 2, PATH_MAX as i32, false);
    let rng = &mut g.rng.ai;
    let claimed = d.claim.0 == i as i32;
    x.human_record(&mut d.humans[i], &row, &b, &mut w, &mut d.path, &mut d.seen, claimed, &mut || rng.next());
    d.records = w.records;
}

/// The match's first point: each doubles player's formation from its placement draw (`Team::pick`; players 2 and 3
/// take their partner's), and everyone placed again for the serve with it (the set-up placed them before the rows
/// were loaded).
pub(super) fn formations(g: &mut Game, draws: &[u32]) {
    use hst_sim::position::Team;
    if g.players.len() != 4 {
        return;
    }
    let human = |g: &Game, k: usize| g.humans.get(k) == Some(&true);
    for (i, &draw) in draws.iter().enumerate().take(4) {
        let m = i ^ 2;
        g.players[i].formation = if i < 2 {
            let rows = (g.players[i].ai.formation, g.players[m].ai.formation);
            // the menus' setup words have no third byte (slots 3 and 5 read 0)
            Team::pick((human(g, i), human(g, m)), rows, 0, draw)
        } else {
            g.players[m].formation
        };
    }
    for i in 0..4 {
        let p = &g.players[i];
        let at = serve_placement(i as i32, &g.score, g.rally.faults, p.stance, p.formation, g.score.swapped);
        let p = &mut g.players[i];
        (p.pos, p.prev, p.home, p.body.pos) = (at.pos, at.pos, at.pos, at.pos);
    }
}

#[cfg(test)]
mod handover;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Args, GameSpace};

    /// B57: a rally ball landing 1 m past the baseline gives the path mode 2 (out) and gap 1 m; one landing in, mode
    /// 1, and the gap stays from the path before.
    #[test]
    fn path_mode_and_gap() {
        let id = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
        let lines = hst_sim::judge::Lines { shots: 3, doubles: true, side: 0, hitter_far: false, margin: 0.0 };
        let shot = hst_sim::ball::Shot { params: hst_sim::ball::Params::default(), ..Default::default() };
        let mut r = hst_sim::path::Path::default();
        let mut landing = |vz: f32| {
            let mut f = Flight::new(Ball { pos: [0.0, -1.0, 0.5], vel: [0.0, 0.0, vz], spin: 0.0 }, id, id);
            f.lines = Some(lines);
            r.hit(&f, &shot, &COURTS[1], true, true);
            let first = loop {
                r.frame(&shot, &COURTS[1], true, Some(false));
                if r.predictor.contacts > 0 {
                    break r.predictor.landing.0[2];
                }
            };
            (r.mode as u8, r.gap, first)
        };
        let (mode, out, z) = landing(0.55);
        assert!(z > 11.885, "{z}");
        assert_eq!(mode, 2);
        assert!((out - (z - 11.885)).abs() < 0.05, "{out} {z}");
        let (mode, gap, z) = landing(0.3);
        assert!(z < 11.885, "{z}");
        assert_eq!((mode, gap), (1, out));
    }

    const ISO: &str = hst_data::fixture::ISO_PATH;

    /// B15: singles bots get the point set-up's walk-back spot (the middle, 10 or 11 back) and their level's net reach
    /// (3.2), so after their own shot they go back to the baseline instead of running into the net, where the
    /// opponent's next ball went past them.
    #[test]
    fn singles_bots_go_back_to_the_baseline() {
        if !hst_data::fixture::has_iso() {
            return;
        }
        // every player a bot
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 1, stage: None, play: true, singles: true, chars: vec![0, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, control, simulate, motions, character::tick, played_out, held_ball).chain());
        app.update();
        let (mut stepped, mut pinned, mut rallies) = (0, 0, 0);
        for _ in 0..60 * 60 {
            app.world_mut().run_schedule(FixedUpdate);
            let g = app.world().resource::<Game>();
            if g.phase != Phase::Rally {
                continue;
            }
            rallies += 1;
            for (i, p) in g.players.iter().enumerate() {
                let me = &g.doubles_ai.me[i];
                if me.set_up {
                    stepped += 1;
                    let z = if p.ai.style == style::NET { -10.0 } else { -11.0 };
                    assert_eq!((me.rally.spot[0], me.rally.spot[2]), (0.0, p.end * z), "p{i}");
                    assert_eq!(me.rally.reach, 3.2, "p{i}");
                }
                // its own ball on the way: never up against the net (the court stops a player 1.5 from it)
                if g.last_hitter == i as i32 && p.pos[2].abs() < 1.6 {
                    pinned += 1;
                }
            }
        }
        assert!(rallies > 0 && stepped > 0, "{rallies} {stepped}");
        assert_eq!(pinned, 0);
    }

    /// P11l6: the singles AI's wiring. Each player's shot target is its own latest aim (y and w 0), kept through the
    /// opponent's shot; an opponent's hit leaves its kind and the hitter's position at the strike (w 1) as the AI's
    /// mark and doesn't re-enter the routine (no early roll); the reach carries the character's TParam under_min.
    #[test]
    fn singles_wiring() {
        if !hst_data::fixture::has_iso() {
            return;
        }
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 1, stage: None, play: true, singles: true, chars: vec![0, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, control, simulate, motions, character::tick, played_out, held_ball).chain());
        app.update();
        {
            let g = app.world().resource::<Game>();
            let mut iso = hst_data::fixture::iso().unwrap();
            for (i, &c) in g.chars.iter().enumerate() {
                let want = loco::ReachStats::from_tparam(&tparam(&mut iso, c as usize).join(",")).under_min;
                assert!(want > 0.3 && want < 0.6, "p{i} {want}");
                assert_eq!(g.under_min[i], want, "p{i}");
            }
        }
        let (mut shots, mut heard, mut kept) = (0, 0, 0);
        for _ in 0..60 * 40 {
            let before = app.world().resource::<Game>().doubles_ai.targets;
            app.world_mut().run_schedule(FixedUpdate);
            let g = app.world().resource::<Game>();
            assert!(g.doubles_ai.me.iter().all(|m| m.early.is_none()), "a singles hit drew the re-entry's roll");
            if g.shots == shots {
                continue;
            }
            shots = g.shots;
            if g.shots == 0 || g.last_hitter < 0 {
                continue;
            }
            let who = g.last_hitter as usize;
            let t = g.doubles_ai.targets[who];
            assert_eq!((t[1], t[3]), (0.0, 0.0), "p{who} target {t:?}");
            // the other player's target is its own, unchanged by this shot
            assert_eq!(g.doubles_ai.targets[who ^ 1], before[who ^ 1]);
            kept += (before[who ^ 1] != [0.0; 4]) as i32;
            let r = &g.doubles_ai.me[who ^ 1].rally;
            if let Some(seen) = g.players[who ^ 1].ai_seen[0] {
                let p = g.players[who].pos;
                assert_eq!((r.opp_kind, r.last[3]), (seen.kind, 1.0));
                assert!((r.last[0] - p[0]).abs() < 0.2 && (r.last[2] - p[2]).abs() < 0.2, "{:?} {p:?}", r.last);
                heard += 1;
            }
        }
        assert!(heard > 5 && kept > 5, "{heard} {kept}");
    }

    /// A singles bot's dive press dives: the routine lets go of the stick as it presses, and before the press kept
    /// the run, that frame's stand stopped the body and the dive search (a running player's) found nothing, so every
    /// dive the AI picked was an air swing.
    #[test]
    fn bots_dive() {
        if !hst_data::fixture::has_iso() {
            return;
        }
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, control, simulate, motions, character::tick, played_out, held_ball).chain());
        app.update();
        let dove = (0..60 * 60 * 4).position(|_| {
            app.world_mut().run_schedule(FixedUpdate);
            app.world().resource::<Game>().players.iter().any(|p| p.dive.is_some())
        });
        eprintln!("first dive at frame {dove:?}");
        assert!(dove.is_some(), "no bot dove in 4 minutes");
    }

    /// B574: each player's smash stand offset sideways is its own, bit for bit as the original measures it at load
    /// (slot 3's characters, read from each player object there); the AI used one character's for everyone.
    #[test]
    fn smash_side_per_character() {
        if !hst_data::fixture::has_iso() {
            return;
        }
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 4, stage: None, play: true, singles: false, chars: vec![6, 4, 3, 11], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 6, pads: None });
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.update();
        let g = &mut *app.world_mut().resource_mut::<Game>();
        let x: Vec<u32> = (0..4).map(|i| smash_side(g, i).to_bits()).collect();
        assert_eq!(x, [0xbd75_cdd6, 0xbd89_49e8, 0xbdc8_5d9e, 0xbd03_01a8]);
    }

    /// P111 probe: 1P (human) rushes the net as `tools/record_ai_rally.py`'s `HST_DRIVE=1` does (holds 4 m off the
    /// net, steers to a ball coming its way, presses ○ once per ball within 2.5 m, serving presses ○ now and then);
    /// prints each of the bot's presses (routine, plan, contact kind, contact height over its stroke height, 1P's z).
    /// `P111_CHARS` (2 singles, 4 doubles; default 0,1), `P111_NET` (m off the net, 4), `P111_TICKS` (5000);
    /// `tools/check.sh -p hst-play net_rush_probe -- --ignored --nocapture` (journal 2026-10-20-p111-net-rush)
    #[test]
    #[ignore]
    fn net_rush_probe() {
        use crate::play::tick_input::{IDLE, Replay, TickPads};
        use hst_sim::replay::Pad;
        if !hst_data::fixture::has_iso() {
            return;
        }
        let chars: Vec<usize> = std::env::var("P111_CHARS").map_or(vec![0, 1], |s| s.split(',').map(|c| c.parse().unwrap()).collect());
        let singles = chars.len() == 2;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles, chars, outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 6, pads: None });
        app.insert_resource(Pads { seats: Some([true, false, false, false]), autoplay: false, ..default() });
        app.insert_resource(Replay(Default::default()));
        app.init_resource::<TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, crate::play::tick_input::systems(), control, simulate, motions, character::tick, played_out, held_ball).chain());
        app.update();
        let n = std::env::var("P111_TICKS").map_or(5000, |s| s.parse().unwrap());
        let net: f32 = std::env::var("P111_NET").map_or(4.0, |s| s.parse().unwrap());
        let (mut pressed_for, mut idle, mut hold) = (-1, 0, 0);
        let mut was = [0u8; 4];
        let (mut zs, mut lobs, mut near) = (vec![], 0, 0);
        for _ in 0..n {
            let g = app.world().resource::<Game>();
            let me = g.players[0].pos;
            let [bx, _, bz] = g.flight.ball.pos;
            let vz = g.flight.ball.vel[2];
            let side = if me[2] > 0.0 { 1.0 } else { -1.0 };
            let coming = vz * side > 0.0;
            let serving = g.phase == Phase::Serve && g.score.server == 0;
            let (tx, tz) = if serving { (me[0], me[2]) } else if coming { (bx, if bz * side > 0.0 { bz } else { net * side }) } else { (0.0, net * side) };
            let (dx, dz) = (tx - me[0], tz - me[2]);
            let d = (dx * dx + dz * dz).sqrt();
            let (sx, sy) = if d > 0.4 { (-side * dx / d, side * dz / d) } else { (0.0, 0.0) };
            let byte = |v: f32| (128.0 + (v * 10.0).round() / 10.0 * 127.0).clamp(0.0, 255.0) as u8;
            let mut pad = Pad { lx: byte(sx), ly: byte(sy), ..IDLE };
            idle += 1;
            if coming && pressed_for != g.shots && bz * side > 0.0 && ((bx - me[0]).powi(2) + (bz - me[2]).powi(2)).sqrt() < 2.5 {
                (pressed_for, idle, hold) = (g.shots, 0, 3);
            } else if idle > if serving { 50 } else { 120 } {
                (idle, hold) = (0, 3);
            }
            if hold > 0 {
                hold -= 1;
                pad.buttons = Pad::CIRCLE;
            }
            if g.phase == Phase::Rally {
                zs.push(me[2].abs());
            }
            app.world_mut().resource_mut::<Replay>().0.push_back([pad, IDLE, IDLE, IDLE]);
            app.world_mut().run_schedule(FixedUpdate);
            let g = app.world().resource::<Game>();
            #[allow(clippy::needless_range_loop)] // i indexes players, doubles_ai.me and was together
            for i in 1..g.players.len() {
                let x = &g.doubles_ai.me[i].rally;
                let s = if g.players[i].ai_mind.is_some_and(|m| m.phase == Ai::Receive) { x.state } else { x.sub };
                if s == step::SWING && was[i] != step::SWING && i & 1 == 1 {
                    let oz = g.players[0].pos[2];
                    let lob = matches!(x.plan, hst_sim::aim::plan::LOB | hst_sim::aim::plan::VOLLEY_LOB);
                    if oz.abs() < 4.1 {
                        near += 1;
                        lobs += lob as i32;
                    }
                    eprintln!("p{i} {:?} shots {} plan {} kind {} over {:.2} kind_now {} opp z {:.2} me z {:.2}", g.players[i].ai_mind.map(|m| m.phase), g.shots, x.plan, x.kind, x.ball[1] - g.reaches[i].stroke_height, g.players[i].kind, oz, g.players[i].pos[2]);
                }
                was[i] = s;
            }
        }
        zs.sort_by(f32::total_cmp);
        eprintln!("1P |z| in rallies: median {:.2}; bot lobs with 1P inside 4.1: {lobs}/{near}", zs.get(zs.len() / 2).copied().unwrap_or(0.0));
    }
}

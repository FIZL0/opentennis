//! Right-stick emotes (B145, ours, layered on top): a human flicks the right stick fully up / down / left / right
//! (keys 1–4) and their player plays one of four of the character's own motions with its voice line, standing: no
//! run, no stroke and no swing press until the motion is out. A ball that comes meanwhile goes by unhit — that's the
//! cost. In live play only: in a rally, waiting for a serve and serving before the toss, not mid-stroke. Also in the
//! gap after a call (a point's, a fault's or a let's), while the player can still move, until the players react
//! (B591); never from the reaction to the next serve, nor in the changes of ends (B472). One started in the rally
//! or the gap plays on until it's out, the point's reaction ends it (B278) or the players are placed for the next serve, and not at all once the point has won
//! a game or set (B363). The flick is part of the match input (`tick_input`'s right stick bytes), so replays and online play see it alike; bots never emote. Nothing here draws from the match's generators.

use bevy::prelude::*;
use hst_sim::score::Event;
use hst_sim::sound;

use super::{Game, Phase, controls::Action, set_motion};

/// The emotes by flick (up, down, left, right): the character's post-point reactions `gu_set` (the big win),
/// `di_set` (the big loss), `gu` (a won point), `di` (a lost one) — a mod character's from its pack.
pub const MOTIONS: [usize; 4] = crate::mods_online::EMOTES;

/// Keys 1–4 by default, in flick order.
pub const KEYS: [Action; 4] = [Action::EmoteUp, Action::EmoteDown, Action::EmoteLeft, Action::EmoteRight];

/// A right stick (`Gamepad::right_stick`, +y up) pushed fully toward one side: its emote direction.
pub fn flick(v: Vec2) -> Option<u8> {
    // ponytail: "fully" is 0.9 of the way out, so worn sticks still reach it
    (v.length() >= 0.9).then(|| match v.y.abs() > v.x.abs() {
        true if v.y > 0.0 => 0,
        true => 1,
        false if v.x < 0.0 => 2,
        false => 3,
    })
}

/// B271: the browser's touch pad (web/touch.js) has no right stick, so its L1 / R1 stand in for the left / right
/// flick, on press (held, it doesn't repeat). Only that pad: a real pad's shoulders never emote.
pub fn touch(g: &Gamepad, name: Option<&Name>) -> Option<u8> {
    name.is_some_and(|n| is_touch(n))
        .then(|| [(GamepadButton::LeftTrigger, 2), (GamepadButton::RightTrigger, 3)].into_iter().find(|(b, _)| g.just_pressed(*b)))
        .flatten()
        .map(|(_, d)| d)
}

/// A pad button bound to an emote in `controls.txt` (none by default), just pressed: its direction (held, it doesn't repeat).
pub fn bound(g: &Gamepad, bind: &super::controls::Bindings) -> Option<u8> {
    KEYS.iter().position(|a| bind.pad_just_pressed(g, *a)).map(|d| d as u8)
}

/// The touch pad's id (web/touch.js `pad.id`), as gilrs names it.
pub fn is_touch(name: &str) -> bool {
    name.starts_with("Touch controls")
}

/// Human player `i`'s turn while it may emote: `start` is this tick's new flick. True while the emote holds the
/// player (`control` skips its turn).
pub(super) fn tick(g: &mut Game, i: usize, start: Option<u8>) -> bool {
    // the server too, until its toss (B228: online the host is 1P, who serves first)
    let serving = g.phase == Phase::Serve && g.score.server == i as i32;
    // B278: one from the rally plays on into the post-point time; the reaction (`react`) ends it
    // B363: not once the point has won a game or set (the match too): the game's and set's reactions and show
    let open = match g.phase {
        Phase::Rally => true,
        Phase::Post => !ends_a_game(g),
        Phase::Serve => !serving || g.serving.toss.is_none(),
        Phase::ChangeEnds(_) => false,
    };
    if !open {
        g.players[i].emote = 0;
        return false;
    }
    // B472: not between points once the players react; B591: the gap after the call, before the reaction, is
    // still the player's (it can move there), so a flick there starts one
    if let Some(d) = start
        && !reacting(g)
        && g.players[i].emote == 0
        && idle(g, i)
        && let Some(clip) = g.data[i].motions.get(&MOTIONS[d as usize])
    {
        let ticks = clip.length.ceil() as u32;
        let p = &mut g.players[i];
        p.emote = ticks.max(1);
        p.body.running = false;
        p.body.vel = [0.0; 3];
        set_motion(p, MOTIONS[d as usize] as i32, 1.0, false, None);
        voice(g, i, MOTIONS[d as usize]);
    }
    let p = &mut g.players[i];
    if p.emote == 0 {
        return false;
    }
    p.emote -= 1;
    (p.prev, p.vel, p.aim, p.dpad) = (p.pos, Vec2::ZERO, Vec2::ZERO, 0);
    if serving {
        // the ball stays in hand (`held_ball` puts it at the emote's hand), as `serve_turn` holds it
        super::hold_ball(g);
    }
    true
}

/// The post-point time from the players' reaction on (the reaction, the placement): no new emote (B472).
fn reacting(g: &Game) -> bool {
    g.phase == Phase::Post && g.post.as_ref().is_none_or(|p| p.reacted)
}

/// The point over ended a game or a set (or the match): its transition, where nobody emotes (B363).
fn ends_a_game(g: &Game) -> bool {
    g.post.as_ref().is_some_and(|p| matches!(p.event, Some(Event::Game | Event::Set)))
}

/// While the server emotes before its toss, a mod character's ball (B252): its own emote clip is what's drawn
/// (`CharacterData::visual`), but `held_ball` places the ball on the donor's, so the ball would float off the hand
/// — it's hidden (with its shadow and outline) until the emote ends, then back in hand for the toss. A disc
/// character's ball stays in its hand, seen. Draw-side only.
pub(super) fn hide_ball(g: Res<Game>, mut q: Query<&mut Visibility, Or<(With<super::BallView>, With<crate::shade::Ball>)>>, mut hidden: Local<bool>) {
    let i = g.score.server as usize;
    let hide = g.phase == Phase::Serve
        && g.players.get(i).is_some_and(|p| p.emote > 0 && g.data[i].visual.contains_key(&p.cmd.id));
    // only on a change, so the ceremony's hiding (`match_stats::ceremony::hide`) stays
    if std::mem::replace(&mut *hidden, hide) != hide {
        for mut v in &mut q {
            *v = if hide { Visibility::Hidden } else { Visibility::Inherited };
        }
    }
}

/// Nothing under way: no stroke, dive, whiff, serve follow-through, reaction or body hit.
fn idle(g: &Game, i: usize) -> bool {
    let p = &g.players[i];
    p.swing.is_none()
        && p.after.is_none()
        && p.contact.is_none()
        && p.pending.is_none()
        && p.approach.is_none()
        && p.whiff.is_none()
        && p.dive.is_none()
        && p.served.is_none()
        && p.follow.is_none()
        && p.wait_swing.is_none()
        // a played-out reaction may be followed by an emote (its root has stopped moving the player)
        && (p.root.is_none() || p.played)
        && !super::bodyhit::standing(g, i)
}

/// The reaction's voice line for the emote's motion (`voice_keys`), its key the next of the player's emote takes:
/// B300 and B500, the key memory is its own, slot 2, which neither the shouts (0, 1) nor the reactions (3–5) use, so
/// the real reactions pick as before. The memory is match state (snapshots carry it), so every peer picks alike;
/// nothing draws from the match's generators.
fn voice(g: &mut Game, i: usize, motion: usize) {
    // B474: muted here only after the pick, so the key memory (match state) moves the same on every peer
    if let Some(play) = line(&mut g.voices[i], i, motion, &g.data[i].voice_takes, g.players.len()).filter(|_| crate::audio::EMOTE_VOICES.load(std::sync::atomic::Ordering::Relaxed)) {
        g.lines.push(Line::Emote(i, play));
    }
}

/// An emote's voice program and its takes `lo..=hi`, by motion: the reaction's (`sound::reaction_voice` as a
/// winner, never the losers' group groan, the singles keys whatever the player count: the character's bank has them
/// all), widened to every take of the program on the disc banks (B500: gu's 7 has 0–2 — key 0 the instant replay's —
/// di's 8 0–4 — 2 the replay's, 3–4 the losing side's groan); gu_set's 9 and di_set's 10 have only their two.
pub fn voice_keys(motion: usize) -> Option<(u8, i32, i32)> {
    let (program, lo, hi, _) = sound::reaction_voice(2, motion as i32, false, false, || 100)?;
    Some(match program {
        7 => (7, 0, 2),
        8 => (8, 0, 4),
        _ => (program, lo, hi),
    })
}

/// `voice_keys` for a character with `takes(program)` takes of its own (a mod's, B596): every one of them, whatever
/// the disc's range; the disc's for a program it has none of.
pub fn own_voice_keys(motion: usize, takes: impl Fn(u8) -> usize) -> Option<(u8, i32, i32)> {
    let (program, lo, hi) = voice_keys(motion)?;
    Some(match takes(program) {
        0 => (program, lo, hi),
        n => (program, 0, n as i32 - 1),
    })
}

/// The last emote take of `program` a disc voice bank has with `players` in the match (B599): the doubles banks
/// (three or four players) hold only keys 0–1 of programs 7 and 8 and key 0 of 9 and 10, so a pick past those would
/// stay silent; the singles banks have every take `voice_keys` lists.
pub fn bank_last(program: u8, players: usize) -> i32 {
    match program {
        _ if players < 3 => i32::MAX,
        7 | 8 => 1,
        _ => 0,
    }
}

/// A voice line that cuts emote lines still sounding (B473): an emote's line its own player's last one, a post-point
/// reaction's every player's.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum Line {
    Emote(usize, sound::Play),
    Reaction(usize, sound::Play),
    /// B619: player's emote line cut, its reaction starting.
    Cut(usize),
}

/// Plays the lines due this tick at their player, each once however often its tick runs (`heard`), cutting the emote
/// lines still sounding (`audio::Sound::emote`, `stop_emotes`). A mod's take with a line of its own
/// (`CharacterData::drawn_voice`) speaks that one in place of the key picked, and one without never speaks another
/// take's (`Talk::key`): sound only, the pick and its draws stay.
pub(super) fn play_lines(
    mut g: ResMut<Game>,
    figures: Query<(Entity, &super::Figure, &crate::character::Motion, Option<&crate::character::Talk>)>,
    sound: Option<Res<crate::audio::Sound>>,
    voices: Option<Res<crate::audio::VoiceBanks>>,
    heard: Res<super::heard::Heard>,
    mut played: ResMut<super::heard::Played>,
    mut commands: Commands,
    time: Res<Time<Virtual>>,
) {
    let lines = std::mem::take(&mut g.lines);
    let (Some(sound), Some(voices)) = (sound, voices) else { return };
    // a line done or cut gives the mouth back to the clip
    for (e, .., talk) in &figures {
        if talk.is_some_and(|t| !sound.playing(t.id)) {
            commands.entity(e).remove::<crate::character::Talk>();
        }
    }
    for line in lines {
        let (i, p) = match line {
            Line::Emote(i, mut p) | Line::Reaction(i, mut p) => {
                if let Some((_, _, m, _)) = figures.iter().find(|(_, f, ..)| f.0 == i)
                    && voice_keys(m.id).is_some_and(|v| v.0 == p.program)
                    && let (Some(d), Some(b)) = (g.data.get(i), voices.0.get(p.slot as usize - 1).and_then(|b| b.as_ref()))
                {
                    let Some(k) = crate::character::Talk::key(d, b, p.program, m.id, m.sets, p.key) else { continue };
                    p.key = k;
                }
                (i, p)
            }
            // B619: a reaction starting cuts its player's emote line, once however often its tick runs
            Line::Cut(i) => {
                if played.first(heard.tick, ("cut", i)) {
                    sound.stop_emote(i);
                }
                continue;
            }
        };
        let at = g.players[i].pos;
        if !played.first(heard.tick, ("line", p, at)) {
            continue;
        }
        let bank = voices.0.get(p.slot as usize - 1).and_then(|b| b.as_ref());
        // B594: the player's one emote voice, through the mixer's (`audio::Sound::emote`)
        let id = if let Line::Emote(..) = line {
            sound.emote(i, |s| bank.map_or(0, |b| s.play_at(b, p, at)))
        } else {
            sound.stop_emotes();
            bank.map_or(0, |b| sound.play_at(b, p, at))
        };
        // a mod's line (its own wav) moves the mouth with it
        if let (Some(b), Some((e, ..)), Some(d)) = (bank, figures.iter().find(|(_, f, ..)| f.0 == i), g.data.get(i))
            && let Some(talk) = crate::character::Talk::of(d, b, p.program, p.key, id, time.elapsed_secs_f64())
        {
            commands.entity(e).insert(talk);
        }
    }
}

/// Player `i`'s emote line for `motion` on its voice `v`: the take after its last, over its own `takes` if a mod's,
/// else over the ones its disc bank has with `players` in the match (`bank_last`).
fn line(v: &mut sound::Voice, i: usize, motion: usize, takes: &std::collections::HashMap<u8, u8>, players: usize) -> Option<sound::Play> {
    let own = |p| takes.get(&p).map_or(0, |&n| n as usize);
    let (program, lo, hi) = own_voice_keys(motion, own)?;
    let hi = if own(program) == 0 { hi.min(bank_last(program, players)) } else { hi };
    let k = v.cycle(lo, hi, EMOTE_SLOT);
    v.react(i, (program, k, k, None), || 0)
}

/// The emotes' key-memory slot in `sound::Voice`.
const EMOTE_SLOT: usize = 2;

#[cfg(test)]
mod tests {
    use super::super::tick_input::{IDLE, TickPads, decode};
    use super::*;
    use hst_sim::motion::id;

    /// Pad buttons rebound to emotes (bumpers, triggers) play them; unbound, a shoulder never does.
    #[test]
    fn rebound_pad_buttons_emote() {
        let (bind, errors) = super::super::controls::Bindings::parse("emote_left = PadLeftTrigger\nemote_up = PadRightTrigger2\n");
        assert!(errors.is_empty(), "{errors:?}");
        let mut g = Gamepad::default();
        assert_eq!(bound(&g, &bind), None);
        g.digital_mut().press(GamepadButton::LeftTrigger);
        assert_eq!(bound(&g, &bind), Some(2));
        let mut g = Gamepad::default();
        g.digital_mut().press(GamepadButton::RightTrigger2);
        assert_eq!(bound(&g, &bind), Some(0));
        let mut g = Gamepad::default();
        g.digital_mut().press(GamepadButton::LeftTrigger);
        assert_eq!(bound(&g, &Default::default()), None);
    }

    /// B300 / B500: repeated emotes of one direction go through every take of their program in turn, and leave the
    /// reactions' key memory alone.
    #[test]
    fn emote_lines_vary() {
        for (m, program, keys) in [(id::WON_GAME, 9, &[0, 1, 0][..]), (id::LOST_GAME, 10, &[0, 1, 0]), (id::WON, 7, &[0, 1, 2, 0]), (id::LOST, 8, &[0, 1, 2, 3, 4, 0])] {
            let mut v = sound::Voice::default();
            let got: Vec<_> = keys.iter().map(|_| line(&mut v, 1, m as usize, &Default::default(), 2).map(|p| (p.program, p.key)).unwrap()).collect();
            assert_eq!(got, keys.iter().map(|&k| (program, k)).collect::<Vec<_>>(), "motion {m:#x}");
            let rv = sound::reaction_voice(2, m, false, false, || 0).unwrap();
            let mut fresh = sound::Voice::default();
            assert_eq!(v.react(1, rv, || 3).map(|p| p.key), fresh.react(1, rv, || 3).map(|p| p.key), "motion {m:#x}");
        }
    }

    /// B596: a mod's emotes go through every take it has of their program, past the disc's keys or short of them.
    #[test]
    fn a_mods_emote_lines_go_through_its_own_takes() {
        // T-Bone's counts: four of 9, five of 7, three of 8 (fewer than the disc's five), none of 10 (the disc's two)
        let takes = [(9, 4), (7, 5), (8, 3)].into_iter().collect();
        for (m, program, keys) in [(id::WON_GAME, 9, &[0, 1, 2, 3, 0][..]), (id::WON, 7, &[0, 1, 2, 3, 4, 0]), (id::LOST, 8, &[0, 1, 2, 0]), (id::LOST_GAME, 10, &[0, 1, 0])] {
            let mut v = sound::Voice::default();
            let got: Vec<_> = keys.iter().map(|_| line(&mut v, 1, m as usize, &takes, 2).map(|p| (p.program, p.key)).unwrap()).collect();
            assert_eq!(got, keys.iter().map(|&k| (program, k)).collect::<Vec<_>>(), "motion {m:#x}");
        }
    }

    /// B599: every pick of every emote, cycled round twice, sounds for every stock character on every voice bank a
    /// match loads (singles and doubles, sets a and b, court 7's c and d); the doubles banks' fewer takes come round
    /// in turn. Lists each character, bank and pick that stays silent.
    #[test]
    fn every_emote_pick_speaks() {
        let Ok(mut iso) = hst_data::fixture::iso() else { return };
        let sound = crate::audio::Sound::test(&mut iso);
        let mut silent = Vec::new();
        for c in 0..14 {
            for (players, court, b) in [2, 4].into_iter().flat_map(|n| [1, 7].into_iter().flat_map(move |s| [false, true].map(|b| (n, s, b)))) {
                let bank = std::sync::Arc::new(crate::audio::voice_bank(&mut iso, c, players, court, b).expect("voice bank"));
                for m in MOTIONS {
                    let mut v = sound::Voice::default();
                    for _ in 0..12 {
                        let Some(p) = line(&mut v, 1, m, &Default::default(), players) else {
                            silent.push(format!("c{c} players {players} court {court} set {b} motion {m:#x}: no line"));
                            break;
                        };
                        let id = sound.play_at(&bank, p, [0.0; 3]);
                        let heard = id != 0 && (0..60).any(|_| sound.render_frame().iter().any(|x| x.abs() > 1e-3));
                        sound.stop(id);
                        (0..20).for_each(|_| _ = sound.render_frame());
                        if !heard {
                            silent.push(format!("c{c} players {players} court {court} set {b} motion {m:#x}: {}:{}", p.program, p.key));
                        }
                    }
                }
            }
        }
        assert!(silent.is_empty(), "silent emote picks:\n{}", silent.join("\n"));
    }

    /// B599: in doubles the emotes cycle over the doubles banks' takes.
    #[test]
    fn doubles_emote_lines_keep_to_the_doubles_takes() {
        for (m, program, keys) in [(id::WON_GAME, 9, &[0, 0][..]), (id::LOST_GAME, 10, &[0, 0]), (id::WON, 7, &[0, 1, 0]), (id::LOST, 8, &[0, 1, 0])] {
            let mut v = sound::Voice::default();
            let got: Vec<_> = keys.iter().map(|_| line(&mut v, 1, m as usize, &Default::default(), 4).map(|p| (p.program, p.key)).unwrap()).collect();
            assert_eq!(got, keys.iter().map(|&k| (program, k)).collect::<Vec<_>>(), "motion {m:#x}");
        }
    }

    /// B596: a mod's emotes cycle over the takes its voice folder has, a costume pack's over its disc character's.
    #[test]
    fn a_mod_counts_its_own_takes() {
        let dir = std::env::temp_dir().join(format!("hst-b596-{}", std::process::id()));
        let voice = dir.join("voice");
        std::fs::create_dir_all(&voice).unwrap();
        for f in ["9_0.wav", "9_1.wav", "9_2.wav", "9_3.wav", "7_0.wav", "notes.txt"] {
            std::fs::write(voice.join(f), b"").unwrap();
        }
        let mut m = crate::mods_online::for_match(&crate::mods_online::Wire { voices: vec![], ..wire() }, None);
        (m.dir, m.voice, m.voice_takes) = (dir.clone(), "voice/".into(), None);
        assert_eq!(crate::mods::voice_takes(&m), [(7, 1), (9, 4)]);
        m.costume_of = Some(3);
        assert_eq!(crate::mods::voice_takes(&m), []);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn wire() -> crate::mods_online::Wire {
        crate::mods_online::Wire { emotes: vec![], voices: vec![], shots: [None; 4], id: "x".into(), name: "X".into(), donor: 1, hand: 1.0, base: 1, overrides: vec![], ai_row: 1, names: vec!["a".into()], parent: vec![None], rest: vec![[0; 16]], looks: 0, dir: None }
    }

    #[test]
    fn flicks() {
        assert!(is_touch("Touch controls (STANDARD GAMEPAD)") && !is_touch("Xbox Controller"));
        assert_eq!([Vec2::Y, -Vec2::Y, -Vec2::X, Vec2::X, Vec2::new(0.5, 0.5), Vec2::ZERO].map(flick), [Some(0), Some(1), Some(2), Some(3), None, None]);
    }

    /// A flick goes over the wire as the right stick and comes back once, on the tick it starts.
    #[test]
    fn flick_on_the_wire() {
        for d in 0..4 {
            let mut s = super::super::SlotPad { emote: Some(d), ..default() };
            let p = super::super::tick_input::take(&mut s);
            assert_eq!(s.emote, None);
            assert_eq!((decode(p, IDLE).emote, decode(p, p).emote, decode(IDLE, p).emote), (Some(d), None, None));
            let mut t = TickPads::default();
            t.push([p, IDLE, IDLE, IDLE]);
            assert_eq!(t.slot(0).emote, Some(d));
        }
        assert_eq!(decode(super::super::tick_input::recorded(hst_sim::replay::Pad { rx: 0x80, ry: 0, ..IDLE }), IDLE).emote, None);
    }
}

/// An emote in the app's singles match: 2P (a disc character, 1P a bot that serves first) flicks the right stick during the rally; it
/// stands, its motion plays and its presses do nothing for the motion's length, then it runs and swings again.
/// Needs the ISO at the repository root (skipped without).
#[cfg(test)]
mod match_tests {
    use super::super::tick_input::{IDLE, Replay, TickPads};
    use super::MOTIONS;
    use hst_sim::motion::id;
    use super::super::*;
    use crate::{Args, GameSpace};
    use hst_sim::replay::Pad;

    const ISO: &str = hst_data::fixture::ISO_PATH;

    fn app(pads: Vec<[Pad; 4]>) -> App {
        seated([false, true, false, false], pads)
    }

    fn seated(seats: [bool; 4], pads: Vec<[Pad; 4]>) -> App {
        modded(seats, pads, None)
    }

    /// `seated` with 2P playing mod `m`.
    fn modded(seats: [bool; 4], pads: Vec<[Pad; 4]>, m: Option<crate::mods::Mod>) -> App {
        let mut app = App::new();
        if let Some(m) = m {
            app.insert_resource(crate::mods::MatchMod(vec![None, Some(m)]));
        }
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.insert_resource(Pads { seats: Some(seats), autoplay: false, ..default() });
        app.insert_resource(Replay(pads.into_iter().collect()));
        app.init_resource::<TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, super::super::tick_input::systems(), control, simulate, motions, character::tick, played_out, held_ball).chain());
        app.update();
        for i in 0..2 {
            app.world_mut().spawn((Figure(i), character::Motion::default()));
        }
        app
    }

    /// 2P's position, emote ticks left, whether a press is under way (search, whiff or swing), motion; the phase.
    fn step(app: &mut App) -> (V3, u32, bool, usize, Phase) {
        app.world_mut().run_schedule(FixedUpdate);
        let g = app.world().resource::<Game>();
        let p = &g.players[1];
        (p.pos, p.emote, p.pending.is_some() || p.whiff.is_some() || p.swing.is_some(), p.cmd.id, g.phase.clone())
    }

    #[test]
    fn an_emote_locks_then_plays_on() {
        if !hst_data::fixture::has_iso() {
            return;
        }
        let left = Pad { lx: 0, ..IDLE };
        let flick = Pad { rx: 0x80, ry: 0, ..left };
        let cross = Pad { buttons: 0x2000, ..left };
        let n = 600;
        let rally = {
            let mut a = app(vec![]);
            (0..n).find(|_| step(&mut a).4 == Phase::Rally).expect("a rally")
        };
        let len = app(vec![]).world().resource::<Game>().data[1].motions[&MOTIONS[0]].length.ceil() as usize;
        // waiting for 1P's serve: 2P emotes from tick `at`, stick held left and ✕ pressed meanwhile, then ✕ in the rally
        let at = 10;
        assert!(at + len + 20 < rally, "the emote ({len} ticks) outlasts the serve's wait ({rally})");
        let mut pads = vec![[IDLE, left, IDLE, IDLE]; n];
        pads[at][1] = flick;
        pads[at + 5][1] = cross;
        pads[rally + 10][1] = cross;
        let mut a = app(pads);
        let mut last = None;
        for t in 0..rally + 12 {
            let (pos, emote, pressed, motion, phase) = step(&mut a);
            if (at..at + len).contains(&t) {
                assert_eq!((emote as usize, motion, pressed), (at + len - 1 - t, MOTIONS[0], false), "tick {t}");
                assert!(last.is_none_or(|l| l == pos), "moved while emoting, tick {t}");
                last = Some(pos);
            }
            if t == at + len + 2 {
                assert_ne!(Some(pos), last, "didn't run after the emote");
            }
            if t == rally + 10 {
                assert!(phase == Phase::Rally && pressed, "the press after the emote did nothing");
            }
        }
        // the same flick as the serve comes: it goes by unhit, and the point's end doesn't cut it (B278): the
        // emote plays on into the post-point time until it's out or the reaction takes over
        // (the flick lands half an emote before the point's end without it, so the end falls inside the emote
        // whichever serve the bot plays)
        let over = {
            let mut a = app(vec![]);
            (0..n).find(|&t| step(&mut a).4 != Phase::Rally && t > rally).expect("the point ends")
        };
        let flicked = over.saturating_sub(len / 2).max(rally + 2);
        let mut pads = vec![[IDLE; 4]; n];
        pads[flicked][1] = flick;
        pads[flicked + 8][1] = cross;
        let mut a = app(pads);
        let (mut emoted, mut ended, mut last) = (false, None, 0);
        for t in 0..n {
            let (_, emote, pressed, motion, phase) = step(&mut a);
            assert!(!pressed, "pressed while emoting, tick {t}");
            emoted |= emote > 0;
            if let Some(e) = ended {
                let reacted = a.world().resource::<Game>().post.as_ref().is_some_and(|p| p.reacted);
                if emote == 0 {
                    assert!(last == 1 || reacted, "the emote was cut at tick {t} ({last} left, point ended {e})");
                    return eprintln!("emote {len} ticks, rally from {rally}, the point ended at {e}, emote out at {t}, reacted {reacted}");
                }
                assert_eq!((emote, motion), (last - 1, MOTIONS[0]), "tick {t}");
            } else if emoted && phase != Phase::Rally {
                assert!(emote > 0, "the emote was out before the point ended");
                ended = Some(t);
            }
            last = emote;
        }
        panic!("the point never ended");
    }

    /// B228: the server emotes before its toss (online the host is 1P and serves first), the ball in its hand
    /// all the while, then tosses as usual.
    #[test]
    fn the_server_emotes_before_the_toss() {
        if !hst_data::fixture::has_iso() {
            return;
        }
        let flick = Pad { rx: 0, ry: 0x80, ..IDLE };
        let cross = Pad { buttons: 0x2000, ..IDLE };
        let len = app(vec![]).world().resource::<Game>().data[0].motions[&MOTIONS[2]].length.ceil() as usize;
        let (at, n) = (10, 600);
        let mut pads = vec![[IDLE; 4]; n];
        pads[at][0] = flick;
        pads[at + len + 5][0] = cross;
        let mut a = seated([true, false, false, false], pads);
        for t in 0..n {
            a.world_mut().run_schedule(FixedUpdate);
            let drawn = a.world_mut().query::<(&Figure, &character::Motion)>().iter(a.world()).find(|(f, _)| f.0 == 0).unwrap().1.id;
            let g = a.world().resource::<Game>();
            let p = &g.players[0];
            if (at..at + len).contains(&t) {
                assert_eq!((p.emote as usize, p.cmd.id, g.phase.clone()), (at + len - 1 - t, MOTIONS[2], Phase::Serve), "tick {t}");
                let d = (0..3).map(|k| (g.flight.ball.pos[k] - p.pos[k]).powi(2)).sum::<f32>().sqrt();
                assert!(d < 2.5 && g.flight.ball.vel == [0.0; 3], "the ball left the hand, tick {t}: {d} m");
            }
            // B231: the drawn body plays the emote, not the serve stance, then goes back to the stance
            if (at..at + len - 1).contains(&t) {
                assert_eq!(drawn, MOTIONS[2], "the body kept the stance, tick {t}");
            }
            if t == at + len + 2 {
                assert!((id::SERVE_STANCE as usize..=id::SERVE_RIGHT as usize).contains(&drawn), "no stance after the emote, tick {t}: {drawn:#x}");
            }
            // the press tosses (a second would swing)
            if g.serving.toss.is_some() {
                assert_eq!(t, at + len + 5, "tossed at tick {t}");
                return;
            }
        }
        panic!("never tossed");
    }

    /// B252: a disc server's ball stays seen through its emote; a mod's (its own emote clip drawn, here the donor's
    /// copied in as one) is hidden for the emote's length and back for the toss.
    #[test]
    fn a_mods_ball_hides_while_it_emotes() {
        if !hst_data::fixture::has_iso() {
            return;
        }
        let flick = Pad { rx: 0, ry: 0x80, ..IDLE };
        for own_clip in [false, true] {
            let (at, n) = (10, 120);
            let mut pads = vec![[IDLE; 4]; n];
            pads[at][0] = flick;
            let mut a = seated([true, false, false, false], pads);
            a.add_systems(FixedUpdate, super::hide_ball.after(held_ball));
            let ball = a.world_mut().spawn((BallView(false), Visibility::Inherited)).id();
            // the rigs share the data: dropped, so this test may change it
            let rigs: Vec<_> = a.world_mut().query_filtered::<Entity, With<character::Rig>>().iter(a.world()).collect();
            for e in rigs {
                a.world_mut().entity_mut(e).remove::<character::Rig>();
            }
            let len = {
                let mut g = a.world_mut().resource_mut::<Game>();
                let d = std::sync::Arc::get_mut(&mut g.data[0]).expect("the match's own character data");
                let clip = d.motions[&MOTIONS[2]].clone();
                let len = clip.length.ceil() as usize;
                if own_clip {
                    d.visual.insert(MOTIONS[2], clip);
                }
                len
            };
            for t in 0..at + len + 5 {
                a.world_mut().run_schedule(FixedUpdate);
                let hidden = a.world().get::<Visibility>(ball) == Some(&Visibility::Hidden);
                assert_eq!(hidden, own_clip && (at..at + len - 1).contains(&t), "own clip {own_clip}, tick {t}");
            }
        }
    }

    /// B473: 2P's second emote cuts its first one's voice line, and a post-point reaction's line cuts the second.
    #[test]
    fn a_new_line_cuts_the_last_emote_line() {
        if !hst_data::fixture::has_iso() {
            return;
        }
        let flick = Pad { rx: 0x80, ry: 0, ..IDLE };
        let mut a = app(vec![]);
        let len = a.world().resource::<Game>().data[1].motions[&MOTIONS[0]].length.ceil() as usize;
        let (at, again) = (10, 10 + len + 3);
        let mut pads = vec![[IDLE; 4]; again + 2];
        (pads[at][1], pads[again][1]) = (flick, flick);
        a.insert_resource(Replay(pads.into_iter().collect()));
        let sound = crate::audio::Sound::test(&mut hst_data::fixture::iso().unwrap());
        a.insert_resource(sound.clone());
        a.add_plugins(heard::plugin);
        a.add_systems(FixedUpdate, super::play_lines.after(held_ball));
        let mut lines = Vec::new();
        for t in 0..=again {
            let before = sound.plays();
            step(&mut a);
            if t == at || t == again {
                assert_eq!(sound.plays(), before + 1, "tick {t}: no emote line");
                lines.push(sound.plays());
            }
        }
        assert!(!sound.playing(lines[0]) && sound.playing(lines[1]), "the second emote didn't cut the first's line");
        // a reaction line, here 1P's, cuts every emote line
        let p = sound::Play { slot: 1, program: 7, key: 1, volume: 0x80, speed: 1.0 };
        a.world_mut().resource_mut::<Game>().lines.push(super::Line::Reaction(0, p));
        step(&mut a);
        assert!(!sound.playing(lines[1]) && sound.playing(sound.plays()), "the reaction didn't cut the emote line");
    }

    /// B619: the post-point reaction starting cuts its players' emote lines, even with no reaction line to play.
    #[test]
    fn the_reaction_cuts_the_emote_line() {
        if !hst_data::fixture::has_iso() {
            return;
        }
        let flick = Pad { rx: 0x80, ry: 0, ..IDLE };
        let mut a = app(vec![]);
        let at = 10;
        let mut pads = vec![[IDLE; 4]; at + 4];
        pads[at][1] = flick;
        a.insert_resource(Replay(pads.into_iter().collect()));
        let sound = crate::audio::Sound::test(&mut hst_data::fixture::iso().unwrap());
        a.insert_resource(sound.clone());
        a.add_plugins(heard::plugin);
        a.add_systems(FixedUpdate, super::play_lines.after(held_ball));
        for _ in 0..=at {
            step(&mut a);
        }
        let line = sound.plays();
        assert!(sound.playing(line), "no emote line");
        react(&mut a.world_mut().resource_mut::<Game>(), Event::Point);
        step(&mut a);
        assert_eq!(sound.plays(), line, "a line played");
        assert!(!sound.playing(line), "the reaction didn't cut the emote line");
    }

    /// A mod character's emotes speak its own takes (programs 9, 10, 7, 8): each of the four flicks plays a line.
    /// Needs the ISO and the mods repo's packaged mods in `mods/hst-mods` (skipped without).
    #[test]
    fn a_mod_emote_speaks() {
        const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods/hst-mods/");
        if !hst_data::fixture::has_iso() || !std::path::Path::new(ROOT).is_dir() {
            return eprintln!("no ISO or mods/hst-mods, skipped");
        }
        for id in ["opentee2_pc10_carly", "opentee2_pc03_shu", "oob_pc12_anya"] {
            let m = crate::mods::read(&std::path::Path::new(ROOT).join(id)).unwrap();
            // one match per flick: a mod's emote lasts its own clip, longer than the wait for the serve
            for (d, flick) in [Pad { rx: 0x80, ry: 0, ..IDLE }, Pad { rx: 0x80, ry: 0xff, ..IDLE }, Pad { rx: 0, ry: 0x80, ..IDLE }, Pad { rx: 0xff, ry: 0x80, ..IDLE }].into_iter().enumerate() {
                let mut a = modded([false, true, false, false], vec![], Some(m.clone()));
                let mut pads = vec![[IDLE; 4]; 12];
                pads[10][1] = flick;
                a.insert_resource(Replay(pads.into_iter().collect()));
                let sound = crate::audio::Sound::test(&mut hst_data::fixture::iso().unwrap());
                a.insert_resource(sound.clone());
                a.add_plugins(heard::plugin);
                a.add_systems(FixedUpdate, super::play_lines.after(held_ball));
                for _ in 0..10 {
                    step(&mut a);
                }
                let before = sound.plays();
                assert!(step(&mut a).1 > 0, "{id}: flick {d} started no emote");
                assert_eq!(sound.plays(), before + 1, "{id}: flick {d}'s emote spoke no line");
            }
        }
    }

    /// A mod's emote line moves its mouth: every flick's line, each a mouth of its own (the take's line). Needs the
    /// ISO and `mods/hst-mods` (skipped without).
    #[test]
    fn a_mod_emote_moves_its_mouth() {
        const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods/hst-mods/oob_pc12_anya");
        if !hst_data::fixture::has_iso() || !std::path::Path::new(ROOT).is_dir() {
            return eprintln!("no ISO or mods/hst-mods, skipped");
        }
        let m = crate::mods::read(std::path::Path::new(ROOT)).unwrap();
        let mut a = modded([false, true, false, false], vec![], Some(m));
        let mut pads = vec![[IDLE; 4]; 12];
        pads[10][1] = Pad { rx: 0x80, ry: 0, ..IDLE };
        a.insert_resource(Replay(pads.into_iter().collect()));
        let sound = crate::audio::Sound::test(&mut hst_data::fixture::iso().unwrap());
        a.insert_resource(sound.clone());
        a.add_plugins(heard::plugin);
        a.add_systems(FixedUpdate, super::play_lines.after(held_ball));
        for _ in 0..12 {
            step(&mut a);
        }
        let talks: Vec<u64> = a.world_mut().query::<(&Figure, &crate::character::Talk)>().iter(a.world()).filter(|(f, _)| f.0 == 1).map(|(_, t)| t.id).collect();
        assert!(talks.contains(&sound.plays()), "the emote's line moves no mouth");
    }

    /// A mod's emote holds the player for its own clip, not the donor's: offline, and online alike whether this
    /// peer has the looks or plays the stand-in (the wire carries the lengths). Needs the ISO and `mods/hst-mods`
    /// (skipped without).
    #[test]
    fn a_mod_emote_lasts_its_own_clip() {
        const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods/hst-mods/oob_pc12_anya");
        if !hst_data::fixture::has_iso() || !std::path::Path::new(ROOT).is_dir() {
            return eprintln!("no ISO or mods/hst-mods, skipped");
        }
        let m = crate::mods::read(std::path::Path::new(ROOT)).unwrap();
        let w = crate::mods_online::Wire::of(&m, 0).unwrap();
        let held = |m: crate::mods::Mod| {
            let mut a = modded([false, true, false, false], vec![], Some(m));
            let mut pads = vec![[IDLE; 4]; 400];
            pads[10][1] = Pad { rx: 0x80, ry: 0, ..IDLE };
            a.insert_resource(Replay(pads.into_iter().collect()));
            let f = a.world_mut().query::<(Entity, &Figure)>().iter(a.world()).find(|(_, f)| f.0 == 1).unwrap().0;
            let (mut n, mut own) = (0, (0.0, 0));
            for t in 0..400 {
                n += (step(&mut a).1 > 0) as u32;
                if t == 10 {
                    let (g, m) = (a.world().resource::<Game>(), a.world().get::<character::Motion>(f).unwrap());
                    own = (g.data[1].drawn_take(m.id, m.sets).unwrap().length, g.data[1].emote_ticks(m.id, m.sets).unwrap_or(0));
                }
            }
            (n + 1, own) // the last tick it holds reads 0
        };
        let (offline, own) = held(m);
        let donor = held(crate::mods::read(std::path::Path::new(ROOT)).map(|m| crate::mods::Mod { motions: None, ..m }).unwrap()).0;
        let (own, moves) = own;
        assert_eq!(offline, moves, "held {offline} ticks, the mod's clip moves for {moves}");
        assert!(moves <= own.ceil() as u32 && offline != donor, "the mod's clip ({own}, moves {moves}), the donor's {donor}");
        eprintln!("held {offline}, clip {own}, donor {donor}");
        let looks = held(crate::mods_online::for_match(&w, Some(std::path::Path::new(ROOT)))).0;
        let stand_in = held(crate::mods_online::for_match(&w, None)).0;
        assert_eq!((looks, stand_in), (offline, offline), "online: with the looks, as the stand-in; offline {offline}");
    }

    /// A mod's emote take with a line of its own says it whatever the key picked; one without never says a line
    /// another take owns (those are timed to their own clips). Needs the ISO and `mods/hst-mods` (skipped without).
    #[test]
    fn a_take_without_a_line_says_none_a_take_owns() {
        const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods/hst-mods/oob_pc12_anya");
        if !hst_data::fixture::has_iso() || !std::path::Path::new(ROOT).is_dir() {
            return eprintln!("no ISO or mods/hst-mods, skipped");
        }
        let m = crate::mods::read(std::path::Path::new(ROOT)).unwrap();
        let mut iso = hst_data::fixture::iso().unwrap();
        let d = crate::mods::load(&mut iso, &m, 0, &mut Assets::default(), &mut Assets::default(), &mut Assets::default(), &mut Assets::default()).unwrap();
        let bank = crate::mods::voice(&m).unwrap();
        let (mut own, mut free) = (0, 0);
        for id in MOTIONS {
            let program = super::voice_keys(id).unwrap().0;
            let n = bank.wav[&(program as usize)].len() as u8;
            let owned: Vec<u8> = d.take_voices.iter().filter(|(t, _)| t.0 == id).map(|(_, &k)| k).collect();
            for sets in 0..64 {
                let keys: Vec<Option<u8>> = (0..n).map(|k| character::Talk::key(&d, &bank, program, id, sets, k)).collect();
                if keys.iter().all(|&k| k == keys[0]) && keys[0].is_some_and(|k| owned.contains(&k)) {
                    own += 1;
                } else {
                    assert!(keys.iter().flatten().all(|k| !owned.contains(k)), "motion {id:#x} set {sets}: {keys:?} says another take's line ({owned:?})");
                    free += 1;
                }
            }
        }
        assert!(own > 0 && free > 0, "own {own}, free {free}");
        // a take with its own line and a moving mouth keeps it; one with a line and a still mouth gets the line's
        let mouths: Vec<usize> = (0..d.morph_names.len()).filter(|&i| d.morph_names[i].contains("mouth")).collect();
        let moves = |k: usize| d.own_faces[&(id::WON_GAME as usize, k)].moves(&mouths);
        assert!(moves(2) && !moves(3), "re_gu_set.2 talks, .3 holds still");
    }

    /// B591 (B472, B363 before it): 1P tosses and swings as the ball drops past (a whiff: a fault); a flick in the
    /// gap after the call, before the players react, emotes — not in one that won a game (the fault's post-point
    /// turned into one) or a set; the reaction ends it, a flick from the reaction to the next serve starts nothing,
    /// and once the next serve's wait is on, one does again.
    #[test]
    fn no_emote_between_points() {
        if !hst_data::fixture::has_iso() {
            return;
        }
        let flick = Pad { rx: 0, ry: 0x80, ..IDLE };
        let cross = Pad { buttons: 0x2000, ..IDLE };
        let n = 900;
        let mut pads = vec![[IDLE; 4]; n];
        (pads[10][0], pads[134][0]) = (cross, cross);
        let post = {
            let mut a = seated([true, false, false, false], pads.clone());
            (0..n).find(|_| {
                a.world_mut().run_schedule(FixedUpdate);
                a.world().resource::<Game>().phase == Phase::Post
            })
            .expect("the fault's call")
        };
        let at = post + 3;
        pads[at][0] = flick;
        for event in [Some(Event::Game), Some(Event::Set)] {
            let mut a = seated([true, false, false, false], pads.clone());
            for t in 0..=at {
                if t == at {
                    a.world_mut().resource_mut::<Game>().post.as_mut().expect("post-point").event = event;
                }
                a.world_mut().run_schedule(FixedUpdate);
            }
            let g = a.world().resource::<Game>();
            assert_eq!((g.phase.clone(), g.players[0].emote), (Phase::Post, 0), "{event:?}");
        }
        // the gap's flick emotes until the reaction; flicked again two ticks into the reaction, and in the second
        // serve's wait
        let mut a = seated([true, false, false, false], pads.clone());
        let (mut reacted, mut serve) = (None, None);
        for t in 0..n {
            let g = a.world().resource::<Game>();
            if reacted.is_none() && g.post.as_ref().is_some_and(|p| p.reacted) {
                reacted = Some(t);
            }
            if serve.is_none() && t > at && g.phase == Phase::Serve {
                serve = Some(t);
            }
            if let Some(p) = a.world_mut().resource_mut::<Replay>().0.front_mut()
                && (reacted.is_some_and(|r| t == r + 2) || serve.is_some_and(|s| t == s + 2))
            {
                p[0] = flick;
            }
            a.world_mut().run_schedule(FixedUpdate);
            let g = a.world().resource::<Game>();
            let emote = g.players[0].emote;
            if t == at {
                assert!(g.phase == Phase::Post && !g.post.as_ref().unwrap().reacted, "the flick missed the gap");
                assert!(emote > 0, "no emote in the gap after the call");
            } else if reacted.is_some() && serve.is_none() {
                assert_eq!(emote, 0, "emoted after the reaction, tick {t}");
            } else if serve.is_some_and(|s| t == s + 2) {
                assert!(emote > 0, "no emote waiting to serve, tick {t}");
                return;
            }
        }
        panic!("no second serve");
    }
}

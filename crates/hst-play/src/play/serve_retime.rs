//! B700a: a computer server's swing kind, aim and flat-or-not don't depend on when it presses. Its timing error moves
//! only the swing's timing offset (the original, re-timed live from a save at the toss: 15 of 15 serves kept their
//! kind, contact stick and ball kind).

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::{Args, GameSpace};

    const ISO: &str = hst_data::fixture::ISO_PATH;

    /// An all-CPU singles match of these characters, set up, not ticked yet.
    fn bot_match(chars: Vec<usize>) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.init_asset::<crate::gs::GsMaterial>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars, outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 6, pads: None });
        app.insert_resource(Pads { seats: Some([false; 4]), autoplay: false, ..default() });
        app.init_resource::<tick_input::TickPads>();
        app.init_resource::<instant_replay::Recorder>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_plugins((cutaway::plugin, bodyhit::plugin, surprise::plugin, reaction_voices::plugin, match_stats::plugin, panel::plugin, popups::plugin, npcs::plugin, weather::plugin, heard::plugin));
        app.add_systems(PostStartup, setup);
        app.add_systems(FixedUpdate, (remember, effects::tick, tick_input::systems(), control, simulate, start_effects, age_balloons, motions, character::tick, played_out, held_ball, instant_replay::run).chain());
        app.finish();
        app.cleanup();
        app.world_mut().run_schedule(Startup);
        app.world_mut().run_schedule(PostStartup);
        app
    }

    /// One play of a serve from its toss: what the press drew and what the contact made of it.
    #[derive(Debug, PartialEq)]
    struct Served {
        /// The swing kind drawn at the press (the AI's plan and the press kind).
        plan: u8,
        press_kind: i32,
        /// Ticks from the toss to the contact (the same ball frame whatever the press).
        contact_tick: u32,
        stick: Vec2,
        dash: Option<[f32; 2]>,
        /// The shot as struck: kind (flat included) and class.
        kind: i32,
        class: u8,
        /// The AI generator's next draw once the aim is drawn, the tick before the contact: the same draws were made
        /// from the toss, in the same order.
        next_ai: u32,
    }

    /// What follows the error: the swing's timing offset, the press's frames to the contact, and the ticks from the
    /// toss on which the AI generator drew.
    #[derive(Debug)]
    struct Timed {
        offset: i32,
        frames: u32,
        drawn: Vec<u32>,
    }

    /// Plays the serve on from the toss with server `i`'s serve timing error forced to `err`.
    fn play(app: &mut App, i: usize, err: i32) -> Option<(Served, Timed)> {
        let mut g = app.world_mut().resource_mut::<Game>();
        g.players[i].ai_timing.serve = err;
        g.serving.bot_due = Some((SWEET_FRAME - err).max(0) as usize);
        let (mut plan, mut press, mut aimed) = (None, None, None);
        let next_ai = |app: &App| app.world().resource::<Game>().rng.ai.clone().next();
        let mut drawn = Vec::new();
        for t in 1..240 {
            let before = next_ai(app);
            app.world_mut().run_schedule(FixedUpdate);
            if next_ai(app) != before {
                drawn.push(t);
            }
            let g = app.world().resource::<Game>();
            if g.serving.whiffed || g.phase != Phase::Serve && g.players[i].served.is_none() {
                return None;
            }
            if press.is_none()
                && let Some(sw) = g.serving.swing
            {
                plan = g.players[i].ai_serve_swing;
                press = Some((sw.kind, sw.offset, sw.frames));
            }
            // the tick before the contact: the aim is drawn
            if g.serving.swing.is_some_and(|sw| sw.left == 1) {
                let mut ai = g.rng.ai.clone();
                aimed = Some((g.serving.bot_aim, g.players[i].ai_serve_dash, ai.next()));
            }
            if g.players[i].served == Some(0) {
                let (press_kind, offset, frames) = press?;
                let (stick, dash, next_ai) = aimed?;
                return Some((
                    Served {
                        plan: plan?,
                        press_kind,
                        contact_tick: t,
                        stick,
                        dash,
                        kind: g.shot.kind,
                        class: g.shot.class,
                        next_ai,
                    },
                    Timed { offset, frames, drawn },
                ));
            }
        }
        None
    }

    /// Every CPU serve of an all-CPU singles match, replayed from its toss with the server's serve timing error forced
    /// to −4, 0, 2 and 4: the swing kind drawn at the press, the contact tick, the contact stick (and dash), the
    /// struck kind (flat included) and the AI generator's state once the aim is drawn are the same in all four, until a
    /// flat and a topspin serve have both been seen; the generator moves only at the press, the aim and the contact.
    /// The offset follows the error: `−err` for a late press (err ≥ 0).
    /// ponytail: the early side (err < 0 → offset 0, never early) is B700's press rule; with it in, assert
    /// `offset == −max(err, 0)` for −4 too.
    #[test]
    fn retimed_cpu_serve_keeps_its_kind() {
        if !hst_data::fixture::has_iso() {
            return hst_data::fixture::skip(format_args!("skip: no disc"));
        }
        let mut app = bot_match(vec![11, 6]);
        let (mut flat, mut topspin, mut serves) = (0, 0, 0);
        let mut was_tossed = false;
        for _ in 0..60 * 60 * 8 {
            if flat >= 1 && topspin >= 1 && serves >= 6 {
                break;
            }
            app.world_mut().run_schedule(FixedUpdate);
            let g = app.world().resource::<Game>();
            let tossed = g.phase == Phase::Serve && g.serving.tossed && g.serving.swing.is_none() && !g.serving.whiffed;
            let fresh = tossed && !was_tossed;
            was_tossed = tossed;
            if !fresh || g.serving.toss != Some(Toss::Strong) {
                continue;
            }
            let i = g.score.server as usize;
            let at = snapshot::save(app.world_mut());
            let mut runs = Vec::new();
            for err in [-4, 0, 2, 4] {
                snapshot::restore(app.world_mut(), &at);
                runs.push((err, play(&mut app, i, err)));
            }
            snapshot::restore(app.world_mut(), &at);
            let Some((first, _)) = &runs[0].1 else { continue };
            for (err, run) in &runs {
                let (served, timed) = run.as_ref().unwrap_or_else(|| panic!("serve {serves}: err {err} whiffed"));
                assert_eq!(served, first, "serve {serves}: err {err} changed the serve");
                if *err >= 0 {
                    assert_eq!(timed.offset, -err, "serve {serves}: err {err}: {timed:?}");
                }
                // the AI generator draws the swing at the press, the aim the tick before the contact and the contact's
                // own draws on it, nothing else: the original's aim (on the contact frame, ahead of the contact) comes
                // in the same order
                let (press, contact) = (served.contact_tick - timed.frames, served.contact_tick);
                assert_eq!(timed.drawn, [press, contact - 1, contact], "serve {serves}: err {err}: {timed:?}");
            }
            if first.press_kind == kind::TOPSPIN {
                match first.kind {
                    kind::FLAT => flat += 1,
                    kind::TOPSPIN => topspin += 1,
                    _ => {}
                }
            }
            serves += 1;
        }
        assert!(flat >= 1 && topspin >= 1, "{serves} serves: {flat} flat, {topspin} topspin");
    }
}

//! B40g1: Set Handicap on the local confirm screen. △ on a player's card (their own, or a computer player's) steps it
//! None → one weight → two → None, shown in a pill on the card's footer (a tap steps it too); the match gets it as
//! `--handicap` by match slot (`play::handicap`). Online matches play without handicaps: both peers' bodies have to
//! agree and the lobby doesn't carry them yet.

use super::roster::{PX, PY, rounded};
use super::{CONFIRM_ROWS, Dev, Draw, Menu, Press, Screen, pointer};
use super::frame::hex;

/// The pill's words for a level.
pub(super) fn label(h: u8) -> &'static str {
    ["No handicap", "1 weight", "2 weights"][h.min(2) as usize]
}

impl Menu {
    /// Whether `dev`'s △ steps the handicap of the plate `sel` is on: a local match, the device's own card or a
    /// computer player's.
    pub(super) fn can_handicap(&self, dev: Dev) -> bool {
        let q = self.sel.wrapping_sub(CONFIRM_ROWS);
        !self.online() && q < self.players() && (self.seats[q] == Some(dev) || self.bot(q))
    }

    /// The chosen plate's player one step on: None → one weight → two → None.
    pub(super) fn step_handicap(&mut self) {
        let p = &mut self.players[self.sel - CONFIRM_ROWS];
        p.handicap = (p.handicap + 1) % 3;
    }

    /// `--handicap` for the match (menu player → match slot by `order`), none online or with nobody handicapped.
    pub(super) fn handicap_flags(&self, order: &[usize]) -> Vec<String> {
        let mut levels = vec![0; order.len()];
        for (p, &m) in order.iter().enumerate() {
            levels[m] = self.players[p].handicap;
        }
        match crate::play::handicap::Handicaps::flag(&levels) {
            Some(f) if !self.online() => vec!["--handicap".into(), f],
            _ => vec![],
        }
    }
}

/// Player `p`'s handicap pill at the right end of the card's footer `[x, y, w, h]` on the local confirm screen; the
/// footer width left of it.
pub(super) fn pill(d: &mut Draw, m: &Menu, p: usize, [x, y, w, h]: [f32; 4]) -> f32 {
    if m.online() || m.screen != Screen::Confirm || !m.players[p].ready {
        return w;
    }
    let h_ = m.players[p].handicap;
    let (s, words) = (12.0 * PY, label(h_));
    let pw = d.width(words, s) + 20.0 * PX;
    let rect = [x + w - pw, y, pw, h];
    let (fill, ink) = if h_ == 0 { (hex(0x31415c), hex(0xb8c4d8)) } else { (hex(0xe8743b), hex(0xffffff)) };
    rounded(d, rect, 10.0, fill, 128.0);
    d.centred(words, x + w - pw / 2.0, y + (h - s) / 2.0, s, ink);
    d.hit(rect, pointer::Act::Row(CONFIRM_ROWS + p, Press { card: true, ..Default::default() }));
    w - pw - 8.0 * PX
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::{App, AssetApp, AssetPlugin, Entity, Image, Mesh, MinimalPlugins, StandardMaterial, Startup, Transform, Visibility, With, default};

    /// △ on a plate steps its player's handicap round (their own card or a computer's, not another person's), and
    /// the match gets it by match slot; the setup gives each body its level and hangs the weights on the calves.
    #[test]
    fn the_confirm_screen_sets_each_players_handicap() {
        let triangle = Press { card: true, ..default() };
        let pad = Dev::Pad(Entity::from_raw_u32(7).unwrap());
        let mut m = Menu { screen: Screen::Confirm, seats: [Some(Dev::Keys), Some(pad), None, None], doubles: true, ..default() };
        for p in &mut m.players {
            p.ready = true;
        }
        m.sel = CONFIRM_ROWS + 1;
        m.step(Dev::Keys, triangle);
        assert_eq!(m.players[1].handicap, 0, "2P's card is 2P's");
        assert!(!m.handicap_flags(&[0, 2, 1, 3]).iter().any(|f| f == "--handicap"), "nobody handicapped: no flag");
        for (sel, presses) in [(CONFIRM_ROWS, 4), (CONFIRM_ROWS + 2, 2)] {
            m.sel = sel;
            for _ in 0..presses {
                m.step(Dev::Keys, triangle);
            }
        }
        assert_eq!([m.players[0].handicap, m.players[2].handicap], [1, 2], "None → 1 → 2 → None → 1");
        let a = m.args(&[]);
        let flag = |f: &str| a.iter().position(|x| x == f).map(|i| a[i + 1].clone());
        // menu player 2 (1P's partner, a computer) is match slot 1
        assert_eq!(flag("--handicap").as_deref(), Some("1,2,0,0"));

        const ISO: &str = hst_data::fixture::ISO_PATH;
        if !hst_data::fixture::has_iso() {
            return;
        }
        let cli = crate::parse([ISO, "--play", "--singles", "--chars", "0,6", "--handicap", "1,2"].into_iter().map(String::from));
        crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(cli.args).insert_resource(cli.swap_hands).insert_resource(cli.handicaps);
        app.init_resource::<super::super::super::Pads>().init_resource::<super::super::super::tick_input::TickPads>();
        app.world_mut().spawn((crate::GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, super::super::super::setup);
        app.update();
        let g = app.world().resource::<super::super::super::Game>();
        assert_eq!([0, 1].map(|i| crate::play::handicap::size(g, i)), [80, 70]);
        let parts = app.world_mut().query_filtered::<Entity, With<crate::character::WornGs>>().iter(app.world()).count();
        assert!(parts > 0 && parts.is_multiple_of(3), "one weight's parts on Ashley, two on Carol: {parts}");
    }
}

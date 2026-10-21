//! B40g1: Set Handicap on the local confirm screen. △ on a player's card (their own, or a computer player's) steps it
//! None → one weight → two → None, shown in a pill on the card's footer (a tap steps it too); the match gets it as
//! `--handicap` by match slot (`play::handicap`). B40g3: online the host sets them in the lobby: L2 / R2 on a taken
//! seat's card (a person's or the computer's) steps its player's level down / up, the room carries it on the seat's
//! pick to every peer (`net::Cmd::Handicap`) and each peer's match takes it from there (`netplay::flags`), so every
//! peer's bodies run alike. The cards show a handicap set in a pill.

use super::roster::{PX, PY, rounded};
use super::{CONFIRM_ROWS, Dev, Draw, Effect, Menu, Press, Screen, pointer};
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

    /// Player `p`'s handicap as the cards show it: online the room's on its seat (the host's), else the confirm
    /// screen's.
    pub(super) fn handicap_of(&self, p: usize) -> u8 {
        match self.net.room.as_ref().filter(|_| self.online()) {
            Some(r) => r.seats.get(p).cloned().flatten().map_or(0, |s| s.pick.handicap),
            None => self.players[p].handicap,
        }
    }

    /// B40g3: the taken seat whose handicap the host's L2 / R2 steps in the lobby: the one under the cursor, while
    /// the host arranges (no prompt up, no countdown, not started).
    pub(super) fn lobby_handicap_seat(&self) -> Option<usize> {
        self.lobby_focus().filter(|&i| self.lobby_handicaps(i))
    }

    /// B611: whether the host can step seat `i`'s handicap in the lobby now: it's taken, the host arranges (no
    /// prompt up, no countdown, not started).
    pub(super) fn lobby_handicaps(&self, i: usize) -> bool {
        self.net.hosting
            && !self.net.started
            && self.screen == Screen::Lobby
            && !self.lobby_asking()
            && self.counting() == 0
            && self.net.room.as_ref().is_some_and(|r| r.seats.get(i).is_some_and(|s| s.is_some()))
    }

    /// B40g3: the host's L2 / R2 on a seat's card in the lobby: its player's handicap one down / up (round None, one
    /// weight, two), sent to the lobby. False: not a handicap press here.
    pub(super) fn lobby_handicap(&mut self, k: Press, out: &mut Vec<Effect>) -> bool {
        let Some(i) = self.lobby_handicap_seat().filter(|_| k.kind_prev != k.kind_next) else { return false };
        let h = (self.handicap_of(i) + if k.kind_next { 1 } else { 2 }) % 3;
        out.extend([Effect::Sound(0), Effect::Online(super::online::Fx::Net(crate::net::Cmd::Handicap(i, h)))]);
        true
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

/// B611: what a tap on a lobby card's handicap pill presses on its seat: R2, one up.
fn up() -> Press {
    Press { kind_next: true, ..Default::default() }
}

/// B611: the lobby's handicap pills' taps on top of the seats' (`lobby::over` hits whole cards after the cards drew).
pub(super) fn lift(d: &mut Draw, m: &Menu) {
    let pill = |a: &pointer::Act| m.screen == Screen::Lobby && matches!(*a, pointer::Act::Row(_, k) if k == up());
    let (pills, mut hits): (Vec<_>, Vec<_>) = std::mem::take(&mut d.hits).into_iter().partition(|h| pill(&h.act));
    hits.extend(pills);
    d.hits = hits;
}

/// Player `p`'s handicap pill at the right end of the card's footer `[x, y, w, h]` on the local confirm screen and,
/// for the host, on each taken seat's card in the lobby (a tap steps it), and online on the lobby and select while one
/// is set; the footer width left of it.
pub(super) fn pill(d: &mut Draw, m: &Menu, p: usize, [x, y, w, h]: [f32; 4]) -> f32 {
    let h_ = m.handicap_of(p);
    let local = !m.online() && m.screen == Screen::Confirm && m.players[p].ready;
    let host = m.online() && m.lobby_handicaps(p);
    if !local && !host && !(m.online() && h_ > 0 && matches!(m.screen, Screen::Lobby | Screen::Chars)) {
        return w;
    }
    let (s, words) = (12.0 * PY, label(h_));
    let pw = d.width(words, s) + 20.0 * PX;
    let rect = [x + w - pw, y, pw, h];
    let (fill, ink) = if h_ == 0 { (hex(0x31415c), hex(0xb8c4d8)) } else { (hex(0xe8743b), hex(0xffffff)) };
    rounded(d, rect, 10.0, fill, 128.0);
    d.centred(words, x + w - pw / 2.0, y + (h - s) / 2.0, s, ink);
    if local {
        d.hit(rect, pointer::Act::Row(CONFIRM_ROWS + p, Press { card: true, ..Default::default() }));
    }
    if let Some(row) = m.seat_row(p).filter(|_| host) {
        d.hit(rect, pointer::Act::Row(row, up()));
    }
    w - pw - 8.0 * PX
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::{App, AssetApp, AssetPlugin, Entity, Image, Mesh, MinimalPlugins, StandardMaterial, Startup, Transform, Visibility, With, default};

    /// B40g3: online the host's L2 / R2 on any taken seat's card in the lobby steps that player's handicap (sent to
    /// the lobby, which carries it); a guest's do nothing, nor an open seat's; the cards show the room's level.
    #[test]
    fn the_lobby_host_sets_each_players_handicap() {
        use super::super::online::{Fx, Go, sample};
        use crate::net::Cmd;
        let (r2, l2) = (Press { kind_next: true, ..default() }, Press { kind_prev: true, ..default() });
        let net = |fx: Vec<Effect>| fx.iter().find_map(|e| if let Effect::Online(Fx::Net(c)) = e { Some(c.clone()) } else { None });
        let mut m = Menu { screen: Screen::Lobby, ..default() };
        sample(&mut m);
        (m.net.me, m.seats[0], m.net.go) = ("host".into(), Some(Dev::Keys), Some(Go::Host));
        let seat = |m: &mut Menu, i: usize| m.sel_seat(i);
        seat(&mut m, 1);
        assert!(super::super::legend::acts(&m, Dev::Keys).iter().any(|(_, w)| w == "Handicap"));
        assert_eq!(net(m.step(Dev::Keys, r2)), Some(Cmd::Handicap(1, 1)));
        assert_eq!(net(m.step(Dev::Keys, l2)), Some(Cmd::Handicap(1, 2)), "down from none: two");
        m.net.room.as_mut().unwrap().seats[1].as_mut().unwrap().pick.handicap = 2;
        assert_eq!((m.handicap_of(1), m.handicap_of(0)), (2, 0), "the room's");
        assert_eq!(net(m.step(Dev::Keys, r2)), Some(Cmd::Handicap(1, 0)));
        seat(&mut m, 3);
        assert_eq!(net(m.step(Dev::Keys, r2)), None, "an open seat");
        seat(&mut m, 0);
        assert_eq!(net(m.step(Dev::Keys, r2)), Some(Cmd::Handicap(0, 1)), "the host's own");
        m.net.hosting = false;
        assert_eq!(net(m.step(Dev::Keys, r2)), None, "a guest's");
    }

    /// B611: the host's tap on a lobby card's handicap pill (a person's or the computer's, the host's own full card
    /// or a compact one) steps that player's handicap, over the card's own hit; a guest's cards have no such pill.
    #[test]
    fn a_tap_on_a_lobby_pill_steps_the_handicap() {
        use super::super::online::{Fx, Go, sample};
        use super::super::pointer::{Act, hit_at, tests::hits};
        use crate::net::Cmd;
        let mut m = Menu { screen: Screen::Lobby, ..default() };
        sample(&mut m);
        (m.net.me, m.seats[0], m.net.go) = ("host".into(), Some(Dev::Keys), Some(Go::Host));
        let taken: Vec<usize> = (0..4).filter(|&i| m.lobby_handicaps(i)).collect();
        assert!(taken.len() >= 2 && taken.contains(&0), "{taken:?}");
        for i in taken {
            let row = m.seat_row(i).unwrap();
            let h = hits(&m).0;
            let pill = h.iter().find(|h| h.act == Act::Row(row, up())).unwrap_or_else(|| panic!("seat {i}: no pill"));
            let [x, y, w, hh] = pill.rect;
            let at = hit_at(&h, bevy::prelude::Vec2::new((x + w / 2.0) / 640.0, (y + hh / 2.0) / 448.0), 4.0 / 3.0, 1.0, 0.0);
            assert_eq!(at, Some(pill.act), "seat {i}: the pill on top");
            let fx = m.point(Dev::Keys, pill.act, true);
            let net = fx.iter().find_map(|e| if let Effect::Online(Fx::Net(c)) = e { Some(c.clone()) } else { None });
            assert_eq!(net, Some(Cmd::Handicap(i, 1)), "seat {i}");
        }
        m.net.hosting = false;
        assert!(!hits(&m).0.iter().any(|h| matches!(h.act, Act::Row(_, k) if k == up())), "a guest's");
    }

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

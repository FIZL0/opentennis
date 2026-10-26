//! M149: each input's last pick (character or Random, costume, mod character, hand), kept by the keyboard or pad that
//! made it, not by its seat. Whenever that input readies on its own seat the pick is noted; the next select it takes
//! a seat in (a new local match, the controller assignment's way in, an online lobby, a join with ✕) opens its cursor
//! there. Saved in `picks.txt` beside `settings.txt`, so it survives a restart. A mod that's gone puts the cursor on
//! the default pick (the Random card); a costume that's gone, on that character's first. Computer players keep B269's
//! `Menu::last`.
//!
//! A pad is known by its vendor and product ids and, for two of the same model, its place among them (the lowest
//! entity first): `pad 054c:09cc 0`. The keyboard is `keys`.

use bevy::prelude::*;

use super::{Dev, GRID, Menu, Who};

const FILE: &str = "picks.txt";

/// One input's last pick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Saved {
    /// `random`, else a character's favorites key (`hst 3`, `mod <folder>`).
    who: String,
    costume: usize,
    /// The hand toggle (`Player::swap`).
    swap: bool,
}

/// The saved picks by input key, and the keys of the pads plugged in now.
#[derive(Clone, Debug, Default)]
pub(super) struct Picks {
    saved: std::collections::BTreeMap<String, Saved>,
    pads: Vec<(Entity, String)>,
    dirty: bool,
}

fn path(iso: &str) -> std::path::PathBuf {
    super::Settings::path(iso).with_file_name(FILE)
}

/// One line per input: `<input>\t<who>\t<costume>\t<0|1 hand toggle>`.
fn parse(text: &str) -> std::collections::BTreeMap<String, Saved> {
    text.lines()
        .filter_map(|l| {
            let mut f = l.split('\t');
            let (dev, who, costume, swap) = (f.next()?, f.next()?, f.next()?.parse().ok()?, f.next()?);
            Some((dev.to_string(), Saved { who: who.to_string(), costume, swap: swap == "1" }))
        })
        .collect()
}

fn text(saved: &std::collections::BTreeMap<String, Saved>) -> String {
    saved.iter().map(|(d, s)| format!("{d}\t{}\t{}\t{}\n", s.who, s.costume, s.swap as u8)).collect()
}

/// The saved picks (none if there's no file yet).
pub(super) fn load(iso: &str) -> Picks {
    let saved = crate::store::read_to_string(path(iso)).map(|t| parse(&t)).unwrap_or_default();
    Picks { saved, ..default() }
}

/// The readable pads' keys, `pads` sorted (two of one model told apart by their order).
fn pad_keys(pads: &[(Entity, (u16, u16))]) -> Vec<(Entity, String)> {
    let mut seen: Vec<(u16, u16)> = Vec::new();
    pads.iter()
        .map(|&(e, id)| {
            let n = seen.iter().filter(|&&s| s == id).count();
            seen.push(id);
            (e, format!("pad {:04x}:{:04x} {n}", id.0, id.1))
        })
        .collect()
}

impl Menu {
    /// The key input `d` is saved under (`None`: an open seat, or a pad not seen yet).
    fn input_key(&self, d: Dev) -> Option<String> {
        match d {
            Dev::Keys => Some("keys".into()),
            Dev::Pad(e) => self.last_picks.pads.iter().find(|(x, _)| *x == e).map(|(_, k)| k.clone()),
            Dev::Open => None,
        }
    }

    /// Notes the pick of every input ready on its own seat; true when one changed.
    pub(super) fn note_picks(&mut self) -> bool {
        let mut changed = false;
        for p in 0..self.players() {
            let pl = self.players[p];
            let Some(key) = self.seats[p].filter(|_| pl.ready).and_then(|d| self.input_key(d)) else { continue };
            let who = if pl.random { "random".into() } else { self.fav_key(self.who(p)) };
            let s = Saved { who, costume: pl.costume, swap: pl.swap };
            if self.last_picks.saved.get(&key) != Some(&s) {
                self.last_picks.saved.insert(key, s);
                changed = true;
            }
        }
        self.last_picks.dirty |= changed;
        changed
    }

    /// Seat `p`'s input's last pick on its cursor, not ready; nothing if it has none (it stays as it was).
    pub(super) fn recall(&mut self, p: usize) {
        let Some(s) = self.seats[p].and_then(|d| self.input_key(d)).and_then(|k| self.last_picks.saved.get(&k).cloned()) else { return };
        let who = match s.who.as_str() {
            "random" => None,
            w => match self.saved_who(w) {
                Some(w) => Some(w),
                // the mod is gone: the default pick
                None => {
                    self.players[p] = super::Player { swap: s.swap, ..super::Player::fresh() };
                    return;
                }
            },
        };
        self.players[p] = super::Player { swap: s.swap, ..super::Player::fresh() };
        self.put(p, who);
        if let Some(w) = who {
            self.players[p].costume = if s.costume < self.costumes(w) { s.costume } else { 0 };
        }
        self.skip_taken();
    }

    /// [`recall`](Self::recall) for every seat with an input on it.
    pub(super) fn recall_all(&mut self) {
        for p in 0..4 {
            self.recall(p);
        }
    }

    /// A favorites key back to a character this machine has (its own mods only, and none in a mods-off lobby).
    fn saved_who(&self, key: &str) -> Option<Who> {
        if let Some(c) = key.strip_prefix("hst ") {
            return c.parse().ok().filter(|c| GRID.iter().any(|g| g.0 == *c)).map(Who::Disc);
        }
        let folder = key.strip_prefix("mod ")?;
        (0..self.own_mods()).find(|&i| self.mods[i].folder() == folder).filter(|_| !self.vanilla_lobby()).map(Who::Mod)
    }
}

/// Keeps the pads' keys current, notes the picks readied, writes `picks.txt` when it changes.
pub(super) fn track(mut menu: ResMut<Menu>, gamepads: Query<(Entity, &Gamepad)>, args: Res<crate::Args>) {
    let mut pads: Vec<_> = gamepads.iter().filter(|(_, g)| super::controls::readable(g)).map(|(e, g)| (e, (g.vendor_id().unwrap_or(0), g.product_id().unwrap_or(0)))).collect();
    pads.sort();
    let keys = pad_keys(&pads);
    let m = menu.bypass_change_detection();
    if m.last_picks.pads != keys {
        m.last_picks.pads = keys;
    }
    m.note_picks();
    if std::mem::take(&mut m.last_picks.dirty) && std::env::var_os("HST_MENU").is_none() {
        let p = path(&args.iso);
        if let Err(e) = crate::store::write(&p, text(&m.last_picks.saved)) {
            warn!("{}: {e}", p.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{COSTUMES, Player, Screen};
    use super::*;

    fn pad(n: u32) -> Dev {
        Dev::Pad(Entity::from_raw_u32(n).unwrap())
    }

    /// A menu with the keyboard and two pads of one model plugged in.
    fn menu() -> Menu {
        let mut m = Menu::default();
        m.last_picks.pads = pad_keys(&[(Entity::from_raw_u32(7).unwrap(), (0x054c, 0x09cc)), (Entity::from_raw_u32(9).unwrap(), (0x054c, 0x09cc))]);
        m
    }

    fn on(m: &mut Menu, p: usize, ch: usize, costume: usize, swap: bool) {
        m.put(p, Some(Who::Disc(ch)));
        (m.players[p].costume, m.players[p].swap, m.players[p].ready) = (costume, swap, true);
    }

    #[test]
    fn two_pads_of_one_model_have_their_own_keys() {
        let m = menu();
        assert_eq!(m.input_key(pad(7)).as_deref(), Some("pad 054c:09cc 0"));
        assert_eq!(m.input_key(pad(9)).as_deref(), Some("pad 054c:09cc 1"));
        assert_eq!(m.input_key(Dev::Keys).as_deref(), Some("keys"));
        assert_eq!(m.input_key(Dev::Open), None);
    }

    #[test]
    fn the_file_round_trips() {
        let mut m = menu();
        m.seats = [Some(Dev::Keys), Some(pad(9)), None, None];
        on(&mut m, 0, 4, 2, true);
        m.players[1] = Player { ready: true, ..Player::fresh() };
        assert!(m.note_picks());
        assert!(!m.note_picks(), "nothing new");
        let t = text(&m.last_picks.saved);
        assert_eq!(t, "keys\thst 4\t2\t1\npad 054c:09cc 1\trandom\t0\t0\n");
        assert_eq!(parse(&t), m.last_picks.saved);
    }

    /// Local: the keyboard is 1P, a pad joins; both ready and play. Back from the match (the menu as it was kept), a
    /// new local match's select opens the keyboard on its pick, and the pad on its own when it joins — on the seat
    /// it gets this time, not the one it had.
    #[test]
    fn a_local_match_after_another_starts_each_input_on_its_last_pick() {
        let mut m = menu();
        m.local_start(Dev::Keys, &mut Vec::new());
        assert_eq!(m.screen, Screen::Chars);
        m.step(pad(7), super::super::Press { ok: true, ..default() });
        assert_eq!(m.seats[1], Some(pad(7)));
        on(&mut m, 0, 5, 1, false);
        on(&mut m, 1, 2, 3, true);
        m.note_picks();
        // the match, then back to the main menu (Kept) and Local again with the pad as 1P this time
        let mut m = Menu { screen: Screen::Main, ..m.clone() };
        m.local_start(pad(7), &mut Vec::new());
        assert_eq!((m.cell(0), m.players[0].costume, m.players[0].swap, m.players[0].ready), (Some(Who::Disc(2)), 3, true, false));
        m.step(Dev::Keys, super::super::Press { ok: true, ..default() });
        assert_eq!(m.seats[1], Some(Dev::Keys));
        assert_eq!((m.cell(1), m.players[1].costume, m.players[1].swap), (Some(Who::Disc(5)), 1, false));
        // a pad with no pick yet starts as before: the Random card
        let mut m = Menu { screen: Screen::Main, ..m.clone() };
        m.local_start(pad(9), &mut Vec::new());
        assert!(m.players[0].random);
    }

    /// Online: the keyboard's pick from a local match opens the lobby's select (Quickplay, Host, a code), and a pad
    /// joining the lobby gets its own.
    #[test]
    fn an_online_select_starts_each_input_on_its_last_pick() {
        let mut m = menu();
        m.seats = [Some(Dev::Keys), Some(pad(9)), None, None];
        on(&mut m, 0, 7, 2, false);
        on(&mut m, 1, 1, 1, true);
        m.note_picks();
        let mut m = Menu { screen: Screen::Online, players: [Player::default(); 4], ..m.clone() };
        m.seats = [Some(Dev::Keys), None, None, None];
        m.net.go = Some(super::super::online::Go::Host);
        m.online_enter(&mut Vec::new());
        assert_eq!(m.screen, Screen::Chars);
        assert_eq!((m.cell(0), m.players[0].costume, m.players[0].ready), (Some(Who::Disc(7)), 2, false));
        let host = crate::net::Seat { peer: m.net.me.clone(), pick: crate::net::Pick::disc(7, 2) };
        m.doubles = true;
        m.net.room = Some(crate::net::Room { doubles: true, seats: vec![Some(host), None, None, None], ..default() });
        assert!(m.lobby_join(pad(9), &mut Vec::new()));
        let i = (0..4).find(|&i| m.seats[i] == Some(pad(9))).unwrap();
        assert_eq!((m.cell(i), m.players[i].costume, m.players[i].swap), (Some(Who::Disc(1)), 1, true));
    }

    #[test]
    fn a_missing_mod_or_costume_falls_back() {
        let mut m = menu();
        m.seats = [Some(Dev::Keys), Some(pad(7)), None, None];
        m.last_picks.saved.insert("keys".into(), Saved { who: "mod gone".into(), costume: 1, swap: true });
        m.last_picks.saved.insert("pad 054c:09cc 0".into(), Saved { who: "hst 3".into(), costume: COSTUMES + 4, swap: false });
        m.recall_all();
        assert!(m.players[0].random && m.players[0].swap, "the default pick, hand kept");
        assert_eq!((m.cell(1), m.players[1].costume), (Some(Who::Disc(3)), 0));
    }

    /// Computer seats keep B269's: a recall touches only seats with an input on them.
    #[test]
    fn computer_seats_are_left_alone() {
        let mut m = menu();
        m.last_picks.saved.insert("keys".into(), Saved { who: "hst 3".into(), costume: 1, swap: false });
        m.seats = [Some(Dev::Keys), None, None, None];
        m.players[1] = Player { cursor: 6, ..Player::fresh() };
        let was = m.players[1];
        m.recall_all();
        assert_eq!(m.players[1], was);
        assert_eq!(m.cell(0), Some(Who::Disc(3)));
    }
}

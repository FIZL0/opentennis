//! M153: Reload brings the mods in without a restart. The installed character mods are read again and graded one a
//! frame (each its TParam.csv row, play style and △ grades, as the startup roster's `read_text`), then, once no mod
//! picture is loading or drawing, the roster is swapped in place: the umpire and court mods read again, and everything
//! that held a roster index (players' mod picks, the last readied cell, broken costumes, the umpire and court picks,
//! the select's pictures and faces, the match's early loads) follows its mod by folder, else by id. A pick of a mod
//! that's gone falls back to the Random card (an umpire or court, to Random). An overlay over the menu shows it (n of N
//! mods) and takes every press until the swap. The new and changed mods' pictures then draw in the background on any
//! screen (`previews::reroster`). Reload is only offered on the calm screens (`transfers::calm`): never on the select, in a
//! lobby or a match, where players hold picks.

use bevy::prelude::*;
use hst_data::iso::Iso;

use super::super::{Anchor, Art, Draw, GOLD, INK, Icons, Menu, Player, TParam, Tex, WHITE, Who, previews};
use super::super::{COURTS, UMPIRES};
use crate::Args;
use crate::mods::Mod;

/// A select card's play style and △ grades (`Text::mods`'s entries).
type Card = (usize, [u8; 5]);

/// A reload under way.
#[derive(Clone, Debug, Default)]
pub(in crate::play::main_menu) struct Reloading {
    /// The character mods installed now, and the cards of those graded so far (None: its row can't be read, left out
    /// as at startup).
    found: Vec<Mod>,
    cards: Vec<Option<Card>>,
}

impl Reloading {
    /// The mods in folder `root` (none with `HST_NO_MODS`, as at startup).
    fn start(root: &std::path::Path) -> Self {
        let found = if std::env::var_os("HST_NO_MODS").is_some() { Vec::new() } else { crate::mods::list(root) };
        Reloading { found, ..default() }
    }

    /// The overlay's line and its bar's share.
    fn progress(&self) -> (String, f32) {
        let all = self.found.len();
        (format!("Reading mods {} of {all}", (self.cards.len() + 1).min(all)), self.cards.len() as f32 / all.max(1) as f32)
    }
}

impl Menu {
    /// Reload: the mods as they are now, read in while the overlay is up ([`run`]).
    pub(in crate::play::main_menu) fn reload_live(&mut self) {
        self.add.reloading = Some(Reloading::start(super::root()));
        (self.add.ask, self.add.play) = (None, None);
        self.add.child = None;
    }

    /// The new roster `mods` (its select cards `cards`), umpire and court mods in place of the old: every roster index
    /// the menu holds follows its mod; a gone one's pick falls back (a player to the Random card, an umpire or court to
    /// Random). `text` is the select's cards (`Text::mods`). Lent lobby mods (after the roster) go with the old.
    fn swap_roster(&mut self, text: &mut Vec<Card>, mods: Vec<Mod>, cards: Vec<Card>, umpires: Vec<crate::mods_umpire::Umpire>, courts: Vec<crate::mods_court::Court>) {
        let own = self.own_mods();
        let to: Vec<Option<usize>> = self.mods[..own].iter().map(|o| mods.iter().position(|n| n.dir == o.dir).or_else(|| mods.iter().position(|n| n.id == o.id))).collect();
        let at = |i: usize| to.get(i).copied().flatten();
        let who = |w: Who| match w {
            Who::Mod(i) => at(i).map(Who::Mod),
            disc => Some(disc),
        };
        for pl in &mut self.players {
            if !pl.modded {
                continue;
            }
            match at(pl.pick) {
                Some(j) => {
                    pl.pick = j;
                    if pl.costume >= mods[j].costumes.len().max(1) {
                        pl.costume = 0;
                    }
                }
                None => *pl = Player { swap: pl.swap, handicap: pl.handicap, ..Player::fresh() },
            }
        }
        self.last = match self.last {
            Some(Some(w)) => who(w).map(Some),
            last => last,
        };
        self.followed = [None; 4];
        // the costumes found broken are found again (`previews::reroster` checks the files again)
        self.broken.clear();
        if let Some(k) = (self.umpire as usize).checked_sub(UMPIRES + 1) {
            let id = self.umpire_mods.get(k).map(|u| (u.dir.clone(), u.id.clone()));
            let j = id.and_then(|(dir, id)| umpires.iter().position(|u| u.dir == dir).or_else(|| umpires.iter().position(|u| u.id == id)));
            self.umpire = j.map_or(UMPIRES, |j| UMPIRES + 1 + j) as u8;
        }
        if let Some(k) = self.court.checked_sub(COURTS + 1) {
            let id = self.court_mods.get(k).map(|c| (c.dir.clone(), c.id.clone()));
            let j = id.and_then(|(dir, id)| courts.iter().position(|c| c.dir == dir).or_else(|| courts.iter().position(|c| c.id == id)));
            self.court = j.map_or(COURTS, |j| COURTS + 1 + j);
        }
        (self.umpire_mods, self.court_mods) = (umpires, courts);
        // their pictures read again (`umpire_mods::load`, `court_mods::load`): an updated one keeps its folder
        (self.umpire_pics, self.umpire_voices, self.court_pics) = (Vec::new(), Vec::new(), Vec::new());
        self.newest = super::super::newest(&mods);
        (self.mods, *text) = (mods, cards);
        let v = &mut self.add;
        (v.changed, v.fresh, v.due, v.without, v.ask, v.play) = (false, false, false, false, None, None);
    }
}

/// Steps a reload: a mod graded a frame, then the swap once no mod picture is loading or drawing.
#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    mut menu: ResMut<Menu>,
    args: Res<Args>,
    art: Option<ResMut<Art>>,
    csv: Option<Res<TParam>>,
    loader: Option<ResMut<previews::Loader>>,
    baker: Option<ResMut<previews::Baker>>,
    (mut icons, mut card): (ResMut<Icons>, ResMut<previews::ModCard>),
) {
    let (Some(r), Some(mut art), Some(csv)) = (menu.add.reloading.as_ref(), art, csv) else { return };
    let (graded, all) = (r.cards.len(), r.found.len());
    if graded < all {
        let m = &r.found[graded];
        let text = &art.2;
        let got = Iso::open(&args.iso).map_err(|e| e.to_string()).and_then(|mut iso| {
            let row = crate::mods::tparam(&mut iso, m)?;
            let gain = crate::grades::power_gain(&mut iso, m);
            Ok((super::super::mod_style(m, &text.style), super::super::mod_grades(m, &row, &csv.0, gain, &text.grades)))
        });
        let got = got.map_err(|e| warn!("{}: {e}", m.dir.display())).ok();
        menu.add.reloading.as_mut().unwrap().cards.push(got);
        return;
    }
    // the swap waits for any mod picture loading or drawing under its old index
    let (Some(mut loader), Some(mut baker)) = (loader, baker) else { return swap(&mut menu, &mut art.2.mods, None) };
    baker.hold = true;
    if !previews::mods_idle(&loader, &baker) {
        return;
    }
    swap(&mut menu, &mut art.2.mods, Some((&args.iso, &mut loader, &mut baker, &mut icons, &mut card)));
}

/// The parts of the select's pictures a swap re-maps.
type Pictures<'a> = (&'a str, &'a mut previews::Loader, &'a mut previews::Baker, &'a mut Icons, &'a mut previews::ModCard);

/// The graded roster swapped in, the pictures re-mapped and the new ones queued; the overlay goes.
fn swap(menu: &mut Menu, text: &mut Vec<Card>, pics: Option<Pictures>) {
    let Some(r) = menu.add.reloading.take() else { return };
    let (mods, cards): (Vec<Mod>, Vec<Card>) = r.found.into_iter().zip(r.cards).filter_map(|(m, c)| c.map(|c| (m, c))).unzip();
    let root = super::root();
    let (umpires, courts) = if std::env::var_os("HST_NO_MODS").is_some() { (Vec::new(), Vec::new()) } else { (crate::mods_umpire::list(root), crate::mods_court::list(root)) };
    info!("reloaded mods: {} custom characters, {} umpires, {} courts", mods.len(), umpires.len(), courts.len());
    menu.swap_roster(text, mods, cards, umpires, courts);
    menu.umpire_in_range();
    menu.court_in_range();
    menu.add.refresh();
    crate::preload::forget_mods();
    if let Some((iso, loader, baker, icons, card)) = pics {
        previews::reroster(iso, menu, loader, baker, icons, card);
    }
    menu.add.status = format!("Mods reloaded: {} characters, {} courts, {} umpires.", menu.mods.len(), menu.court_mods.len(), menu.umpire_mods.len());
}

/// The overlay over whatever screen is up while a reload runs: the menu dimmed, a panel with what it's doing and a bar.
/// Whether it drew.
pub(super) fn layout(d: &mut Draw, m: &Menu) -> bool {
    let Some(r) = &m.add.reloading else { return false };
    use super::super::super::menu::PANEL;
    if let Some(i) = d.out.iter().rposition(|q| q.tex == Tex::Cursor) {
        d.out.remove(i);
    }
    d.popup(None);
    d.anchor = Anchor::Span;
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 640.0, 448.0], [0.0; 3], 80.0);
    d.anchor = Anchor::Centre;
    let top = 170.0;
    d.box3([160.0, top, 320.0, 100.0], 16.0, [Some(INK), Some(WHITE), Some(PANEL)], 128.0);
    d.outlined(|d| d.label("Reloading mods", 320.0, top + 14.0, 22.0, 288.0, WHITE));
    let (line, share) = r.progress();
    d.label(&line, 320.0, top + 44.0, 16.0, 288.0, WHITE);
    let bar = [184.0, top + 72.0, 272.0, 6.0];
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], bar, [50.0; 3], 128.0);
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [bar[0], bar[1], bar[2] * share.clamp(0.0, 1.0), bar[3]], GOLD, 128.0);
    d.said = "New mods join the select screens, updated ones change and removed ones leave.".into();
    true
}

/// `HST_MENU=reloading`: the overlay part way through (for `--shot`s).
pub(super) fn sample() -> Reloading {
    let found = (0..12).map(|i| blank(&format!("mods/m{i}"), 1)).collect();
    Reloading { found, cards: vec![None; 5] }
}

/// A character mod in folder `dir` (its id the folder's name) with `costumes` costumes and nothing else.
fn blank(dir: &str, costumes: usize) -> Mod {
    Mod {
        dir: dir.into(),
        id: dir.rsplit('/').next().unwrap_or(dir).into(),
        name: String::new(),
        source: String::new(),
        costumes: vec!["model/c00.glb".into(); costumes],
        donor: 0,
        hand: 1.0,
        base: 0,
        overrides: vec![],
        ai_row: 0,
        texture_face: false,
        no_face: false,
        voice: "voice/".into(),
        motions: None,
        costume_of: None,
        skeleton: None,
        emotes: None,
        voice_takes: None,
        upscale: true,
        build: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn md(dir: &str, costumes: usize) -> Mod {
        blank(dir, costumes)
    }

    fn id(dir: &str) -> String {
        dir.rsplit('/').next().unwrap().into()
    }

    fn ump(dir: &str) -> crate::mods_umpire::Umpire {
        crate::mods_umpire::Umpire { dir: PathBuf::from(dir), id: id(dir), name: String::new(), source: String::new(), donor: 0, voice: "voice/".into() }
    }

    fn court(dir: &str) -> crate::mods_court::Court {
        crate::mods_court::Court { dir: PathBuf::from(dir), id: id(dir), name: String::new(), source: String::new(), layout: 0, light: None, sky: false, gimmick: false, bgm: None, ambience: Vec::new(), lens_flare: None, people: None, flyover: None, scroll: Vec::new() }
    }

    fn on(m: &mut Menu, p: usize, i: usize, costume: usize) {
        m.players[p] = Player { pick: i, modded: true, costume, swap: true, ..Player::default() };
    }

    /// A mod installed ahead of others in the list, one removed, one updated in place: picks, the last readied cell,
    /// the umpire and court picks and the select's cards follow their mods; the removed mod's pick goes to Random.
    #[test]
    fn a_reload_keeps_picks_on_their_mods() {
        let mut m = Menu { mods: vec![md("mods/alex", 3), md("mods/bea", 2), md("mods/cy", 1)], ..default() };
        let mut text = vec![(0, [1; 5]), (1, [2; 5]), (2, [3; 5])];
        on(&mut m, 0, 2, 0); // Cy
        on(&mut m, 1, 1, 1); // Bea, costume 1 (gone in her update)
        on(&mut m, 2, 0, 2); // Alex (removed)
        m.players[3] = Player { cursor: 4, ..Player::default() };
        m.last = Some(Some(Who::Mod(2)));
        m.broken.insert((1, 1));
        (m.umpire_mods, m.umpire) = (vec![ump("mods/u1"), ump("mods/u2")], (UMPIRES + 2) as u8);
        (m.court_mods, m.court) = (vec![court("mods/c1")], COURTS + 1);
        m.add.changed = true;
        // a new mod sorts first; Alex is gone; Bea now has one costume
        let new = vec![md("mods/aaron", 1), md("mods/bea", 1), md("mods/cy", 1)];
        let cards = vec![(3, [4; 5]), (1, [5; 5]), (2, [6; 5])];
        m.swap_roster(&mut text, new, cards.clone(), vec![ump("mods/u0"), ump("mods/u2")], vec![court("mods/c0"), court("mods/c1")]);
        assert_eq!((m.players[0].pick, m.players[0].modded), (2, true), "Cy moved down one");
        assert_eq!((m.players[1].pick, m.players[1].costume), (1, 0), "Bea's costume 1 is gone: her first");
        let p2 = m.players[2];
        assert!(!p2.modded && p2.random && !p2.ready && p2.swap, "Alex's pick falls back to the Random card, the hand kept");
        assert_eq!(m.players[3], Player { cursor: 4, ..Player::default() }, "a disc pick stays");
        assert_eq!(m.last, Some(Some(Who::Mod(2))));
        assert!(m.broken.is_empty());
        assert_eq!(m.umpire as usize, UMPIRES + 2, "u2 is still the second");
        assert_eq!(m.court, COURTS + 2, "c1 moved down one");
        assert_eq!(text, cards);
        assert_eq!(m.mods[0].id, "aaron");
        assert!(!m.add.changed, "nothing left to reload");
        assert_eq!(m.newest.len(), 3);
    }

    /// The removed umpire's and court's picks go to Random; the last readied cell on a removed mod is forgotten.
    #[test]
    fn a_removed_mods_picks_fall_back() {
        let mut m = Menu { mods: vec![md("mods/alex", 1)], ..default() };
        let mut text = vec![(0, [1; 5])];
        m.last = Some(Some(Who::Mod(0)));
        (m.umpire_mods, m.umpire) = (vec![ump("mods/u1")], (UMPIRES + 1) as u8);
        (m.court_mods, m.court) = (vec![court("mods/c1")], COURTS + 1);
        m.swap_roster(&mut text, Vec::new(), Vec::new(), Vec::new(), Vec::new());
        assert_eq!((m.last, m.umpire as usize, m.court), (None, UMPIRES, COURTS));
        assert!(m.mods.is_empty() && text.is_empty());
        // a disc pick's last cell stays
        m.last = Some(Some(Who::Disc(3)));
        m.swap_roster(&mut text, Vec::new(), Vec::new(), Vec::new(), Vec::new());
        assert_eq!(m.last, Some(Some(Who::Disc(3))));
    }

    /// The overlay counts the mods read.
    #[test]
    fn the_overlay_counts_up() {
        assert_eq!(sample().progress(), ("Reading mods 6 of 12".into(), 5.0 / 12.0));
    }
}

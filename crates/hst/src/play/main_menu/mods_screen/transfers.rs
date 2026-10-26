//! M136: the mod downloads and uploads under way, shown on every menu screen: a pill in the upper middle, a part per
//! direction (the mod going now, its place in the batch, a bar for the whole batch), and once
//! the last download of a batch that changed the mods is in and no upload is still going, the Reload / Later
//! popup (`ask_layout`) on whichever menu screen is up, or on the next calm one. M141: once it's all done, a Reload
//! Mods button in the pill's place while the new mods wait to load, and Play local / online asks again (Reload now /
//! Play without them) if the popup was put off; nothing asks while anything is queued or going.

use super::super::pointer::{self, Act};
use super::super::{Anchor, Dev, Draw, GOLD, INK, Menu, Screen, Tex, WHITE};
use super::{Up, View};

/// The pill's place on the 640×448 screen: centred on the title band, between the title's tab and the corner's
/// words.
const X: f32 = 175.0;
const W: f32 = 290.0;
const TOP: f32 = 0.0;
const H: f32 = 22.0;

/// One direction's batch: its verb, the mod going now, how many are done, under way (side by side) and in all, and
/// the share of the batch done (the items going now counted by their own progress, where the download says it).
#[derive(Debug, PartialEq)]
pub(super) struct Strip {
    verb: &'static str,
    now: String,
    done: usize,
    going: usize,
    all: usize,
    share: f32,
}

/// "Alex" with one going, "Alex +2" with three.
fn named(first: String, going: usize) -> String {
    if going > 1 { format!("{first} +{}", going - 1) } else { first }
}

/// "45" from a download's "Downloading X... 3 / 7 MB (45%)".
fn percent(s: &str) -> Option<f32> {
    let head = s.strip_suffix("%)")?;
    let n: f32 = head[head.rfind('(')? + 1..].parse().ok()?;
    Some(n.clamp(0.0, 100.0) / 100.0)
}

/// "3/5" (the one going now is the batch's 3rd of 5), "1-3/5" (three going side by side).
fn place(s: &Strip) -> String {
    let (from, to) = ((s.done + 1).min(s.all), (s.done + s.going).clamp(1, s.all));
    if to > from { format!("{from}-{to}/{}", s.all) } else { format!("{from}/{}", s.all) }
}

impl View {
    /// The downloads' strip, while any is queued or under way.
    fn downloading(&self) -> Option<Strip> {
        let now = self.doing.first()?;
        let going: f32 = self.doing.iter().filter_map(|e| self.jobs.get(&e.key()).and_then(|j| percent(&j.1))).sum();
        let (done, all) = (self.batch.0, self.batch.1.max(self.batch.0 + self.doing.len() + self.queue.len()));
        let n = self.doing.len();
        Some(Strip { verb: "Downloading", now: named(now.meta.name.clone(), n), done, going: n, all, share: (done as f32 + going) / all.max(1) as f32 })
    }

    /// The uploads' strip, while any is queued or under way. B601: the store packs a batch's mods one after another,
    /// each under way from its start till the batch's one commit lands: the pill names the one started last, and a
    /// mod under way counts half done (packed or packing, not sent yet).
    fn uploading(&self) -> Option<Strip> {
        let (first, left) = self.uploads();
        if left == 0 {
            return None;
        }
        let going = self.ups.values().filter(|u| **u == Up::Uploading).count();
        let (done, all) = (self.up_batch.0, self.up_batch.1.max(self.up_batch.0 + left));
        let latest = self.up_now.as_ref().filter(|d| self.ups.get(*d) == Some(&Up::Uploading)).map(|dir| {
            let m = self.list.iter().chain(&self.courts).chain(&self.umpires).find(|m| m.dir == *dir);
            m.map(|m| m.name.clone()).unwrap_or_else(|| dir.file_name().unwrap_or_default().to_string_lossy().into_owned())
        });
        let now = match latest.or(first) {
            Some(n) => named(n, going),
            None => "waiting its turn".into(),
        };
        Some(Strip { verb: "Uploading", now, done, going, all, share: (done as f32 + 0.5 * going as f32) / all.max(1) as f32 })
    }

    /// Both directions' strips, downloads first.
    pub(super) fn strips(&self) -> Vec<Strip> {
        self.downloading().into_iter().chain(self.uploading()).collect()
    }

    /// M141: no download queued or going and no upload: the mods are as they'll stay, so Reload may be offered.
    pub(super) fn idle(&self) -> bool {
        self.doing.is_empty() && self.queue.is_empty() && self.uploads().1 == 0
    }

    /// An upload batch's count: `n` more queued (a new batch if none was going).
    pub(super) fn up_queued(&mut self, n: usize) {
        if self.uploads().1 == n {
            self.up_batch = (0, 0);
        }
        self.up_batch.1 += n;
    }
}

/// `HST_TRANSFERS=1` (for `--shot`): a made-up batch of each under way, on whatever `HST_MENU` screen.
pub(in crate::play::main_menu) fn sample(m: &mut Menu) {
    let Some(how) = std::env::var_os("HST_TRANSFERS") else { return };
    let v = &mut m.add;
    // `HST_TRANSFERS=ask`: the batch done instead, the popup up over the screen
    if how == "ask" {
        (v.changed, v.ask) = (true, Some(0));
        return;
    }
    // M141: `done`: the batch in, the popup put off (the Reload Mods button); `play`: Play local picked since
    if how == "done" || how == "play" {
        v.changed = true;
        if how == "play" {
            (v.ask, v.play) = (Some(0), Some(Dev::Keys));
        }
        return;
    }
    let fake = |id: &str, name: &str| {
        let meta = crate::modstore::Meta { id: id.into(), name: name.into(), author: "sample".into(), ..Default::default() };
        crate::modstore::Entry { meta, rev: String::new(), verified: false }
    };
    let now = fake("ace", "Ace Pilot");
    v.jobs.insert(now.key(), ("Downloading", "Downloading Ace Pilot... 12 / 30 MB (40%)".into()));
    v.doing = vec![now];
    v.queue = [fake("bo", "Bo"), fake("cy", "Cy")].into();
    v.batch = (2, 5);
    v.ups.insert("mods/court_garden".into(), Up::Uploading);
    v.ups.insert("mods/umpire_kit".into(), Up::Queued);
    v.up_batch = (1, 3);
}

/// A screen the popup may come up over: a menu with nothing else asking, no players picking, no lobby, no match.
fn calm(m: &Menu) -> bool {
    let v = &m.add;
    use Screen::*;
    let screen = matches!(m.screen, Main | Mode | Mods | Extras | Online | Lobbies | Code | Display | Graphics | Advanced | Sound | ModSettings | Mobile | Controls);
    screen
        && !m.paused
        && m.net.room.is_none()
        && m.over.is_none()
        && m.picking.is_none()
        && m.switch.is_none()
        && m.reset_ask.is_none()
        && m.listing.is_none()
        && m.quit.is_none()
        && m.stale.0.is_none()
        && !m.takeover_asking()
        && !m.mod_asking()
        && v.child.is_none()
        && v.wipe.is_none()
        && v.nophone.is_none()
        && v.ask.is_none()
}

/// After the store's news each frame: a download batch that changed the mods ended (`due`), and no upload is still
/// going: the popup comes up once the screen is calm (B205's ask, taken from leaving the Mods page).
pub(super) fn offer(m: &mut Menu) {
    if !m.add.due || !m.add.idle() || !calm(m) {
        return;
    }
    m.add.due = false;
    if std::mem::take(&mut m.add.fresh) {
        m.add.ask = Some(0);
    }
}

/// M141: the Reload Mods button is up: everything done, mods changed since the start, the popup down, a calm screen
/// (not the select or a lobby, where a restart would drop the players' picks).
pub(in crate::play::main_menu) fn button(m: &Menu) -> bool {
    m.add.changed && m.add.idle() && calm(m)
}

/// M141: Play local / Play online picked by `dev` with new mods waiting and everything done: unless Play without them
/// was chosen since they changed, the popup asks first (Reload now / Play without them). Whether it asked.
pub(in crate::play::main_menu) fn play_ask(m: &mut Menu, dev: Dev) -> bool {
    let v = &mut m.add;
    if !v.changed || v.without || !v.idle() || v.ask.is_some() {
        return false;
    }
    (v.ask, v.play, v.fresh) = (Some(0), Some(dev), false);
    true
}

/// M141: the cursor stays on its tile (or pill) of the Mods grid as the footer's Reload comes and goes with the
/// queue; on Reload itself as it goes, the last word before it.
pub(super) fn hold_cursor(m: &mut Menu) {
    let n = m.add.words().len();
    let was = std::mem::replace(&mut m.add.words_were, n);
    if m.screen != Screen::Mods || !m.add.grid() || was == 0 || n == was || m.sel < 2 {
        return;
    }
    m.sel = if m.sel >= 2 + was { m.sel + n - was } else { m.sel.min(1 + n) };
}

/// The pill over whatever menu screen is up (not in a match): a half each while both directions are going. M141:
/// once they've all ended with mods to load, the Reload Mods button in its place.
pub(in crate::play::main_menu) fn layout(d: &mut Draw, m: &Menu) {
    // the game's own update (Always installs it with no one pressing Update) shares the pill, after the mods
    let parts: Vec<_> = m.add.strips().iter().map(|s| (format!("{} {}: {}", s.verb, place(s), s.now), Some(s.share))).chain(crate::update::progress()).collect();
    if parts.is_empty() && button(m) {
        reload_button(d, m);
    }
    if parts.is_empty() || m.screen == Screen::Playing || m.paused {
        return;
    }
    // centred across, at the top of a tall window too
    d.anchor = Anchor::Top;
    d.box3([X, TOP, W, H], 9.0, [Some(INK), Some(WHITE), Some([24.0, 30.0, 44.0])], 118.0);
    let part = (W - 8.0) / parts.len() as f32;
    for (k, (line, share)) in parts.iter().enumerate() {
        strip(d, line, *share, X + 4.0 + part * k as f32, part);
    }
    d.anchor = Anchor::Centre;
}

/// M141: "Reload Mods" (the phone apps': "Close to load mods") in the pill's place, gold under the pointer; a tap
/// reloads.
fn reload_button(d: &mut Draw, m: &Menu) {
    d.anchor = Anchor::Top;
    let rect = [X + 50.0, TOP, W - 100.0, H + 4.0];
    let on = pointer::over(rect, Anchor::Top);
    d.box3(rect, 9.0, [Some(INK), Some(WHITE), Some(if on { [96.0, 78.0, 24.0] } else { [24.0, 30.0, 44.0] })], 118.0);
    let word = if m.add.app.0 { "Close to load mods" } else { "Reload Mods" };
    d.label(word, X + W / 2.0, TOP + 5.0, 14.0, W - 120.0, if on { WHITE } else { GOLD });
    d.hit(rect, Act::Reload);
    d.anchor = Anchor::Centre;
}

/// One part in `w` px from `x`: "Downloading 3/5: Ace Pilot" (the one going now is the batch's 3rd of 5; B601:
/// "1-3/5: Ace Pilot +2" with three side by side), or the update's words, its bar under it where it has one.
fn strip(d: &mut Draw, line: &str, share: Option<f32>, x: f32, w: f32) {
    super::super::home::fit_text(d, line, x + 6.0, TOP + 3.0, 12.0, w - 12.0, WHITE);
    let Some(share) = share else { return };
    let bar = [x + 6.0, TOP + H - 5.0, w - 12.0, 2.0];
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], bar, [50.0; 3], 128.0);
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [bar[0], bar[1], bar[2] * share.clamp(0.0, 1.0), bar[3]], super::super::GOLD, 128.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modstore::{Entry, Meta};

    fn entry(id: &str) -> Entry {
        Entry { meta: Meta { id: id.into(), name: id.to_uppercase(), author: "a".into(), ..Default::default() }, rev: String::new(), verified: false }
    }

    #[test]
    fn reads_a_downloads_percent() {
        assert_eq!(percent("Downloading Mia... 3 / 7 MB (45%)"), Some(0.45));
        assert_eq!(percent("Downloading Mia... 3 MB"), None);
        assert_eq!(percent("Checking Mia..."), None);
    }

    #[test]
    fn a_download_batch_counts_down() {
        let mut v = View::default();
        assert!(v.strips().is_empty());
        for id in ["x", "y", "z"] {
            v.enqueue(entry(id), false, 1);
        }
        v.jobs.insert(entry("x").key(), ("Downloading", "Downloading X... 1 / 2 MB (50%)".into()));
        let s = v.downloading().unwrap();
        assert_eq!((s.now.as_str(), s.done, s.all), ("X", 0, 3));
        assert!((s.share - 0.5 / 3.0).abs() < 1e-6, "{}", s.share);
        v.finish(&entry("x").key(), &Ok("X".into()));
        v.next(false, 1);
        let s = v.downloading().unwrap();
        assert_eq!((s.now.as_str(), s.done, s.all), ("Y", 1, 3));
    }

    /// B601: three downloading side by side (Downloads at once 3): their places, the first named.
    #[test]
    fn downloads_side_by_side_show_their_places() {
        let mut v = View::default();
        for id in ["x", "y", "z", "w"] {
            v.enqueue(entry(id), false, 3);
        }
        let s = v.downloading().unwrap();
        assert_eq!((place(&s), s.now.as_str()), ("1-3/4".into(), "X +2"));
        for id in ["x", "y"] {
            v.finish(&entry(id).key(), &Ok(id.into()));
        }
        v.next(false, 3);
        let s = v.downloading().unwrap();
        assert_eq!((place(&s), s.now.as_str()), ("3-4/4".into(), "Z +1"));
        v.finish(&entry("z").key(), &Ok("z".into()));
        let s = v.downloading().unwrap();
        assert_eq!((place(&s), s.now.as_str()), ("4/4".into(), "W"));
    }

    /// B601: an upload batch as the store reports it (`modstore::publish`): each mod under way as it's packed, all of
    /// them till the one commit lands, then each one's outcome. The pill names the one being packed and moves on.
    #[test]
    fn an_upload_batch_as_the_store_sends_it() {
        let mut v = View::default();
        let dirs = ["m/a", "m/b", "m/c"].map(std::path::PathBuf::from);
        for d in &dirs {
            v.ups.insert(d.clone(), Up::Queued);
        }
        v.up_queued(3);
        let mut shares = vec![v.uploading().unwrap().share];
        let mut seen = Vec::new();
        for d in &dirs {
            v.uploaded(d.clone(), Up::Uploading);
            let s = v.uploading().unwrap();
            seen.push((place(&s), s.now.clone()));
            shares.push(s.share);
        }
        assert_eq!(seen, [("1/3".into(), "a".to_string()), ("1-2/3".into(), "b +1".into()), ("1-3/3".into(), "c +2".into())]);
        v.uploaded(dirs[0].clone(), Up::Done("ok".into()));
        shares.push(v.uploading().unwrap().share);
        assert!(shares.windows(2).all(|w| w[1] > w[0]), "the bar moves on: {shares:?}");
        v.uploaded(dirs[1].clone(), Up::Done("ok".into()));
        v.uploaded(dirs[2].clone(), Up::Skipped("same".into()));
        assert!(v.strips().is_empty());
    }

    #[test]
    fn an_upload_batch_counts_down() {
        let mut v = View::default();
        let dirs = ["m/a", "m/b"].map(std::path::PathBuf::from);
        for d in &dirs {
            v.ups.insert(d.clone(), Up::Queued);
        }
        v.up_queued(2);
        v.uploaded(dirs[0].clone(), Up::Uploading);
        let s = v.uploading().unwrap();
        assert_eq!((s.now.as_str(), s.done, s.all), ("a", 0, 2));
        v.uploaded(dirs[0].clone(), Up::Done("ok".into()));
        let s = v.uploading().unwrap();
        assert_eq!((s.done, s.all), (1, 2));
        v.uploaded(dirs[1].clone(), Up::Failed("no".into()));
        assert!(v.strips().is_empty());
    }

    #[test]
    fn the_popup_waits_for_a_calm_screen() {
        let mut m = Menu { screen: Screen::Chars, ..Default::default() };
        (m.add.due, m.add.fresh) = (true, true);
        offer(&mut m);
        assert_eq!(m.add.ask, None, "not over the select");
        m.screen = Screen::Graphics;
        offer(&mut m);
        assert_eq!((m.add.ask, m.add.due, m.add.fresh), (Some(0), false, false), "on a settings page");
    }

    #[test]
    fn the_popup_waits_for_the_uploads() {
        let mut m = Menu { screen: Screen::Main, ..Default::default() };
        (m.add.due, m.add.fresh) = (true, true);
        m.add.ups.insert("m/a".into(), Up::Uploading);
        offer(&mut m);
        assert_eq!(m.add.ask, None);
        m.add.uploaded("m/a".into(), Up::Done("ok".into()));
        offer(&mut m);
        assert_eq!(m.add.ask, Some(0));
    }

    fn ok() -> crate::play::main_menu::Press {
        crate::play::main_menu::Press { ok: true, ..Default::default() }
    }

    /// M141: the Reload Mods button and the main menu's Mods note wait for the queue; then the button reloads.
    #[test]
    fn the_reload_button_waits_for_the_queue() {
        let mut m = Menu { screen: Screen::Graphics, ..Default::default() };
        m.add.changed = true;
        m.add.enqueue(entry("x"), false, 1);
        assert!(!button(&m) && !super::super::reload_due(&m), "not while a download is going");
        m.add.finish(&entry("x").key(), &Ok("X".into()));
        m.add.next(false, 1);
        m.add.ups.insert("m/a".into(), Up::Queued);
        assert!(!button(&m), "not while an upload is queued");
        m.add.uploaded("m/a".into(), Up::Done("ok".into()));
        assert!(button(&m) && super::super::reload_due(&m));
        m.add.ask = Some(0);
        assert!(!button(&m), "not under the popup");
        (m.add.ask, m.screen) = (None, Screen::Chars);
        assert!(!button(&m), "not over the select");
        m.screen = Screen::Main;
        let hits = super::super::super::pointer::tests::hits(&m).0;
        assert!(hits.iter().any(|h| matches!(h.act, Act::Reload)));
    }

    /// M141: the Mods grid's Reload comes with the queue's end, the cursor kept on its tile.
    #[test]
    fn the_footers_reload_waits_for_the_queue() {
        let mut m = Menu::default();
        (m.screen, m.add.page) = (Screen::Mods, super::super::Page::Browse);
        m.add.changed = true;
        assert!(m.add.grid());
        assert_eq!(m.add.words().last(), Some(&"Reload"));
        hold_cursor(&mut m);
        let tile = m.add.browse_acts() + 2;
        m.sel = tile;
        m.add.enqueue(entry("x"), false, 1);
        assert!(!m.add.words().contains(&"Reload"));
        hold_cursor(&mut m);
        assert_eq!(m.sel, tile - 1, "the same tile");
        m.add.finish(&entry("x").key(), &Ok("X".into()));
        m.add.next(false, 1);
        hold_cursor(&mut m);
        assert_eq!(m.sel, tile);
    }

    /// M141: leaving the Mods page mid-download doesn't ask; the end of the queue does.
    #[test]
    fn leaving_mid_download_asks_at_the_end() {
        let mut m = Menu { screen: Screen::Mods, ..Default::default() };
        m.add.changed();
        m.add.enqueue(entry("x"), false, 1);
        m.mods_back(&mut Vec::new());
        assert_eq!((m.screen, m.add.ask), (Screen::Main, None));
        offer(&mut m);
        assert_eq!(m.add.ask, None);
        m.add.finish(&entry("x").key(), &Ok("X".into()));
        m.add.next(false, 1);
        offer(&mut m);
        assert_eq!(m.add.ask, Some(0));
    }

    /// M141: Play local with the popup put off asks Reload now / Play without them; Play without them goes on to the
    /// select and Play doesn't ask again until the mods change. Nothing asks while a download is going.
    #[test]
    fn play_asks_again() {
        use super::super::super::home::Row;
        let mut m = Menu { screen: Screen::Main, ..Default::default() };
        m.sel = m.main_at(Row::Local);
        m.add.changed = true;
        m.add.enqueue(entry("x"), false, 1);
        m.step(Dev::Keys, ok());
        assert_eq!((m.screen, m.add.ask), (Screen::Chars, None), "not while queued");
        m.add.finish(&entry("x").key(), &Ok("X".into()));
        m.add.next(false, 1);
        (m.screen, m.sel) = (Screen::Main, m.main_at(Row::Local));
        m.step(Dev::Keys, ok());
        assert_eq!((m.screen, m.add.ask), (Screen::Main, Some(0)));
        assert_eq!(super::super::ask_words(&m.add).1, ["Reload now", "Play without them"]);
        m.step(Dev::Keys, crate::play::main_menu::Press { down: true, ..Default::default() });
        m.step(Dev::Keys, ok());
        assert_eq!((m.screen, m.add.ask, m.add.play), (Screen::Chars, None, None));
        (m.screen, m.sel) = (Screen::Main, m.main_at(Row::Local));
        m.step(Dev::Keys, ok());
        assert_eq!((m.screen, m.add.ask), (Screen::Chars, None), "once per change");
        m.add.changed();
        assert!(play_ask(&mut m, Dev::Keys));
        assert_eq!(super::super::ask_words(&m.add).1[1], "Play without them");
    }
}

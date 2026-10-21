//! M136: the mod downloads and uploads under way, shown on every menu screen: a pill in the upper middle, a part per
//! direction (the mod going now, its place in the batch, a bar for the whole batch), and once
//! the last download of a batch that changed the mods is in and no upload is still going, the Reload / Later
//! popup (`ask_layout`) on whichever menu screen is up, or on the next calm one.

use super::super::{Anchor, Draw, INK, Menu, Screen, Tex, WHITE};
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
    if !m.add.due || m.add.uploads().1 > 0 || !calm(m) {
        return;
    }
    m.add.due = false;
    if std::mem::take(&mut m.add.fresh) {
        m.add.ask = Some(0);
    }
}

/// The pill over whatever menu screen is up (not in a match): a half each while both directions are going.
pub(in crate::play::main_menu) fn layout(d: &mut Draw, m: &Menu) {
    let strips = m.add.strips();
    if strips.is_empty() || m.screen == Screen::Playing || m.paused {
        return;
    }
    // centred across, at the top of a tall window too
    d.anchor = Anchor::Top;
    d.box3([X, TOP, W, H], 9.0, [Some(INK), Some(WHITE), Some([24.0, 30.0, 44.0])], 118.0);
    let part = (W - 8.0) / strips.len() as f32;
    for (k, s) in strips.iter().enumerate() {
        strip(d, s, X + 4.0 + part * k as f32, part);
    }
    d.anchor = Anchor::Centre;
}

/// One direction in `w` px from `x`: "Downloading 3/5: Ace Pilot" (the one going now is the batch's 3rd of 5; B601:
/// "1-3/5: Ace Pilot +2" with three side by side), its batch's bar under it.
fn strip(d: &mut Draw, s: &Strip, x: f32, w: f32) {
    let line = format!("{} {}: {}", s.verb, place(s), s.now);
    super::super::home::fit_text(d, &line, x + 6.0, TOP + 3.0, 12.0, w - 12.0, WHITE);
    let bar = [x + 6.0, TOP + H - 5.0, w - 12.0, 2.0];
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], bar, [50.0; 3], 128.0);
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [bar[0], bar[1], bar[2] * s.share.clamp(0.0, 1.0), bar[3]], super::super::GOLD, 128.0);
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
        let mut m = Menu::default();
        m.screen = Screen::Chars;
        (m.add.due, m.add.fresh) = (true, true);
        offer(&mut m);
        assert_eq!(m.add.ask, None, "not over the select");
        m.screen = Screen::Graphics;
        offer(&mut m);
        assert_eq!((m.add.ask, m.add.due, m.add.fresh), (Some(0), false, false), "on a settings page");
    }

    #[test]
    fn the_popup_waits_for_the_uploads() {
        let mut m = Menu::default();
        m.screen = Screen::Main;
        (m.add.due, m.add.fresh) = (true, true);
        m.add.ups.insert("m/a".into(), Up::Uploading);
        offer(&mut m);
        assert_eq!(m.add.ask, None);
        m.add.uploaded("m/a".into(), Up::Done("ok".into()));
        offer(&mut m);
        assert_eq!(m.add.ask, Some(0));
    }
}

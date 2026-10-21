//! M140: the first time a run finds a newer version (Settings → Updates on Ask), the main menu asks Update? over a
//! dimmed screen, in the newer-version popup's style (`home::stale_layout`): Always (remembered: later ones install by
//! themselves), Yes, No (the notice stays put away for the run, as its Later does) or Never (remembered: no more
//! checks, Settings → Updates on Never). Like the mods' online prompt (`mods_online::Ask`), and changeable in Settings.

use super::{Draw, Effect, INK, Menu, Press, Screen, Tex, UPDATE_CHECK, WHITE, pointer};

/// The popup's pills, top to bottom.
pub(super) const CHOICES: [&str; 4] = ["Always", "Yes", "No", "Never"];
/// `CHOICES`' Yes, the one chosen when it opens.
const YES: usize = 1;
/// `CHOICES`' No (○ on the popup).
const NO: usize = 2;
/// The first pill's top (640×448).
const TOP: f32 = 196.0;

impl Menu {
    /// Each frame: a newer version found while nothing else is up on the main menu opens the popup, or (Always) starts
    /// its install.
    pub(super) fn update_offer(&mut self) {
        let free = self.screen == Screen::Main && !self.paused && self.over.is_none() && self.add.ask.is_none() && self.stale.0.is_none() && self.update_ask.is_none();
        if !free {
            return;
        }
        match crate::update::offer() {
            Some(crate::update::Due::Ask) => self.update_ask = Some(YES),
            Some(crate::update::Due::Install) => crate::update::act(),
            None => {}
        }
    }

    /// One press while the popup is up (it takes them all): up/down move, ✕ answers, ○ is No. Returns whether it was up.
    pub(super) fn update_ask_step(&mut self, k: Press, out: &mut Vec<Effect>) -> bool {
        let Some(a) = self.update_ask else { return false };
        if k.up || k.down {
            self.update_ask = Some((a as isize + if k.up { -1 } else { 1 }).rem_euclid(CHOICES.len() as isize) as usize);
            out.push(Effect::Sound(0));
        } else if k.ok {
            self.update_answer(a, out);
        } else if k.back {
            self.update_answer(NO, out);
        }
        true
    }

    /// Choice `a` of `CHOICES` taken.
    fn update_answer(&mut self, a: usize, out: &mut Vec<Effect>) {
        use std::sync::atomic::Ordering::Relaxed;
        self.update_ask = None;
        out.push(Effect::Sound(if a == NO { 1 } else { 2 }));
        match CHOICES[a] {
            "Always" => {
                self.settings.11 = true;
                crate::update::ALWAYS.store(true, Relaxed);
                out.push(Effect::SaveSettings);
                crate::update::act();
            }
            "Yes" => crate::update::act(),
            "Never" => {
                self.settings.0[UPDATE_CHECK] = false;
                crate::update::CHECK.store(false, Relaxed);
                out.push(Effect::SaveSettings);
            }
            _ => crate::update::later(),
        }
        // the notice row went with it: the hand stays on the list
        self.sel = self.sel.min(self.main_rows().len().saturating_sub(1));
    }
}

/// The popup over a dimmed main menu.
pub(super) fn layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    let Some(a) = m.update_ask else { return };
    use super::super::menu::{CHOSEN, PANEL, UNCHOSEN};
    if let Some(i) = d.out.iter().rposition(|q| q.tex == Tex::Cursor) {
        d.out.remove(i);
    }
    // a tap off the popup is ○
    d.popup(Some(pointer::Act::Row(0, Press { ok: false, back: true, ..pointer::OK })));
    d.anchor = super::Anchor::Span;
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 640.0, 448.0], [0.0; 3], 80.0);
    d.anchor = super::Anchor::Centre;
    let n = CHOICES.len() as f32;
    d.box3([160.0, TOP - 70.0, 320.0, 70.0 + 40.0 * n + 8.0], 16.0, [Some(INK), Some(WHITE), Some(PANEL)], 128.0);
    let new = crate::update::latest().unwrap_or_default();
    let says = format!("{new} is out (you have {}).", crate::update::ours());
    d.outlined(|d| d.label("Update?", 320.0, TOP - 56.0, 22.0, 288.0, WHITE));
    d.outlined(|d| d.label(&says, 320.0, TOP - 32.0, 18.0, 288.0, WHITE));
    for (k, word) in CHOICES.into_iter().enumerate() {
        let y = TOP + 40.0 * k as f32;
        let on = k == a;
        d.pill_at([176.0, y, 288.0, 32.0], [INK, WHITE, if on { CHOSEN } else { UNCHOSEN }], 128.0);
        d.hit([176.0, y, 288.0, 32.0], pointer::Act::Row(k, pointer::OK));
        d.label(word, 320.0, y + 6.0, 20.0, 160.0, if on { WHITE } else { UNCHOSEN.map(|c| c * 1.4) });
    }
    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [116.0 + hand_x, TOP + 40.0 * a as f32, 64.0, 32.0], WHITE, 128.0);
    d.said = format!("Update? {says} Always, Yes, No or Never: Settings, Updates changes it later.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::default;

    /// Down from Yes to No and ✕: put off for the run, nothing saved; Never turns the checks off and saves; ○ is No.
    #[test]
    fn answers() {
        let mut m = Menu { update_ask: Some(YES), ..default() };
        let mut out = Vec::new();
        assert!(m.update_ask_step(Press { down: true, ..default() }, &mut out));
        assert_eq!(m.update_ask, Some(NO));
        assert!(m.update_ask_step(Press { ok: true, ..default() }, &mut out));
        assert_eq!((m.update_ask, out.contains(&Effect::SaveSettings)), (None, false));
        // up from Always wraps to Never
        m.update_ask = Some(0);
        m.update_ask_step(Press { up: true, ..default() }, &mut out);
        assert_eq!(m.update_ask, Some(3));
        m.update_ask_step(Press { ok: true, ..default() }, &mut out);
        assert!(!m.settings.0[UPDATE_CHECK] && out.contains(&Effect::SaveSettings));
        m.update_ask = Some(YES);
        m.update_ask_step(Press { back: true, ..default() }, &mut out);
        assert_eq!(m.update_ask, None);
        assert!(!m.update_ask_step(Press { ok: true, ..default() }, &mut out), "closed: presses go to the menu");
        crate::update::CHECK.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

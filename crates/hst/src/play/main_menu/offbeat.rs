//! B40g2 (concept, `concept/offbeat-rules`): the local confirm screen's Offbeat Rules. Select / − (keyboard: C) opens
//! a box over the screen with a pill per rule (`play::offbeat`): ↑↓ choose, ✕ (or a tap) turns it on or off, ○ or
//! Select again closes it. The match gets them as `--offbeat`. Not online: the room doesn't carry them.

use super::pointer::Act;
use super::{Draw, Effect, Menu, Press, WHITE};

/// Each rule's pill and help line, in `Offbeat::NAMES`' order.
const RULES: [(&str, &str); 4] = [
    ("Irregular bounce", "Every bounce kicks off at an angle: up to 15° flatter and 15° to either side (20° for a lob)."),
    ("Slow motion", "The whole point plays at two-thirds speed, from the serve to the last bounce."),
    ("Turbo rally", "Each shot after the return speeds the rally up a tenth, up to double speed."),
    ("Ghost ball", "The ball vanishes once it crosses the net and shows again where it bounces. Watch the shadow."),
];

impl Menu {
    /// The legend's word for Select on the confirm screen: the rules, and how many are on.
    pub(super) fn rules_label(&self) -> Option<&'static str> {
        let on = (0..RULES.len()).filter(|&i| self.offbeat.get(i)).count();
        (!self.online()).then(|| ["Rules", "Rules (1 on)", "Rules (2 on)", "Rules (3 on)", "Rules (4 on)"][on])
    }

    /// The confirm screen's press with the rules box in mind: Select opens it; while it's up it takes every press.
    /// False: not the box's.
    pub(super) fn rules_step(&mut self, k: Press, out: &mut Vec<Effect>) -> bool {
        let Some(at) = self.rules else {
            if k.hand && !self.online() {
                self.rules = Some(0);
                out.push(Effect::Sound(2));
                return true;
            }
            return false;
        };
        if k.back || k.hand {
            self.rules = None;
            out.push(Effect::Sound(1));
        } else if k.ok {
            self.offbeat.set(at, !self.offbeat.get(at));
            out.push(Effect::Sound(0));
        } else if k.up || k.down {
            let n = RULES.len();
            self.rules = Some(if k.up { (at + n - 1) % n } else { (at + 1) % n });
            out.push(Effect::Sound(0));
        }
        true
    }

    /// `--offbeat` for the match, none online or with every rule off.
    pub(super) fn offbeat_flags(&self) -> Vec<String> {
        match self.offbeat.flag() {
            Some(f) if !self.online() => vec!["--offbeat".into(), f],
            _ => vec![],
        }
    }
}

/// The rules box over the confirm screen, while it's up: a centred box (it keeps its shape at any window size), a
/// pill per rule with its On / Off, the chosen rule's help under them.
pub(super) fn draw(d: &mut Draw, m: &Menu) {
    let Some(at) = m.rules else { return };
    d.anchor = super::Anchor::Span;
    d.q(super::Tex::Solid, [0.0; 4], [0.0, 0.0, 640.0, 448.0], [0.0; 3], 64.0);
    d.anchor = super::Anchor::Centre;
    d.popup(Some(Act::Press(Press { back: true, ..Default::default() })));
    let (sw, sh) = (300.0, 104.0 + 36.0 * RULES.len() as f32);
    let (sx, sy) = (320.0 - sw / 2.0, 224.0 - sh / 2.0);
    d.box3([sx, sy, sw, sh], 14.0, [Some(super::INK), Some(WHITE), Some(super::CREAM)], 128.0);
    d.label("Offbeat Rules", sx + sw / 2.0, sy + 14.0, 14.0, sw - 20.0, super::BROWN);
    for (i, (name, _)) in RULES.iter().enumerate() {
        let rect = [sx + 10.0, sy + 40.0 + 36.0 * i as f32, sw - 20.0, 30.0];
        let words = format!("{name}: {}", if m.offbeat.get(i) { "On" } else { "Off" });
        super::side_button(d, rect, &words, at == i, false);
        d.hit(rect, Act::Row(i, Press { ok: true, ..Default::default() }));
    }
    let help = RULES[at].1;
    for (k, l) in super::wrap(help, 52).iter().take(2).enumerate() {
        d.label(l, sx + sw / 2.0, sy + sh - 52.0 + 12.0 * k as f32, 10.0, sw - 20.0, super::BROWN);
    }
    let note = "Rules can be mixed; slow motion and turbo multiply.";
    d.label(note, sx + sw / 2.0, sy + sh - 22.0, 9.0, sw - 20.0, super::BROWN);
}

/// The rules box's pointer rows: a hover chooses the rule (`Menu::choose`).
pub(super) fn choose(m: &mut Menu, i: usize) -> Option<usize> {
    m.rules.replace(i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::offbeat::Offbeat;
    use super::super::{Dev, Screen};

    /// Select opens the box, ✕ turns the chosen rule on, ↓ ✕ the next, ○ closes it; the match gets them as
    /// `--offbeat`, and nothing online.
    #[test]
    fn the_rules_box_sets_the_matchs_rules() {
        let mut m = Menu { screen: Screen::Confirm, seats: [Some(Dev::Keys), None, None, None], sel: 0, ..Default::default() };
        let press = |f: fn(&mut Press)| {
            let mut k = Press::default();
            f(&mut k);
            k
        };
        let mut out = vec![];
        assert!(!m.rules_step(press(|k| k.ok = true), &mut out), "closed: ✕ isn't the box's");
        assert!(m.rules_step(press(|k| k.hand = true), &mut out));
        assert_eq!(m.rules, Some(0));
        m.rules_step(press(|k| k.ok = true), &mut out);
        m.rules_step(press(|k| k.down = true), &mut out);
        m.rules_step(press(|k| k.down = true), &mut out);
        m.rules_step(press(|k| k.ok = true), &mut out);
        m.rules_step(press(|k| k.back = true), &mut out);
        assert_eq!(m.rules, None);
        assert_eq!(m.offbeat, Offbeat { irregular: true, turbo: true, ..Default::default() });
        assert_eq!(m.offbeat_flags(), ["--offbeat", "irregular,turbo"]);
        assert_eq!(m.rules_label(), Some("Rules (2 on)"));
    }
}

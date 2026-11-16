//! M171: crossplay in the menus. A lobby plays its host's disc's rules (US, Japanese, European): the lobby list and
//! card say so when they aren't this disc's, and a guest joining one gets a small note listing, in player terms, how
//! those rules play differently from its own disc's (dismissed with ✕ or ○). Voices and text stay this disc's own.

use super::pointer::{Act, OK};
use super::{Draw, Effect, Menu, Press, Tex, WHITE};
use crate::handshake;

/// A lobby's kind for the list and card: "Singles", or with its host's rules when they aren't this disc's
/// ("Singles, Japanese rules").
pub(super) fn kind(doubles: bool, rules: &str) -> String {
    kind_of(doubles, rules, handshake::rules())
}

fn kind_of(doubles: bool, rules: &str, own: &str) -> String {
    let k = if doubles { "Doubles" } else { "Singles" };
    match handshake::rules_name(rules) {
        name if name.is_empty() || rules == own => k.into(),
        name => format!("{k}, {name} rules"),
    }
}

/// What the note says on joining a lobby whose host plays `rules`: its title and lines, `None` when they're this
/// disc's (`own`) or not known.
fn note(rules: &str, own: &str) -> Option<(String, Vec<&'static str>)> {
    let lines = handshake::rules_differ(rules, own);
    (!lines.is_empty()).then(|| (format!("This lobby plays the {} rules", handshake::rules_name(rules)), lines))
}

/// The first line-up of a lobby this machine joins (`rules`: its host's): the note to show (`View::rules_note`), if
/// they differ from this disc's.
pub(super) fn on_join(rules: &str) -> Option<String> {
    note(rules, handshake::rules()).map(|_| rules.to_string())
}

impl Menu {

    fn rules_note_shown(&self) -> Option<&str> {
        self.net.rules_note.as_deref().filter(|_| self.net.room.is_some() && !self.net.started)
    }

    /// The note's keys: ✕ or ○ puts it away; while it shows, nothing under it moves.
    pub(super) fn rules_note_step(&mut self, k: Press, out: &mut Vec<Effect>) -> bool {
        if self.rules_note_shown().is_none() {
            return false;
        }
        if k.ok || k.back {
            self.net.rules_note = None;
            out.push(Effect::Sound(0));
        }
        true
    }
}

/// The note over whatever screen the lobby is on, dimming it; a press off the box puts it away too.
pub(super) fn draw_rules_over(d: &mut Draw, m: &Menu) {
    let Some((title, lines)) = m.rules_note_shown().and_then(|r| note(r, handshake::rules())) else { return };
    d.anchor = super::Anchor::Span;
    d.q(Tex::Solid, [0.0; 4], [0.0, 0.0, 640.0, 448.0], [0.0; 3], 64.0);
    d.anchor = super::Anchor::Centre;
    d.popup(Some(Act::Press(Press { back: true, ..Default::default() })));
    let (sw, sh) = (300.0, 104.0 + 14.0 * lines.len() as f32);
    let (sx, sy) = (320.0 - sw / 2.0, 224.0 - sh / 2.0);
    d.box3([sx, sy, sw, sh], 14.0, [Some(super::INK), Some(WHITE), Some(super::CREAM)], 128.0);
    d.label(&title, sx + sw / 2.0, sy + 14.0, 14.0, sw - 20.0, super::BROWN);
    d.label("How they play differently from your disc's:", sx + sw / 2.0, sy + 34.0, 10.0, sw - 20.0, super::BROWN);
    for (i, l) in lines.iter().enumerate() {
        d.label(l, sx + sw / 2.0, sy + 52.0 + 14.0 * i as f32, 11.0, sw - 24.0, super::INK);
    }
    let rect = [sx + sw / 2.0 - 60.0, sy + sh - 42.0, 120.0, 32.0];
    super::side_button(d, rect, "OK", true, false);
    d.hit(rect, Act::Row(0, OK));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The note only for another release's rules, and the list's kind says whose.
    #[test]
    fn a_note_only_for_other_rules() {
        assert_eq!(note("us", "us"), None);
        assert_eq!(note("jp", ""), None);
        let (title, lines) = note("jp", "us").unwrap();
        assert_eq!(title, "This lobby plays the Japanese rules");
        assert!(lines.len() > 3);
        assert_eq!(kind_of(true, "jp", "us"), "Doubles, Japanese rules");
        assert_eq!(kind_of(true, "us", "us"), "Doubles");
        assert_eq!(kind_of(false, "", "us"), "Singles");
    }
}

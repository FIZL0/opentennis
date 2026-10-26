//! B671a (ours, the user's board D): with any Offbeat rule on, a navy tab sits on the pause panel's gold frame, left
//! of its middle, saying "OFFBEAT RULES" and a chip per rule on (the Confirm strip's short names). It wraps into more
//! rows, growing upward, and the dialog above it moves up (and, with no more room under the top bar, shrinks) so the
//! tab never sits over it. No rules on, no tab. Drawing only: nothing here touches the match.

use bevy::prelude::*;

use super::super::name_tags;
use super::Game;

/// The tab's left edge: the frame's (88) and a little in. Its widest: as far in again from the frame's right (552).
pub(super) const X: f32 = 108.0;
const MAX_W: f32 = 552.0 - 20.0 - X;
/// The tab's foot runs this far onto the frame's top, under its rim.
const SINK: f32 = 3.0;
/// The room kept between the dialog and the tab, and the lowest the top bar's rim reaches.
const GAP: f32 = 4.0;
const BAR_FLOOR: f32 = 58.0;

/// The rules the tab says, and its image and size (disc px) while any is on.
#[derive(Resource, Default)]
pub(super) struct RulesTab {
    made: Option<Vec<&'static str>>,
    pub(super) tab: Option<(Handle<Image>, [f32; 2])>,
}

impl RulesTab {
    pub(super) fn size(&self) -> Option<[f32; 2]> {
        self.tab.as_ref().map(|t| t.1)
    }
}

/// Sets the tab again when the rules change (a new match), only while paused.
pub(super) fn refresh(menu: Res<super::Menu>, g: Res<Game>, mut tab: ResMut<RulesTab>, mut images: ResMut<Assets<Image>>) {
    if !menu.open {
        return;
    }
    let want = super::super::main_menu::offbeat::chips(&g.offbeat);
    if tab.made.as_ref() == Some(&want) {
        return;
    }
    tab.tab = (!want.is_empty()).then(|| {
        let (img, size) = name_tags::rules_tab(&want, MAX_W);
        (images.add(img), size)
    });
    tab.made = Some(want);
}

/// The tab's place, `size` its size, on the frame whose top is at `frame_top`.
pub(super) fn place(frame_top: f32, [w, h]: [f32; 2]) -> [f32; 4] {
    [X, frame_top + SINK - h, w, h]
}

/// Where the dialog goes with the tab under it: moved up, then shrunk about the middle of its top, as far as it must
/// to clear the tab.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Fit {
    from: f32,
    to: f32,
    pub(super) s: f32,
}

impl Fit {
    pub(super) const NONE: Fit = Fit { from: 0.0, to: 0.0, s: 1.0 };

    /// The dialog's box `[top, height]` over a tab `tab_h` tall (none: as it is) on the frame at `frame_top`.
    pub(super) fn new([top, h]: [f32; 2], frame_top: f32, tab_h: Option<f32>) -> Fit {
        let Some(tab_h) = tab_h else { return Fit::NONE };
        let floor = frame_top + SINK - tab_h - GAP;
        let lift = (top + h - floor).max(0.0).min((top - BAR_FLOOR).max(0.0));
        let to = top - lift;
        Fit { from: top, to, s: ((floor - to) / h).clamp(0.0, 1.0) }
    }

    pub(super) fn y(&self, y: f32) -> f32 {
        self.to + (y - self.from) * self.s
    }

    pub(super) fn rect(&self, [x, y, w, h]: [f32; 4]) -> [f32; 4] {
        [320.0 + (x - 320.0) * self.s, self.y(y), w * self.s, h * self.s]
    }
}

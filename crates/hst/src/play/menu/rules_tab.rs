//! B671a (ours, the user's board D): with any Offbeat rule on, a tab in the score frame's own rim saying "OFFBEAT
//! RULES" and a chip per rule on (the Confirm strip's short names), wrapping into more rows. F2: it sits under the
//! pause dialog (`super::scene`), the two fitted on the screen with the live board. No rules on, no tab. Drawing only:
//! nothing here touches the match.

use bevy::prelude::*;

use super::super::name_tags;
use super::Game;

/// The tab's left edge: the frame's (88) and a little in. Its widest: as far in again from the frame's right (552).
pub(super) const X: f32 = 108.0;
const MAX_W: f32 = 552.0 - 20.0 - X;

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

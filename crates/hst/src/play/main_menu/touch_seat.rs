//! B656: whether the on-screen pad holds a seat (`seats::hold_touch`): it took 1P on the splash, or sits in one of the
//! menus' seats. Then it stays when a controller connects (`touch_pad.rs`, `web/touch.js`), and the controller joins
//! as the next player instead of taking 1P over (`Menu::pad_takes_touch_seat`).

use bevy::prelude::*;

use super::{Dev, Menu};
use crate::play::controls;
use crate::play::emote;

/// The flag, from the menus' seats and the gamepads present now.
pub(super) fn hold(menu: Res<Menu>, gamepads: Query<(Entity, &Gamepad, &Name)>) {
    let touch = gamepads.iter().filter(|(_, g, n)| controls::readable(g) && emote::is_touch(n)).map(|(e, ..)| e);
    hst_base::seats::hold_touch(holds(&menu, touch));
}

/// Whether one of the on-screen pads `touch` is the splash's 1P or in a seat.
fn holds(menu: &Menu, mut touch: impl Iterator<Item = Entity>) -> bool {
    touch.any(|e| menu.p1 == Some(Dev::Pad(e)) || menu.seats.contains(&Some(Dev::Pad(e))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_by_the_splash_or_a_seat() {
        let [t, a] = [4, 5].map(|n| Entity::from_raw_u32(n).unwrap());
        let mut m = Menu::default();
        assert!(!holds(&m, [t].into_iter()));
        m.p1 = Some(Dev::Pad(t));
        assert!(holds(&m, [t].into_iter()));
        // gone (hidden): holds nothing
        assert!(!holds(&m, std::iter::empty()));
        // a real pad on 1P, the touch pad on 2P
        (m.p1, m.seats) = (Some(Dev::Pad(a)), [Some(Dev::Pad(a)), Some(Dev::Pad(t)), None, None]);
        assert!(holds(&m, [t].into_iter()));
        // the screen's taps (the keyboard) on 1P: the touch pad holds nothing
        (m.p1, m.seats) = (Some(Dev::Keys), [Some(Dev::Keys), None, None, None]);
        assert!(!holds(&m, [t].into_iter()));
    }
}

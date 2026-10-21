//! B605: the match's collision world when the disc's court data can't be read (a damaged image or pack). Instead of
//! stopping the game, the match plays on court 10's collision (the stage-less default) and says why in the notice
//! box for a few seconds; only when court 10's can't be read either is there nothing to play on.

use std::sync::Mutex;

use bevy::prelude::*;
use hst_data::iso::Iso;
use hst_sim::court;
use hst_sim::mesh::World;

use super::netplay::OnlineNotice;

/// The court whose collision stands in for one that can't be read.
const FALLBACK: u32 = 10;

/// How long the error stays up, s.
const SHOWN: f32 = 6.0;

/// The last set-up's load error, for `show` to put up.
static ERROR: Mutex<Option<String>> = Mutex::new(None);

/// Disc court `n`'s collision world (opened up for a court mod when `open`), or court 10's when it can't be read,
/// with the reason kept for the notice.
pub(super) fn world(iso: &mut Iso, n: u32, open: bool) -> World {
    let w = court::world(iso, n).or_else(|e| {
        let text = format!("Court {n}'s data couldn't be read ({e}).\nPlaying on court {FALLBACK}'s instead.");
        error!("{}", text.replace('\n', " "));
        *ERROR.lock().unwrap_or_else(|p| p.into_inner()) = Some(text);
        court::world(iso, FALLBACK)
    });
    let w = w.unwrap_or_else(|e| panic!("no court data to play on: {e}"));
    if open { court::open(w) } else { w }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, show);
}

/// Puts a set-up's load error in the notice box and takes it down after `SHOWN` seconds (unless something else
/// has replaced it).
fn show(notice: Option<ResMut<OnlineNotice>>, time: Res<Time<Real>>, mut until: Local<Option<(f32, String)>>) {
    let Some(mut notice) = notice else { return };
    let now = time.elapsed_secs();
    if let Some(text) = ERROR.lock().unwrap_or_else(|p| p.into_inner()).take() {
        notice.text = text.clone();
        *until = Some((now + SHOWN, text));
    }
    if let Some((end, text)) = until.as_ref()
        && now >= *end
    {
        if notice.text == *text {
            notice.text.clear();
        }
        *until = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A court that isn't on the disc plays on court 10's collision and leaves its reason for the notice.
    #[test]
    fn unreadable_court_falls_back() {
        let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else { return eprintln!("no ISO, skipped") };
        assert!(court::world(&mut iso, 99).is_err());
        let w = world(&mut iso, 99, false);
        assert_eq!(w.props.len(), court::world(&mut iso, FALLBACK).unwrap().props.len());
        let text = ERROR.lock().unwrap().take().expect("a load error");
        assert!(text.starts_with("Court 99's data couldn't be read (COURT/99/CMN.XB"), "{text}");
    }
}

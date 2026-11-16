//! Match art on another release's disc (M163a1): the Japanese disc keeps some entries in another folder of the same
//! archive (`INPANE`'s info sheet) and lacks a few the US build added (the counter banner, the skull pop-up). A read
//! that would have stopped the match finds the moved entry by its file name, or draws nothing for a missing one; on
//! the US disc every entry is where the reads look first.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use hst_data::xb::{Archive, Entry};

/// The entry whose path ends in `name` (`"/2d/info.tm2"`), else the one with `name`'s file name in another folder.
pub(super) fn entry<'a>(arc: &'a Archive, name: &str) -> Option<&'a Entry> {
    let path = |e: &Entry| e.name.to_ascii_lowercase().replace('\\', "/");
    let name = name.to_ascii_lowercase();
    let file = name.rsplit('/').next().unwrap_or(&name);
    arc.entries.iter().find(|e| path(e).ends_with(&name)).or_else(|| arc.entries.iter().find(|e| path(e).rsplit('/').next() == Some(file)))
}

/// A clear 1×1 image for art this disc doesn't have (the art it stands for is never shown there).
pub(super) fn blank(images: &mut Assets<Image>) -> Handle<Image> {
    images.add(Image::new(Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, TextureDimension::D2, vec![0; 4], TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Japanese disc's match art: the moved info sheet is found, the US-only entries aren't (and are blanked).
    /// Skipped without `fixtures/discs/SCPS-15113.iso` (not the user's disc).
    #[test]
    fn the_japanese_disc_finds_moved_art() {
        let p = hst_data::fixture::path("fixtures/discs/SCPS-15113.iso");
        let Ok(mut iso) = hst_data::iso::Iso::open(&p) else {
            return eprintln!("skipped: no {}", p.display());
        };
        let data = iso.read("AZUMA/INPANE/INPANE.XB0").expect("INPANE");
        let arc = Archive::parse(&data).expect("xb");
        assert!(entry(&arc, "/2d/info.tm2").is_some());
        assert!(entry(&arc, "/inpane_counter.tm2").is_none());
        let data = iso.read("AZUMA/C_EFF/EFFCT.XB0").expect("EFFCT");
        let arc = Archive::parse(&data).expect("xb");
        assert!(entry(&arc, "fukidasi_dokuro.tm2").is_none());
        assert!(entry(&arc, "A_fukidasi_10.tm2").is_some());
    }
}

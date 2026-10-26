//! B678a: the plants' `.MOR` (palm leaves, grass and hibiscus snaps, the trees' sway) bind every track to a morph
//! target of their own model, and those targets move vertices.

use hst_data::{fixture, mdl, mor, xb::Archive};
use hst_sim::court_anim::CourtAnim;

#[test]
fn plant_morphs_bind() {
    let Ok(mut iso) = fixture::iso() else { return fixture::skip("disc") };
    const STILL: [(u32, &str); 4] = [(3, "aroe_s1111"), (3, "kabeki-2_s1111"), (4, "uekomi_s1111"), (6, "sugi_s1111")];
    let mut seen = 0;
    for n in [2, 3, 4, 6, 9] {
        for hol in ["HOL01.XB", "SSN1/HOL01.XB"] {
            let data = iso.read(&format!("COURT/{n:02}/{hol}")).expect("hole archive");
            let arc = Archive::parse(&data).unwrap();
            let file = |name: &str| {
                let e = arc.entries.iter().find(|e| e.name.rsplit(['\\', '/']).next().is_some_and(|f| f.eq_ignore_ascii_case(name)))?;
                arc.read(e).ok()
            };
            for e in arc.entries.iter().filter(|e| e.name.replace('\\', "/").contains("/tree/") && e.name.to_ascii_lowercase().ends_with(".mor")) {
                let leaf = e.name.rsplit(['\\', '/']).next().unwrap();
                let stem = &leaf[..leaf.len() - 4];
                let tracks = mor::parse(&arc.read(e).unwrap(), 1).unwrap();
                let model = mdl::parse(&file(&format!("{stem}.mdl")).unwrap_or_else(|| panic!("{stem}: no model"))).unwrap();
                // the aloe, the ivy wall (court 3), the hedge (4) and the cedar (6) carry a `.MOR` but their models
                // have no morph targets: nothing of theirs moves
                if model.morph_names.is_empty() {
                    assert!(STILL.contains(&(n, stem)), "court {n} {stem}: no morph targets");
                    continue;
                }
                for t in &tracks.tracks {
                    assert!(model.morph_names.contains(&t.name), "court {n} {stem}: {} not among {:?}", t.name, model.morph_names);
                }
                let moved = model.materials.iter().flatten().flat_map(|p| &p.morphs).filter(|t| !t.is_empty()).count();
                assert!(moved > 0, "court {n} {stem}: no morph offsets");
                let mut a = CourtAnim::new(&model, None, None, &[]).with_morphs(&model, Some(&tracks));
                for _ in 0..30 {
                    a.tick();
                }
                assert!(a.morph_weights().iter().any(|&w| w != 0.0), "court {n} {stem}: no weight after 30 frames");
                seen += 1;
            }
        }
    }
    // palms, grass and hibiscus (2); maple and cherry (4); shrub and birches (6); maple (9); both seasons
    assert_eq!(seen, 2 * (5 + 2 + 4 + 1));
}

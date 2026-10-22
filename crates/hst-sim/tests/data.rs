//! The tests here skip when the disc or the recordings are missing (they're never in git); a run that has to check
//! the port against the game sets `HST_REQUIRE_DISC=1` (tools/check.sh does), and then every test that reads them
//! through `hst_data::fixture` fails instead. This one fails such a run up front when the folders aren't there.

use hst_data::fixture;

#[test]
fn the_game_data_is_here_when_required() {
    if !fixture::required() {
        return;
    }
    for dir in [fixture::ISO, "fixtures/iso/ZZBIN/GAME.BIN", "fixtures/ram", "fixtures/xb", "fixtures"] {
        assert!(fixture::path(dir).exists(), "HST_REQUIRE_DISC: no {dir}");
    }
}

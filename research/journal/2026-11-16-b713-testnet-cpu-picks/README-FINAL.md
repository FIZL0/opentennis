# B713: short_code_on_a_testnet's CPU picks

The ignored DHT testnet test asserted the open seats' computers get characters `[0, 1]` (the first free ones).
Commit 341500ea deliberately changed `Room::bots` to draw disc characters nobody picked at random by the match
seed (so the stand-ins aren't always Ashley), with its own unit test `start_bots_are_random`. So random is
intended; the integration test was stale. It now checks the two computers are distinct disc characters (< 14)
that no person in the lobby holds. Passes: `tools/check.sh -p hst-base short_code_on_a_testnet -- --ignored`.

## Not verified / not 1:1
none (online lobby only; not in the original game).

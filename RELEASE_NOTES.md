<!-- What's new since the last release, for players, in plain English: one "- " line per change. tools/release.sh puts this in the release and empties it. -->
- No disc? The start screen can now join a game nearby on your network and play it from the host's disc, without keeping a copy.
- Joining a game nearby from a phone's browser no longer crashes the page when the host has a big texture pack on: the phone plays with the game's own textures instead.
- A browser joining a game nearby never asks for an ISO any more: when it can't reach the host it says so and keeps trying, and a download cut off part-way picks up where it stopped.
- A browser with WebRTC switched off (an iPhone in Lockdown Mode, for one) is told so, and how to turn it back on, before it downloads anything, instead of failing in the lobby.
- The host's log now shows what a browser joining it did while it loaded, and why it stopped, so a phone's failed join can be worked out.
- New download for handhelds with PortMaster (muOS, ROCKNIX, Knulli and others): opentennis.zip installs OpenTennis as a port; put your Hot Shots Tennis (USA) ISO in ports/opentennis/ and the first start imports it, keeping settings and saves in that folder.

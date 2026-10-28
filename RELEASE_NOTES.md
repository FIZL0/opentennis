<!-- What's new since the last release, for players, in plain English: one "- " line per change. tools/release.sh puts this in the release and empties it. -->
- No disc? The start screen can now join a game nearby on your network and play it from the host's disc, without keeping a copy.
- Joining a game nearby from a phone's browser no longer crashes the page when the host has a big texture pack on: the phone plays with the game's own textures instead.
- A browser joining a game nearby never asks for an ISO any more: when it can't reach the host it says so and keeps trying, and a download cut off part-way picks up where it stopped.
- A browser with WebRTC switched off (an iPhone in Lockdown Mode, for one) can now join a game nearby and play, and a browser whose connection to the host is blocked by the host's firewall tries again another way before giving up.
- The host's log now shows what a browser joining it did while it loaded, and why it stopped, so a phone's failed join can be worked out.
- The ARM Linux build now draws straight to the screen on handhelds without a desktop (muOS, or Knulli on the Mali driver, such as the RG34XX SP) instead of quitting at start, and its launcher also works from PortMaster's Ports folder.
- Hosting online while the relay isn't up yet (or at all), the lobby's Copy long code button copies the full join code, so players on the same network or who can reach you directly can join by it in Join by Code.

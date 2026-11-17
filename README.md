# OpenTennis — a remaster of Hot Shots Tennis (PS2)

<p align="center">
  <a href="https://github.com/FIZL0/opentennis/releases/latest"><img alt="Download the latest release" src="https://img.shields.io/github/v/release/FIZL0/opentennis?label=download&color=2ea44f"></a>
  <a href="https://opentennis.dev"><img alt="opentennis.dev" src="https://img.shields.io/badge/play-opentennis.dev-2ea44f"></a>
  <a href="https://ko-fi.com/fizlo"><img alt="Support my work on Ko-fi" src="https://img.shields.io/badge/Ko--fi-support%20my%20work-FF5E5B?logo=kofi&logoColor=white"></a>
  <a href="https://discord.gg/kCkz5hFNXG"><img alt="Join us on Discord" src="https://img.shields.io/badge/Discord-join%20us-5865F2?logo=discord&logoColor=white"></a>
</p>

A from-scratch rewrite of the PlayStation 2 game **Hot Shots Tennis** (US, 2007), known as **Everybody's Tennis**
in Europe and **Minna no Tennis** (みんなのテニス) in Japan, in Rust with the [Bevy](https://bevyengine.org) engine.

**Play in your browser: <https://opentennis.dev>** (you pick your own ISO; nothing is uploaded).

Newest unreleased build (every change to main, less tested): <https://beta.opentennis.dev>.

**Download** for Windows, Mac (Apple Silicon), Linux, Android or iOS: [latest release](https://github.com/FIZL0/opentennis/releases/latest).
The Mac app isn't notarized: the first time, right-click OpenTennis → Open, or on macOS 15 and later open it once, then
System Settings → Privacy & Security → Open Anyway.

**iPhone / iPad:** [add the source in one tap](https://opentennis.dev/ios.html) to Feather, AltStore or SideStore, or add
`https://fizl0.github.io/apps/source.json` under Sources. You sign it
with your own certificate, and updates arrive through the app.

Work in progress. Building from source, modding by hand and development: [docs/DEVELOPING.md](docs/DEVELOPING.md).

## Screenshots

Taken from the remaster running on a retail disc (the art comes from the disc; none of it is in this repository).

<p align="center">
  <img src="docs/screenshots/rally-grass.jpg" alt="A rally on a grass court" width="32%">
  <img src="docs/screenshots/menu-chars.jpg" alt="Character selection with mod characters" width="32%">
  <img src="docs/screenshots/menu-lobby.jpg" alt="Online lobby" width="32%">
</p>

<details>
<summary><b>More screenshots</b></summary>

| | |
|---|---|
| ![A player celebrating a point](docs/screenshots/hero.jpg) | ![Serving in doubles, with the score panel up](docs/screenshots/serve.jpg) |
| ![A rally on an autumn court](docs/screenshots/rally-autumn.jpg) | ![Celebrating at sunset on the river court](docs/screenshots/closeup-sunset.jpg) |
| ![A mod character on a mod court](docs/screenshots/mod-castle.jpg) | ![A mod character](docs/screenshots/mod-character.jpg) |
| ![A mod court](docs/screenshots/mod-amusement.jpg) | ![Court selection with mod courts](docs/screenshots/menu-courts.jpg) |
| ![Title screen](docs/screenshots/menu-title.jpg) | ![Main menu](docs/screenshots/menu-main.jpg) |
| ![Umpire selection with mod umpires](docs/screenshots/menu-umpires.jpg) | ![Mod store](docs/screenshots/menu-browsemods.jpg) |

</details>

## You must provide the game disc

**This repository contains no game data**: no models, textures, animations, sounds, music or program code from the
original. Everything the game shows or plays is read **at runtime from your own copy of the disc**. Without it,
nothing runs.

What you need:

| | |
|---|---|
| **Disc** | Any region, version 1.00 (PS2): **Hot Shots Tennis** (US, SCUS-97610), **Everybody's Tennis** (Europe, SCES-54535) or **Minna no Tennis** (Japan, SCPS-15113) |
| **Format** | A disc image (`.iso`) you made from **a disc you own** |
| **Where** | Anywhere; the game asks for it the first time (the browser keeps it in private storage, nothing is uploaded) |

- **Any region plays.** The game checks the disc and refuses other games or versions with `unsupported disc`.
- **Have more than one?** Keep them in the same folder: Settings → Base Disc picks which one the game runs from,
  Rules picks the US or Japanese game, and Voice Language offers every disc's voices.
- **Don't ask for, share or commit disc images or anything extracted from them.** Nobody here can provide one.
- **You don't need a PS2 BIOS, an emulator or any other files** to play. Those are only for the research tools
  (see [docs/DEVELOPING.md](docs/DEVELOPING.md)).

## Controls

Player 1 uses the keyboard or the first controller. In doubles, a second controller plays player 1's partner (P1 + P2
vs 2 CPUs); in singles it plays the opponent. Every other player is a CPU.

| Action | Keyboard | Controller |
|---|---|---|
| Move (aim while swinging) | WASD | Left stick / d-pad |
| Topspin | J | A |
| Slice | K | B |
| Lob | L | Y |
| Serve | any shot key | any shot button |
| Emote | 1–4 | Right stick |
| Pause | Esc | Start |
| Swap hand (character select) / switch camera (match) | C | Select |

## Mods

Main menu → Mods adds characters, courts and umpires from the store
([`FIZL0/opentennis-mods`](https://huggingface.co/datasets/FIZL0/opentennis-mods)), on desktop and in the browser.

- **Browse mods**: pick one to install it.
- **Upload my mod**: sign in with a Hugging Face *Write* token, then pick a mod to upload. It shows as
  *Not reviewed* until merged.
- **Made your own?** Put its folder (`mod.json` + `.glb` files) in `mods/<id>/` in the game's data folder (`~/.local/share/hst`, `%APPDATA%\hst`, `~/Library/Application Support/hst`); it shows in the
  game and in Upload my mod. How to make one: [opentennis-mods](https://github.com/FIZL0/opentennis-mods).

Texture replacements: texture packs (a mod whose `mod.json` says `"kind": "textures"`; several stack in an order) and
`mods/texture-replacements/` replace the disc's textures, a PCSX2 texture pack made for the original game works as is, and
mods can carry upscaled textures of their own (the "Texture Replacements" setting). How to make either: [docs/DEVELOPING.md](docs/DEVELOPING.md).

## Online play

Main menu → Online. Up to four players, singles or doubles; the computer plays any open seat. Browser, desktop and
phone players can all play together.

- **Host** opens a lobby with a 4-letter code. Public lobbies are listed in **Find** and picked up by **Quick play**;
  Private ones only take the code or link.
- **Join** takes the code, or the host's copied link (opening the link joins too).
- Players connect directly where they can; otherwise traffic goes through a relay (iroh's public ones by default, or
  your own: Online → Relay → Custom...).
- Codes are looked up through our code server, `pkarr.opentennis.dev`. If it's down the game switches to the public
  `relay.pkarr.org` by itself; that one is rate-limited per network, so a house full of players can hit "no answer".

**Same network, no internet** (Relay → Off):

- Desktop and phone apps find each other's lobbies in Find.
- Browsers can't see the local network, so there's no code: the host copies their link (Copy link) and sends it;
  the guest opens it or taps **Paste** in Join by Code, then copies their own link back, and the host taps **Paste**
  in the lobby. On iOS Safari, tap Paste a second time if it asks.

**Host your own servers.** Nothing is required, but you can run both servers yourself:

- *Code server* (short codes, Find): [tools/pkarr-relay/](tools/pkarr-relay/) has a Docker Compose file (works as a
  TrueNAS custom app) and an nginx config. Put it behind HTTPS on your domain, then add
  `code_server = https://your.domain` to `online.txt` in the data folder on each player. Codes go into the same public
  DHT either way, so players on different code servers still find each other.
- *Relay* (traffic when a direct path fails): `iroh-relay`, see [docs/DEVELOPING.md](docs/DEVELOPING.md). Every
  player in a lobby needs the same relay.

More (settings file, terminal client): [docs/DEVELOPING.md](docs/DEVELOPING.md).

## License

The code is licensed under the [GNU General Public License v3.0 or later](LICENSE).
This covers the code only, not the game or its data.

## Legal

This is an unofficial fan project, not affiliated with or endorsed by Sony Interactive Entertainment or Clap
Hanz. Hot Shots Tennis, Everybody's Tennis and Minna no Tennis are trademarks of their owners. No copyrighted game
data is included or distributed. You must own the game to use this.

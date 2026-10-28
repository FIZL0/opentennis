## Notes

Thanks to Clap Hanz and Sony Computer Entertainment for making *Hot Shots Tennis* (PS2, 2007), the game this remaster
plays. OpenTennis is a fan project: none of the original game is included. It reads everything from your own disc
image when it runs.

You need your own *Hot Shots Tennis* (USA) disc image (SCUS-97610) as an `.iso` (`.chd` images can't be read). Put it
in `ports/opentennis/` (or your PS2 roms folder) and start OpenTennis: the first start imports the disc once into
`disc.pack` beside the game, and from then on the game starts from that, so the ISO can go. Settings, mods and saves
stay in `ports/opentennis/` too. `log.txt` there says what happened on the last start.

Source, other platforms and online play: https://github.com/FIZL0/opentennis · https://opentennis.dev/

## Controls

| Button | Action |
|--|--|
| D-Pad / Left Stick | Move |
| Bottom face button (B on Anbernic, A on Xbox) | Topspin, serve |
| Right face button (A on Anbernic, B on Xbox) | Slice, serve |
| Top face button (X on Anbernic, Y on Xbox) | Lob |
| Right Stick (flick) | Emotes |
| Select | Behind-the-player camera; swap hand at character select |
| Start | Pause |
| Start + Select | Quit |

Buttons can be remapped in the game's settings.

## Compile

```sh
git clone https://github.com/FIZL0/opentennis.git
cd opentennis
tools/build.sh linux-aarch64   # cross-compiles in Docker (Debian 11, glibc 2.31)
# dist/linux-aarch64/opentennis.zip is this port, dist/linux-aarch64/opentennis/ its PortMaster folder
```

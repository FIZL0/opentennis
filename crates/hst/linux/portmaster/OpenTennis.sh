#!/bin/bash
# OpenTennis as a PortMaster port (M155a, https://portmaster.games/packaging.html): the standard launch script.
# tools/portmaster.sh packs it at the top of opentennis.zip; PortMaster puts it in the Ports menu's folder and the
# opentennis/ folder (the game, opentennis.aarch64) in ports/.

XDG_DATA_HOME=${XDG_DATA_HOME:-$HOME/.local/share}

if [ -d "/opt/system/Tools/PortMaster/" ]; then
  controlfolder="/opt/system/Tools/PortMaster"
elif [ -d "/opt/tools/PortMaster/" ]; then
  controlfolder="/opt/tools/PortMaster"
elif [ -d "$XDG_DATA_HOME/PortMaster/" ]; then
  controlfolder="$XDG_DATA_HOME/PortMaster"
else
  controlfolder="/roms/ports/PortMaster"
fi

# muOS's control.txt sets its own controlfolder (/mnt/mmc/MUOS/PortMaster) when the one above isn't there
source $controlfolder/control.txt
[ -f "${controlfolder}/mod_${CFW_NAME}.txt" ] && source "${controlfolder}/mod_${CFW_NAME}.txt"
get_controls

GAMEDIR=/$directory/ports/opentennis
CONFDIR="$GAMEDIR/conf"
mkdir -p "$CONFDIR"
cd "$GAMEDIR" || exit 1

> "$GAMEDIR/log.txt" && exec > >(tee "$GAMEDIR/log.txt") 2>&1

# the game keeps its imported disc (disc.pack), settings.txt, mods/ and saves beside itself, in this folder; anything it
# would put in the per-user data folder goes in conf/
export XDG_DATA_HOME="$CONFDIR"
export LD_LIBRARY_PATH="$GAMEDIR/libs.${DEVICE_ARCH}:$LD_LIBRARY_PATH"
# the game reads SDL controller mappings (its pads are gilrs, which takes this variable)
export SDL_GAMECONTROLLERCONFIG="$sdl_controllerconfig"
# Mali GPUs: wgpu's GL ES backend (WGPU_BACKEND=vulkan tries Vulkan where the device has it)
export WGPU_BACKEND="${WGPU_BACKEND:-gl}"

# First start: no file dialog on these systems, so the game imports the first ISO it finds here or in the PS2 roms
# folder (it checks the disc is Hot Shots Tennis (USA), SCUS-97610, and keeps its own copy, disc.pack: the ISO can go
# afterwards). disc.pack is the "installed" mark: once it is there, the game starts straight from it.
set --
if [ ! -f "$GAMEDIR/disc.pack" ] && [ ! -f "$CONFDIR/hst/disc.pack" ]; then
  shopt -s nullglob
  for iso in "$GAMEDIR"/*.iso "$GAMEDIR"/*.ISO \
      /$directory/{ps2,PS2,roms/ps2,roms/PS2,ROMS/ps2,ROMS/PS2}/*[Hh]ot*[Ss]hots*[Tt]ennis*.{iso,ISO}; do
    set -- --import "$iso"
    break
  done
  shopt -u nullglob
  if [ $# -eq 0 ]; then
    pm_message "OpenTennis needs your own Hot Shots Tennis (USA) disc image: put the .iso (not a .chd) in ports/opentennis/ and start OpenTennis again."
    sleep 10
    pm_finish
    exit 1
  fi
  pm_message "First start: importing $(basename "$iso"), once. This takes a minute or two."
fi

$ESUDO chmod +x "$GAMEDIR/opentennis.${DEVICE_ARCH}"

# no key mapping: the game reads the pad itself; gptokeyb only gives the device's quit combo (Start + Select)
$GPTOKEYB "opentennis.${DEVICE_ARCH}" &
pm_platform_helper "$GAMEDIR/opentennis.${DEVICE_ARCH}"
./opentennis.${DEVICE_ARCH} "$@"

if [ $# -gt 0 ] && [ ! -f "$GAMEDIR/disc.pack" ] && [ ! -f "$CONFDIR/hst/disc.pack" ]; then
  pm_message "Couldn't import $(basename "$iso"): it must be Hot Shots Tennis (USA), SCUS-97610, as an .iso. What went wrong is in ports/opentennis/log.txt."
  sleep 10
fi

pm_finish

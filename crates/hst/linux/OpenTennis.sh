#!/bin/bash
# OpenTennis on ARM Linux (also from a handheld's Ports menu). tools/build.sh linux-aarch64 puts it beside the game.
# Two layouts: the whole OpenTennis folder in roms/ports/ (Knulli, Batocera, ROCKNIX), or PortMaster's (muOS): this
# script in ROMS/Ports/ and the folder in ports/ (/mnt/mmc/ports/OpenTennis).
here=$(dirname "$(readlink -f "$0")")
# PortMaster's environment where it's installed: $directory (mnt/mmc on muOS) and the pad mapping for the built-in controls
XDG_DATA_HOME=${XDG_DATA_HOME:-$HOME/.local/share}
for controlfolder in /opt/system/Tools/PortMaster /opt/tools/PortMaster "$XDG_DATA_HOME/PortMaster" /roms/ports/PortMaster; do
    if [ -f "$controlfolder/control.txt" ]; then
        # shellcheck disable=SC1091
        . "$controlfolder/control.txt"
        # shellcheck disable=SC1090
        [ -f "$controlfolder/mod_${CFW_NAME}.txt" ] && . "$controlfolder/mod_${CFW_NAME}.txt"
        type get_controls >/dev/null 2>&1 && get_controls
        [ -n "$sdl_controllerconfig" ] && export SDL_GAMECONTROLLERCONFIG="$sdl_controllerconfig"
        break
    fi
done
game=
for d in "$here" "$here/OpenTennis" ${directory:+"/$directory/ports/OpenTennis"} /mnt/mmc/ports/OpenTennis /mnt/sdcard/ports/OpenTennis; do
    if [ -x "$d/opentennis" ]; then
        game=$d
        break
    fi
done
cd "${game:-$here}" || exit 1
# a Ports start has no terminal: what went wrong is in OpenTennis.log beside the game
exec >OpenTennis.log 2>&1
[ -n "$game" ] || { echo "opentennis not found beside $here or in ports/OpenTennis"; exit 1; }
# Mali GPUs: wgpu's GL ES (WGPU_BACKEND=vulkan tries PanVK, where the device has it). With no Wayland or X server
# (muOS, Knulli on the Mali blob) the game draws to the framebuffer itself.
export WGPU_BACKEND="${WGPU_BACKEND:-gl}"
# no udev daemon (muOS): gilrs finds the pads in /dev/input itself; the mapping above names their buttons
[ -e /run/udev/control ] || [ -d /run/udev/data ] || export GILRS_DISABLE_UDEV=1
# no file dialog on these systems: with no disc imported yet, the first ISO in this folder or in the PS2 roms folder
# (the game checks it is Hot Shots Tennis (USA), SCUS-97610, and keeps its own copy, so the ISO can go afterwards)
if [ $# -eq 0 ] && [ ! -f "$XDG_DATA_HOME/hst/disc.pack" ]; then
    for iso in ./*.iso ./*.ISO ../../ps2/*[Hh]ot*[Ss]hots*[Tt]ennis*.iso ../../ps2/*[Hh]ot*[Ss]hots*[Tt]ennis*.ISO \
        /mnt/*/ROMS/[Pp][Ss]2/*[Hh]ot*[Ss]hots*[Tt]ennis*.[Ii][Ss][Oo]; do
        if [ -f "$iso" ]; then
            set -- --import "$iso"
            break
        fi
    done
    [ $# -gt 0 ] || echo "No disc yet: put your Hot Shots Tennis (USA) ISO in $PWD or the PS2 roms folder (an .iso, not a .chd)"
fi
exec ./opentennis "$@"

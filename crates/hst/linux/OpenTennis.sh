#!/bin/sh
# OpenTennis on ARM Linux (also from a handheld's Ports menu: the OpenTennis folder goes in roms/ports/).
# tools/build.sh linux-aarch64 puts it beside the game.
cd "$(dirname "$(readlink -f "$0")")" || exit 1
# a Ports start has no terminal: what went wrong is in OpenTennis.log beside it
exec >OpenTennis.log 2>&1
# Mali GPUs on Panfrost: wgpu's GL ES (WGPU_BACKEND=vulkan tries PanVK, where the device has it)
export WGPU_BACKEND="${WGPU_BACKEND:-gl}"
# no file dialog on these systems: with no disc imported yet, the first ISO in this folder or in roms/ps2 (the game
# checks it is Hot Shots Tennis (USA), SCUS-97610, and keeps its own copy, so the ISO can go afterwards)
if [ $# -eq 0 ] && [ ! -f "${XDG_DATA_HOME:-$HOME/.local/share}/hst/disc.pack" ]; then
    for iso in ./*.iso ./*.ISO ../../ps2/*[Hh]ot*[Ss]hots*[Tt]ennis*.iso ../../ps2/*[Hh]ot*[Ss]hots*[Tt]ennis*.ISO; do
        if [ -f "$iso" ]; then
            set -- --import "$iso"
            break
        fi
    done
    [ $# -gt 0 ] || echo "No disc yet: put your Hot Shots Tennis (USA) ISO in $PWD or roms/ps2/ (an .iso, not a .chd)"
fi
exec ./opentennis "$@"

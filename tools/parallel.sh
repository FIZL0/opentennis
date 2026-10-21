#!/usr/bin/env bash
# Parallel unattended run through agent-runner (settings: runner.toml). Start it inside tmux:
#   tmux new -s hst tools/parallel.sh [N] [p|d] [f]    # N slots (5), p = plan order (default) | d = different files,
#                                                      # f = keep starting past the weekly cap
#                                                      # --shutdown = power off once nothing is left to do
# Stop gently: s + Enter in its pane, or `agent-runner stop`. Log: context/notes/run.log, ledger.md, slotN.log.
# Install: python3 -m pip install --user --break-system-packages "git+https://github.com/FIZL0/agent-runner@v0.4.1"
cd "$(dirname "$(realpath "$0")")/.."
command -v agent-runner >/dev/null || { sed -n 7p "$0"; exit 2; }
args=()
for a in "$@"; do case $a in p|d) args+=(--mode "$a");; f) args+=(--force);; *) args+=("$a");; esac; done
export HST_PCSX2_MAX_HOLD=${HST_PCSX2_MAX_HOLD:-1200}
exec agent-runner run "${args[@]}"

#!/usr/bin/env bash
# No retail code addresses or Ghidra names in shipped code (AGENT.md): greps crates/*/src for FUN_/DAT_ names and
# for every function entry address in context/decomp/*.funcs.tsv (skipped when context/ isn't there). Run by done.sh.
# exe.rs (the documented data-offset exception) and hst-oracle (looks functions up by address) are exempt.
# Also: Ghidra's local names (uVar1, param_2, local_10, undefined4) in shipped code, Ghidra names on the public pages
# (web/, docs/, README), decompiled C pasted into notes (a Ghidra name on a line ending in ; or {), and a clean-room
# SPEC.md (PLAN.md) naming anything from the decompile: addresses, Ghidra names, struct offsets.
cd "$(dirname "$(realpath "$0")")/.." || exit 1
files=(':(glob)crates/*/src/**' ':!crates/hst-data/src/exe.rs' ':!crates/hst-oracle')
hits=$(git grep -nE '\b(FUN|DAT|LAB|PTR)_[0-9a-fA-F]{6,}' -- "${files[@]}")
if compgen -G 'context/decomp/*.funcs.tsv' >/dev/null; then
  # every 0x… literal, kept when it's a function entry (a 9.6k-pattern grep -f takes minutes)
  hits+=$'\n'$(git grep -noiE '\b0x[0-9a-f]{5,8}\b' -- "${files[@]}" |
    awk -F: 'NR==FNR { a[tolower(gensub(/^0+/, "", 1, $1))]; next }
             { h = tolower($NF); sub(/^0x0*/, "", h) } h in a' <(cut -f1 context/decomp/*.funcs.tsv) -)
fi
GHIDRA='\b((u|i|f|b|s|l|c|pu|pi|pf|pc|ppu|au)Var[0-9]+|in_stack_[0-9a-f]+|local_[0-9a-f]+|param_[0-9]+|extraout_\w+|unaff_\w+|undefined[1248])\b'
NAMES='\b(FUN|DAT|LAB|PTR)_[0-9a-fA-F]{6,}'
hits+=$'\n'$(git grep -nE "$GHIDRA" -- "${files[@]}")
hits+=$'\n'$(git grep -nE "$NAMES" -- web docs README.md CREDITS.md)
hits+=$'\n'$(git grep -nE "($NAMES|$GHIDRA).*[;{]\s*$" -- 'research/**/*.md' 'plan/*.md')
hits+=$'\n'$(git grep -nE "$NAMES|$GHIDRA|\+ ?0x[0-9a-fA-F]+|\b0x[0-9a-fA-F]{5,8}\b" -- 'research/journal/**/SPEC.md')
hits=$(grep . <<<"$hits") || exit 0
echo "AUDIT FAILED: retail addresses / Ghidra names / decompiled code where they don't belong (describe the behaviour instead):"
echo "$hits" | head -20
exit 1

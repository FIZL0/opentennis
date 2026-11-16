#!/bin/sh
# ponytail: shim for agents started before the move to `agent-runner plan` (2026-11-16); delete once no slot
# worktree or running prompt still names tools/ctx.py.
[ -n "$HST_CTX_NOCOMMIT" ] && export AGENT_RUNNER_NOCOMMIT=1
exec agent-runner plan "$@"

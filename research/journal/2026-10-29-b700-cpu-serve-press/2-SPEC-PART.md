# B700 spec round 2 (PART: the port session continues from SPEC.md)

The spec had no real open questions: its `## Open questions` section read `none (…)` with a note after it, and
`tools/ctx.py brief` only treats a bare `none` as empty, so the runner briefed another spec phase. The note moved
to its own `## Out of scope` section and the section now reads `none`. Nothing in the rule changed; no decompile
read this round. Next session: port phase from SPEC.md (press rule in `bot_serve`/the doubles serve, look-ahead
2..180 in `serve::ai_search`, the unit test, and extending B700a's `retimed_cpu_serve_keeps_its_kind` to
offset == −max(err, 0) for err −4..4).

## Not verified / not 1:1
Unchanged from 1-SPEC-PART.md (doubles CPU serves not recorded live; err = 0 presses not seen live).

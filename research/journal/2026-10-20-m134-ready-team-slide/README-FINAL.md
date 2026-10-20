# M134 — team switch slides on the "everyone ready" screen

Once every seat has picked, the local select becomes the confirm screen (same columns, the foot's rows reachable).
A non-1P human's □ / Tab there still switches team (input on Confirm for anyone not driving the foot is handled as the
select's), but the column-slide tracker (`assign_shake.rs` `columns`) only ran on the select and the lobby, so the
columns jumped. It now runs on the confirm screen too; the online confirm screen draws no columns, so it's harmless
there. Test: `a_side_switch_slides_once_everyone_is_ready`.

## Not verified / not 1:1
- Not looked at live (no shot): front-end only, ours to design; the unit test checks the slide offsets are set.

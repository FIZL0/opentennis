# V3: release 1.9.1 (B765's fixes)

Not released from this slot. Main dropped V3 in 5969a663 ("plan: drop V3 released by hand from the control
session") after this run started: the control session does the 1.9.1 release itself. At the time of checking,
`gh release list` showed v1.9.0 as the latest release (no v1.9.1, no draft), and no opentennis `release.sh` was
running here. The only `build.sh all` running was iFizBox's. Running `tools/release.sh 1.9.1 publish` here as
well would race the control session for the same v1.9.1 tag and "Release v1.9.1" commit, so this slot does
nothing.

## Not verified / not 1:1

none (release only; no gameplay change). Not checked: whether the control session's 1.9.1 release went out.

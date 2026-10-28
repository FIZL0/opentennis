# B689a — iPhone guest run through the host's log

Blocked: needs the user's iPhone 15 Pro. Unattended run, no phone here.

## What I looked for
- Any `adhoc guest <ip>: …` lines from a real iPhone (B689's `POST /adhoc/log`) in the game's data dir
  (`~/.local/share/hst/`), `context/` and the runner slot logs: none. The only matches are diffs and the
  WebKitGTK stand-in runs from B689 (`context/b689/`), which B689 already acted on.

## What unblocks it (the user)
1. Start a desktop build on main as an adhoc host and keep its stdout/stderr, e.g.
   `cargo run --release -p hst 2>&1 | tee context/b689a/host.log`.
2. On the iPhone, open the host's address in Safari and let it load as far as it gets (try a match too).
3. `grep 'adhoc guest' context/b689a/host.log` holds the UA line, each load step, `WebRTC: true/false`,
   errors and where it stopped. If `WebRTC: false`: Settings → Privacy & Security → Lockdown Mode.
4. Unblock B689a (`tools/ctx.py unblock B689a "log in context/b689a/host.log"`), and an agent fixes what it shows.

## Not verified / not 1:1
- Everything: there was no iPhone run to read. No code changed.

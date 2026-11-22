# B758 — log file under Steam

Steam hands the game's stderr to journald (a pipe or socket), so `logfile::unwatched_to` (which only redirects when
nobody reads stderr) left the Deck with no `hst-log.txt` and no fps lines.

`main.rs` `steam_log_to`, called right after `unwatched_to`: when stderr isn't a terminal or a file and either
`SteamDeck=1` or stderr is a pipe/socket, stderr (where every log line goes; stdout is left alone) goes into a pipe whose reader thread copies every byte to
the original stderr (journald keeps getting it) and to `<data dir>/hst-log.txt` (previous launch kept as
`hst-log-previous.txt`), and the frame-rate log (fps / slow frame / stall lines) is turned on. It creates the data
folder first (with `HST_DATA` set, nothing has made it yet at that point).

Check (context-free, scratch `XDG_DATA_HOME`): `SteamDeck=1 hst <iso> --shot … --shot-at 12 2>&1 | cat > piped.txt`
→ `hst-log.txt` appears with `fps 179.7 (slowest frame 159.5 ms) at 5 s`, `fps 219.2 … at 10 s`; piped.txt and the
file both have the same 118 lines. Again with only stderr piped (`2> >(cat)`, stdout to a file): both get the fps
lines.

Spec review: stdout was first captured too and sent to stderr's place (fixed: stderr only); dup2's result is now
checked; an extra banner line dropped.

## Not verified / not 1:1

- Not run on a real Steam Deck under Steam (no Deck here): checked with `SteamDeck=1` and stderr piped, as the task's
  check says. If Steam's stderr were a regular file it is left alone (already a log).
- The tee is a reader thread: lines written in the last instant before the process exits can miss the file and
  journald (no flush at exit). Fine for the fps/stall lines the task is for.
- Any piped launch (not just Steam) now also writes hst-log.txt with the frame-rate log on, as the task asks.
- Not gameplay; nothing to match against the original.

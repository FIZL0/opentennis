# M152e — Adhoc: tell the host when the firewall keeps guests out

M152d found the host PC's ufw dropping every phone's request until the user opened the port; the game said nothing.

## What it does now (`crates/hst-base/src/adhoc.rs`)
- The accept loop marks the server `reached` once a connection comes from an address that is neither loopback nor one
  of this machine's own interface addresses (netdev). After that the card never shows a firewall hint.
- `Status.hint` (until reached), from the firewall check (re-read every 2 s, so opening the port clears it live):
  - Linux, ufw on (`/etc/ufw/ufw.conf` `ENABLED=yes`) and `/etc/ufw/user.rules` readable: walk its `### tuple ###`
    lines in order, first incoming tcp/any rule covering the port (single, list, `a:b` range, `any`) decides
    (allow/limit = open, deny/reject = blocked); none → `DEFAULT_INPUT_POLICY` from `/etc/default/ufw`. A rule
    limited to some sources (a LAN range) counts as open. Blocked → shown at once:
    "This PC's firewall blocks guests: allow them with sudo ufw allow 8080/tcp". Open → nothing.
  - ufw on but rules unreadable (some distros make user.rules 0640), firewalld installed (`/etc/firewalld`), or
    nothing readable (Windows, macOS, other Linux): after 20 s with no guest, a "No one yet? …" line with the
    matching way to allow it (ufw / firewall-cmd command, Windows Firewall private networks, macOS Firewall settings,
    or "allow TCP port N").
- Card (`crates/hst/src/play/main_menu/online/adhoc.rs`): with a hint, the address sentence drops "in a browser … (or
  scan the code)" to keep it within the card's four lines.
- Shot state `HST_MENU=online-adhoc` (Adhoc on and selected).

## Checked
- On this PC (ufw on, DROP, no 8080 rule): `firewall(8080)` = blocked, `firewall(53317)` = open (it has an allow
  rule), `firewall(8443)` = blocked (deny rule). Unit test `ufw_rules` covers udp-only, ranges, LAN-only sources,
  deny-before-allow, outgoing rules, lists, interface-bound rules.
- Shots at 16:9 and 4:3 (`context/m152e/adhoc.png`, `adhoc43.png`): the hint fits in 3 and 4 lines.

## Not verified / not 1:1
- Not 1:1: new feature, no counterpart in the original.
- Not tested with a real phone after opening the port (the `reached` path): only reasoned; the user's phones were
  what M152d used.
- firewalld, Windows and macOS wording untested on those systems (no such machine here); firewalld's live rules need
  root to read, so it only gets the delayed "may block" hint.
- nftables/iptables rules written by hand (no ufw/firewalld) aren't read: those get the generic delayed hint.

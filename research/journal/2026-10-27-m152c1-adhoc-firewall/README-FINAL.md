# M152c1: adhoc behind one firewall rule

## What changed

- **One UDP port for every browser guest** (`net/rtc/native.rs`, *the pinned port*). While adhoc serves, `adhoc::set`
  calls `net::rtc::pin(Some(port))` with the page's own port number. Then every guest's WebRTC on the host goes through
  one socket per LAN address, bound to `<ip>:<port>` and shared by all guests (`Mux`). Packets are routed by the remote
  address:
  - an address we have sent to belongs to the connection that sent there (this is registered on each transmit);
  - otherwise, the first ICE binding request's USERNAME starts with our side's ufrag, and the address is claimed
    for that connection.
  Ended connections' entries drop out lazily, the next time a route or ufrag is registered.
  `pin(None)` (adhoc off) goes back to ephemeral per-connection sockets. A running match keeps the mux alive: it holds
  an `Arc`, and the readers stop when the last one drops.
  If no socket on the port binds, the host falls back to ephemeral sockets. Native dialling (guest side) is unchanged.
- **Port pick**: adhoc now takes the first of 80/8080/8000/8888 that's free for *both* TCP and UDP.
- **Firewall on the card** (Linux): `adhoc::firewall` looks for firewalld (`systemctl is-active firewalld`) or ufw
  (`/etc/ufw/ufw.conf` says `ENABLED=yes` and the unit is active). `Status.firewall` then gives the command, and the
  card says "This device's firewall blocks guests: run `<cmd>` first."
  - firewalld: `sudo firewall-cmd --add-port=P/tcp --add-port=P/udp`. This changes the running rules only, which suits
    adhoc being per run.
  - ufw: `sudo ufw allow P`, which covers both TCP and UDP.
  - Without either (and on Windows/macOS, which prompt on the first listen), the card keeps the generic hint.
- **Clear failure on the guest** (`rtc.rs` web `dial`): when signalling worked but the connection didn't come up, the
  message now names the port: "…If the host's firewall is on, it must let port P through for UDP too, not just TCP."

## Verified

- `net::rtc::tests::guests_share_the_pinned_port` (loopback, str0m both ends): two guests connect at once through the
  host's single pinned socket. Each one's message reaches its own connection and back, and the mux holds exactly one
  socket at the pinned port, routing to 2 live connections. This test and `a_native_host_takes_a_guest_by_signalling`
  share global state (listener, lobby, pin), so they take a lock and run one at a time.
- `native::tests::stun_user_names` (USERNAME parse; responses, DTLS and truncated packets give none) and
  `adhoc::tests::firewall_commands` (ufw.conf parse, the two commands).
- **Browser guest, real match** (`tools/web_guest.sh`, `context/m152c1/run2`): release host on port 8080, with a
  freshly built `web/pkg`.
  - Headless Chromium loaded the page, joined and started the match.
  - The rollback hashes are identical on both ends at tick 0 (`09b5ee62…`) and tick 1800 (`9714c52c…`).
  - `ss -uanp` during the match: the host's WebRTC UDP is only on `<ip>:8080` sockets, with no per-guest ephemeral
    ones (the rest are iroh/mDNS, as before).
  - run1 failed only because `web/pkg` was a stale Oct 16 build: the guest's wasm crashed at start.
- This machine runs ufw with `ENABLED=yes`, so the card would show `sudo ufw allow 8080`. The detection logic is
  covered by the unit test; it wasn't seen in a screenshot.
- The wasm build (`cargo build -p hst --target wasm32-unknown-unknown --release --features ship`) compiles with the
  guest message change.

## Not verified / not 1:1

- **The firewall itself was not exercised**: "connects with the rule, fails clearly without" needs a second device.
  On one machine, traffic to its own LAN IP goes over loopback, which ufw allows. Also, there is no sudo in an
  unattended run to add or remove a rule. The guest's new failure message wasn't seen live.
- **firewalld detection is untested on a firewalld machine**; only ufw is present here. Other setups (raw
  nftables/iptables, distros without systemd) aren't detected and get the generic hint.
- **The card's new line wasn't screenshotted**. Turning adhoc on in the menu needs a menu drive. The text goes through
  the same `words()` path as before.
- The mux also binds docker/VPN interface addresses at the port (`addresses()` takes every interface that's up, as the
  ephemeral path did).
- Not 1:1: a new feature with no counterpart in the original.

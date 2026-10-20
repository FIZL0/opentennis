# B598: the net's sound (FINAL)

Report (user 2026-10-20): a ball hitting the net seems to play no "thwip" in the remaster, though B85's word shows.

## What the original plays
Four recordings of slot 5 (`tools/record_sound.py 5 300 context/fixtures/net_sound_<name>.bin vel=V,x,y,z`; the new
`vel=` argument overwrites the live ball's velocity once after vsync V, as `research/b85_net_rec.py`):

| Recording | Velocity overwrite | Net contact | Game's key-on (court bank program 2) |
|---|---|---|---|
| `cord_over3` | 7864, (-0.056, -0.1, -0.55) | net mesh (material 2) | key 2, the frame after the touch |
| `serve_let` | 7785, (-0.148, 0, 0.58), a serve | net mesh (material 2) | key 2, the frame after |
| `net_low` | 7864, (-0.056, -0.06, -0.55) | the net's top band (material 26) | key 1, the frame after |
| `post_back` | 7864, (-0.092, -0.092, -0.58) | the post (material 22) | nothing |

- One voice per touch. Keys 1 and 2 share one sample and envelope; key 2 plays it about 10 semitones up (note 41
  over root 31), key 1 at its root (note 40). The pitch command's top half (root, note) tells them apart.
- The material table's sound key (N3c4) already gives exactly these: 2 → key 2, 26 → key 1, 22 → 0 (silent).

## The remaster
It already plays all of these, at the same moments, L/R bit-exact:
- `sound::Bounces` gives key 2 / key 1 / nothing for the four touches, and the same falloff and stereo as the game
  (the net's soft rule holds a second net touch silent until a court contact, as the original).
- `simulate` queues it while `Phase::Rally` (serves are Rally from the hit, so serve lets sound too). A 60000-tick
  random-walk match (scratch, not kept) queued a net sound on every one of its 23 net touches (serves and rallies,
  band and mesh), and with the sound app on, the mixer started a play on every touch tick.
- Every court bank (1..11) has program 2 keys 1 and 2, as loud as the bounce (peak ≈0.40, centred, full volume).

So the report could not be reproduced: nothing was missing or changed. Two tests now guard it:
- `hst-sim` `tests/sound.rs` `net_sounds_match_the_game`: the four recordings. Each net play keyed this frame or
  the next, L/R bit-exact, on its key's own pitch; the count of the game's net-sample key-ons equals ours; the post is silent.
- `hst-play` `play::snapshot::tests::a_net_touch_sounds`: a bot match's rally ball sent into the net mesh queues
  program 2 key 2 on the touch tick, and the mixer starts a play (0.6 s).

## Not verified / not 1:1
- Not compared by ear in a live remaster session (unattended run, no audio out). The mixer render, levels and
  pitch were checked in tests instead.
- The remaster queues the sound on the tick the bounce count changes; the game's driver keys it one frame later.
  This is the same for every bounce sound (N3c4's tests accept this frame or the next), under 17 ms.
- No net touch by the computer in a natural rally was recorded: all four are velocity overwrites in slot 5's
  rally/serve. The rule doesn't care who hit the ball.
- If the user still hears no net sound, the cause is somewhere these tests don't reach (output device, a volume
  setting, a court mod's bank). Next step would be a live `HST_ASSET_LOG` run showing `snd … 2:2` at a net touch.

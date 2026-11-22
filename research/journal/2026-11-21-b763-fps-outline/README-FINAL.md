# B763 Show FPS counter: dark outline instead of a drop shadow

Bevy UI text (0.20) has only `TextShadow`, one offset copy; there is no text outline, and the menus' outlined
words in `hst` (`look.rs`, `menu_art.rs`) are drawn into images by our own rasterizer, not usable for live UI text.
So the counter (`graphics.rs` `fps_counter`) now spawns a box holding eight dark copies of the words, each one PS2
line (1/448 of the 4:3 height, in the same Vh/Vw unit as the font size) off in the eight directions, with the yellow
words on top; `fps_outline` copies the text and font onto them when they change (4 times a second for On). The words
sit one line in from the box's edges so the window edge doesn't cut the outline off (it did in the first shot).
Detailed keeps its dark box (now on the outer box), with the outline too.

Checked with `tools/shot.sh … --play --court 1 --show-fps[-detailed] --shot-at 8` (HST_WEATHER=0), 1280x720 and a
600x900 tall window: even outline all round, no shadow, nothing clipped (`context/b763/run2/`).

## Not verified / not 1:1

none (remaster-only setting; the original has no FPS counter).

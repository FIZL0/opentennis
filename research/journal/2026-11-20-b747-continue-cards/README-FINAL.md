# B747 — Continue screen cards in our menu style

The Continue screen's save cards were dark navy cards with red/blue header bars (the character-select frame look),
unlike the main menu's panels. Restyled `continue_screen::card` after the main menu's preview panel:

- ink-rimmed paper box (`box3` INK / ring / PAPER), the ring gold (`home::ON`, as the main menu's chosen row) when
  the card is chosen, white otherwise;
- a picture area in the main menu's backdrop colours (Local green, Online blue, grey when it can't be resumed) with
  an ink rule under it, as the panel's picture;
- the chip in the main menu Continue picture's style (ink rim, dark green Ready / slate refusal);
- faces with an outlined white "vs", the mode ("Local singles") in outlined white over the picture's foot, as the
  panel's name;
- the names in ink on the paper, the navy mini board (same as the main menu's Continue picture), and Court / Saved
  as fact boxes (muted caps label, ink value) like the panel's facts.

Design size and grid unchanged, so it still scales as one at any aspect ratio (`cards_fit_each_aspect` passes).
A/B shots (`HST_MENU=continue-screen`, 1280×720 and 600×800, plus `continue-dead`): `context/b747/run1/before-*.png`,
`context/b747/run2/after-*.png`.

## Not verified / not 1:1

- Front-end screen: ours to design (PLAN.md), nothing to match against the original.
- The foot bar (navy strip) and the subtitle were left as they were: the task named the score card. On a tall view
  the subtitle sits well under the tab (the shared `home::span` layout), unchanged here.

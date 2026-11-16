# M167 — Settings → Gameplay tab

- The Mobile tab is now **Gameplay** (`Screen::Gameplay`, renamed from `Screen::Mobile`; `HST_MENU=gameplay`, `mobile`
  still works). It's the last tab, teal band as before.
- Rows: `GAMEPLAY` in `options.rs`. Vibration is last and only on a touch screen (`gameplay()` drops it elsewhere).
  M175 (Rules) and M172 (Match Rumble) add their rows **before** Vibration in `GAMEPLAY`. Once one of them is there
  on every platform, the tab shows on desktop too.
- `tabs_shown()` hides the tab while it has no rows (desktop today), so no empty page; it stays last, so `TABS[..5]`
  still works as the prefix that `tab()`/`step_tabs` index into.
- Layout tests (`pages_fit`, `short_pages_scroll`) now go through the tabs on show, because the page is empty without
  touch. Ran them with `HST_TOUCH=1` too: layout and `every_page` pass. Two Display/Graphics default tests fail under
  `HST_TOUCH=1` on main as well (touch defaults, nothing to do with this change).
- Shots (`HST_TOUCH=1`, debug build) at 1280×720, 720×1280 and 960×720 (the tablet layout): tab bar, page and side
  panel lay out fine. A desktop shot shows five tabs. Shots are in `context/m167/`.

## Not verified / not 1:1
- Front-end only; the original has no such tab (our own design, PLAN.md). No PINE check needed.
- The Rules (M175) and Match Rumble (M172) rows aren't here: those tasks haven't landed. This task only gives them a
  home, and they add the rows themselves.
- Not tried on a real phone. The touch layout was checked with `HST_TOUCH=1` on desktop.

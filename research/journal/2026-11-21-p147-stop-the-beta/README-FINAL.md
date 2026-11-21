# P147 — Stop the beta

User 2026-11-21: stop deploying beta builds and remove the beta links from the repo.

## What changed

- **pages.yml**: deploys the website to opentennis.dev only. The `beta` job is gone, and with it the `linux`,
  `linux-aarch64` and `windows` jobs (they built executables only for the beta bundle) and the `web` artifact the
  bundle took. No more `HST_SITE=https://beta.opentennis.dev/`. SITE_FROM keeps its meaning for opentennis.dev
  (unset/main: main's site; release: releases' site); with `release` a push to main now runs only `ci` (the site
  build is skipped instead of being built for the beta). No dispatch to FIZL0/opentennis-beta, no
  BETA_DISPATCH_TOKEN, no clean-up of the old rolling `beta` release.
- **tools/beta-site.yml** and **tools/beta-bundle.sh** deleted (the latter only fed the beta job).
- **update.rs**: only release builds (HST_VERSION) check, and only GitHub releases; the Actions-build commit check
  against beta.opentennis.dev/download/commit.txt is gone. `RELEASES` (the Download fallback page) is the GitHub
  latest-release page. HST_LATEST still fakes a notice. 13 update tests pass.
- **net.rs**: SITE comment; `HST_SITE` stays as a build-time override for a test copy of the site.
- **ship-local.sh**: builds without HST_SITE and refuses while SITE_FROM is release (opentennis.dev shows only
  releases then; there is no beta site to put HEAD on).
- **release.sh**: release notes no longer end with the beta link; header comment updated.
- **README, web/index.html** (beta noindex script and comment), **ci.yml / advisories.yml / done.sh** comments;
  advisories.yml's "stale beta" step (tag `beta` behind main) removed, since there is no beta tag.
- plan/HUMAN.md: dropped the obsolete "copy the new beta deploy workflow" item.
- GitHub: deleted the BETA_DISPATCH_TOKEN repo secret; the `beta` release/tag was already gone (checked:
  `gh release list` shows only v* tags). FIZL0/opentennis-beta left alone.

Check: `git grep -i -E "beta\.opentennis|opentennis-beta|beta-site|BETA_DISPATCH"` outside journal/plan is empty.

## Not verified / not 1:1

- pages.yml was not run (it runs only on main): checked as YAML and by reading the job conditions. With SITE_FROM
  currently `release`, the next push to main runs only `ci`; the next published release deploys the site.
- beta.opentennis.dev keeps serving its last bundle until someone takes down FIZL0/opentennis-beta's Pages (left
  alone as asked).
- Not a gameplay change: no comparison against the original applies.

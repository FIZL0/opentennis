# B745 — the rolling beta off GitHub Releases

Quiver Launcher took the `beta` pre-release (rebuilt under one tag on every push) for the newest version and flagged
the project as "rolling builds". Now this repo's only non-draft releases are tools/release.sh's vX.Y.Z.

## What changed

- **pages.yml `beta` job**: bundles the whole beta site with `tools/beta-bundle.sh` (new): the website, plus
  `download/` with the four builds (Windows zip, Linux x86_64 + aarch64 tarballs, PortMaster zip), an `index.html`
  listing them, `files.txt` and `commit.txt` (full commit). The bundle goes up as `beta-site.tar.gz` on a **draft**
  release of FIZL0/opentennis-beta (via BETA_DISPATCH_TOKEN; older drafts there deleted first), and the dispatch
  carries the draft's id. Then it deletes this repo's `beta` and `web-prebuilt` releases and tags (no-ops once gone).
  The "workflow files changed on main" skip went (it was about tagging, which no longer happens); the "beta already
  newer" check now reads https://beta.opentennis.dev/version.txt. A prebuilt (ship-local) run bundles the new website
  with `download/` copied from the live beta site, so the downloads stay.
- **Guard**: before it touches anything, the job checks opentennis-beta's deploy.yml mentions `beta-site.tar.gz`
  (i.e. is the new `tools/beta-site.yml`); if not it fails with an error and the old `beta` release stays, so the
  beta site never ends up with nothing to deploy. Copying that file is a plan/HUMAN.md item (agents don't push to
  other repos).
- **tools/beta-site.yml** (opentennis-beta's workflow): downloads the draft's bundle by id (or the newest draft when
  run by hand) with `contents: write` (drafts need push access to see) and deploys it as is.
- **ship-local.sh**: uploads web.tar.gz as a **draft** `web-prebuilt` release (gh api, id captured) and passes the id
  as pages.yml's new `release` input. A new `prebuilt` job (the only one with `contents: write`, no build) checks the
  id is a web-prebuilt draft, downloads the asset, deletes the draft and hands it on as the `web-prebuilt` artifact;
  `build` waits on it. The `tag_name != 'web-prebuilt'` release-event exclusion went (a draft never publishes).
- **release.sh**: the "newest changes" line points at beta.opentennis.dev and its /download/ page.
- **update.rs** (not on the task line, needed so beta builds keep their update notice): Actions builds read
  `download/commit.txt` from the beta site instead of the `beta` release's target commit; the notice's Download opens
  https://beta.opentennis.dev/download/. Test `the_beta_commit_is_a_full_hash`.

## Checked

- `tools/beta-bundle.sh` locally: with two fake builds (layout, page table, commit.txt), with no builds against a
  local HTTP server standing in for the live site (download/ copied), and with no builds against the real site (no
  download/ yet: website alone).
- Both workflows parse as YAML; current beta assets total ~267 MB, each < 60 MB (Pages limits are fine).
- opentennis-beta's current deploy.yml is identical to the old tools/beta-site.yml (so the guard will fire until it is
  replaced).

## Not verified / not 1:1

- Not run on GitHub Actions: the workflows can only run after the merge and a push (agents don't push), and the
  beta deploy also needs the HUMAN.md step. So the task's check (no `beta`/`web-prebuilt` release after a push, the
  builds served from beta.opentennis.dev) is unverified; in particular cross-host details: `gh api` following the
  asset download redirect, the uploads.github.com POST with the fine-grained token, and `client_payload[release]`.
- The `download-artifact` brace pattern `{web,linux,linux-aarch64,windows}` (minimatch braces) is untested in CI.
- Beta builds already out there ask for the `beta` release, which will be gone: they lose their update notice
  (silently, as on no network). New beta builds use the beta site.
- The beta website's own Download links (web/) still go to the latest release; not changed (web/ not on this task).

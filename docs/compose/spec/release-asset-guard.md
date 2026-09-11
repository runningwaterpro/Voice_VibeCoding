---
feature: release-asset-guard
status: delivered
updated: 2026-09-10
branch: fix/release-asset-guard
commits: 1643e16..HEAD
---

# Release Asset Guard

## Report

**What was built** — `.github/workflows/build.yml` now refuses to attach a release installer unless the tag, `package.json`, `tauri.conf.json`, and the single NSIS basename all agree on the same `x.y.z` version (matched as a `_version_` token so `1.0.1` cannot swallow `1.0.10`). On the release path it also deletes any existing `*-setup.exe` whose name lacks that token, and same-ref builds are serialized via a concurrency group. First tag pushes without a Release yet skip cleanup instead of failing. Separately, the wrong `Voice.VibeCoding_1.1.0_x64-setup.exe` asset was removed from the published v1.0.1 release via API.

**Verification** — Local decision-matrix dry-runs of the guard: PASS for match+stale-delete (`v1.0.2` keeps 1.0.2, deletes 1.1.0); FAIL for tag/`package.json` mismatch (historical `v1.0.1` vs `1.1.0`); FAIL for installer `1.0.10` under tag `v1.0.1`; PASS for `v1.0.10` keeping only `_1.0.10_`; FAIL for `v1.0.2-rc`. Workflow file re-read confirms concurrency group, guard `if:` aligned with upload `if:`, and `draft: false`. Fresh API check: v1.0.2 has only `Voice.VibeCoding_1.0.2_x64-setup.exe` (`sha256:0002a928…`); v1.0.1 has zero installer assets after deletion.

**Journey log** — softprops `overwrite_files` only replaces same-named assets, so a differently versioned installer accumulates; that is why v1.0.2 briefly held both 1.0.2 and 1.1.0. v1.0.1's filename was 1.1.0 because the tag pointed at code still at 1.1.0, not a bundler bug. Reviewer flagged unanchored substring version match; tightened to `_${version}_` token and `^vX.Y.Z$`.

## [S1] Problem

Release assets can carry the wrong installer version:

1. **v1.0.1** shipped `Voice.VibeCoding_1.1.0_x64-setup.exe` because the tag pointed at code whose `package.json` / `tauri.conf.json` version was still `1.1.0`. Filename comes from code version, not tag name.
2. **v1.0.2** briefly held both `1.0.2` and `1.1.0` installers. `softprops/action-gh-release` `overwrite_files` only replaces **same-named** assets; a prior build with a different version filename is never cleaned up.
3. Dual triggers (`push: tags: v*` + `release: published`) rebuild and re-upload without validating tag↔version consistency or removing stale mismatched assets.

Observed on 2026-09-10: user saw `Voice VibeCoding_1.0.2_x64-setup.exe` and `Voice VibeCoding_1.1.0_x64-setup.exe` on draft/published v1.0.2; v1.0.1 still had the 1.1.0-named installer until manual API deletion.

## [S2] Design

Harden `.github/workflows/build.yml` so a release upload cannot attach a wrong-version installer.

### Contracts

- **Release path condition** (unchanged): upload when `startsWith(github.ref, 'refs/tags/v')` OR `github.event_name == 'release'`.
- **Expected version** = tag name with leading `v` stripped. Tag must match `^v\d+\.\d+\.\d+$`. Non-conforming tag fails the job.
- **Source of truth for code version**: `package.json` `.version` and `src-tauri/tauri.conf.json` `.version`. Both must equal expected version, else fail before upload.
- **Installer filename check**: exactly one NSIS `*.exe` under `src-tauri/target/release/bundle/nsis/`; its basename must contain the delimited token `_${version}_` (Tauri shape `Name_version_arch-setup.exe`). Else fail.
- **Stale asset cleanup**: before upload, for the target tag's GitHub Release, delete every existing `*-setup.exe` asset whose name does **not** contain `_${version}_`. Same-name assets are left for softprops overwrite. If the Release does not exist yet (first tag push), skip cleanup.
- **Concurrency**: `concurrency.group` = `build-nsis-${{ github.ref }}`, `cancel-in-progress: false`.
- **Non-release builds** (main / fix/* / dispatch without tag): build + artifact only; no version-vs-tag gate, no release mutation.

### Failure behavior

Any contract violation fails the job **before** `softprops/action-gh-release`, so a bad build never mutates the Release.

### Out of band (already done)

- Deleted v1.0.1 asset `Voice.VibeCoding_1.1.0_x64-setup.exe` (API). v1.0.1 now has no installer assets; historical code at that tag still says 1.1.0 — not rebuilt.
- v1.0.2 currently holds only `Voice.VibeCoding_1.0.2_x64-setup.exe`.

## [S3] Out of Scope

- Rebuilding or renaming historical v1.0.1 binaries.
- Changing version fields on old tags.
- MSI upload, auto-tagging, version-bump automation, release notes generation.
- Non-Windows bundlers.

## Tasks

- [x] T1: Add pre-upload guard step in `build.yml` — acceptance: on release/tag path, job fails if tag/version/installer mismatch; deletes mismatched `*-setup.exe` assets; non-release path skips guard. (covers: S2)
- [x] T2: Add release concurrency group — acceptance: same-ref release jobs cannot run concurrently. (covers: S2)
- [x] T3: Validate workflow YAML locally — acceptance: `build.yml` parses; guard script logic reviewable. (covers: S2; depends: T1, T2)

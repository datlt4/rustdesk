# datlt4/rustdesk fork

## Release
1. Sync: `git fetch upstream && git rebase upstream/master` (on a branch, then merge into master).
2. Bump: `res/fork/set-version.sh 1.5.102` (pass the upstream version as a second argument when the sync moved it, e.g. `res/fork/set-version.sh 1.6.101 1.6.0`).
3. Merge into master. `.github/workflows/fork-release.yml` builds every OS with `flutter-build.yml`,
   signs `rustdesk-update.json`, and marks the release latest. A version that already has a tag is skipped.

Versions: `MAJOR.MINOR` follow upstream, the fork counts the patch from 101.
Peers and the rendezvous server are sent `UPSTREAM_VERSION` from `src/fork.rs`, never the fork version.

## Updates in the app
- Windows / macOS / Linux (deb, rpm, AppImage): "Update" downloads, checks the signed metadata and SHA-256, installs, restarts.
- Android: "Download new version" opens the universal APK of the latest release.
- A card shows when rustdesk/rustdesk has a newer release than `UPSTREAM_VERSION`.

## Secrets (Settings → Secrets and variables → Actions)
| Name | Value |
|---|---|
| `RUSTDESK_UPDATE_ED25519_SEED` | base64 seed; its public key is in `src/update_metadata.rs` |
| `ANDROID_SIGNING_KEY` | base64 PKCS#12 keystore |
| `ANDROID_ALIAS` | key alias |
| `ANDROID_KEY_STORE_PASSWORD`, `ANDROID_KEY_PASSWORD` | keystore password |

Losing the seed or the keystore means installed apps can no longer update in place; keep backups.

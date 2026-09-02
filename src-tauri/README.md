# `src-tauri` — the Android and Linux host

The second implementation of the one TypeScript surface (DECISIONS 0005,
[0093](../docs/DECISIONS.md#0093--two-cores-one-frontend-the-tauri-host-is-an-invoke-bridge)):
the same `cabas-app`, compiled natively, reached over `invoke` instead of over
`wasm-bindgen`. `src/lib.rs` is the whole of it and it is nothing but
translation — read `crates/app/src/wasm.rs` beside it, because the two are the
same surface twice and a method in one and not the other is a bug in whichever
is missing it.

## Why it is here and not under `crates/`

`cargo tauri android init` generates a Gradle project in `gen/android`, next to
`tauri.conf.json`. Keeping that out of `crates/` — which holds hand-written
Rust — is worth one top-level directory, and it is the same trade
`cabas-relay/` already makes for the Supervisor's sake.

## Why it is not in `default-members`

On Linux the `tauri` crate links the desktop GUI stack (dbus, gtk3, libsoup,
webkit2gtk), so a crate that built by default would put that closure in the
everyday `nix develop` and in every CI run. Cross-compiled to Android it needs
none of it, because there the webview is Android's own.

The consequence is stated rather than hidden: **nothing in CI compiles this
crate today.** `tauri-check` in the `.#android` shell is what checks it, and it
stays a local gate until M8 brings a desktop shell that needs those libraries
anyway.

## Commands

Run these from `src-tauri/` — the CLI looks for `tauri.conf.json` beside it.

```sh
nix develop ..#android                                  # the CLI, the NDK, NDK_HOME
tauri-check                                             # clippy, aarch64-linux-android
cargo-tauri android init --skip-targets-install         # generates gen/android — once
cargo-tauri android dev                                 # onto a connected phone
cargo-tauri android build --debug --apk --target aarch64
```

`--skip-targets-install` because the Rust targets come from the flake and not
from a rustup this shell does not have. **The first build needs the network
and takes minutes**: `gradlew` fetches its own Gradle and the whole AGP tree
into `~/.gradle` (about 1.5 GB), none of it Nix's.

## The icons

Generated, committed, and **not** produced by `ui/tools/render-icons.mjs`.
That renderer drives chromium, and chromium writes an opaque screenshot as a
colour-type 2 PNG; Tauri's bundler refuses anything that is not RGBA, from a
proc macro, so it surfaces as a compile error in `lib.rs` rather than as a
problem with a file. Its own generator does it correctly:

```sh
nix develop .#android -c cargo-tauri icon ui/public/icons/icon-512.png -o src-tauri/icons
```

It also emits iOS, macOS and Windows sets, and those are **deleted** rather
than committed: iOS ships as a PWA (0003) and neither of the other two is a
target (0002). What is kept is `icon.png` (the bundler's source), `android/`
(the launcher mipmaps), and the three Linux desktop sizes.

## The frontend it packages

`../ui/dist-tauri`, and **not** `../ui/dist`. `cargo tauri android build` runs
`beforeBuildCommand`, so a shared directory would mean an Android build
silently replacing the bundle `crates/relay/build.rs` compiles into the relay
(DECISIONS 0048) with one that has no wasm and no service worker in it —
green everywhere, blank on the phones. `vite.config.ts` picks the directory
from the mode so there is no flag to forget.

## Signing, and why CI builds a debug APK

`cargo tauri android build` without `--debug` produces
`app-universal-release-unsigned.apk`, and an unsigned APK cannot be installed
at all. So the CI job builds the debug one, which Gradle signs with a debug
keystore.

**That keystore is generated per machine, and a CI runner is a new machine
every time.** The consequence is concrete rather than theoretical: installing
build N+1 over build N fails with `INSTALL_FAILED_UPDATE_INCOMPATIBLE`, and
the only way through is to uninstall first. Uninstalling takes the app's data
directory with it — the replica, the photos, and `identity.json`. The library
comes back from the relay, but the **device identity does not**: the phone
mints a new device id, joins as a new device, and leaves a dead peer on the
group's roster for good. That is the defect DECISIONS 0068 exists to prevent,
arriving through a different door.

So the debug APK is fine for *a* build on *a* phone, and it is not a way to
keep a phone updated. Closing that needs one decision and one secret:

1. Generate a keystore once, off CI, and keep it — losing it means never
   updating that installation again.
2. Put it in the repository's secrets, base64'd, with its passwords.
3. Add a `signingConfigs` block to `gen/android/app/build.gradle.kts` (which
   is committed for exactly this kind of edit) and build `--release`.

Until then, treat every CI APK as a fresh install.

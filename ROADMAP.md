# Roadmap — cabas

Binding rules: [CONSTITUTION.md](CONSTITUTION.md). Historical record of the
choices behind this plan: [docs/DECISIONS.md](docs/DECISIONS.md). Every
milestone has a demonstrable goal and a **measurable exit criterion**; a
milestone is not started until the previous one's criterion holds.

**Closing a milestone includes looking at CI on the commit that closes it.** Not
at the rule that says it must be green — at the run. From M2 to M4 the browser
job failed on every single push while these pages said the vertical was
verified on each one, and nobody was told, because nobody looked. A red gate
that goes unread is worse than an absent one: it costs the same to run and it
buys a false belief instead of no belief.

## Overview (status as of 2026-08-08)

| Milestone | Content | Exit criterion | Status |
|---|---|---|---|
| **M0** | Scaffolding: workspace, nix flake, docs, CI | `cargo test --workspace` and `wasm-check` green inside `nix develop`, green CI | ✅ |
| **M1** | Domain: units, conversions, scaling, recipe DAG, cart derivation | Property tests green; a recipe list produces a correct aggregated cart, offline, in `cargo test` | ✅ |
| **M2** | Store: Loro schema, snapshots, `Storage` trait | Round-trip persistence on both backends; two in-memory replicas converge | ✅ |
| **M3** | App surface: commands, view-models, wasm + native bindings | Both targets build in CI; a scripted shopping scenario runs headless | ✅ |
| **M4** | **PWA, single device**: Svelte UI, offline, installable | Installed on the iPhone, usable in airplane mode, data survives a cold restart | ✅ |
| **M5** | Relay + sync: axum, E2EE, pairing, users, attribution | Two devices converge, **including when never online at the same time** | ⬜ |
| **M6** | Deployment: HAOS add-on, CI image, Cloudflare Tunnel, backups | Reachable from 4G; a backup restore is tested and works | ⬜ |
| **M7** | Android via Tauri v2 | APK installed; same frontend, native core; parity with the PWA | ⬜ |
| **M8** | Linux desktop via Tauri | Runs on NixOS from the flake | ⬜ |

Legend: ✅ done · 🚧 in progress · ⬜ not started.

**Why M4 comes before M5.** The PWA is the mandatory target (it is the only
way onto iOS — DECISIONS 0003), and shipping it single-device first proves
the whole vertical — domain → store → app → wasm → Svelte → installed on a
phone — while every bug still has one replica and one cause. Adding sync
before that would mean debugging a distributed system and an unproven stack
at the same time. Between M4 and M5 the app is genuinely usable by one
person on one device; that is a deliberate, shippable state.

---

## Resuming work (fresh session)

```sh
nix develop                                            # or `direnv allow`
cargo nextest run --workspace                # tests
wasm-check                                   # Rule 8: the four shared crates on wasm32
cargo clippy --workspace --all-targets -- -D warnings
check-wasm-bindgen                           # CLI/crate version match (Rule 13)
nix develop .#wasm-test -c wasm-test         # IndexedDB, in headless chromium

# The PWA (M4)
pnpm -C ui install                           # once
build-wasm [--dev]                           # the core, into ui/src/lib/wasm
pnpm -C ui check                             # types, against the generated bindings
pnpm -C ui dev                               # dev server, reachable from the phone
pnpm -C ui build                             # ui/dist
nix develop .#wasm-test -c ui-test           # the whole vertical, in a browser
ui-serve                                     # serve ui/dist over TLS, for the phone
```

`build-wasm` comes before anything that type-checks the frontend: the glue it
writes into `ui/src/lib/wasm/` is a build product, gitignored, and `core.ts`
imports it.

All project knowledge lives in the repo: binding rules in
[CONSTITUTION.md](CONSTITUTION.md), the reasoning behind every choice in
[docs/DECISIONS.md](docs/DECISIONS.md), setup and commands in
[README.md](README.md).

One command generates the frontend's types; it is not part of the everyday
loop, but CI fails if its output is stale:

```sh
cargo test -p cabas-app --features typescript export_bindings
```

**Next action: M5 — the relay and sync.** M4 is closed: the app is installed on
the iPhone, it opens in airplane mode, and its library is there when it does.
What the device settled, and what it did not, is recorded at the end of the M4
section below. M5 is the first milestone with two replicas in it and the first
with cryptography — `cabas-relay` on axum, one shared family key, pairing with
its recovery phrase, and the sync seam M3 deliberately left unfinished
(`App::version`, `App::changes_since`, `App::merge`, bytes in and bytes out).

Putting a later build on the phone again is two commands, since the certificate
is installed and trusted once and for all:

```sh
build-wasm && pnpm -C ui build     # the artifact that gets installed
ui-serve                           # prints both URLs and the CA fingerprint
```

Install from **the same address every time**: the origin is the app's identity
on iOS (DECISIONS 0012), and a different one is a different app with empty
storage.

One thing M4 could not observe, because it takes a *second* build: the **iOS
update path**. A new build's worker installs on one launch and takes over on the
next, since activating early would delete the caches a running page is still
loading from. The next time a build goes onto the phone is the first chance to
watch it, and it is not worth changing before it is seen.

The frontend's shape, for anyone picking it up: `ui/src/lib/core.ts` is the
only place the wasm `any` meets a generated type, `ui/src/lib/session.svelte.ts`
holds the one piece of state and the save policy, and `ui/src/lib/labels.ts` is
the only file with French tables in it. `ui-test` drives the whole thing in a
browser, recipe editor included.

---

## M0 — Scaffolding

- [x] 5-crate workspace (`domain`, `store`, `sync`, `app`, `relay`), resolver 3, edition 2024
- [x] Nix flake: stable toolchain from `rust-toolchain.toml` with the `wasm32-unknown-unknown` target, wasm tooling (`wasm-pack`, `wasm-bindgen-cli`, `binaryen`), Node/pnpm for the frontend, `check-wasm-bindgen` guard
- [x] Separate `.#android` shell so the multi-GB SDK/NDK is not in the everyday shell (its pins are validated at M7, not before)
- [x] Versions centralised in `[workspace.dependencies]` with per-milestone comments (Rule 13)
- [x] Crate-level docs stating each crate's constitutional boundary
- [x] [CONSTITUTION.md](CONSTITUTION.md), [ROADMAP.md](ROADMAP.md), [docs/DECISIONS.md](docs/DECISIONS.md), [README.md](README.md)
- [x] `wasm-check` and `check-wasm-bindgen` helpers in the shell, so Rules 8 and 13 are one command each
- [x] GitHub Actions CI **running inside the flake**: `fmt`, `clippy -D warnings`, tests, `wasm-check`, `check-wasm-bindgen`, plus an eval-only guard on the Android shell
- [x] `LICENSE-MIT` + `LICENSE-APACHE` (DECISIONS 0027)
- [x] `CLAUDE.md` session guide
- [x] Pushed to `github.com/alexisjapas/cabas`
- [x] **Green CI confirmed** — the first run passed every gate *that existed
      then*. The two things that could only fail on a runner, and did not: the
      `cachix/install-nix-action` version pin, and the cold-cache cost of
      `nix develop`. The browser job arrived at M2 and did not pass until M4;
      the note under M2 says why.

**Exit**: `cargo test --workspace` and `wasm-check` green inside
`nix develop`; green CI. ✅

## M1 — Domain

Goal: the whole product logic, correct and tested, with nothing on screen.
This is the dense part of the project (Rule 1).

- [x] **Units and quantities**: `Dimension` (mass / volume / count / unmeasured), unit variants carrying their locale (FR vs US tablespoon, metric vs US cup — Rule 5), exact rationals throughout (Rule 4), conversion within a dimension
- [x] **Cross-dimension conversion**: per-ingredient density (g/ml) and unit weight (g/piece); absent the coefficient, amounts stay on separate lines — never a guess
- [x] **Ingredient**: canonical entity, aliases, aisle (the cart's sort order), `staple` flag
- [x] **Recipe**: ingredient usages, `servings`, optional **`yield`** — without a yield a sub-recipe is not scalable (DECISIONS 0017)
- [x] **Instruction steps as segments**: `Text` | `IngredientRef { usage, display }`, so quantities re-render at the scaled amount; dangling refs render as a warning, never panic and never block deletion (DECISIONS 0022)
- [x] **Sub-recipe DAG**: expansion with cycle detection and a depth bound
- [x] **Scaling**: by servings or by yield, exact
- [x] **Cart aggregation**: group by (ingredient, dimension), sum in base units, sort by aisle; `Count` rounds up in the cart while the recipe keeps the exact value (DECISIONS 0016)
- [x] **Check-state derivation** (Rule 3): explicit overlay wins; default is `AutoChecked` for a staple sourced only from recipes, `ToBuy` otherwise; explicit `Unchecked` is persisted; adding an ingredient to the list purges its overlay entry
- [x] **List entry completion**: an entry disappears once all its ingredient contributions are checked; a recipe shows partial progress meanwhile (DECISIONS 0020)
- [x] Property tests (Rule 11): `scale ∘ aggregate == aggregate ∘ scale`, conversions round-trip, expansion terminates, `Unchecked` survives re-derivation
- [x] `finish_shopping`, pruning the overlay selectively so a partially bought entry keeps its progress (DECISIONS 0028)

**Exit**: a shopping list holding recipes, sub-recipes and bare ingredients
produces a correct aggregated cart in `cargo test`, with no I/O. ✅ —
`crates/domain/tests/shopping_scenario.rs`, 69 tests green.

One deliberate gap in the property tests: `scale ∘ aggregate ==
aggregate ∘ scale` is asserted on mass only. Rounding a countable line up is
not linear, so the identity genuinely does not hold there — asserting it
would be asserting a bug.

## M2 — Store

- [x] **Domain prerequisite**: `User`, `Device` (`people`) and the capped
      event log (`event`) — the schema has to persist them (DECISIONS 0024)
      and M1 had only minted their ids. Pure types, so they belong in
      `domain` rather than being invented by `store`
- [x] Loro document schema: recipes, ingredients, the single list, the check overlay, users, devices, the event log — laid out in `crates/store/src/schema.rs`, which is the file to read first (DECISIONS 0029)
- [x] Mapping both ways between plain domain structs and the CRDT — no Loro type escapes (Rule 2), not even on the error path or in the sync surface, where versions travel as opaque bytes
- [x] Snapshot serialisation; history compaction so the document does not grow without bound (`compacted_snapshot`)
- [x] `Storage` trait; file backend (native), with an atomic write — a snapshot is the whole library, so a half-finished save is a destroyed one
- [x] IndexedDB backend (wasm), tested in headless chromium — the only place IndexedDB exists (DECISIONS 0030)
- [x] Convergence tests: two in-memory replicas, concurrent edits, including check/uncheck of the same ingredient, plus the never-online-together case through a relay
- [x] Measure a realistic document (≈200 recipes) — snapshot size and load time budget the PWA's cold start

**Measured** (`crates/store/tests/document_size.rs`, 200 recipes over a
300-ingredient vocabulary, x86-64 release):

| | |
|---|---|
| Snapshot | **154 kB** |
| Compacted snapshot | **101 kB** (−34 %) |
| `Document::load` | **0.42 ms** |
| Read the whole library back | **9.8 ms** |

That settles the premise of DECISIONS 0008: the library is a 154 kB blob, so
a serialized snapshot is the right shape and SQLite would have been solving a
problem this project does not have. Cold start costs ~10 ms of core time
natively; the wasm figure will be some multiple of that and gets measured on
the actual phone at M4, which is the only place the number means anything.
The debug build is ~10× slower (85 ms to read back) — worth knowing before
anyone benchmarks a dev build and panics.

**Exit**: persistence round-trips on both backends; two replicas converge on
concurrent edits. ✅ — round-trip on the file, memory and IndexedDB backends
(`crates/store/tests/indexeddb.rs` runs in a real browser); convergence in
`crates/store/tests/persistence.rs`, including the case where the two
replicas are never online at the same time. 33 native tests plus 5 in
chromium.

**That last figure was a local one until M4.** CI ran the browser job from the
day this milestone wrote it and it failed on every push, for a reason with
nothing to do with the tests: chromedriver launches the browser it finds, not
the one the flake pins, so on a runner it started Google Chrome 150 against
chromedriver 151 and refused the session. The runner then carried on with the
failed session's id, so the only thing reaching the log was `http status: 404`
— naming neither Chrome nor a version. Fixed at M4 in 41444f7. For twelve
commits the claim above was true on a developer machine and untrue in CI, which
is the exact shape of failure the note at the top of this file now guards
against.

## M3 — App surface

- [x] Command set — 13 of them, coarse-grained on purpose (Rule 9): the
      library (`SaveIngredient`, `SaveRecipe`, and their deletions), the list
      (`AddRecipeToList`, `AddIngredientToList`, `SetEntryServings`,
      `RemoveListEntry`), the cart (`ToggleCartItem`, `FinishShopping`), and
      what is on screen (`OpenRecipe`, `CloseRecipe`, `RenameUser`)
- [x] A single state stream: every change returns a **complete** `StateView`,
      rebuilt from the document; no getter surface (DECISIONS 0033)
- [x] `wasm-bindgen` bindings (PWA) — `CabasApp`, with `apply` synchronous and
      `flush` async so a render never waits on IndexedDB (DECISIONS 0032).
      The native entry point is `App` itself over `FileStorage`; Tauri's
      `invoke` handlers at M7 wrap the same three calls
- [x] TypeScript types generated by `ts-rs` behind an optional feature, into
      `ui/src/lib/bindings/`, with CI failing on a stale file (DECISIONS 0036)
- [x] Platform abstractions: a `Platform` trait for the clock and the
      randomness (`web-time`, `getrandom` with its web backend, both proven
      in a browser), storage already behind `store`'s trait
- [x] Headless scenario test, running **natively and in headless chromium**:
      build a library, put a recipe on the list for six, rescale it, uncheck
      a staple, add it by hand, finish the trip, reopen from storage
- [x] A third browser test drives the `wasm-bindgen` binding itself — a
      command built as a JS object, through IndexedDB, back as a state object

**Exit**: both targets build in CI; the scenario test passes on both. ✅ —
121 native tests plus 3 in chromium (`crates/app/tests/scenario.rs` is the
same file on both targets). Half of that was CI's word and half was not: both
targets did build there, but the chromium half only *ran* in CI from M4 on —
see the note under M2.

**The transport trait is deliberately absent.** M3 was to put the two swaps
of DECISIONS 0005 behind traits, and it did: storage is `store`'s `Storage`,
the core is the crate itself. The network is not a swap — there is one
transport, it does not exist yet, and a trait invented now would be shaped by
guesses about a protocol M5 has not written. What M3 does provide is the seam
it will drive: `App::version`, `App::changes_since` and `App::merge`, bytes
in and bytes out, with no protocol in sight.

Two things M3 uncovered, both now load-bearing:

- **`getrandom` on `wasm32-unknown-unknown` needs two separate opt-ins** —
  the `wasm_js` feature *and* `--cfg getrandom_backend="wasm_js"` in
  `.cargo/config.toml`. Either alone fails, loudly at compile time here and
  much less loudly at M5's key generation.
- **`serde-wasm-bindgen` serialises `None` as `undefined` by default**, while
  the generated TypeScript says `| null`. A UI written against those types
  would have tested for a value that never arrives; the serializer is now
  configured with `serialize_missing_as_null`, and a browser test asserts it.

## M4 — PWA, single device

The first genuinely usable artifact.

- [x] Svelte 5 frontend, vanilla CSS with design tokens only (Rule 10),
      typed against the generated `ui/src/lib/bindings/*.ts`. Plain Vite, no
      SvelteKit (DECISIONS 0037)
- [x] Device identity in `localStorage`, minted once by `CabasApp.mintIdentity`
      — nothing else can run before the device knows who it is (DECISIONS 0031)
- [x] The French label tables: units, aisles, check states, problem kinds.
      They live here and only here (DECISIONS 0035)
- [x] The cart screen — the home screen, because that is what you open in the
      shop: grouped by aisle, one tap per line, progress, finish the trip
- [x] Two collapsed sections at the bottom of the cart: "Acheté" and "Déjà à
      la maison", which do not mean the same thing (DECISIONS 0023)
- [x] The list screen: entries, per-entry progress, rescaling, problems
      rendered in place (DECISIONS 0034)
- [x] The ingredient library: create, edit, delete, aisle, staple, and the two
      conversion coefficients
- [x] Settings: rename the person this device belongs to
- [x] Current screen persisted, so an iOS cold reload resumes where you were
      (DECISIONS 0003)
- [x] End-to-end test in a real browser — `ui-test`, the frontend's
      counterpart to `crates/app/tests/scenario.rs`
- [x] Recipe view and recipe edit — the two biggest screens, and with them the
      last five commands (`SaveRecipe`, `DeleteRecipe`, `AddRecipeToList`,
      `OpenRecipe`, `CloseRecipe`). **Every command is now reachable from the
      UI.** Read at any serving count, added to the list at the count it was
      read at
- [x] `@`-mention autocomplete in the step editor, scoped to the recipe's own
      usages (DECISIONS 0022). A line is named when it is added rather than
      when it is saved, so the whole recipe — lines and the prose pointing at
      them — goes out in one command (DECISIONS 0039)
- [x] Service worker, offline-first; **no user action ever waits on the
      network** (Rule 6) — hand-written (DECISIONS 0038). Cache-first over a
      shell precached from the bundle Vite just produced, so the precache list
      and the cache name are both build outputs and neither can be forgotten.
      The files copied verbatim out of `public/` are picked up by a runtime
      cache in the same versioned bucket instead
- [x] Manifest and icons: `manifest.webmanifest`, an `apple-touch-icon` because
      iOS ignores SVG for the home screen, and a maskable variant. The PNGs are
      committed and rasterised from their SVG source by
      `ui/tools/render-icons.mjs`
- [x] Scroll position persisted alongside the screen — per screen, so returning
      to a tab returns to where it was, and a cold reload does not drop you at
      the top of a list you were halfway down. `history.scrollRestoration` is
      `manual`: the browser's own restoration aims at a document that has not
      rendered yet, since this one waits for the wasm core
- [x] `visualViewport` keyboard handling — the covered height, measured and
      published as `--keyboard-inset`, which the layout reads through `max()`
      (DECISIONS 0040). A screen's body pads by the larger of the tab bar and
      the keyboard, the bar goes down with it, and the mention picker — the one
      control that opens *because* of what was typed, and so opens into the
      keys — is scrolled out of them. At `0px` every one of those expressions
      is the one that was there before, so there is no mode to leave
- [x] `ui-serve` — the built PWA over TLS, from a local CA signed for this
      machine's `<hostname>.local` and its LAN address, plus the plain-HTTP
      endpoint the phone installs that CA from (DECISIONS 0041). Without a
      secure context there is no service worker at all, so the LAN dev server
      could show the app on the phone and never make it installable. Verified
      by running `ui-test` unchanged against the TLS origin through `APP_URL`
- [x] Installed and tested on the actual iPhone, in airplane mode

**Exit**: installed on the iPhone from the home screen, fully usable offline,
data survives a cold restart of the app. ✅ — installed from the home screen,
a library built on the device, then airplane mode and a force-quit: the app
opened and everything was there.

**What the device settled.** Three questions had been left open on purpose,
because a browser can only rehearse them:

- The **cold start is instantaneous** on the phone. That closes the size
  question this milestone was told to defer: 713 kB gzipped of wasm is not worth
  working on, and the core stays as it is. M2's premise held at both ends — the
  document is small, and the engine, while not small, is not the problem either.
- The **keyboard behaves as designed** — fields stay visible, the mention picker
  opens above the keys, the tab bar goes down with them. Every number behind
  that came from `visualViewport` on a simulated keyboard until now
  (DECISIONS 0040), so this is the first evidence any of it was right.
- The **install works**, with one correction to how it is reached: see below.

**What it corrected.** `<hostname>.local` was the recommended origin and it does
not resolve, because NixOS enables avahi as a resolver and leaves
`publish.enable` off — the host never announces its own name, and
`avahi-resolve -n $(uname -n).local` times out on the machine itself. The app is
therefore installed from the LAN address, which makes the DHCP lease part of the
app's identity (DECISIONS 0012): **reserve it on the router, or the app loses
its library the day the lease moves.** Turning avahi publishing on is the other
answer and is written up in the README.

The **iOS update path** stays unobserved, and cannot be observed until a second
build reaches the phone. It is the one M4 question carried into M5.

**Measured** (release build, `wasm-release` + `wasm-opt -Oz`):

| | |
|---|---|
| wasm core | **1.81 MB**, 713 kB gzipped |
| JS bundle | 106 kB, **36.8 kB gzipped** |
| CSS | 26.5 kB, 4.1 kB gzipped |
| service worker | 0.97 kB, 0.6 kB gzipped |

The recipe screens cost 5.5 kB gzipped of JS and 1 kB of CSS — the two
biggest screens in the app, against a core that is twenty times the whole
frontend put together. The service worker is a rounding error next to the
thing it exists to keep on the phone.

Five things this half of M4 uncovered, three of them invisible until the network
is actually off and one until a keyboard is in front of it:

- **`Vary` makes a precache miss its own entries.** A server that answers
  `Vary: Origin` — Vite's preview does — makes the Cache API match on the
  request's `Origin` header, and the worker fills the precache with requests
  that have none while the page asks for its JS and CSS with one, because Vite
  marks both tags `crossorigin`. Every asset present, every lookup a miss, and
  online it is invisible because the miss falls through to a network that
  answers. `cache.match(request, { ignoreVary: true })` is the fix and the
  reason it is not a shortcut: every URL here has exactly one representation.
- **A build tool that rewrites string literals can hide a placeholder.** The
  precache list is injected by replacing a token in the built worker, and
  rolldown's minifier normalises quotes to backticks — a pattern that only knew
  about `'` matched nothing. The build now fails on a token it cannot find,
  because the alternative is shipping a worker whose cache is named after the
  placeholder.
- **Emulated "offline" is per-target and per-document.** A service worker is
  its own DevTools target, so a page put offline still has a worker behind it
  that reaches the network on a cache miss; and the emulation does not survive
  a navigation. Both are handled in `ui/tests/smoke.mjs`, and until they were,
  the offline test passed against a server that was up the whole time.
- **A `scroll` event arrives a frame after the scrolling**, so the ones left
  over from a screen being left are delivered once `screen` already names the
  one arriving — and recording them overwrites the offset that was about to be
  restored. Switching tabs and back landed at the top about half the time.
  `Session` reads the outgoing offset synchronously in `show()` and ignores
  scroll events until the restore has run.
- **A keyboard cannot be emulated by making the window smaller.** iOS keeps the
  layout viewport at full height and draws the keys over it, so the interesting
  case is a page that still believes it is 640 px tall while `visualViewport`
  says a third of that is gone — and every DevTools command that shrinks
  anything shrinks the layout viewport, telling the page the truth and testing
  nothing. Overriding the `height` accessor and firing `resize` is what
  reproduces the shape (DECISIONS 0040).

The core is the whole download, and Loro is most of the core — M2's premise was
that the *document* is small (154 kB), and it is; the *engine* is not. That was
the number to watch, and the device answered it: the cold start is
instantaneous, so the size stays as it is. What is still untested is the
*first* download over 4G rather than over the LAN, which is an M6 concern —
by then the shell is precached and the question only arises once per install.

## M5 — Relay and sync

- [ ] `cabas-relay`: axum, WebSocket per family, **persists the encrypted snapshot and deltas** — a pure broadcast relay never reconciles two devices that are never online together
- [ ] E2EE: XChaCha20-Poly1305, one shared family key, sealed before leaving the device (Rule 7)
- [ ] Pairing by QR code, **with the 12-word recovery phrase as a mandatory fallback** — the camera is historically brittle in an installed iOS PWA, and the phrase doubles as the key backup (DECISIONS 0021)
- [ ] Users and devices: pairing asks who you are; a device screen that states plainly that revoking means rotating the key and re-pairing everyone
- [ ] Attribution: `added_by`, `checked_by`, capped event log — declarative, never presented as access control (Rule 7). The log is already *written* by every deletion and edit since M3; what is missing is a view-model for it and a screen
- [ ] Drive the sync seam M3 left: `App::version`, `App::changes_since`, `App::merge` — opaque bytes, sealed by `sync` on the way out
- [ ] Sync on foreground, live WebSocket while active, **no background sync** (DECISIONS 0011)
- [ ] Test: two replicas that are never online simultaneously still converge through the relay

**Exit**: two devices converge in both the simultaneous and the
never-simultaneous case.

## M6 — Deployment

- [ ] Home Assistant OS add-on: repo layout, `config.yaml`, `build.yaml`
- [ ] CI builds the arm64 image (Svelte bundle embedded into the binary via `rust-embed`) and pushes to ghcr.io; the add-on references the prebuilt image rather than building on the Pi
- [ ] Data in `/data` so HA's own backups cover it — the recovery point if all devices are lost
- [ ] Cloudflare Tunnel onto an owned domain; **the origin is permanent** — changing it later makes iOS treat the PWA as a new app and drops its storage (DECISIONS 0012)
- [ ] Restore drill: wipe a device, re-pair, verify the data comes back

**Exit**: reachable from 4G; a backup restore is tested end to end.

## M7 — Android (Tauri v2)

- [ ] Validate the `.#android` shell pins (SDK, build-tools, NDK) — deliberately unvalidated until now
- [ ] Tauri v2 wrapper around the **unchanged** Svelte frontend; the Rust core switches from wasm to native, storage from IndexedDB to a file — both already behind traits since M3
- [ ] `arm64-v8a` first, `armeabi-v7a` only if an actually old device needs it; `--split-per-abi`
- [ ] APK distributed to the family directly (no store)

**Exit**: APK installed, feature parity with the PWA, native core.

## M8 — Linux desktop

- [ ] Tauri desktop build from the flake
- [ ] Known caveat: webkit2gtk compositing may need `WEBKIT_DISABLE_DMABUF_RENDERER=1` on some setups

**Exit**: runs on NixOS from the flake.

---

## After v1

Not scheduled, and out of scope until a DECISIONS entry reopens them
(Rule 14):

- **Recipe import** from the web (schema.org/Recipe). The unit parser is
  already built locale-aware at M1 so this does not become a data migration.
- **Push notifications** ("someone added milk"). Would need APNs + FCM, and
  must send *content-free* pushes that trigger a local sync, or the
  zero-knowledge property is lost at Apple and Google.
- **Pantry / stock**, **multiple lists**, **ad-hoc items at the shop** —
  explicitly cut (DECISIONS 0018).
- **Visual identity** — the vanilla look is deliberate and temporary
  (DECISIONS 0026); Rule 10 is what keeps the cost of doing it later low.
- **i18n** — the app is French-only for now; the persisted language of the
  repo stays English regardless.

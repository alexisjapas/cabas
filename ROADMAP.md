# Roadmap — cabas

Binding rules: [CONSTITUTION.md](CONSTITUTION.md). Historical record of the
choices behind this plan: [docs/DECISIONS.md](docs/DECISIONS.md). Every
milestone has a demonstrable goal and a **measurable exit criterion**; a
milestone is not started until the previous one's criterion holds.

## Overview (status as of 2026-08-08)

| Milestone | Content | Exit criterion | Status |
|---|---|---|---|
| **M0** | Scaffolding: workspace, nix flake, docs, CI | `cargo test --workspace` and `wasm-check` green inside `nix develop`, green CI | ✅ |
| **M1** | Domain: units, conversions, scaling, recipe DAG, cart derivation | Property tests green; a recipe list produces a correct aggregated cart, offline, in `cargo test` | ✅ |
| **M2** | Store: Loro schema, snapshots, `Storage` trait | Round-trip persistence on both backends; two in-memory replicas converge | ✅ |
| **M3** | App surface: commands, view-models, wasm + native bindings | Both targets build in CI; a scripted shopping scenario runs headless | ⬜ |
| **M4** | **PWA, single device**: Svelte UI, offline, installable | Installed on the iPhone, usable in airplane mode, data survives a cold restart | ⬜ |
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
```

All project knowledge lives in the repo: binding rules in
[CONSTITUTION.md](CONSTITUTION.md), the reasoning behind every choice in
[docs/DECISIONS.md](docs/DECISIONS.md), setup and commands in
[README.md](README.md).

**Next actions, in order:**

1. Start **M3** — the app surface. Both layers underneath it are settled:
   `cabas_domain` for the rules, `cabas_store::Document` for persistence and
   the sync bytes. M3's job is the command set and the view-models, and the
   discipline that matters is Rule 9 — the UI gets pushed view-models and
   emits intents, and computes nothing. Start from
   `crates/store/src/document.rs` to see the surface `app` wraps, and note
   that a command usually maps to *two* store calls (the domain rule, then
   its persisted effect) — `add_to_list` is the worked example, since Rule 3
   makes it add an entry **and** purge an overlay entry.

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
- [x] **Green CI confirmed** — the first run passed every gate. The two
      things that could only fail on a runner, and did not: the
      `cachix/install-nix-action` version pin, and the cold-cache cost of
      `nix develop`.

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

## M3 — App surface

- [ ] Command set (`add_to_list`, `toggle_cart_item`, `set_servings`, `finish_shopping`, …) — coarse-grained on purpose (Rule 9)
- [ ] A single state stream pushing view-models; no getter surface
- [ ] `wasm-bindgen` bindings (PWA) and the native entry points (Tauri, M7) behind one API shape
- [ ] TypeScript types generated from Rust, not hand-written
- [ ] Platform abstractions: `web-time`, `getrandom` web backend, storage and transport traits (Rule 8)
- [ ] Headless scenario test: build a list, derive a cart, check items, finish shopping

**Exit**: both targets build in CI; the scenario test passes on both.

## M4 — PWA, single device

The first genuinely usable artifact.

- [ ] Svelte 5 frontend, vanilla CSS with design tokens only (Rule 10)
- [ ] Screens: cart (the home screen — that is what you open in the shop), list, recipe view, recipe edit, ingredient library, settings
- [ ] `@`-mention autocomplete in the step editor, scoped to the recipe's own usages (DECISIONS 0022)
- [ ] Two collapsed sections at the bottom of the cart: "Bought" and "Already at home", which do not mean the same thing (DECISIONS 0023)
- [ ] Service worker, offline-first, IndexedDB; **no user action ever waits on the network** (Rule 6)
- [ ] UI state (current screen, scroll position) persisted so an iOS cold reload resumes where you were — the most visible flaw of an iOS PWA, and the one with the cheapest fix (DECISIONS 0003)
- [ ] Manifest, icons, `env(safe-area-inset-*)`, `visualViewport` keyboard handling — budget a day for the iOS keyboard, not an hour
- [ ] Installed and tested on the actual iPhone, in airplane mode

**Exit**: installed on the iPhone from the home screen, fully usable offline,
data survives a cold restart of the app.

## M5 — Relay and sync

- [ ] `cabas-relay`: axum, WebSocket per family, **persists the encrypted snapshot and deltas** — a pure broadcast relay never reconciles two devices that are never online together
- [ ] E2EE: XChaCha20-Poly1305, one shared family key, sealed before leaving the device (Rule 7)
- [ ] Pairing by QR code, **with the 12-word recovery phrase as a mandatory fallback** — the camera is historically brittle in an installed iOS PWA, and the phrase doubles as the key backup (DECISIONS 0021)
- [ ] Users and devices: pairing asks who you are; a device screen that states plainly that revoking means rotating the key and re-pairing everyone
- [ ] Attribution: `added_by`, `checked_by`, capped event log — declarative, never presented as access control (Rule 7)
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

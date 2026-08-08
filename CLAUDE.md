# cabas — session guide

Offline-first shopping list and recipe manager for two people and their
phones. Rust core (domain, storage, sync, relay), Svelte frontend shipping as
an installed PWA on iOS/web and as a Tauri app on Android/Linux.

**Binding rules** in [CONSTITUTION.md](CONSTITUTION.md); **plan and status**
in [ROADMAP.md](ROADMAP.md) ("Resuming work" section); **why every choice was
made** in [docs/DECISIONS.md](docs/DECISIONS.md).

**Current state**: M0, M1 and M2 complete, CI green on `main`.
`crates/domain` holds the product logic as pure functions (69 tests);
`crates/store` holds the Loro schema, the two-way mapping, snapshots,
compaction and the `Storage` trait over file + IndexedDB (33 native tests, 5
in a real browser). **M3 (the app surface) is next.** `sync`, `app` and
`relay` are still boundary docs only.

## Environment and commands

Everything goes through the nix flake (`cargo` does not exist outside the
devShell):

```sh
nix develop -c cargo nextest run --workspace
nix develop -c cargo clippy --workspace --all-targets -- -D warnings
nix develop -c cargo fmt --all
nix develop -c wasm-check                     # Rule 8: the 4 shared crates on wasm32
nix develop -c check-wasm-bindgen             # Rule 13: CLI/crate version match
nix develop .#wasm-test -c wasm-test          # IndexedDB, in headless chromium
nix develop .#android                         # Android SDK/NDK shell (M7 only)
```

`wasm-check` proves the shared crates *compile* for wasm32; `wasm-test` is
the only thing that *runs* wasm, and it is scoped to the one test target that
needs a browser (DECISIONS 0030). Chromium lives in its own shell for the
same reason the Android SDK does.

CI runs all of these **inside the flake** — deliberately, because Rule 13
makes nixpkgs authoritative for the `wasm-bindgen-cli` version, and a CI with
its own toolchain would be validating a different pair than the one that
ships. Never push anything that breaks a gate.

## Hard rules (CONSTITUTION — non-negotiable)

1. **`crates/domain` is pure**: no I/O, no async, no platform, no clock, no
   randomness, no CRDT type. — Rule 1.
2. **Loro is named only in `crates/store`** and the workspace registry. No
   Loro type in `domain`, `sync`, `app`, `relay` or the UI. — Rule 2.
3. **The cart is derived and never synced.** Only sources are persisted:
   recipes, ingredients, the list, users, devices, the event log, and an
   overlay of **explicit actions only**. — Rule 3.
4. **Quantities are exact rationals**, never floats; `f64` only at the last
   rendering step, outside `domain`. — Rule 4.
5. **No cross-dimension conversion** without the ingredient's own density or
   unit weight. Two honest lines beat one invented number. — Rule 5.
6. **No user action ever waits on the network.** Local first, render
   immediately, sync in the background. — Rule 6.
7. **The relay never sees plaintext**; all crypto lives in `crates/sync`.
   Attribution is declarative, not access control. — Rule 7.
8. **`domain`, `store`, `sync`, `app` build for wasm32 AND native**, always.
   — Rule 8.
9. **The frontend holds no business state** and no hardcoded visual value.
   — Rules 9, 10.
10. **English for everything persisted** — code, comments, docs, commits,
    branches, PRs, issues. French only in live discussion.
11. **Scope is closed** (no pantry, one list, no background sync, no push, no
    ad-hoc cart items). Reopening any of it starts with a DECISIONS entry, not
    with code. — Rule 14.

## Architecture

```
device:  ui/ (Svelte)  ←view-models / intents→  cabas-app
                                                    │
                        cabas-domain  ←────────  cabas-store
                        (pure logic)            (Loro replica)
                                                    │
                                               cabas-sync  ──sealed──┐
RPi4 (HAOS add-on):  cabas-relay — serves the PWA + brokers sync  ←──┘
                     holds no key; persists ciphertext in /data
```

| Crate | Role |
|---|---|
| `crates/domain` | Units, conversions, scaling, recipe DAG, cart derivation |
| `crates/store` | Loro schema, snapshots, `Storage` trait (file / IndexedDB) |
| `crates/sync` | E2EE, pairing, WebSocket transport |
| `crates/app` | Commands + view-models — the only surface the UI touches |
| `crates/relay` | Sync broker + PWA host, shipped as an HA add-on |

`crates/domain` is the only one with code (M1). Its modules, bottom-up — each
depends only on the ones above it:

| Module | Holds |
|---|---|
| `units` | `Dimension`, `Unit`, exact conversion factors, `convert` |
| `quantity` | `Quantity`, scaling, addition, `ceil_to_whole`, `humanized` |
| `ingredient` | `Ingredient`, `Aisle`, cross-dimension conversion, `resolve` |
| `recipe` | `Recipe`, usages, `Segment` steps, `dangling_refs` |
| `expand` | DAG flattening, cycle detection, `MAX_DEPTH` |
| `overlay` | `Explicit`, `CheckState`, `resolve` (state derivation) |
| `list` | `ShoppingList`, `ListEntry`, add-purges-overlay |
| `cart` | `derive`, unit merging, `progress`, `finish_shopping` |
| `people` | `User`, `Device` — attribution names, not access control |
| `event` | `Event`, `EventLog` — deletions and edits, capped |

The end-to-end scenario — M1's exit criterion, and the best place to see how
it all fits — is `crates/domain/tests/shopping_scenario.rs`, which uses the
public API only.

`crates/store` (M2) — read `schema.rs` first, it is the persisted layout in
one file and a compatibility surface (DECISIONS 0029):

| Module | Holds |
|---|---|
| `schema` | Container and key names, `SCHEMA_VERSION`, the layout diagram |
| `codec` | `LoroValue` ⇄ primitives: rationals, units, aisles, timestamps |
| `mapping` | Domain struct ⇄ document, one pair per entity |
| `document` | `Document`: lifecycle, reads, writes, snapshots, sync bytes |
| `storage` | `Storage` trait; `MemoryStorage`, `FileStorage` (native), `IndexedDbStorage` (wasm) |
| `error` | `StoreError` — carries strings, never a `LoroError` (Rule 2) |

Key domain shapes, all settled in DECISIONS:

- A recipe has `servings` **and an optional `yield`** — without a yield a
  sub-recipe cannot be scaled (0017). Sub-recipes form a **DAG**: expansion
  detects cycles and bounds depth.
- Instruction steps are **segments**, `Text` or `Ingredient { usage, display }`,
  referencing a *usage* (a specific ingredient line) so the rendered quantity
  is the scaled one (0022).
- Cart state = explicit overlay entry, else a derived default: `AutoChecked`
  for a staple sourced only from recipes, `ToBuy` otherwise (0019, 0023).
- A list entry disappears once **all** its ingredient contributions are
  checked; purge is deferred to "finish shopping" so undo survives (0020),
  and that purge is *selective* — an ingredient shared with an unfinished
  entry keeps its check (0028).
- When coefficients allow a choice, merging prefers **count over mass over
  volume** (`Dimension::MERGE_PREFERENCE`): "5 tomatoes" is what you can act
  on in a shop, "680 g of tomatoes" is not.

## Conventions

- Conventional Commits (`feat:`, `fix:`, `chore:`, …), in English — Rule 15.
  Version is semver of the shipped artifact; a release is an annotated
  `vX.Y.Z` tag whose message **is** the changelog.
- Doc-comments explain the *why* and cite the rule or decision ("Rule 3",
  "DECISIONS 0019") — the reasoning is the part that rots.
- `docs/DECISIONS.md` is **append-only**: a reversed choice gets a new
  superseding entry; the old one stays, marked, with its reasoning intact.
- Operational knowledge goes into ROADMAP/README/CONSTITUTION/docs, never
  into chat messages.

## Known pitfalls

- **A new file is invisible to Nix until `git add`ed.** Flakes only see
  git-tracked files, and the error ("Path 'x' … is not tracked by Git")
  appears at the first `nix develop`, not at file creation.
- **`cargo` outside `nix develop` → `command not found`.** By design.
- **`wasm-bindgen` crate and CLI must match to the patch.** nixpkgs decides,
  Cargo follows — currently pinned `=0.2.121`. A mismatch is a blank page
  with no useful error; `check-wasm-bindgen` is what catches it.
- **`std::time::Instant` panics on wasm32** → use `web-time`. And `getrandom`
  needs its web backend enabled, or key generation fails to link.
- **`relay` is server-side and must stay out of `wasm-check`** — it will not
  build for wasm32 once axum lands, and a check people learn to ignore is
  worse than no check.
- **The PWA's origin is its identity on iOS.** Changing the domain makes iOS
  treat it as a different app: icon gone, IndexedDB dropped. That is why the
  relay lives behind a domain we own (0012).
- **An explicit `Unchecked` must be persisted**, or the next derivation
  silently re-checks a staple the user just unchecked. Symmetrically, adding
  an ingredient to the list **purges** its overlay entry so it becomes
  visible again (Rule 3).
- **The `.#android` shell pins are unvalidated** until M7 opens; nothing
  depends on them before then.
- **`Ratio::new_raw` does not reduce the fraction, and `Ratio`'s equality
  compares numerator and denominator directly.** An unreduced constant
  silently fails to equal its own reduced form. Always `Ratio::new` — which
  is why the factor helper in `units.rs` is not a `const fn`.
- **Quantities are `Ratio<i128>`, not `i64`.** The exact imperial factors
  (1 oz = 28.349523125 g) overflow 64 bits when multiplied during conversion.
- **A method named `from_*` must not take `self`** — clippy's
  `wrong_self_convention` rejects it, and `-D warnings` makes that an error.
- `scale ∘ aggregate == aggregate ∘ scale` holds for mass and volume but
  **not for counts**: rounding a countable line up is not linear. The
  property test is restricted to mass on purpose.
- Recent crate versions may have moved since training data: check
  `~/.cargo/registry/src/` or the docs rather than assuming an API.
- **`LoroMap::get_or_create_container` is the wrong constructor** (and
  deprecated): it gives the child an operation-derived id, so two devices
  creating the same entity offline get two containers under one key and one
  side's fields vanish on merge. Always `ensure_mergeable_*`.
- **A Loro map returns its keys in hash order**, which differs between
  replicas. Every keyed read in `store` sorts by id — drop that and two
  devices show the same library in different orders.
- **`LoroValue` has no exact numeric type**, only `I64` and `Double`. Every
  rational is encoded as a `"numer/denom"` string; the guard that keeps it
  that way is `no_float_ever_reaches_the_document` in `document.rs`.
- **`js-sys`, `web-sys` and `wasm-bindgen-test` each pin an exact
  `wasm-bindgen`**, so bumping any of them can drag the lockfile off the
  flake's CLI version even when nothing else changed. That is why
  `check-wasm-bindgen` checks `Cargo.lock` and not just `Cargo.toml`; the fix
  is `cargo update -p js-sys --precise <version matching the CLI>`. The set
  that currently agrees with CLI 0.2.121: `js-sys` and `web-sys` 0.3.98,
  `wasm-bindgen-futures` 0.4.71, `wasm-bindgen-test` 0.3.71.
- **`#[test]` does not run on wasm32** — browser cases need
  `#[wasm_bindgen_test]`. That is why `wasm-test` targets only
  `--test indexeddb`, and why `tests/persistence.rs` and
  `tests/document_size.rs` are `cfg`-gated to native.

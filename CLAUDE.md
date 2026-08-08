# cabas — session guide

Offline-first shopping list and recipe manager for two people and their
phones. Rust core (domain, storage, sync, relay), Svelte frontend shipping as
an installed PWA on iOS/web and as a Tauri app on Android/Linux.

**Binding rules** in [CONSTITUTION.md](CONSTITUTION.md); **plan and status**
in [ROADMAP.md](ROADMAP.md) ("Resuming work" section); **why every choice was
made** in [docs/DECISIONS.md](docs/DECISIONS.md).

**Current state**: M0 (scaffolding). No product code yet — the crates carry
their boundary docs and nothing else. M1 (domain) is next, starting with
units and quantities, on which everything else depends.

## Environment and commands

Everything goes through the nix flake (`cargo` does not exist outside the
devShell):

```sh
nix develop -c cargo nextest run --workspace
nix develop -c cargo clippy --workspace --all-targets -- -D warnings
nix develop -c cargo fmt --all
nix develop -c wasm-check                     # Rule 8: the 4 shared crates on wasm32
nix develop -c check-wasm-bindgen             # Rule 13: CLI/crate version match
nix develop .#android                         # Android SDK/NDK shell (M7 only)
```

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
  checked; purge is deferred to "finish shopping" so undo survives (0020).

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
- Recent crate versions may have moved since training data: check
  `~/.cargo/registry/src/` or the docs rather than assuming an API.

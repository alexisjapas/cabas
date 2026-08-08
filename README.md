# cabas

Offline-first shopping list and recipe manager for two people and their
phones. Recipes compose from ingredients and sub-recipes, scale by servings,
and aggregate into a single cart sorted by supermarket aisle. Everything is
edited locally and converges through a self-hosted, **zero-knowledge** relay
— no cloud service, no account, no subscription.

Rust core (domain, storage, sync, relay) behind a Svelte frontend that ships
as an installed PWA on iOS and the web, and as a Tauri app on Android and
Linux.

- **Rules of work (binding)**: [CONSTITUTION.md](CONSTITUTION.md)
- **Plan and status**: [ROADMAP.md](ROADMAP.md)
- **Why every choice was made**: [docs/DECISIONS.md](docs/DECISIONS.md)

**Status**: M2 complete, nothing on screen yet. `crates/domain` holds the
whole product logic as pure, tested functions — units and exact conversions,
recipe scaling, the sub-recipe DAG, cart aggregation and check-state
derivation. `crates/store` persists it: a Loro document, snapshots with
history compaction, and a `Storage` trait over a file (native) and IndexedDB
(the PWA). Two replicas that edited while apart converge, including through a
relay they never used at the same time. 109 tests green. M3 (the app surface)
is next; resuming work starts at the "Resuming work" section of the
[ROADMAP](ROADMAP.md).

A family library of 200 recipes is a **154 kB** snapshot that loads in
**0.4 ms** — which is what makes a plain serialized blob the right shape
([DECISIONS 0008](docs/DECISIONS.md#0008--serialized-snapshots-not-sqlite)).

## Getting started

The development environment is managed by a nix flake — `cargo` does not
exist outside it:

```sh
nix develop            # or `direnv allow` if you use direnv
cargo nextest run --workspace
wasm-check                                # Rule 8 — must always pass
cargo clippy --workspace --all-targets -- -D warnings
check-wasm-bindgen                        # CLI/crate version match (Rule 13)
```

Two things need more than the everyday shell, so they get their own — a
browser and an Android SDK are both large downloads that most work never
touches ([DECISIONS 0013](docs/DECISIONS.md#0013--nix-flake-with-a-separate-android-shell)):

```sh
nix develop .#wasm-test -c wasm-test      # IndexedDB, in headless chromium
nix develop .#android                     # SDK/NDK — M7 only
```

`wasm-test` is the only thing that *runs* wasm; `wasm-check` proves the
shared crates compile for it, which is a weaker and much faster claim.

Nothing is needed for iOS: it ships as a PWA, which is also what makes the
project buildable without a Mac ([DECISIONS 0003](docs/DECISIONS.md#0003--ios-ships-as-a-pwa)).

## Architecture

```
        ┌───────────────────────────── device ─────────────────────────────┐
        │  ui/ (Svelte)   ← view-models / intents →   cabas-app            │
        │                                                  │               │
        │                          cabas-domain  ←─────  cabas-store       │
        │                        (pure logic)          (Loro replica)      │
        │                                                  │               │
        │                                             cabas-sync           │
        └──────────────────────────────────────────────────│───────────────┘
                                         sealed payloads    │
        ┌──────────────────────────── RPi4 (HAOS) ──────────│───────────────┐
        │  cabas-relay — serves the PWA + brokers sync, holds no key        │
        └───────────────────────────────────────────────────────────────────┘
```

| Crate | Role | Boundary |
|---|---|---|
| `crates/domain` | Units, conversions, scaling, recipe DAG, cart derivation | pure — no I/O, no async, no CRDT |
| `crates/store` | Loro schema, snapshots, `Storage` trait | the only crate that names Loro |
| `crates/sync` | E2EE, pairing, WebSocket transport | the only crate with cryptography |
| `crates/app` | Commands and view-models | must build for wasm32 **and** native |
| `crates/relay` | Sync broker + PWA host (HA add-on) | never sees plaintext |
| `ui/` | Svelte frontend | holds no business state |

Four ideas carry the design, each with its rationale recorded:

- **Local-first.** Every action applies to the local replica and renders
  immediately; no user action ever waits on the network. Supermarkets are
  Faraday cages, and two people edit at once from different places.
- **The cart is derived, never stored.** Only sources are persisted and
  synced — recipes, ingredients, the list, and an overlay of *explicit*
  check actions. Everything else is a pure function
  ([0019](docs/DECISIONS.md#0019--the-cart-is-derived-the-overlay-stores-only-explicit-actions)).
- **The relay cannot read anything.** Payloads are sealed on the device
  under one shared family key, which makes where it runs a matter of
  convenience rather than trust
  ([0009](docs/DECISIONS.md#0009--zero-knowledge-relay-with-app-layer-e2ee)).
- **Exact quantities.** Rationals end to end, and no cross-dimension
  conversion without the ingredient's own density or unit weight — two
  honest lines beat one invented number
  ([0014](docs/DECISIONS.md#0014--quantities-are-exact-rationals),
  [0015](docs/DECISIONS.md#0015--no-cross-dimension-conversion-without-an-explicit-coefficient)).

## Platforms

| Target | Ships as | Priority |
|---|---|---|
| iOS | installed PWA (Add to Home Screen) | primary |
| Android | Tauri v2 APK | primary |
| Web | the same PWA | secondary |
| Linux | Tauri desktop | secondary |

Windows and macOS are out of scope
([0002](docs/DECISIONS.md#0002--target-platforms)).

## Contributing

Read [CONSTITUTION.md](CONSTITUTION.md) first — the rules there are binding,
and most of them exist because of a specific trade-off recorded in
[docs/DECISIONS.md](docs/DECISIONS.md). Everything written for the project is
in English; French is fine in live discussion, never in anything persisted.

Changing what the project does — reopening the pantry, adding a second list,
enabling background sync — starts with a new DECISIONS entry, not with code
(Rule 14).

## License

MIT OR Apache-2.0 (provisional —
[0027](docs/DECISIONS.md#0027--license-mit-or-apache-20)).

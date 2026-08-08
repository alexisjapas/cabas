# cabas — session guide

Offline-first shopping list and recipe manager for two people and their
phones. Rust core (domain, storage, sync, relay), Svelte frontend shipping as
an installed PWA on iOS/web and as a Tauri app on Android/Linux.

**Binding rules** in [CONSTITUTION.md](CONSTITUTION.md); **plan and status**
in [ROADMAP.md](ROADMAP.md) ("Resuming work" section); **why every choice was
made** in [docs/DECISIONS.md](docs/DECISIONS.md).

**Current state**: M0 through M3 complete; **M4 (the PWA) is in progress**.
`crates/domain` holds the product logic as pure functions (69 tests);
`crates/store` holds the Loro schema, the two-way mapping, snapshots,
compaction and the `Storage` trait over file + IndexedDB; `crates/app` holds
the command set, the view-models and the wasm binding. 123 native tests plus
4 in a real browser.

`ui/` is a working Svelte 5 app: identity, the cart, the list, the recipes
(list, reader and editor), the ingredient library and settings, driven end to
end by `ui-test` in headless chromium. **Every command is reachable from the
UI**, the app is **installable**, and **it opens with the network off** — the
service worker, the manifest and the icons are done. What M4 still needs is the
scroll position and the iPhone itself, keyboard included — see ROADMAP "Next
actions". `sync` and `relay` are still boundary docs only.

## Environment and commands

Everything goes through the nix flake (`cargo` does not exist outside the
devShell):

```sh
nix develop -c cargo nextest run --workspace
nix develop -c cargo clippy --workspace --all-targets -- -D warnings
nix develop -c cargo fmt --all
nix develop -c wasm-check                     # Rule 8: the 4 shared crates on wasm32
nix develop -c check-wasm-bindgen             # Rule 13: CLI/crate version match
nix develop .#wasm-test -c wasm-test          # store + app, in headless chromium
nix develop .#android                         # Android SDK/NDK shell (M7 only)

nix develop -c cargo test -p cabas-app --features typescript export_bindings
```

The PWA (M4):

```sh
nix develop -c build-wasm [--dev]             # the core → ui/src/lib/wasm/
nix develop -c pnpm -C ui install             # once
nix develop -c pnpm -C ui check               # svelte-check + the worker's own tsc
nix develop -c pnpm -C ui dev                 # dev server, bound to the LAN
nix develop -c pnpm -C ui build               # ui/dist
nix develop .#wasm-test -c ui-test            # the whole vertical, in a browser

nix develop .#wasm-test -c node ui/tools/render-icons.mjs   # the PNG icons
```

`pnpm check` runs **two** TypeScript programs: `svelte-check` over the app, and
`tsc -p tsconfig.sw.json` over `src/sw.js` alone. A service worker's globals
come from `lib.webworker`, which declares the same names as `lib.dom` with
different types, so the two cannot share a program — the one file that needs
the worker library gets its own config rather than the app losing the DOM.

The icon renderer is not part of any loop: the PNGs are committed, because iOS
reads `apple-touch-icon` as a bitmap. Run it when the drawing changes.

**`build-wasm` first, always.** `ui/src/lib/wasm/` is a build product and is
gitignored, so a fresh checkout has no glue for `core.ts` to import and
`pnpm check` fails with a missing module rather than a missing step. `--dev`
is seconds instead of a minute and produces a module ten times the size;
never ship it. `ui-test` wants a built `ui/dist` and says so if it is absent.

`wasm-check` proves the shared crates *compile* for wasm32; `wasm-test` is
the only thing that *runs* wasm, and it is scoped to the two test targets
that need a browser — `store`'s IndexedDB and `app`'s scenario (DECISIONS
0030). Chromium lives in its own shell for the same reason the Android SDK
does.

The last command regenerates `ui/src/lib/bindings/*.ts` from the Rust types.
It is not part of the everyday loop, but **CI fails if its output is stale**,
so run it after touching anything in `command.rs`, `view.rs` or `tags.rs`.

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

`domain`, `store` and `app` hold code; `sync` and `relay` are boundary docs.
`crates/domain`'s modules, bottom-up — each depends only on the ones above it:

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

`crates/app` (M3) — read `view.rs` first, it is the screen list; then
`tests/scenario.rs`, which is the whole vertical through the public surface:

| Module | Holds |
|---|---|
| `command` | `Command` and the input types — intents, coarse-grained (Rule 9) |
| `view` | `StateView` and everything under it — pushed whole, every time |
| `app` | `App`: `open`, `apply` (sync), `persist` (async), the sync seam |
| `project` | Library → views; triages the list so a derivation cannot fail |
| `library` | The whole document, read into plain domain values |
| `number` | Text ⇄ exact rational, and the two renderings (pretty, lossless) |
| `tags` | The enum spellings the frontend sees — its own contract, not the schema's |
| `platform` | `Platform` (clock + randomness), `SystemPlatform`, `Identity` |
| `wasm` | `CabasApp` — the PWA binding, and nothing but translation |

The shape to keep in mind: **`apply` is synchronous and returns the whole new
state; `persist` writes.** A render never waits on storage, and the borrow is
released before anything is awaited — which is what keeps a second tap from
panicking at the wasm boundary (DECISIONS 0032).

`ui/` (M4) — plain Vite + Svelte 5, no SvelteKit (DECISIONS 0037). Read
`lib/session.svelte.ts` first, it is the state and the save policy in one
file:

| Path | Holds |
|---|---|
| `lib/bindings/` | Generated from Rust, committed, diffed by CI (0036) |
| `lib/wasm/` | Generated by `build-wasm`, gitignored |
| `lib/core.ts` | The typed edge — the only place a cast meets the wasm `any`, plus the identity in `localStorage` (0031) |
| `lib/session.svelte.ts` | The one `$state.raw`, `run(command)`, the debounced flush, the persisted screen |
| `lib/labels.ts` | The French for every tag the core sends, and nothing else (0035) |
| `lib/format.ts` | Rendered number meets word: decimal comma, "≈", plurals, relative time, French name order |
| `app.css` | The tokens. No component writes a literal value (Rule 10) |
| `screens/`, `components/` | The screens, and what more than one of them needs |
| `sw.js` | The service worker: precache, one versioned cache, cache-first (0038) |
| `vite.config.ts` | The build, and the plugin that writes the precache list into the worker |
| `public/` | Served verbatim: the manifest, the favicon, the icons |
| `tools/render-icons.mjs` | SVG → the committed PNGs, over CDP. Not part of any loop |
| `tests/smoke.mjs` | The vertical in a browser, over CDP, zero dependencies |

`screens/Recipes.svelte` is three views behind one tab — the shelf, the one
being read, and the one being written — and the shape is worth knowing before
touching it. Which recipe is *open* is core state (`OpenRecipe`, never
synced); the recipe being *edited* is a draft that lives in `Recipes.svelte`
and is handed to `RecipeEditor` as a `$bindable`, because a form seeded from a
prop captures the value once and ignores the next one. The draft **is** a
`RecipeInput`, so saving is one command and no mapping. `RecipeReader` scales
nothing: `focus.recipe` arrives already rendered at the current servings.

Three shapes worth knowing before editing it. The state is **`$state.raw`**,
because every command returns a whole new tree and a deep proxy would track
mutations that never happen. `run()` returns a boolean, so a form can stay
open when a command is refused. And the flush is debounced *and* hooked to
`visibilitychange`/`pagehide`, because a pending timer dies with the page and
iOS backgrounds a PWA whenever it likes.

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
  `#[wasm_bindgen_test]`. That is why `wasm-test` names its test targets one
  by one, and why `tests/persistence.rs` and `tests/document_size.rs` are
  `cfg`-gated to native. `crates/app/tests/scenario.rs` is the pattern for a
  file that runs on both: one `async fn` body, two thin wrappers.
- **`getrandom` on wasm32 needs two opt-ins, not one** — the `wasm_js`
  feature *and* `--cfg getrandom_backend="wasm_js"` (in `.cargo/config.toml`).
  Either alone is a `compile_error!`.
- **`serde-wasm-bindgen` serialises `None` as `undefined`**, while the
  generated TypeScript says `| null`. `wasm::to_js` configures
  `serialize_missing_as_null`; use it rather than `to_value`, or the UI ends
  up testing for a value that never arrives.
- **Never hold a `RefCell` borrow of the app across an `await`.** An exported
  async method keeps its borrow for as long as its promise is pending, so the
  second tap panics. `CabasApp::flush` shows the shape: take the snapshot in
  a statement that ends, *then* await the write.
- **An edit form must render with `number::render_lossless`, not `render`.**
  `render` rounds when a value has no tidy form, and a form that displays a
  rounded amount writes it back on the next save.
- **The `.ts` files under `ui/src/lib/bindings/` are generated and CI
  checks them.** Touching `command.rs`, `view.rs` or `tags.rs` means
  rerunning the export command above.
- **`wasm-opt` rejects the features rustc emits by default.** Bulk memory and
  non-trapping float-to-int are on for `wasm32-unknown-unknown`, and the
  target-features section does not survive `wasm-bindgen`, so every one has to
  be named with `--enable-*`. The failure is a wall of validator output about
  `memory.fill`, which reads like a miscompile and is a missing flag.
  `build-wasm` carries the list.
- **Svelte's `state_referenced_locally` is an error here**, because
  `pnpm check` runs with `--fail-on-warnings`. Seeding a `$state` from
  `session.state.…` trips it, and the warning is right: the field would ignore
  a change arriving from another device. `Settings.svelte` has the shape that
  works — a `$state<string | null>` draft, a `$derived` that falls back to the
  view, and `oninput` instead of `bind:value`.
- **`verbatimModuleSyntax` is on**, so an import that only carries a type must
  say `import type`. Svelte compiles each block in isolation and cannot work
  it out by looking across files.
- **`ui/pnpm-workspace.yaml` is committed on purpose.** pnpm writes it when
  the machine has a global minimum-release-age policy and a pinned dependency
  is newer than it; deleting the file just makes the next install recreate it.
- **`ui-test` refuses to start if 4173 or 9222 is already bound**, and that is
  the point: attaching to a browser some earlier run left behind means
  reporting on a page this run never opened. If it says so, `pkill -f
  'remote-debugging-port=9222'` and check `ss -lptn 'sport = :4173'`. The
  script itself no longer leaks — `exec`ing the harness would have replaced
  the shell and skipped its own cleanup trap.
- **A step can only reference an *ingredient* line.** `Recipe::usage` searches
  `Component::Ingredient` and nothing else, so a segment naming a sub-recipe
  usage renders as `Missing` — not an error, just permanently a warning. The
  `@` picker in `RecipeEditor` filters to ingredient components for that
  reason, and widening it would need a domain change first.
- **A recipe line is named when it is added, not when it is saved**
  (DECISIONS 0039). `mintUsageId()` in `core.ts` is why the editor can mention
  a line the document has never seen. Deleting a line therefore has to strip
  its mentions by hand — a dangling reference is the right rendering for what
  *another device* did, and the wrong thing to manufacture locally.
- **Send `$state.snapshot(draft)` across the wasm boundary, not the proxy.**
  `serde_wasm_bindgen` reads a Svelte proxy correctly today; a plain value is
  what the boundary is specified to take, and it costs one call.
- **A `use:` action that takes a parameter must declare it.** `use:grow={text}`
  against `function grow(node)` is `Expected 1 arguments, but got 2` from
  `svelte-check`, and `noUnusedParameters` then wants the unused one prefixed
  with `_`. The parameter is how an action re-runs when a value changes from
  script rather than from typing.
- **A UI test that sets a `<select>` needs the native setter and a dispatched
  event** — assigning `.value` moves the pixel and tells Svelte nothing. Note
  also that `form select:nth-of-type(1)` matches *every* first-select-child in
  the form, not the first select in it; `querySelectorAll(...)[n]` is what you
  meant. Both traps are already handled in `ui/tests/smoke.mjs`.
- **`Vary` makes the precache miss its own entries.** A server answering
  `Vary: Origin` (Vite's preview does) makes the Cache API match on the
  request's `Origin` header too. The worker precaches with requests that carry
  none; the page then asks for its JS and CSS *with* one, because Vite marks
  both tags `crossorigin`. Every asset cached, every lookup a miss — and
  online it is invisible, because the miss falls through to a network that
  answers. `sw.js` reads through `lookup()`, which passes `ignoreVary: true`.
- **The precache list is injected by replacing a token in the built worker**,
  and rolldown's minifier rewrites string literals to backticks. The pattern in
  `vite.config.ts` accepts all three quotes and **the build fails if it matches
  nothing** — shipping a worker whose cache is named after the placeholder is
  the failure that has to stay impossible.
- **Vite emits `index.html` from a plugin of its own**, so a `generateBundle`
  hook that wants it must declare `order: 'post'`. Without that the bundle has
  the JS in it and no page.
- **The service worker must have no imports and no exports.** It is registered
  as a classic script, because module workers are too recent to rely on across
  iOS versions; the ES output only stays valid as a classic script while the
  file is self-contained. `vite.config.ts` fails the build if it stops being.
- **CDP's offline emulation is per-target and per-document.** A service worker
  is its own target, so a page put offline still has a worker behind it that
  reaches the network on a cache miss; and the emulation does not survive a
  navigation. `smoke.mjs` attaches to the worker target and re-applies after
  every load — before it did, the offline test passed against a live server.
- **`--window-size` sizes the window, not the viewport**, so a headless
  screenshot comes out a browser frame short. `render-icons.mjs` sets the
  viewport with `Emulation.setDeviceMetricsOverride` instead. An inline `<svg>`
  also needs `display:block`, or its text baseline overflows the viewport and
  puts a scrollbar in the icon.

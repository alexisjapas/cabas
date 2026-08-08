# Constitution — the rules of cabas

How we build **cabas**. These rules are stable and binding; the
[ROADMAP](ROADMAP.md) holds the evolving plan and status, the
[README](README.md) holds setup and commands, and
[docs/DECISIONS.md](docs/DECISIONS.md) holds the historical record of every
technical and product choice, with the reasoning that produced it. This
document says *how* the code must behave; DECISIONS says *why* it was
decided that way.

Working language of **everything written for the project** is **English** —
code, comments, docs, commit messages, tag and release notes, branch names,
PR titles and issues: every artifact that lands in the repo or its
infrastructure. French is fine in live discussion, never in anything
persisted. Cite a rule by number ("cf. Rule 3") and a decision by its
identifier ("DECISIONS 0019").

---

## Rule 1 — The domain is pure (the cardinal invariant)

`crates/domain` has **no I/O, no async, no platform, no clock, no
randomness, no CRDT type**. Pure functions over owned types: unit
conversion, scaling, expansion of the recipe DAG, cart aggregation,
derivation of check states.

**Why.** This is where the entire real difficulty of the product lives, and
it is the only part that must be right before anything is visible. Kept
pure, it is exercised by `cargo test` with no browser, no relay, no phone —
so the hard logic gets tested early and often, instead of behind four
platform builds.

**Anchored in.** `crates/domain/Cargo.toml` (empty `[dependencies]` beyond
the workspace registry entries it needs); `crates/domain/src/lib.rs`
(crate doc); ROADMAP M1.

---

## Rule 2 — The CRDT stops at `store`

`loro` is named in exactly two places: the `[workspace.dependencies]`
registry and `crates/store`. No Loro type appears in `domain`, `sync`,
`app`, `relay` or the frontend; `store` translates in both directions
between plain Rust structs and the replicated representation.

**Why.** The CRDT is the youngest and least replaceable-looking dependency
in the stack, and the one most likely to churn. Containment is what turns
"we must migrate to Automerge" into a one-crate rewrite that leaves the
domain logic and the UI untouched.

**Anchored in.** `crates/store/src/lib.rs` (crate doc); `Cargo.toml`
(`[workspace.dependencies].loro` comment); DECISIONS 0006, 0007.

---

## Rule 3 — The cart is derived; only explicit actions are stored

Persisted and synced: recipes, ingredients, the shopping list, users,
devices, the event log, and an overlay holding **only explicit user
actions** (`Checked { by, at }` / `Unchecked`). The cart itself is a pure
function of those and is never written down. A line with no overlay entry
falls back to its derived default — `AutoChecked` for a staple sourced only
from recipes, `ToBuy` otherwise.

Two corollaries that are easy to get wrong:

- An explicit **`Unchecked`** must be persisted, or the next derivation
  silently re-checks a staple the user just unchecked.
- Adding an ingredient to the list **purges its overlay entry**, so it
  returns to its derived default and becomes visible again.

**Why.** Syncing a derived value is how replicas end up disagreeing with
their own inputs. Deriving it instead shrinks the CRDT surface to what
genuinely has concurrent writers, and makes every check-state question
answerable by reading one pure function.

**Anchored in.** `crates/domain` (derivation), `crates/store` (overlay
schema); DECISIONS 0019, 0020, 0023; ROADMAP M1.

---

## Rule 4 — Quantities are exact rationals, never floats

Every quantity is a rational (`num-rational`) from entry to aggregation.
`f64` appears only in the final rendering step, outside `domain`.

**Why.** Scaling is the operation the whole product rests on, and it must
be exact and reversible: a recipe scaled 4→6 then 6→4 must return the
original numbers, and ⅓ cup must stay ⅓ rather than drift into
0.3333333333333333.

**Anchored in.** `Cargo.toml` (`[workspace.dependencies].num-rational`);
`crates/domain`; DECISIONS 0014.

---

## Rule 5 — No conversion without an explicit coefficient

Quantities aggregate **within a dimension** (mass, volume, count). Crossing
dimensions requires the ingredient's own coefficient — density (g/ml) for
mass↔volume, unit weight (g/piece) for count↔mass. Absent the coefficient,
the amounts stay on separate lines. The engine never guesses.

Unit variants carry their locale (a French tablespoon is 15 ml, a US one
14.79; a US cup is 236.6 ml, a metric one 250), even while the UI exposes
metric only.

**Why.** A plausible-looking wrong conversion is worse than two honest
lines: it produces a shopping quantity nobody can trace back. And encoding
the locale now costs nothing, whereas retrofitting it at import time
(DECISIONS 0016) is a data migration.

**Anchored in.** `crates/domain`; DECISIONS 0015, 0016.

---

## Rule 6 — No user action ever waits on the network

Every action applies to the local replica and renders immediately. No
spinner, no disabled button, no error toast on a user action because the
relay is unreachable. Sync is a background concern whose entire UI is a
discreet "last synced" indicator.

**Why.** The app is used in supermarkets, which are Faraday cages, and by
two people editing at once from different places. Offline is the normal
operating mode, not a degraded one — anything that blocks on the network
fails exactly when the product is being used.

**Anchored in.** `crates/app` (commands apply locally, then enqueue);
`crates/sync`; DECISIONS 0009, 0011.

---

## Rule 7 — The relay is zero-knowledge; attribution is declarative

Plaintext never leaves a device. All cryptography lives in `crates/sync`.
The relay stores and forwards sealed payloads and holds no key.

Attribution (`added_by`, `checked_by`, the event log) is a **convenience,
not access control**: with one shared family key, any holder can write as
anyone. This limit is deliberate and must be stated wherever the UI implies
identity — including the device screen, where revoking a lost device means
rotating the key and re-pairing everyone, with no middle ground.

**Why.** Encrypting at the app layer is what makes hosting a matter of
convenience rather than of trust, and lets the relay move to any machine
without revisiting the threat model. And per-device signatures would not
help here: the same shared key that decrypts also lets an attacker enrol a
forged device, so they would add key distribution and revocation for no
gain among two people who trust each other.

**Anchored in.** `crates/sync/src/lib.rs` (crate doc);
`crates/relay/src/main.rs`; DECISIONS 0009, 0024.

---

## Rule 8 — `app` builds for `wasm32-unknown-unknown` and for the host, always

`domain`, `store`, `sync` and `app` compile for both targets on every
commit. Concretely: no `std::time::Instant` (use `web-time`), `getrandom`
with its web backend enabled, networking and storage behind traits with a
native and a wasm implementation.

**Why.** iOS ships as a PWA (wasm) and Android as Tauri (native) from the
*same* crates — this is not a portability nicety, it is the load-bearing
assumption of the whole platform plan (DECISIONS 0003). A regression here
is discovered as a blank page on an iPhone, which is the worst possible
place to discover it.

**Anchored in.** `rust-toolchain.toml` (`targets`); `flake.nix` (the
`wasm-check` helper, scoped to those four crates — `relay` is server-side
only); CI job `wasm` — M0.

---

## Rule 9 — The frontend holds no business state

The UI renders view-models pushed from `cabas-app` and emits intents. It
computes no quantity, resolves no unit, decides no check state. One
TypeScript API surface, two implementations behind it (`wasm-bindgen` in
the PWA, Tauri `invoke` on Android and Linux), with the types generated
from Rust rather than hand-written.

**Why.** Two transports can only stay interchangeable if neither carries
logic. It also keeps the hard rules of this document enforceable in
`cargo test`, instead of duplicated into JavaScript where nothing checks
them.

**Anchored in.** `crates/app/src/lib.rs` (crate doc); `ui/`; DECISIONS
0004, 0005; ROADMAP M3, M4.

---

## Rule 10 — No hardcoded visual value

Colors, spacing, radii, typography and easings live in CSS custom
properties declared once at the root. A component never writes a literal
colour or pixel size. Dark mode follows from the tokens plus
`prefers-color-scheme`.

**Why.** The visual identity is deliberately deferred (DECISIONS 0026):
the current look is vanilla and temporary by decision, not by neglect. That
is only affordable if restyling later is a token change rather than a sweep
through every component.

**Anchored in.** `ui/` (token stylesheet); DECISIONS 0026; ROADMAP M4.

---

## Rule 11 — Every feature ships with a test; the domain gets property tests

Unit tests per module. The invariants this document states are tested where
they can be: `scale ∘ aggregate == aggregate ∘ scale`, conversions
round-trip exactly, DAG expansion terminates and rejects cycles, an
explicit `Unchecked` survives re-derivation, adding a staple to the list
makes it visible.

**Why.** What is not tested regresses. And these particular invariants are
the ones whose violation is invisible in the UI until a shopping trip goes
wrong — the cart is quietly short by 200 g and nobody notices until the
recipe fails.

**Anchored in.** `Cargo.toml` (`[workspace.dependencies].proptest`);
crate tests (`cargo nextest run --workspace`); ROADMAP M1.

---

## Rule 12 — `cargo fmt` is authoritative; the tree stays clippy-clean

No `rustfmt.toml` — the default formatter decides. Every commit leaves
`cargo fmt --all --check` clean and
`cargo clippy --workspace --all-targets -- -D warnings` warning-free, on
both the host and the wasm target.

**Why.** Formatting and lint debates are pure friction; delegating them to
the tools keeps reviews about substance.

**Anchored in.** CI jobs `fmt`, `clippy` — M0.

---

## Rule 13 — One dependency registry; `wasm-bindgen` versions must match

Versions live only in `[workspace.dependencies]`; a crate pins nothing
locally. Every added dependency is justified in its registry comment.

One pin is special: the `wasm-bindgen` crate and the `wasm-bindgen-cli`
provided by the flake must be the **exact same version**, or the generated
glue fails at load time with an opaque error. nixpkgs is authoritative and
Cargo follows it, never the reverse.

**Why.** A single registry is what makes a dependency audit possible at a
glance. And the wasm-bindgen mismatch is the classic footgun of this stack:
it produces a blank page with no useful message, so it is worth a CI check
rather than an afternoon.

**Anchored in.** `Cargo.toml` (`[workspace.dependencies]`); `flake.nix`
(`check-wasm-bindgen`); CI job `wasm` — M0.

---

## Rule 14 — Scope is closed; decisions are recorded, never rewritten

Deliberately out of scope: pantry/stock tracking, multiple lists,
background sync, push notifications, ad-hoc items added at the shop,
Windows and macOS builds. Reopening any of them starts with a new entry in
[docs/DECISIONS.md](docs/DECISIONS.md), not with code.

That file is append-only. A choice that is reversed gets a **new** entry
that supersedes the old one; the superseded entry stays, marked, with its
original reasoning intact.

**Why.** This design came out of one long discussion, and the reasoning is
the part that rots silently — six months on, the *why* behind "the cart is
derived" is worth more than the sentence itself. Rewriting history to look
clean destroys exactly the information that prevents the same mistake
twice.

**Anchored in.** [docs/DECISIONS.md](docs/DECISIONS.md); DECISIONS 0025.

---

## Rule 15 — Commit hygiene, and version as semver of the shipped artifact

Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`, `test:`, `ci:`,
`refactor:`, `perf:`). Never commit a tree that fails Rules 8, 11 or 12.

`[workspace.package].version` is the semver of the shipped artifact — the
relay image and the PWA bundle it serves — not a commit counter: `fix:` →
patch, `feat:` → minor, a breaking change to the sync protocol or the
persisted schema → major, and tooling-only changes bump nothing. A release
is an annotated `vX.Y.Z` tag whose message **is** the changelog. The
`-dev` suffix marks a line that never shipped and drops at the first
release.

**Why.** Sync protocol and schema changes are the breaking changes that
actually hurt here: a phone that has been in a pocket for three weeks must
still converge with the relay. Keeping the version keyed to the artifact's
observable behaviour is what makes "can this old client still talk to that
relay?" answerable.

**Anchored in.** `Cargo.toml` (`[workspace.package].version`); CI release
workflow — M6.

# Decisions — historical record

Every technical and product choice, with the reasoning that produced it.
This file is **append-only** (CONSTITUTION Rule 14): a reversed choice gets a
new entry that supersedes the old one, and the superseded entry stays in
place, marked, with its original reasoning intact.

Entries 0001–0027 come from the design discussion of **2026-08-08**, held
before any code was written. Status is `Accepted` unless stated otherwise.

| # | Decision | Area |
|---|---|---|
| [0001](#0001--rust-for-the-core-the-frontend-language-is-free) | Rust for the core; the frontend language is free | Stack |
| [0002](#0002--target-platforms) | Target platforms | Platform |
| [0003](#0003--ios-ships-as-a-pwa) | iOS ships as a PWA | Platform |
| [0004](#0004--svelte-5-as-the-frontend-framework) | Svelte 5 as the frontend framework | Stack |
| [0005](#0005--tauri-v2-as-a-packaging-layer-after-the-pwa) | Tauri v2 as a packaging layer, after the PWA | Stack |
| [0006](#0006--loro-as-the-crdt) | Loro as the CRDT | Sync |
| [0007](#0007--the-crdt-is-confined-to-store) | The CRDT is confined to `store` | Architecture |
| [0008](#0008--serialized-snapshots-not-sqlite) | Serialized snapshots, not SQLite | Storage |
| [0009](#0009--zero-knowledge-relay-with-app-layer-e2ee) | Zero-knowledge relay with app-layer E2EE | Sync |
| [0010](#0010--the-relay-ships-as-a-home-assistant-os-add-on) | The relay ships as a Home Assistant OS add-on | Deployment |
| [0011](#0011--no-background-sync-no-push-in-v1) | No background sync, no push in v1 | Product |
| [0012](#0012--cloudflare-tunnel-on-an-owned-domain) | Cloudflare Tunnel on an owned domain | Deployment |
| [0013](#0013--nix-flake-with-a-separate-android-shell) | Nix flake with a separate Android shell | Tooling |
| [0014](#0014--quantities-are-exact-rationals) | Quantities are exact rationals | Domain |
| [0015](#0015--no-cross-dimension-conversion-without-an-explicit-coefficient) | No cross-dimension conversion without an explicit coefficient | Domain |
| [0016](#0016--unit-locale-variants-and-count-rounds-up-in-the-cart) | Unit locale variants; count rounds up in the cart | Domain |
| [0017](#0017--recipes-carry-servings-and-an-optional-yield) | Recipes carry servings and an optional yield | Domain |
| [0018](#0018--scope-cuts-no-pantry-a-single-list-no-ad-hoc-cart-items) | Scope cuts: no pantry, a single list, no ad-hoc cart items | Product |
| [0019](#0019--the-cart-is-derived-the-overlay-stores-only-explicit-actions) | The cart is derived; the overlay stores only explicit actions | Architecture |
| [0020](#0020--list-entries-vanish-on-completion-purge-is-deferred) | List entries vanish on completion; purge is deferred | Product |
| [0021](#0021--pairing-by-qr-with-a-mandatory-12-word-fallback) | Pairing by QR with a mandatory 12-word fallback | Sync |
| [0022](#0022--instruction-steps-are-segments-referencing-ingredient-usages) | Instruction steps are segments referencing ingredient usages | Domain |
| [0023](#0023--the-staple-flag-and-its-derived-auto-check) | The staple flag and its derived auto-check | Product |
| [0024](#0024--attribution-is-declarative-not-cryptographic) | Attribution is declarative, not cryptographic | Security |
| [0025](#0025--this-log-is-append-only) | This log is append-only | Process |
| [0026](#0026--the-visual-identity-is-deliberately-deferred) | The visual identity is deliberately deferred | Product |
| [0027](#0027--license-mit-or-apache-20) | License: MIT OR Apache-2.0 | Legal |
| [0028](#0028--finishing-a-trip-prunes-the-overlay-selectively) | Finishing a trip prunes the overlay selectively | Product |

---

## 0001 — Rust for the core; the frontend language is free

**Context.** The initial constraint was "written in Rust". It was relaxed
during the discussion: the logic and backend must be Rust, but a frontend
framework in another language is acceptable if it is more efficient,
stable, fluid and good-looking.

**Decision.** Rust for domain, storage, sync and the relay. The frontend is
chosen on its own merits.

**Consequences.** Opened the door to Flutter and Compose Multiplatform,
which 0004 then evaluated on the merits. The Rust core stays the single
implementation of every rule in the constitution, whatever the UI.

---

## 0002 — Target platforms

**Context.** The initial list was Linux, Windows, web, iOS, Android. It was
then narrowed: Android and iOS are the primary targets, web and Linux are
bonuses, Windows and macOS are dropped.

**Decision.** Primary: **Android**, **iOS**. Secondary: **web**, **Linux**.
Dropped: Windows, macOS.

**Consequences.** Removed WebView2, Windows code signing and macOS
notarisation from the plan. Mobile-first shifted the framework evaluation
substantially (0004) — a shopping list is used on a phone, in a shop.

---

## 0003 — iOS ships as a PWA

**Context.** iOS is a primary target, but building, signing and installing a
native iOS app requires macOS and Xcode — no exception, whatever the
framework. There is no Mac available and none planned.

**Decision.** iOS is served as an **installed PWA** (Add to Home Screen).
The same build serves the web target.

**Consequences.** Nix never has to build for iOS. The app is close to the
ideal PWA case — lists, checkboxes, forms, offline, no background
execution, no OS integration — and an installed PWA gets the JIT, so it is
not a degraded webview. The three real costs, all accepted:

1. **Install friction**: Safari → Share → Add to Home Screen, no automatic
   prompt. One-time, per person.
2. **Cold reload**: iOS may evict the PWA from memory on app switch,
   restarting it from scratch. Mitigated by persisting UI state (screen,
   scroll) — mandated in ROADMAP M4.
3. **Keyboard handling**: `position: fixed` and viewport quirks in Safari
   require `visualViewport` work. Budgeted as a day, not an hour.

Storage: the 7-day ITP cap applies to sites browsed in Safari, and Apple
exempted home-screen web apps; eviction under disk pressure and on icon
deletion remains possible, but with server sync it costs a re-download, not
data. Residual platform risk: Apple briefly announced removing home-screen
web apps in the EU in 2024 before reversing; standard web tech means the
fallback is a Safari tab.

**Rejected.** Building iOS in CI on a macOS runner — installing on a device
still needs the 99 $/yr programme, and debugging blind through CI is not
viable. Renting a cloud Mac — paid, and the same signing requirement.

---

## 0004 — Svelte 5 as the frontend framework

**Context.** After 0002 made mobile primary, the choice narrowed to
Flutter + flutter_rust_bridge versus a web frontend in a webview. Flutter
has a measurably better mobile feel: inertial scrolling, gestures, keyboard,
page transitions.

**Decision.** **Svelte 5**, rendered in the system webview (Tauri) or as a
PWA.

**Consequences.** The deciding factor was 0003. Once iOS must be a PWA,
Flutter would render it through a canvas (CanvasKit/skwasm since the HTML
renderer was removed in 3.29): a payload of roughly a megabyte, poor text
input in mobile Safari, no real DOM and weak accessibility. A DOM PWA is
clearly better there. Symmetrically, the webview's weakness — long
virtualised lists and complex gestures — barely applies to a 30–80 line
shopping list handled by tapping.

Svelte specifically, over React or Vue: the compiler erases the framework,
giving the best profile on an old phone, and `animate:flip` is built in —
list reordering when an item is checked or the cart re-sorts by aisle is
most of the perceived fluidity, for one line of code.

**Rejected.** **Flutter + FRB** (best mobile feel, but see above; also its
web target would need the Rust core as a second wasm module, the least
travelled path in that stack). **Compose Multiplatform + uniffi** (Android
is its home turf and uniffi generates Kotlin *and* Swift, but the Rust
integration is far more manual, Gradle under Nix is painful, and Kotlin/Wasm
would make the web bonus weaker). **Dioxus**, **Slint**, **egui**
(considered while the Rust-only constraint held, superseded by 0001).
**Fully native SwiftUI + Compose** (best quality, twice the UI work, and
impossible on iOS without a Mac).

---

## 0005 — Tauri v2 as a packaging layer, after the PWA

**Context.** With the PWA mandatory for iOS, Android could either be the
same PWA or a native wrapper. "Android native" was requested.

**Decision.** Build the PWA first; **Tauri v2** then packages the byte-identical
frontend for Android (M7) and Linux (M8). The only change is that the Rust
core switches from wasm to native, and storage from IndexedDB to a file.

**Consequences.** Nothing is thrown away between the two — both swaps sit
behind traits introduced at M3 (Rule 8). Android gains a real APK to hand to
the family, durable non-evictable storage, and no install friction. The
ordering matters: the PWA validates the whole vertical while every bug still
has a single replica.

---

## 0006 — Loro as the CRDT

**Context.** The dimensioning use case is concurrent editing with no
connectivity: one person checking items in a shop while the other adds to
the list from home. Writing conflict resolution by hand would be more work
for a less correct result.

**Decision.** **Loro**. It is a pure library — no network, no account, no
telemetry, no licence key: it takes mutations and returns byte blobs, and
the transport and encryption stay ours. MIT, pure Rust, compiles to wasm.

**Consequences.** Compact snapshots (which matters on mobile), a movable
list for reordering steps and ingredients, and built-in history compaction
so the document does not grow without bound.

The real risk is not locality but sustainability: Loro is backed by a
startup, Automerge by a research lab with more track record. MIT means
nothing can be withdrawn, and 0007 caps the cost of switching.

**Rejected.** **Automerge** (more mature and better documented, and the
fallback if Loro stalls; larger documents, no movable list). **cr-sqlite**
(elegant on paper; building it for iOS/Android/wasm is a real cost and the
project has slowed). **A hand-rolled op-log with last-writer-wins** (more
work, worse result).

---

## 0007 — The CRDT is confined to `store`

**Context.** Direct consequence of the sustainability risk in 0006.

**Decision.** `loro` is named only in the `[workspace.dependencies]`
registry and in `crates/store`. Domain types are plain Rust structs;
`store` translates both ways.

**Consequences.** Swapping the CRDT is a one-crate rewrite that leaves
domain logic and UI untouched. Costs a mapping layer that would otherwise
not exist. Binding as CONSTITUTION Rule 2.

---

## 0008 — Serialized snapshots, not SQLite

**Context.** A family library is a few hundred kilobytes — around 200
recipes plus one list. SQLite is the reflex, and the wrong one at this size.

**Decision.** Persist the serialized CRDT snapshot as a blob; query in
memory. `Storage` trait with a file backend (native) and IndexedDB (wasm).

**Consequences.** Removes schema migrations, the relational/CRDT impedance
mismatch, and building SQLite for four targets. If data volume ever
invalidates the premise, the trait is the seam to revisit.

---

## 0009 — Zero-knowledge relay with app-layer E2EE

**Context.** Sync must be secure, must not use a paid cloud service, and an
RPi4 running Home Assistant 24/7 is available but not mandatory.

**Decision.** Each device holds a full replica. A **relay on the RPi4**
brokers sync over WebSocket and **persists the encrypted snapshot and
deltas**. Payloads are sealed with XChaCha20-Poly1305 under one symmetric
key shared by the family; the relay holds no key.

**Consequences.** The relay being *stateful* is essential, not incidental: a
pure broadcast relay never reconciles two devices that are never online at
the same time — the normal case for a phone in a shop and a laptop at home.

Encrypting at the app layer makes hosting a matter of convenience rather
than trust, so the relay can move anywhere without revisiting the threat
model. Accepted trade-off: one shared key means no forward secrecy, and
revoking a lost device requires rotating the key and re-pairing everyone.

**Rejected.** **iroh / P2P QUIC** (attractive — direct device-to-device with
hole punching — but browser support is partial and the document-sync layer
was moved out of the main repo; a candidate for a later LAN optimisation,
too risky as a foundation). **Syncthing** (no iOS client, and it produces
conflict copies to resolve by hand). **Git** (works offline, but no realtime
and manual conflicts — unfit for the shop scenario). **CouchDB** (proven
replication, but JS-centric and its revision tree bloats). **Home Assistant
as the backend** (its entity/state model is a poor fit for structured
application data; the RPi hosts the relay, HA itself is not involved).

---

## 0010 — The relay ships as a Home Assistant OS add-on

**Context.** The RPi4 runs **HAOS**, a locked appliance OS where running
arbitrary `docker compose` is fragile and discouraged.

**Decision.** Package the relay as an **HA add-on**. CI builds the arm64
image — Svelte bundle embedded into the binary via `rust-embed` — and
pushes it to ghcr.io; the add-on references the prebuilt image rather than
building on the Pi.

**Consequences.** Two things fall out for free. Add-ons get a persistent
`/data` volume **covered by HA's own backups**, which makes the relay the
recovery point if every device is lost. And updating the app becomes a
version bump plus the Update button in the HA UI.

A single artifact serves the static PWA and the sync WebSocket from one
origin: no CORS, one certificate, trivial service-worker scope. No HA
ingress — the app is consumed externally, not embedded in the HA UI.

**Rejected.** Plain `docker compose` (would have been the answer on HA
Container over Raspberry Pi OS; not viable on HAOS).

---

## 0011 — No background sync, no push in v1

**Context.** iOS terminates background execution aggressively; chasing it is
a well-known time sink.

**Decision.** Sync on foreground, and hold a live WebSocket while the app is
active. No background sync. No push notifications.

**Consequences.** Sufficient for the actual use — you open the app on
arriving at the shop. Push would need APNs and FCM: free, but it adds server
complexity and would leak notification content to Apple and Google unless
sent content-free to trigger a local sync. Reopening either requires a new
entry (Rule 14).

---

## 0012 — Cloudflare Tunnel on an owned domain

**Context.** The RPi4 sits behind a home router with no fixed public IP, and
a phone on 4G in a supermarket must reach it.

**Decision.** **Cloudflare Tunnel** onto a domain we own (≈10 €/yr). A
daemon on the RPi opens an *outbound* connection; nothing is exposed on the
router and TLS is handled.

**Consequences.** The deciding argument is not security — with 0009
everything is already encrypted end to end, so all the candidates were
acceptable — but **origin permanence**: on iOS the origin *is* the PWA's
identity. Changing domain later makes iOS treat it as a different app,
dropping the icon and its IndexedDB. That rules out an address we do not
control. The ~10 €/yr is not a subscription to a storage service (which was
excluded); it buys a stable address, which is what a durable PWA needs.

**Rejected.** **Tailscale Funnel** (free, no domain, no open port — but the
`ts.net` hostname is not ours, see above). **DuckDNS + port forwarding**
(free, but opens a port and the hostname is not ours). **Tailscale proper**
(disqualified earlier: the PWA would require the Tailscale client active on
each device, and sharing a URL would stop working).

**Related, deliberately separate.** Remote access to Home Assistant itself
was raised as a possible second use of the same tunnel. It is technically a
few lines (multiple ingress hostnames, plus `use_x_forwarded_for` and
`trusted_proxies` in HA), but the risk profiles differ sharply: the relay is
zero-knowledge, whereas HA controls the house and its login page is
continuously scanned. The recommendation on record is **Tailscale for HA,
Cloudflare for the app** — HA has no reason to be public. Deferred by the
owner; out of scope for this repo.

---

## 0013 — Nix flake with a separate Android shell

**Context.** The Android SDK and NDK are a multi-gigabyte download that only
M7 needs.

**Decision.** `devShells.default` carries the Rust toolchain (with the
`wasm32-unknown-unknown` target), wasm tooling and Node/pnpm.
`devShells.android` adds the SDK/NDK, `cargo-ndk` and a JDK. Nix evaluates
shells lazily, so the everyday shell stays small.

**Consequences.** The Android pins are **deliberately unvalidated** until M7
opens; nothing depends on them before then, and M7 starts by validating
them. The flake also ships `check-wasm-bindgen`, which compares the
`wasm-bindgen-cli` version against the Cargo entry — that mismatch produces
a blank page with no useful error, so it is worth a CI check (Rule 13).

iOS needs no Nix support at all, per 0003.

---

## 0014 — Quantities are exact rationals

**Context.** Scaling is the operation the whole product rests on. Floats
drift and render badly.

**Decision.** `num-rational` from entry through aggregation; `f64` only in
the final rendering step, outside `domain`.

**Consequences.** Scaling 4→6 then 6→4 returns the original numbers, and ⅓
cup stays ⅓ instead of 0.3333333333333333. Binding as Rule 4.

---

## 0015 — No cross-dimension conversion without an explicit coefficient

**Context.** 200 g of tomatoes in one recipe and 3 tomatoes in another
cannot simply be summed. This is what separates a good shopping app from a
bad one.

**Decision.** Dimensions are mass, volume, count and unmeasured.
Aggregation happens **within** a dimension. Crossing dimensions requires the
ingredient's own coefficient: **density** (g/ml) for mass↔volume, **unit
weight** (g/piece) for count↔mass. Both optional; absent, the amounts stay
on separate lines.

**Consequences.** The cart may show "Flour: 300 g + 2 tbsp" rather than a
single invented number. That is the honest rendering: a plausible wrong
conversion produces a quantity nobody can trace back. With both coefficients
set, all three dimensions interconvert. Binding as Rule 5.

---

## 0016 — Unit locale variants; count rounds up in the cart

**Context.** A French tablespoon is 15 ml, a US one 14.79; a US cup is
236.6 ml, a metric one 250. Recipe import is wanted later, not now.

**Decision.** Unit variants carry their locale from the start, even though
the UI exposes metric only. Separately: for the `Count` dimension the
**cart** rounds up, while the recipe keeps the exact value.

**Consequences.** Encoding the locale now costs nothing; retrofitting it
when import arrives would be a data migration. And scaling a recipe by 1.5
yields 1.5 eggs — the cart says buy 2, while the instructions can still say
"1 egg + 1 yolk". Mass and volume round only for display.

---

## 0017 — Recipes carry servings and an optional yield

**Context.** "A recipe states how many people its quantities serve" is not
enough for sub-recipes: if a shortcrust pastry serves 4 and a tart uses
200 g of it, the scale factor is undefined.

**Decision.** A recipe declares `servings` **and an optional `yield`**
(a quantity, e.g. "makes 500 g"). A sub-recipe reference carries either a
factor or an absolute amount of that yield. The graph is a DAG: expansion
performs cycle detection and bounds depth.

---

## 0018 — Scope cuts: no pantry, a single list, no ad-hoc cart items

**Context.** Explicit product scoping.

**Decision.** No pantry/stock tracking. **One** shopping list. Nothing
bought spontaneously at the shop is entered into the app.

**Consequences.** The overlay becomes a flat map keyed by canonical
ingredient, with no "which list" dimension, and aggregation loses its stock
subtraction step: `list → expand recipes → sum by (ingredient, dimension) →
sort by aisle`, a pure function of roughly a hundred lines. It also removes
the list picker from the UI, so the home screen can be the cart itself —
the right default when you open the app on arriving at the shop. Reopening
any of these requires a new entry (Rule 14).

---

## 0019 — The cart is derived; the overlay stores only explicit actions

**Context.** The cart is described as derived from the list, but items get
checked, and 0023 adds a derived default on top.

**Decision.** The cart is a **pure function** of the sources, plus a
persisted overlay holding **only explicit user actions**
(`Checked { by, at }` / `Unchecked`). It is never stored and **never
synced**; only sources are — recipes, ingredients, the list, the overlay,
users, devices, the event log.

**Consequences.** Shrinks the CRDT surface to what genuinely has concurrent
writers and removes a class of replica-versus-own-inputs inconsistency. Two
easily-missed corollaries, both binding in Rule 3: an explicit `Unchecked`
**must** be persisted or the next derivation silently re-checks it; and
adding an ingredient to the list **purges its overlay entry** so it returns
to its derived default.

---

## 0020 — List entries vanish on completion; purge is deferred

**Context.** The requirement was that list entries empty out as items are
checked in the cart. This collides with aggregation: one cart line can come
from several list entries, and a recipe is atomic — you cannot remove a
third of it.

**Decision.** A list entry disappears once **all** its ingredient
contributions are checked; until then a recipe shows partial progress
("5/7"). Checking a shared ingredient advances every recipe that needs it at
once. Checked entries move to a collapsed section rather than being deleted;
actual deletion happens on **"finish shopping"**, which also clears the
overlay.

**Consequences.** The visible behaviour requested — the list emptying as you
shop — while keeping undo. This deliberately departs from a literal reading
of the requirement: on a phone in a shop, mis-taps are frequent, and without
undo a wrong tap means rebuilding a recipe by hand.

---

## 0021 — Pairing by QR with a mandatory 12-word fallback

**Context.** QR pairing needs the camera. `getUserMedia` works in installed
iOS PWAs today, but it was broken for years and is the kind of thing that
regresses.

**Decision.** Pair by QR code, **always** with a manual fallback: a
12-word recovery phrase, copy-pasteable.

**Consequences.** Marginal cost is zero — the phrase is needed anyway as the
key backup — and it de-risks pairing entirely. Pairing also asks the new
device which user it belongs to (0024).

---

## 0022 — Instruction steps are segments referencing ingredient usages

**Context.** Quantities must appear inside the instruction text, with
ingredients and amounts emphasised (bold, colour).

**Decision.** A step is a sequence of segments: `Text(String)` or
`Ingredient { usage, display }`. The reference points at a **usage** — a
specific line of the recipe's ingredient list — not at the ingredient,
because a recipe may use flour twice in different amounts in different
steps. `display` handles second mentions ("add **200 g of flour**" … "stir
in **the remaining flour**").

**Consequences.** The point is that the rendered quantity is the **scaled**
one: change a tart from 4 to 6 servings and the instruction text updates
itself. Authoring uses `@`-mention autocomplete scoped to the recipe's own
usages.

Accepted coupling: deleting a referenced ingredient leaves a dangling
reference. Strict referential integrity is impossible under a CRDT — one
device deletes while another references — so the handling is soft: render
the orphan with a warning so it gets fixed, never panic, never block the
deletion.

**Rejected.** Plain markdown steps, which had been the earlier
recommendation on cost grounds; the emphasis-and-scaling requirement
reverses it. Storing steps as a string with embedded placeholders (simpler,
and a text CRDT would give collaborative editing for free, but a user edit
can break a marker — and simultaneous editing of one step is not a real use
case for two people).

---

## 0023 — The staple flag and its derived auto-check

**Context.** Nearly every recipe contains salt, pepper, oil, flour. Without
handling, the cart shows "Salt — 2 pinches" on every trip and you learn to
ignore lines, which is what makes you miss the real ones. The pantry feature
that would solve it properly was cut (0018).

**Decision.** A boolean `staple` on the ingredient. A staple sourced **only
from recipes** defaults to `AutoChecked`, so it is out of the way but still
visible and uncheckable. Adding that ingredient **manually** to the list
purges its overlay entry (0019), so it falls back to `ToBuy` and becomes
visible.

**Consequences.** Reuses the existing check mechanism instead of adding a
second notion of visibility. No quantity to maintain — this is not stock
tracking. Two distinct collapsed sections in the cart, because they do not
mean the same thing: "Bought" (you picked it up) and "Already at home"
(nothing to do); merging them makes unchecking a staple hard to discover.

---

## 0024 — Attribution is declarative, not cryptographic

**Context.** Two users across 4–5 devices, with a requirement to know who
created, added or deleted what. Trust between them is a given.

**Decision.** Model `User` and `Device` separately (a user owns several
devices). Attribution via **fields on the data** (`added_by`/`added_at`,
`checked_by`) plus an **explicit capped event log** for deletions and edits,
which leave no trace on the data itself. Do not derive any of this from
Loro's internal peer ids — those are an implementation detail, awkward to
query and unstable across versions.

**Decision (security).** No per-update signatures. Attribution is a
convenience, **not access control**: with one shared key, any holder can
write as anyone.

**Consequences.** Ed25519 per-device signing was evaluated and rejected on
the merits: the same shared key that decrypts also lets an attacker enrol a
forged device in the roster, so signatures would defend against nothing
under this threat model while adding key distribution and revocation. To be
revisited if a less-trusted third party ever joins.

The UI must state the limit where it implies identity — in particular on the
device screen, since revoking a lost device means rotating the key and
re-pairing everyone, with no middle ground.

A welcome side effect: with `checked_by`, two people shopping in different
aisles each see in real time what the other has just picked up. That is
plausibly the best feature of the product, and it fell out of this decision
for free.

---

## 0025 — This log is append-only

**Context.** The entire design came out of one long discussion. The
reasoning is the part that rots silently.

**Decision.** `docs/DECISIONS.md` is append-only. A reversed choice gets a
new entry that supersedes the old; the superseded entry stays, marked, with
its original reasoning intact.

**Consequences.** Binding as Rule 14. Rewriting history to look tidy would
destroy exactly the information that prevents making the same mistake twice.

---

## 0026 — The visual identity is deliberately deferred

**Context.** "Fluid and elegant" is a stated product requirement, but the
visual direction is explicitly postponed: low-effort and vanilla for now.

**Decision.** Native CSS with Svelte's scoped styles, system font stack,
native form controls, no CSS framework and no component library.

**Consequences.** Only one discipline has to hold in the meantime: **no
hardcoded visual value** — everything through CSS custom properties (Rule
10). That is what turns the future restyle into a token change rather than a
sweep through every component, and dark mode falls out of it for free. Note
that native form controls are also the cheapest path to decent
accessibility.

---

## 0027 — License: MIT OR Apache-2.0

**Context.** Not discussed explicitly; chosen to match the sibling project
and the Rust ecosystem default, and compatible with Loro (MIT).

**Decision.** Dual **MIT OR Apache-2.0**.

**Status.** *Provisional* — to confirm, or replace by a superseding entry,
before the first release. `LICENSE-MIT` and `LICENSE-APACHE` are in the tree
under this assumption; changing the licence means replacing them and adding
that superseding entry.

---

## 0028 — Finishing a trip prunes the overlay selectively

**Date** 2026-08-08 · **Status** Accepted · **Refines** [0020](#0020--list-entries-vanish-on-completion-purge-is-deferred)

**Context.** 0020 settled that "finish shopping" removes completed entries and
clears the overlay. Implementing it in M1 exposed a case the original wording
did not cover: the trip where you *could not* finish an entry, because the
shop was out of one item. Clearing the overlay wholesale would reset the five
things you did buy back to unchecked, on a recipe that stays on the list.

**Decision.** Remove the completed entries, and drop an overlay entry **only
when every list entry that asked for that ingredient is going away** — which
is exactly when its cart line disappears too. Everything else keeps its state.

**Consequences.** A partially bought recipe keeps its checks and its progress
across the end of a trip. An ingredient shared between a finished and an
unfinished entry keeps its check, which is the right answer: you did buy the
flour, and the crepes still need some.

The rule is computable from the cart alone — no re-derivation — because a
cart line already records which entries asked for it.

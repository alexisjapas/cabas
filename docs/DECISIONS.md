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
| [0016](#0016--unit-locale-variants-count-rounds-up-in-the-cart) | Unit locale variants; count rounds up in the cart | Domain |
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
| [0029](#0029--how-the-document-encodes-domain-values) | How the document encodes domain values | Storage |
| [0030](#0030--indexeddb-is-tested-in-a-real-browser) | IndexedDB is tested in a real browser | Tooling |
| [0031](#0031--the-devices-identity-comes-from-the-host) | The device's identity comes from the host | Architecture |
| [0032](#0032--applying-a-command-and-saving-are-two-steps) | Applying a command and saving are two steps | Architecture |
| [0033](#0033--one-state-pushed-whole-rebuilt-from-the-document) | One state, pushed whole, rebuilt from the document | Architecture |
| [0034](#0034--a-broken-reference-is-a-warning-not-an-empty-screen) | A broken reference is a warning, not an empty screen | Product |
| [0035](#0035--the-app-owns-every-number-the-frontend-owns-every-word) | The app owns every number; the frontend owns every word | Architecture |
| [0036](#0036--typescript-types-are-generated-behind-a-feature) | TypeScript types are generated, behind a feature | Tooling |
| [0037](#0037--the-pwa-is-a-plain-vite-spa-not-sveltekit) | The PWA is a plain Vite SPA, not SvelteKit | Stack |
| [0038](#0038--the-service-worker-is-written-by-hand) | The service worker is written by hand | Stack |
| [0039](#0039--the-editor-names-a-recipe-line-before-the-line-exists) | The editor names a recipe line before the line exists | Architecture |
| [0040](#0040--the-keyboard-is-a-length-not-a-mode) | The keyboard is a length, not a mode | Stack |
| [0041](#0041--the-phone-installs-from-a-local-certificate-authority) | The phone installs from a local certificate authority | Tooling |
| [0042](#0042--the-relay-keeps-a-sequenced-log-it-cannot-read) | The relay keeps a sequenced log it cannot read | Sync |
| [0043](#0043--the-pwas-websocket-lives-in-the-frontend) | The PWA's WebSocket lives in the frontend | Sync |
| [0044](#0044--in-development-the-sync-socket-goes-through-ui-serve) | In development the sync socket goes through `ui-serve` | Tooling |
| [0045](#0045--a-cursor-is-not-resumed-on-a-replica-that-never-had-it) | A cursor is not resumed on a replica that never had it | Sync |
| [0046](#0046--a-u64-crosses-the-wasm-boundary-as-text) | A `u64` crosses the wasm boundary as text | Architecture |
| [0047](#0047--the-qr-is-shown-never-scanned-and-the-encoder-is-ours) | The QR is shown, never scanned, and the encoder is ours | Product |
| [0048](#0048--the-bundle-is-compiled-into-the-relay-by-a-build-script) | The bundle is compiled into the relay, by a build script | Deployment |

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

---

## 0029 — How the document encodes domain values

**Date** 2026-08-08 · **Status** Accepted · **Implements**
[0006](#0006--loro-as-the-crdt), [0008](#0008--serialized-snapshots-not-sqlite)

**Context.** M2 had to put domain values into a Loro document. `LoroValue`
offers `Null`, `Bool`, `I64`, `Double`, `String`, `Binary`, `List` and `Map`
— and nothing exact between `I64` and `Double`. Quantities are
`Ratio<i128>` (Rule 4). Three encoding questions followed, plus one that only
appeared once the CRDT was real.

**Decision.**

1. **Rationals are strings**, `"numerator/denominator"`. Not `Double`, which
   would insert a rounding step between two devices that are supposed to
   agree. Not a pair of `i64`, which overflows on the exact imperial factors
   that motivated `i128` in the first place. Reading always goes through
   `Ratio::new`, never `new_raw`, so a value written unreduced by some future
   writer still compares equal to its own reduced form.
2. **Enum variants are string tags**, not integer discriminants. A
   discriminant silently changes meaning when a variant is inserted in the
   middle of an enum, and this document outlives the build that wrote it. One
   deliberate asymmetry: an unknown *aisle* degrades to `Other`, because an
   aisle only decides sort order, while an unknown *unit* refuses — a
   quantity read wrong is a wrong shopping list.
3. **Entities are containers; their sub-structures are plain value maps.** An
   ingredient or a recipe is a container, so two people editing different
   fields of it merge field by field. A recipe's ingredient lines and steps
   are value maps inside a movable list: they are edited one line at a time,
   and 0022 already ruled out two people co-editing a single step. Making
   those containers too would buy a merge nobody performs, at the cost of a
   schema no one can read.
4. **Writes only touch what changed.** Every setter compares before it
   inserts. This keeps unchanged fields out of the history — which is what
   has to stay bounded — but the real reason is merge quality: a coarse
   `put_recipe(&Recipe)` that rewrote every field would turn "one person
   renames the recipe while the other adds an ingredient" into a conflict and
   lose the ingredient.

**Consequences.** Read and write are deliberately asymmetric: writes go
through containers because that is what produces a mergeable operation, reads
go through `get_deep_value()` so every reader walks plain values instead of
branching on container-or-value at each level. At 154 kB for the whole
library, materialising it is not a cost worth optimising against clarity.

Two traps this uncovered, both now load-bearing in the code:

- **`get_or_create_container` is the wrong constructor** and is deprecated
  for exactly this reason: it gives the child an operation-derived id, so two
  devices that create the same ingredient while offline end up with two
  different containers under one key, and the merge keeps one and silently
  drops the other. `ensure_mergeable_map` derives the child's id from the
  key, which is what makes the two creations converge.
- **A Loro map hands its keys back in hash order**, which is not stable
  between replicas. Every keyed read sorts by id, or two devices show the
  same library in different orders — a bug that only ever appears on the
  second device.

The schema is a compatibility surface, not an implementation detail: a phone
left in a pocket for three weeks must still converge with the relay, so
changing any of the above is a breaking change under Rule 15. That is what
the `meta.schema` marker is for, and why a document from the future is
refused outright rather than read partially — a partial read would drop the
fields this build cannot see, and the next save would propagate that loss to
every other device.

---

## 0030 — IndexedDB is tested in a real browser

**Date** 2026-08-08 · **Status** Accepted · **Extends**
[0013](#0013--nix-flake-with-a-separate-android-shell)

**Context.** M2's `Storage` trait has two implementations. The file backend
is testable anywhere. The IndexedDB one is not: IndexedDB is a browser API,
it has no native equivalent, and `wasm-check` only proves the code
*compiles* for wasm32 — a backend that opens no database and stores no bytes
would pass it just as happily.

Three options. Mock IndexedDB behind a trait and test against the mock, which
proves the mock works and nothing else. Ship it untested and find out at M4,
on the phone, where every bug also looks like a Svelte bug. Or run a real
browser.

**Decision.** Run a real browser. `#[wasm_bindgen_test]` cases in
`crates/store/tests/indexeddb.rs`, executed by `wasm-bindgen-test-runner`
against headless chromium, in a **separate devShell** (`.#wasm-test`) and a
**separate CI job** (`wasm-storage`).

**Consequences.** Chromium and chromedriver are a large download that the
everyday gates have no use for, so they follow the rule 0013 already set for
the Android SDK: their own shell, entered only when needed. The CI job is
separate for the same reason plus one more — a browser failure then reads as
a browser failure, instead of turning `fmt` red for reasons nobody can see.

The runner ships inside `wasm-bindgen-cli`, so the Rule 13 pin governs it
too. That is a benefit rather than an accident: a test runner from a
different version than the `wasm-bindgen` crate fails exactly the way a
mismatched release build does — opaquely — except in CI, where there is even
less to go on.

Scoped to `--test indexeddb` deliberately. The rest of the suite is
platform-free and already runs natively in milliseconds; building it for
wasm32 as well would cost minutes per push to re-prove what `wasm-check`
proves in seconds.

What this buys, concretely: the browser job is what showed the whole vertical
works — a Loro document, snapshotted, stored in IndexedDB, read back, with
its exact rationals intact. That is the claim M4 has to be able to assume,
and now it does not have to assume it.

---

## 0031 — The device's identity comes from the host

**Date** 2026-08-08 · **Status** Accepted · **Extends**
[0024](#0024--attribution-is-declarative-not-cryptographic)

**Context.** Every write carries an author: a list entry has `added_by`, a
checked line has `checked_by`, the event log has `by`. So `app` needs to know
which user and which device it is running as, and that answer has to survive
a restart — a replica that mints a fresh identity on every launch would fill
the roster with ghosts and attribute each trip to a different stranger.

The family document is the wrong place to keep it. It holds the `User` and
`Device` *records*, which are shared, but "which of these am I" is a fact
about one device, and the document is the one thing that is identical on all
of them.

**Decision.** [`Identity`] is a parameter of `App::open`, minted once by
`Identity::mint` and persisted by the **host**: `localStorage` in the PWA, a
config file under Tauri. On open, `app` writes the matching `User` and
`Device` into the document if they are not already there, and never
overwrites them.

**Consequences.** The one piece of state the frontend legitimately holds is
four opaque strings it does not read, which is as close to Rule 9 as this can
get: the alternative is a second storage seam in `Storage` for device-local
data, on a trait whose entire virtue is that it stores one blob.

"Never overwrites" is load-bearing in the other direction too. The name in
the document wins over the one the host passes in, because the other device
may have renamed the person since this one last launched — and a launch is
not a rename.

`Identity::mint` needs randomness, which is what puts `getrandom` in this
crate a milestone before the crypto needs it. That is a benefit: the wasm
backend it requires is configured and *proven by a browser test* now, rather
than discovered at M5 as a link error.

**Rejected.** **Deriving the identity from the Loro peer id** — it is Loro's
internal business, 0024 keeps attribution out of it, and it is not stable
across a reinstall either. **Minting inside `app` on first run and storing it
in the document** — every device would then read every other device's
"self", and the first merge would have to pick one.

---

## 0032 — Applying a command and saving are two steps

**Date** 2026-08-08 · **Status** Accepted · **Implements**
[0009](#0009--zero-knowledge-relay-with-app-layer-e2ee)

**Context.** `Storage` is async, because IndexedDB has no blocking form
(0030). The obvious `async fn dispatch(command) -> State` would therefore
make every user action await a browser transaction before anything renders.

**Decision.** `App::apply` is **synchronous** and returns the new state;
`App::persist` is async and writes. `App::dispatch` is the convenience that
does both, for native hosts and tests. The PWA binding exposes the two halves
separately, and the frontend renders on `apply` and lets `flush` resolve
whenever it does.

**Consequences.** Rule 6 says no user action waits on the network; this is
the same argument one layer down, and it matters for the same reason — a tick
in a shop happens on a five-year-old phone with a cold IndexedDB.

It also removes a trap that would otherwise sit exactly where nobody is
watching for it. An exported async method holds its borrow of the app for as
long as its promise is pending, so an `apply` that awaited its own save would
panic on the second tap of an impatient thumb. With the mutation synchronous,
the borrow is released before anything is awaited.

The cost is that a save can be forgotten. It is bounded: `pending_snapshot`
reports the revision it hands out and `mark_saved` only clears up to that
revision, so a command applied while a write is in flight stays pending
rather than being counted as saved. What is *not* bounded is a host that
never calls `flush` at all — which is why the native `dispatch` exists and
why the browser test asserts that the second `flush` writes nothing.

**Rejected.** **Saving inside `apply` and returning a promise** — see the
borrow above. **A background write loop** — a timer the UI cannot see is a
worse contract than a promise it can ignore.

---

## 0033 — One state, pushed whole, rebuilt from the document

**Date** 2026-08-08 · **Status** Accepted · **Implements**
[0004](#0004--svelte-5-as-the-frontend-framework)

**Context.** Rule 9 says the frontend renders view-models and holds no
business state. That leaves two questions: how much state travels per change,
and where it is computed from.

**Decision.** Every state change returns a **complete** `StateView` — cart,
list, library, the open recipe, and whatever could not be made sense of. It
is rebuilt by reading the whole document into plain domain values and
deriving from that, on every command.

**Consequences.** The screen cannot show two things that disagree, because it
only ever received one thing. There is no getter surface to grow, no
invalidation to get wrong, and no incremental update path that can drift from
the document.

Affordable because M2 measured it: 200 recipes are 154 kB of CRDT and read
back in about 10 ms natively. The wasm figure is some multiple of that and
gets measured on the actual phone at M4 — which is also the only place it
means anything. If it is too slow there, the fix is an incremental read
*behind the same seam* (`Library::read`), not a redesign of the API.

Two commands deliberately return a state without touching the document at
all: opening and closing a recipe. Which recipe is open is device-local — two
people reading different recipes is not a conflict — so it lives in `App` and
is never persisted or synced.

**Rejected.** **Deltas or patches** — the diffing would be business logic
that has to be right on both sides of an FFI boundary, to save bytes that
never leave the process. **A getter per screen** — three fine calls in a row
are three chances for the UI to decide what happens between them.

---

## 0034 — A broken reference is a warning, not an empty screen

**Date** 2026-08-08 · **Status** Accepted · **Implements**
[0022](#0022--instruction-steps-are-segments-referencing-ingredient-usages)

**Context.** `cart::derive` refuses a list that names a recipe or an
ingredient it cannot find. That is right for a pure function and wrong for a
screen: under a CRDT, one device deleting a recipe while the other has it on
the list is not an error state, it is Tuesday. An `Err` reaching the frontend
would blank the entire cart because of one row.

**Decision.** `app` triages the list before deriving. An entry whose recipe
is gone — or whose expansion hits a cycle, a depth bound or a missing yield —
is set aside and reported as a `ProblemView`. An **ingredient** that is gone
is replaced by a placeholder carrying its own id as a name, so its line stays
in the cart. What is left cannot fail to derive.

**Consequences.** The user sees a warning and eight ingredients rather than a
warning and nothing, which is the difference between a shopping trip that
works and one that does not. The two failures are handled differently on
purpose: a missing recipe contributes nothing that could be shown, while a
missing ingredient still has a quantity somebody has to buy.

`ProblemView` carries the domain's own message as an English `detail`. That
is a diagnostic, not UI copy — the frontend shows its own sentence per kind
(0035) and keeps the detail for a details view.

The residual `Err` from `derive` is still handled rather than unwrapped:
"unreachable" is a claim about today's domain code, and this is a screen
either way.

---

## 0035 — The app owns every number; the frontend owns every word

**Date** 2026-08-08 · **Status** Accepted · **Implements**
[0026](#0026--the-visual-identity-is-deliberately-deferred)

**Context.** Rule 9 puts all business logic in `app`, and the app is
French-only for now while everything persisted in the repo is English. Both
cannot be true of a view-model that contains the sentence a user reads.

**Decision.** A view-model carries **rendered numbers** and **machine tags**.
`{ amount: "1 1/2", unit: "kg", approximate: false }`, never "1,5 kg". The
frontend maps `"kg"` to its label, `"produce"` to *Rayon frais*,
`"missing_recipe"` to a sentence. The app writes no prose a user reads.

**Consequences.** Every quantity, conversion, scaling and rounding stays in
Rust where the tests are — the frontend cannot compute an amount because it
never receives the pieces to compute one from. And the eventual translation
is a frontend change, not a core change, which is what makes deferring i18n
cheap rather than expensive.

The seam is not free of judgement: the decimal separator is the frontend's
(it renders "1.3" as *1,3*), and so is the choice of "≈" for an approximate
amount. Both are presentation. What is not presentation, and stays here, is
*which* rendering is faithful — a quantity that had to be rounded says so,
because the cart is still adding up the exact value underneath.

One consequence worth stating plainly: an edit form is rendered by a
different function than a screen. `render` may round; `render_lossless` never
does, falling back to a raw `28349523125/1000000000` when the pretty form
would lie. An editor that displayed the rounded text would write it back on
the next save, and a quantity would quietly become 28.35 g because somebody
fixed a typo in the title.

---

## 0036 — TypeScript types are generated, behind a feature

**Date** 2026-08-08 · **Status** Accepted · **Implements**
[0004](#0004--svelte-5-as-the-frontend-framework)

**Context.** Rule 9 requires the frontend's types to be generated from Rust
rather than hand-written. Two ways to do it: `tsify`, which extends the
`wasm-bindgen` glue, or `ts-rs`, which derives an exporter and writes `.ts`
files from a test.

**Decision.** `ts-rs`, behind `cabas-app`'s optional `typescript` feature.
`cargo test -p cabas-app --features typescript export_bindings` writes
`ui/src/lib/bindings/*.ts`; the files are committed, and CI regenerates them
and fails on a diff.

**Consequences.** Nothing of the generator reaches the shipped wasm — the
feature is off for every real build — and the types describe the *serde*
shape, which is exactly what crosses the boundary, on both transports. Tauri
(M7) gets the same declarations for free, which `tsify` would not have given:
its output is tied to the wasm-bindgen glue that Tauri does not use.

Two settings make the declarations true rather than merely plausible, both in
`.cargo/config.toml`: `TS_RS_LARGE_INT = "number"`, because
`serde-wasm-bindgen` hands `i64` to JS as a plain number, and — on the other
side — the serializer is configured with `serialize_missing_as_null`, because
the generated types say `| null` and the default is `undefined`. A
declaration that disagrees with the runtime is worse than no declaration: it
is a test that always passes.

Committing generated files is a deliberate cost. The alternative is a
frontend build that needs a Rust toolchain, and the drift it risks is exactly
what the CI check removes.

**Rejected.** **`tsify`** — see above. **Hand-written types** — Rule 9.
**Generating into `ui/` at build time** — makes the frontend unbuildable
without the whole Nix shell, for no gain over a committed file plus a check.

---

## 0037 — The PWA is a plain Vite SPA, not SvelteKit

**Date** 2026-08-08 · **Status** Accepted · **Implements**
[0004](#0004--svelte-5-as-the-frontend-framework)

**Context.** 0004 settled on Svelte 5 and said nothing about what sits around
it. The two candidates were SvelteKit — file-based routing, SSR, data
loading, an adapter that emits static files — and Vite on its own, with the
framework's compiler and nothing else.

**Decision.** **Vite + Svelte 5 as a single-page app.** No SvelteKit. The
current screen is a value in `Session`, persisted to `localStorage`, and the
whole of `ui/dist` is a folder of static files that M6 embeds into the relay
binary.

**Consequences.** SSR is SvelteKit's centre of gravity and this app has
nothing to render on a server: the data lives in IndexedDB on the phone, and
the relay is forbidden from being able to read it (Rule 7). Adopting it would
have meant `ssr = false` everywhere — carrying the concept in order to
disable it.

Routing is the sharper reason. DECISIONS 0003 requires the current screen to
be **persisted**, so an iOS cold reload resumes where you were; that is
application state, saved next to the identity. SvelteKit would make the URL
the source of truth, leaving two mechanisms doing one job and a
reconciliation between them to write and to keep correct. The SPA has one.

What is genuinely given up: file-based routing, and the ready-made asset
manifest a service worker wants. Six screens do not need the first, and
DECISIONS 0038 covers the second. The back gesture does not move between
screens until `history.pushState` is wired — about ten lines, and the same
ten lines under either choice.

The build pipeline follows from Rule 13 rather than from this entry, but it
is the same seam: the wasm core is built by calling `cargo build` and then
**the `wasm-bindgen` the flake pins**, not by `wasm-pack`, which fetches a CLI
of its own choosing and would quietly reintroduce exactly the version skew
`check-wasm-bindgen` exists to catch. Building it in two steps also gets the
`wasm-release` profile, which `wasm-pack` has no flag for.

**Rejected.** **SvelteKit + `adapter-static`** — see above; reconsider if a
public web surface is ever in scope, which Rule 14 currently forbids.
**A router library** — the screen list is a union type and the switch is an
`{#if}` chain; a dependency would replace four lines with a concept.

---

## 0038 — The service worker is written by hand

**Date** 2026-08-08 · **Status** Accepted · **Implements**
[0003](#0003--ios-ships-as-a-pwa)

**Context.** An installed PWA that opens without network needs a service
worker; it is also what makes the app installable at all. The choice was
between writing one and generating it with `vite-plugin-pwa`, which wraps
Google's Workbox.

**Decision.** **A hand-written `sw.js`**, precaching the app shell from Vite's
own build manifest.

**Consequences.** The scope is much smaller than it first looks: the service
worker caches **the application** — HTML, JS, CSS, the wasm module, icons —
and none of the data, because the recipes and the list are in IndexedDB and
already work offline. That leaves exactly one strategy, cache-first on a
versioned shell, where Workbox is built for the case of many: runtime caching
per route, expiration policies, background sync. None of those exist here and
none are coming (Rule 14, DECISIONS 0011).

The one genuinely fiddly part is **versioning**: a cache name that does not
change with the build serves the old app forever, and on an installed iOS PWA
that is indistinguishable from the app being broken. Vite emits fingerprinted
filenames and can write a manifest of them, so the precache list and the
cache name both come from the build rather than from a hand-maintained array.
That is the part to get right, and it is the part a review should look at.

Reversible at low cost, which is why it is worth trying the small thing
first: the service worker is one file plus its registration. If iOS update
behaviour turns out to need more care than this affords, adopting the plugin
is a contained change rather than a rewrite.

**Rejected.** **`vite-plugin-pwa` / Workbox** — see above. **No service
worker** — not an option: without one the app is not installable and does not
open in a shop with no signal, which is the entire point (Rule 6).

## 0039 — The editor names a recipe line before the line exists

**Date** 2026-08-08 · **Status** Accepted · **Implements**
[0022](#0022--instruction-steps-are-segments-referencing-ingredient-usages)

**Context.** 0022 made an instruction step a run of segments, and made an
ingredient mention reference a **usage** — a specific line of the recipe —
rather than an ingredient, so that a recipe using flour twice renders the
right amount in each step. That reference is an id, and the recipe editor is
the first thing to have to produce one: a person adds "250 g de farine" to a
new recipe and immediately writes "tamiser la farine" mentioning it, while
nothing has been saved and the line therefore has no id.

The ids of everything else are minted inside `App` while a command runs.
`SaveRecipe` does the same for a line whose `id` is absent — but it hands the
minted id back only in the state that follows the save, which is after the
steps referencing it had to be written.

**Decision.** **The host mints the usage id, from the core**, through
`cabas_app::mint_usage_id` and its binding `CabasApp.mintUsageId`. The editor
calls it when a line is added to the draft, uses the result both as the line's
`id` and as the `usage` its steps reference, and sends the whole recipe in one
`SaveRecipe`.

**Consequences.** A recipe is written in a single command, which is what makes
the editor an ordinary form: a local draft, one save, cancel costs nothing.
The alternative shape — save the lines, read their ids back, then write the
prose — is still valid and still exercised by `scenario.rs`, but it is no
longer what an editor has to do.

An id minted this way is indistinguishable from one `SaveRecipe` mints, and
that is the property to keep: same prefix, same width, same random source. It
is asserted directly in `id.rs`, because a second *kind* of usage id would be
a difference the document cannot see and a reader eventually would.

This is the second thing the host mints, after the device identity (0031), and
it is the same bargain for the same reason — the format and the randomness
stay in Rust, the host holds an opaque string. `crypto.randomUUID` in the
frontend would have worked exactly once per device and then, on the day two
phones add a line to the same recipe while both offline, produced two ids
whose collision a CRDT reports as one line rather than as a conflict.

It does widen the surface Rule 9 keeps narrow, by one function that decides
nothing. The line to hold is that the frontend *holds* the id and never
*reads* it: nothing parses the prefix, and the editor treats it as opaque.

**Rejected.** **Save first, then reference** — a recipe half-written into the
family library the moment a second ingredient is added, visible on the other
phone, and left behind entirely if the person changes their mind. It also
makes the editor a hybrid of draft and pushed state, re-seeding its fields
after every structural change, which is the shape that produces a form
overwriting what is being typed into it.
**A frontend-generated id** — see above; wrong source of randomness, and a
format the core did not choose.
**Referencing a line by its position in the recipe** — a reference that breaks
when a line is reordered or deleted, which is exactly what 0022 rejected
prose-with-markers for.

## 0040 — The keyboard is a length, not a mode

**Date** 2026-08-08 · **Status** Accepted · **Implements**
[0003](#0003--ios-ships-as-a-pwa)

**Context.** iOS does not resize the page when the soft keyboard opens. The
layout viewport keeps every pixel of its height and the keys are drawn over
the bottom third of it, so `100dvh`, `position: fixed` and
`env(safe-area-inset-bottom)` all go on describing a viewport that is no
longer there. The consequences are not cosmetic: a form's last field sits
behind the keys with no scroll position that brings it out, because the
document genuinely ends down there. The recipe editor has the worst case in
the app — the mention picker is the one control that appears *because* of what
was typed, which means it is drawn below a caret the keyboard is already
sitting under, and no `scrollIntoView` will move it, since to the browser it
is comfortably inside the viewport.

**Decision.** Measure the covered height from `visualViewport` and publish it
as **one CSS custom property, `--keyboard-inset`**, defaulting to `0px`. The
layout reads it through `max()`: a screen's body pads by the larger of the tab
bar and the keyboard, and the tab bar itself translates down by it. One
function reads the same number back — `reveal`, which scrolls the picker out
of the keys after it opens.

**Consequences.** The closed state is the layout that was there before any of
this existed, because every expression that reads the property collapses to
its old value at `0px`. There is no keyboard mode to enter, nothing to undo,
and no class whose removal can be missed on a screen that stops being looked
at — which matters on the platform that backgrounds an app whenever it likes
(0003). A component that later needs to clear the keyboard reads a length it
already understands.

The measurement is `clientHeight - (visualViewport.height + offsetTop)`, and
`offsetTop` is half of it: iOS scrolls the visual viewport up inside the
layout one to keep the caret above the keys rather than resizing anything. Two
things that shrink that viewport are deliberately *not* keyboards — a pinched
page, and chrome an order of magnitude smaller than any keyboard, such as a
collapsing address bar or an iPad accessory strip.

The padding and the scroll are one mechanism, not two: the picker can only
climb out of the keys because the padding put scrollable document under it.
Neither half is worth shipping alone.

Where to leave the gap above the keys stays in CSS, as the picker's own
`scroll-margin-bottom` — the property already means exactly that, and Rule 10
does not get an exception for a value that happens to be read by script.

`ui-test` covers this by overriding the `VisualViewport.height` accessor and
firing `resize`, because no DevTools command produces the shape: emulation
resizes the *layout* viewport, which is the one thing a keyboard never does,
and a page told the truth about its own height would not exercise any of the
above. What that proves is the half that is ours — given a viewport 300 px
shorter than the page, the layout clears it and the picker climbs out. It is
not the milestone's exit criterion. **The iPhone is**, keys and all.

**Rejected.** **A `keyboard-open` class on the root** — a mode, with a state
machine to leave it, on the platform most likely to background the app mid-word
and least likely to send the event that clears it. The length has no exit.
**`interactive-widget=resizes-content` in the viewport meta** — the honest
one-line fix, and iOS Safari does not implement it; relying on it would leave
the target platform as the only one still broken.
**`scrollIntoView` on the picker** — measured against the layout viewport, so
on iOS it is a no-op precisely when it is needed.
**Leaving it to the browser's own focus scrolling** — that scrolls to the
*field*, and the thing that needs to be seen is the list drawn underneath it,
which does not exist yet at the moment the field is focused.

## 0041 — The phone installs from a local certificate authority

**Date** 2026-08-08 · **Status** Accepted · **Implements**
[0003](#0003--ios-ships-as-a-pwa) · **Relates to**
[0012](#0012--cloudflare-tunnel-on-an-owned-domain)

**Context.** M4's exit criterion is the iPhone — installed from the home
screen, usable in airplane mode, data surviving a cold restart — and none of it
is reachable over the address `pnpm dev` prints. A service worker only
registers in a **secure context**, and `http://192.168.1.x` is not one. The LAN
dev server can therefore display the app on the phone and can never make it
installable: no worker, no precache, and airplane mode is a blank page. The
origin that will serve the app for real is the relay behind a Cloudflare Tunnel
on a domain we own (0012), and that is M6 — two milestones after the one this
blocks.

**Decision.** `ui-serve`: a local certificate authority, generated once per
machine, signing a certificate for that machine's own names, and a
zero-dependency Node server handing `ui/dist` over TLS. The phone installs the
CA once, from a plain-HTTP endpoint the same command serves — because it cannot
fetch the certificate over the HTTPS that certificate is what makes
trustworthy.

The certificate covers `<hostname>.local` as well as the LAN address, and **the
mDNS name is the one to install from**. An installed iOS PWA is identified by
its origin (0012), so an app installed from an address the DHCP lease can move
loses its IndexedDB the day it moves. iOS resolves such a name over Bonjour with
nothing configured on the phone — but only if the host actually announces it,
and NixOS ships avahi with `publish.enable = false`, so out of the box it does
not. Where that is not turned on the LAN address is the fallback, and the origin
then has to be made stable some other way: a reserved DHCP lease on the router
costs nothing and is enough.

**Consequences.** M4 can be exercised on the device, offline, with no internet
involved anywhere — which is what the milestone actually asks for. The
certificate is re-signed automatically when the address changes; the CA
deliberately is not, because re-minting it would mean re-installing a profile
on every phone that trusted the old one.

The cost is real and worth naming plainly: **a custom root installed on a phone
trusts this machine to vouch for any site at all.** The CA key is generated per
machine, `chmod 600`, gitignored, and never travels; if it leaks, whoever holds
it can impersonate any origin to that phone until the profile is removed. That
is the price of a secure context on a LAN with no public name, and it is the
reason the profile should come off the phone once M6's permanent origin exists.

Two iOS specifics, both silent when wrong. Installing a root and trusting it
are **two separate actions in two different screens** — Settings → Profile
Downloaded, then General → About → Certificate Trust Settings — and skipping
the second leaves a certificate that is installed, listed, and still refused;
the instruction page `ui-serve` hands the phone says so, because that is the
step that gets missed. And a server certificate iOS will accept has to carry
`serverAuth` in its extended key usage, live 825 days or fewer, and name its
hosts in the SAN rather than the common name. Miss any of the three and Safari
says only that the connection is not private.

What this is verified against, short of the phone: the existing `ui-test` takes
its target from `APP_URL`, so it runs unchanged against the TLS origin — the
worker registers and takes control, the shell precaches, and the app boots with
the network off. That proves the transport, and nothing about the phone.

The phone then answered the rest, on the day this was written: the profile
installs, iOS accepts the certificate, and the app runs from the home screen
with the network off. Bonjour was the one part that did not hold — the name
never resolved, for the reason above — so the install was done from the LAN
address, whose lease has to be reserved on the router for the origin to stay
put. See ROADMAP M4.

**Rejected.** **A Cloudflare quick tunnel** (`*.trycloudflare.com`) — nothing
to install, and a new hostname on every run, which iOS reads as a new app each
time and whose predecessor's storage it drops (0012). It also puts the family's
shopping list on a public URL, and needs internet for a test whose whole point
is not having any.
**Bringing M6's tunnel forward** — the permanent origin is the right answer and
it arrives with the add-on, the image and the backup drill. Pulling it in to
unblock one measurement would mean shipping the deployment milestone in order to
test the previous one.
**A self-signed certificate with no CA** — Safari's interstitial offers no
"proceed anyway" that yields a secure context, so the worker still would not
register. The exception a desktop browser grants is exactly the one iOS does
not.
**Plain HTTP, and testing the offline path in a desktop browser instead** —
that is what `ui-test` already does, and it is precisely the half that cannot
answer the question. 0040 makes the same point about the keyboard.

---

## 0042 — The relay keeps a sequenced log it cannot read

**Date** 2026-08-08 · **Status** Accepted · **Implements**
[0009](#0009--zero-knowledge-relay-with-app-layer-e2ee) · **Relates to**
[0021](#0021--pairing-by-qr-with-a-mandatory-12-word-fallback),
[0024](#0024--attribution-is-declarative-not-cryptographic)

**Context.** 0009 settled the shape — a stateful relay persisting ciphertext,
XChaCha20-Poly1305 under one family key — but not the protocol. The constraint
that shapes everything: Loro's own sync model is version-vector based ("here
is my version, send me what I lack"), and **the relay cannot read a version
vector**, because it cannot read anything. Whatever coordination the protocol
needs has to come from something the relay can hold without understanding.

**Decision.** Per family, the relay keeps an **append-only log of sealed
frames** and assigns each one a **sequence number** — the only ordering in the
protocol, and the relay's only contribution to it. A device remembers the last
sequence it has applied; on connect it says `since N`, the relay replays
everything after N and then forwards new frames live. A device pushes its own
changes as a sealed delta — `changes_since(version at its last push)`, sealed
before it leaves (Rule 7) — and the relay acknowledges with the assigned
sequence. Loro import is idempotent and commutative, so an overlapping or
replayed delta is harmless by construction, and a delta that includes ops the
receiver already merged costs bytes, not correctness.

**Compaction is device-driven**, because the relay cannot merge what it cannot
read: a device that has applied the log up to N may push a sealed **snapshot
declaring it covers N**, and the relay then drops every frame at or below N.
The frame kind — delta or snapshot — and the covered sequence are therefore
**plaintext metadata**, the irreducible minimum the relay needs to truncate;
payload sizes and timing it would see anyway. A client treats both kinds
identically on receipt, since Loro import accepts either.

**The 12-word BIP39 mnemonic is the single canonical secret** (0021). The
family key is the first 32 bytes of the standard BIP39 seed; the **family
id** — the relay's routing and storage key — is the next 16, hex-encoded.
PBKDF2's output blocks are independent, so publishing the id reveals nothing
of the key, and every device derives both from the same phrase with no second
channel. The id is unguessable, and that is the relay's entire access story:
whoever holds it may read ciphertext (which is the security model working)
and may append frames — a frame that fails to open is dropped by the client,
counted, never merged. The wordlist is BIP39 English even though the UI is
French: it is the list every backup tool understands, its words are unique in
four letters, and the phrase carries its own checksum.

**No argon2.** Key stretching defends low-entropy passwords; this seed is 128
machine-generated bits, and no amount of stretching improves on that. The
registry entry is removed rather than left as a note.

**Consequences.** The never-online-together case — the reason the relay is
stateful at all — falls out of the log: A pushes and leaves, B connects later
and replays. The simultaneous case falls out of the live forwarding, and
`checked_by` arriving mid-shop (0024) rides on it. The relay stays small
enough to audit by reading: append, replay, truncate, forward — no merge, no
conflict handling, no knowledge of what a shopping list is. Sync state on the
device — the cursor and the shadow version — is device-local, persisted
alongside the identity, never synced (Rule 3 applies to it in spirit: it is
derived coordination state, not a source).

A malicious relay can drop, reorder or withhold frames — availability was
never the property the seal buys, and between two people who share a kitchen
that is acceptable; what it cannot do is read or forge one. Protocol messages
carry a version byte so a future incompatible change is a clean refusal
rather than a silent misparse.

**Rejected.** **Per-device mailboxes** (pairwise deltas through the relay —
strictly more state and bookkeeping for zero benefit at two users and a
handful of devices). **A single replaceable sealed snapshot** (no log at all —
needs compare-and-swap the moment two devices are online together, and the
race it loses is exactly the live case that matters in the shop).
**Relay-side merging** (requires plaintext; the entire point is that hosting
never requires trust, 0009). **Sealing the version vector and letting devices
negotiate pairwise** (turns every sync into a round trip between devices that
are by hypothesis never online together).

---

## 0043 — The PWA's WebSocket lives in the frontend

**Date** 2026-08-09 · **Status** Accepted · **Implements**
[0042](#0042--the-relay-keeps-a-sequenced-log-it-cannot-read) · **Relates to**
[0011](#0011--no-background-sync-no-push-in-v1),
[0031](#0031--the-devices-identity-comes-from-the-host) · **Supersedes** the
`ws_stream_wasm` line of the M5 dependency plan

**Context.** The Rust half of M5 ended with a sans-IO client
(`cabas_sync::Session`): sealing, cursor discipline and the epoch reset as
plain calls on bytes, transport deliberately left to whoever owns an event
loop. The PWA needs that transport now, and the boundary doc's original plan
— `ws_stream_wasm` inside the crate — predates the sans-IO shape.

**Decision.** On the PWA, **the WebSocket is the frontend's**: the browser's
own API, owned by a small TypeScript engine next to `session.svelte.ts`. The
wasm binding exposes the session — hello, handle, delta, snapshot — with
bytes crossing as `Uint8Array`. **Plaintext never crosses the boundary**: a
frame that opens is merged inside the core, and what JS receives is the same
whole `StateView` every other mutation pushes (Rule 9's shape, unchanged).

The phrase, the relay URL, the cursor and the shadow version persist in
`localStorage` next to the identity (0031) — the device is the trust
boundary and it holds the full replica anyway. The relay URL defaults to the
app's own origin, because M6 serves the PWA and the sync socket from one
origin (0012); a Settings override exists for development, where the bundle
is served by `ui-serve` and the relay is a separate process.

**Consequences.** Reconnection, backoff and the foreground rule live where
`visibilitychange` and `pagehide` are already handled — 0011 is a
DOM-lifecycle policy and now sits in the file that owns the DOM lifecycle.
The wasm module stays free of timers and `spawn_local` machinery. The native
hosts (M7/M8) drive the same `Session` with `tokio-tungstenite` in Rust: two
thin adapters, one client, which is what 0042 built the session for.
`ws_stream_wasm` leaves the dependency plan.

**Rejected.** **The socket inside the wasm module** (`ws_stream_wasm` + a
`spawn_local` loop): duplicates scheduling and visibility handling the
frontend already owns, for no isolation gain — the UI renders the data, so
"plaintext hidden from JS" was never a property on this side of the relay.
**A worker-owned socket** (SharedWorker/ServiceWorker): background sync is
explicitly out (0011), and worker lifetimes on iOS are the exact time sink
that decision exists to avoid.

---

## 0044 — In development the sync socket goes through `ui-serve`

**Date** 2026-08-09 · **Status** Accepted · **Implements**
[0043](#0043--the-pwas-websocket-lives-in-the-frontend) · **Relates to**
[0041](#0041--the-phone-installs-from-a-local-certificate-authority),
[0012](#0012--cloudflare-tunnel-on-an-owned-domain)

**Context.** 0043 puts the WebSocket in the frontend, which subjects it to the
browser's rules about origins — and M5's exit criterion is two *real* devices,
so the page opening that socket is the installed PWA. An installed PWA needs a
secure context, a secure context on this LAN means `ui-serve`'s TLS (0041), and
**a page served over `https:` may not open a `ws:`** — the browser blocks it as
mixed content. The relay, meanwhile, listens in plaintext on 8787 and terminates
no TLS at all, deliberately: in production the Cloudflare Tunnel does that in
front of it (0012).

So the one configuration M5 has to close on is the one configuration that does
not work, and nothing earlier says so. `ui-test` drives the app over
`http://localhost:4173`, where `ws:` is same-scheme and passes; the desktop
browser is the half that cannot answer the question, exactly as in 0041 and
0040. The failure would first appear with a phone in hand.

**Decision.** `ui-serve` **proxies the WebSocket upgrade on `/sync`** to the
relay — `CABAS_RELAY`, default `127.0.0.1:8787` — by piping raw sockets. It
parses no frame and speaks no WebSocket: the handshake headers are forwarded
verbatim and the bytes after it are copied in both directions, so the sealed
payload passes through something that could not read it even if it were not
sealed (Rule 7), and the file keeps its zero dependencies.

Development therefore has **one origin, which is the shape production will
have**. The relay URL keeps defaulting to the app's own origin (0043) on both.

**Consequences.** The default path is the tested path: `wss://<origin>/sync`
is what runs in development, rather than being tried for the first time at M6.
The Settings override becomes what it should be — an escape hatch for pointing
a device at some other relay — instead of the ordinary way the app is
configured.

Nothing changes on the phone. The CA is installed and trusted once, and 0041
already records that installing a root and trusting it are two screens and that
the second is the one that gets missed; a second certificate would be a second
occasion to miss it.

The proxy is development-only and M6 removes the need for it, since the relay
will serve the bundle and the socket from one origin. It does not become
useless then: debugging sync on a phone against a *development* relay after M6
— M7's parity work, any change to the protocol — needs the same path, and the
alternative is developing against the family's live data.

The native hosts are untouched by all of this. M7 and M8 drive the same
`Session` over `tokio-tungstenite` in Rust (0043): no origin, no mixed-content
rule, nothing to proxy.

**Rejected.** **TLS in the relay** — code the production binary would never
execute, since the tunnel terminates in front of it, carried for the benefit of
one development machine. **A second certificate and a terminator in front of
the relay** — the CA already installed would sign it happily, but it makes
development a two-origin topology that production is not, which is precisely
the thing that leaves the single-origin default unexercised. **Testing sync
between two desktop browsers over plain HTTP and calling M5 closed** — that
runs in `ui-test` already and says nothing about the installed app on iOS,
which is the artifact the milestone is about. **Bringing M6's tunnel forward**
— rejected once in 0041 for the same reason and it has not changed: it means
shipping the deployment milestone in order to test the previous one.

---

## 0045 — A cursor is not resumed on a replica that never had it

**Date** 2026-08-09 · **Status** Accepted · **Implements**
[0042](#0042--the-relay-keeps-a-sequenced-log-it-cannot-read) · **Relates to**
[0043](#0043--the-pwas-websocket-lives-in-the-frontend),
[0031](#0031--the-devices-identity-comes-from-the-host)

**Context.** 0043 persists the sync cursor in `localStorage`, next to the
identity; the replica it describes lives in IndexedDB. Two stores, two
lifetimes — and nothing until now said what happens when only one of them
survives.

What happens is the worst available outcome. The cursor says "I already have
everything up to frame N", the relay believes it and honestly replays nothing,
and the device sits with an empty library. There is no error, no retry and no
recovery: the relay is not wrong, and it will stay that way until somebody else
happens to push something, which on a two-person family can be days. The whole
point of the log is that a device which was away gets what it missed (0042),
and this is the one state where it silently does not.

It surfaced in `ui-test`, which deletes the replica and reloads — and got an
empty screen where the library should have come back.

**Decision.** The core answers the exact question: `App::opened_fresh()` is
true when `open` found **no stored snapshot**, so the document was built from
nothing this launch. The engine starts from a zero cursor and an empty shadow
whenever it is true, and pairing resets both for the same reason — a different
family is a different log.

The cost of being wrong in this direction is one replay of a log that is
bounded by design, applied to a replica where merges are idempotent. The cost
of being wrong in the other direction is a library that never comes back.

**Consequences.** The invariant is stated where it can be enforced rather than
assumed: the host no longer has to reason about which browser storage outlives
which. It also covers the case nobody would have written a test for — an
iOS eviction that takes IndexedDB and leaves `localStorage`, which is not
documented behaviour and is not something to find out about on a phone.

**Rejected.** **Moving the cursor into IndexedDB, beside the snapshot** — the
structurally correct answer, since the two would then be lost together by
construction. `Storage` is deliberately one blob in and one blob out, and
growing it a second slot for this is a change to the trait every backend
implements. Worth revisiting if device-local sync state ever grows past two
numbers and a version vector. **Deciding it from the `StateView`** — an empty
library and a lost one look identical from there, so a genuinely new family
would replay on every launch, and "looks empty" is not "never received"
anyway. **Relying on the two being evicted together** — true in the common
case, undocumented, and the failure it leaves is silent and permanent.

---

## 0046 — A `u64` crosses the wasm boundary as text

**Date** 2026-08-09 · **Status** Accepted · **Relates to**
[0029](#0029--how-the-document-encodes-domain-values),
[0042](#0042--the-relay-keeps-a-sequenced-log-it-cannot-read),
[0043](#0043--the-pwas-websocket-lives-in-the-frontend)

**Context.** The relay mints a log's epoch from 64 bits of the OS's randomness
(0042), so it is above 2^53 nearly always — outside what a JavaScript number
holds exactly. `serde_wasm_bindgen` refuses to serialise such a value rather
than round it, which is the right call and arrives as a thrown error from the
first call that reads a real cursor.

That error was invisible until the PWA met an actual relay: every test up to
then had used an epoch of 0, which fits in a double and proves nothing.

**Decision.** A `u64` that is an **identity** crosses as decimal text —
`#[serde(with = "text_u64")]` and `#[ts(type = "string")]` on
`SyncCursor::epoch`. A `u64` that is a **count** stays a number: the relay
hands out sequence numbers one per frame, so `since` cannot approach the point
where a double stops being exact.

This is the same answer `store` gives an exact rational (0029) for the same
reason — the receiving format has no exact type for the value, so the value
travels as text and the host treats it as opaque. It stores it and hands it
back; it never does arithmetic on it.

**Consequences.** The rule generalises: anything minted as 64 random bits is
text at this boundary, anything counted is a number, and the type says which.
The browser test now uses a real-sized epoch, because the test that used zero
was the reason the bug shipped as far as it did.

**Rejected.** **BigInt** (`serialize_large_number_types_as_bigints`) — the
cursor's entire job is to be persisted, and `JSON.stringify` throws on a
BigInt; the fix would be a custom replacer on every write of a value that is
never computed with. **Truncating the relay's epoch to 53 bits** — changing a
server's data because of a client's number format, and the epoch is a
`postcard` wire field shared with the native hosts, which have no such limit.
**Two 32-bit halves** — one number pretending to be two, reassembled by hand in
every host that touches it.

---

## 0047 — The QR is shown, never scanned, and the encoder is ours

**Date** 2026-08-09 · **Status** Accepted · **Refines**
[0021](#0021--pairing-by-qr-with-a-mandatory-12-word-fallback) · **Relates to**
[0042](#0042--the-relay-keeps-a-sequenced-log-it-cannot-read),
[0038](#0038--the-service-worker-is-written-by-hand)

**Context.** 0021 settled pairing — a QR code, **always** with the twelve words
as a manual fallback — on the reasoning that the camera is historically brittle
in an installed iOS PWA. Building it showed that the two halves of that
sentence cost nothing alike.

Showing a QR is an encoder: a page of arithmetic over a finite field, no
permissions, no hardware. Reading one is a camera prompt, a video element, a
decode loop, and a decoder — iOS Safari has no `BarcodeDetector`, so that is
another dependency in the bundle — plus a designed answer for every way a
person can refuse or a camera can fail. And the thing all of it produces is a
string that the fallback already accepts by hand.

**Decision.** Pairing **displays** the phrase and its QR; the joining device
**types or pastes** the words. There is no scanner in the app.

The encoder is written here (`ui/src/lib/qr.ts`), fixed to **version 6, level
L, byte mode** — the smallest symbol that holds the longest phrase BIP39 can
produce, and the largest one that needs no version-information block. Fixing
the version is what makes a hand-written encoder safe: the format is a wall of
per-version tables, and one wrong row is a picture that renders and does not
scan. One version is one row, and `ui-test` compares every module against
`qrencode`, an implementation that shares none of this one's assumptions.

**Consequences.** Pairing needs no permission of any kind, works the same on
every device, and cannot regress with an iOS release — which is precisely what
0021 was worried about. The path that 0021 called the fallback is now the only
path, so it is the one that gets exercised every time rather than the one
nobody notices is broken.

The QR keeps its point: twelve words read off a screen and retyped is where
transcription errors come from, and a phone camera pointed at the code shows
them as text to copy. Adding a scanner later changes nothing here — it would be
a second input method feeding the same `readPhrase`, and this entry is what it
would supersede.

The encoder is ~330 lines and no new dependency, which is the same trade as the
service worker (0038) and for the same reason: a page and a half of arithmetic
against a supply chain, in a bundle that is already the whole product.

**Rejected.** **An in-app scanner now** — the expensive half, the fragile half
on the mandatory platform, and the only feature in the app that would ask for a
device permission, all to save typing twelve words once per device.
**A dependency for the encoder** — reasonable, and it would have cost a
lockfile entry, a transitive tree to audit, and a version to keep honest,
against arithmetic that is fully specified and now pinned by a test.
**Putting the phrase in a URL for the OS camera to open** — it would make
scanning work today with no decoder at all, and it would write the family key
into browser history, the camera app's log, and whatever the tap passes it
through. The phrase is the key (0042); it goes on a screen, not in a URL.

---

## 0048 — The bundle is compiled into the relay, by a build script

**Date** 2026-08-09 · **Status** Accepted · **Implements**
[0010](#0010--the-relay-ships-as-a-home-assistant-os-add-on) · **Relates to**
[0012](#0012--cloudflare-tunnel-on-an-owned-domain),
[0038](#0038--the-service-worker-is-written-by-hand),
[0044](#0044--in-development-the-sync-socket-goes-through-ui-serve)

**Context.** 0010 said the relay serves the PWA as well as brokering sync, and
0012 said why it has to be the same host: an installed PWA *is* its origin, so
the app and the socket it talks to cannot live at two addresses without one of
them being the app's identity and the other a permanent CORS problem. M6 is
where that stops being a sentence and becomes an image on a Raspberry Pi.

Two ways to put the bundle in the image. Copy `ui/dist` into the container next
to the binary and read it from disk, or compile it in. The first needs a path
that is right in the container and in a `cargo run` on a laptop, and it makes
the add-on two artifacts that can disagree about which build they are. The
second makes `cabas-relay` one file that either works or does not.

`rust-embed` is the usual way to do the second, and the workspace registry
planned for it since M0. It does not fit: its derive macro reads the folder at
compile time, and `ui/dist` is a gitignored build product. A fresh checkout —
CI's own `gates` job included — would fail `cargo clippy --workspace` until
someone had run `pnpm build`, which makes the Rust gates depend on the
frontend's toolchain for no reason any of them can see.

**Decision.** `crates/relay/build.rs` walks `ui/dist` and writes a table of
`include_bytes!` into `OUT_DIR`; `assets.rs` includes it and serves it. No
dependency.

**A missing bundle is an empty table, not an error.** `cargo clippy
--workspace` works in a fresh checkout, and the binary it produces is a sync
broker that serves no app — which is exactly what development wants, because
development serves the app from `ui-serve` over TLS (0041) and proxies `/sync`
to a relay in a terminal (0044). The process says how many files it embedded on
its first line, because "the app does not load" and "this build has no app in
it" are one symptom from a phone.

The release image passes `CABAS_EMBED_UI=required`, and then a missing
`index.html` fails the build. An image with no app in it is the one case where
quiet is unacceptable, and it is invisible until a phone asks for the page.

Three serving rules come with it, and each is a bug already paid for elsewhere
in this repo:

- **`assets/*` is immutable for a year; everything else is `no-cache`.** Vite
  content-hashes what it compiles. `index.html`, `sw.js`, the manifest and the
  icons have stable names — and `sw.js` above all, since the browser decides
  there is a new build by fetching that one file and comparing its bytes
  (0038). A cached service worker is an app that can never update.
- **No `Vary`, ever.** `Vary: Origin` makes the Cache API match on the
  request's `Origin` header; the worker precaches with requests that carry
  none, and the page asks for its `crossorigin` JS and CSS with one. Every
  asset cached, every lookup a miss — invisible online, a blank page offline.
  The worker already defends itself with `ignoreVary`; the server it was
  written against should not need it to.
- **An unknown path is a 404, not the page.** Which screen is open is core
  state and never a URL (0037), so there is no route to fall back for, and
  answering `index.html` to a mistyped asset name turns a missing file into a
  page that loads and does nothing.

**Consequences.** `ui-test` now runs against the relay instead of `pnpm
preview`, on one origin, which is the production topology and one process
fewer. That makes the end-to-end suite the proof that the shipped artifact
serves an app at all — previously nothing would have noticed an empty bundle
until the Pi was flashed. It also removes the preview server's `Vary: Origin`,
which was the reason the worker needed `ignoreVary` in the first place; the
worker keeps it, because a tunnel or a future proxy may put it back.

Rebuilding the relay after `pnpm build` is not optional, and is not a step
anyone has to remember: `build.rs` declares `rerun-if-changed` on `ui/dist`, so
Cargo rebuilds the binary when the bundle changes. The cost is that a frontend
change relinks the relay, which is a second on a laptop.

The binary carries ~2.9 MB of bundle, source maps included. Excluding the maps
would save 0.9 MB and invent a second definition of "the bundle"; on a
Raspberry Pi it buys nothing, and it costs a debuggable production app.

**Rejected.** **`rust-embed`** — the compile-time folder requirement above,
paid for with a dependency, to generate the same `include_bytes!` table.
**`tower-http`'s `ServeDir`** — reads from disk, which is the two-artifact
problem, and the caching policy would still have been written by hand.
**Serving the bundle from Cloudflare and only `/sync` from the Pi** — two
origins, and 0012 says why that is the one thing that cannot be changed later.
**Precompressing with gzip or brotli at build time** — a compression
dependency for a LAN, when the tunnel already compresses everything that
crosses the internet; worth revisiting if the wasm ever ships uncompressed to
4G and feels it.

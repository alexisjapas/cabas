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
| [0049](#0049--the-add-on-is-cross-compiled-here-and-never-built-on-the-pi) | The add-on is cross-compiled here, and never built on the Pi | Deployment |
| [0050](#0050--an-abandoned-family-log-is-forgotten-by-hand-or-not-at-all) | An abandoned family log is forgotten by hand, or not at all | Sync |
| [0051](#0051--the-relay-pings-because-the-proxy-closes-a-silent-socket) | The relay pings, because the proxy closes a silent socket | Sync |
| [0052](#0052--the-edge-must-not-re-ttl-the-service-worker) | The edge must not re-TTL the service worker | Deployment |
| [0053](#0053--a-cursor-must-point-inside-the-log-not-merely-at-its-epoch) | A cursor must point inside the log, not merely at its epoch | Sync |
| [0054](#0054--a-reset-cursor-voids-the-shadow-and-the-answer-is-a-whole-replica) | A reset cursor voids the shadow, and the answer is a whole replica | Sync |
| [0055](#0055--the-running-build-says-its-own-name-in-settings) | The running build says its own name, in Settings | Tooling |
| [0056](#0056--an-ingredient-is-created-where-it-is-needed-not-in-another-tab) | An ingredient is created where it is needed, not in another tab | Product |
| [0057](#0057--items-an-aisle-for-what-is-bought-whole-and-never-cooked) | Items: an aisle for what is bought whole and never cooked | Domain |
| [0058](#0058--anything-chosen-out-of-a-library-is-searched-for) | Anything chosen out of a library is searched for | Product |
| [0059](#0059--the-list-shows-what-is-missing-and-a-recipe-joins-it-from-there) | The list shows what is missing, and a recipe joins it from there | Product |
| [0060](#0060--what-was-searched-for-is-what-gets-created) | What was searched for is what gets created | Product |
| [0061](#0061--purchases-are-recorded-statistics-are-derived-from-them) | Purchases are recorded; statistics are derived from them | Product |
| [0062](#0062--a-photo-is-a-blob-beside-the-document-never-in-it) | A photo is a blob beside the document, never in it | Storage |
| [0063](#0063--a-family-is-called-a-group) | A family is called a group | Product |
| [0064](#0064--the-tabs-run-from-the-shelves-to-the-trip) | The tabs run from the shelves to the trip | Product |
| [0065](#0065--a-photo-can-be-chosen-as-well-as-taken) | A photo can be chosen as well as taken | Product |
| [0066](#0066--an-ingredient-knows-how-much-of-it-one-buys) | An ingredient knows how much of it one buys | Domain |
| [0067](#0067--a-row-goes-on-the-list-by-being-pushed-there) | A row goes on the list by being pushed there | Product |
| [0068](#0068--a-device-joins-a-group-and-then-says-who-is-carrying-it) | A device joins a group, and then says who is carrying it | Product |
| [0069](#0069--the-aisles-are-this-groups-shop-not-a-supermarkets) | The aisles are this group's shop, not a supermarket's | Domain |
| [0070](#0070--an-ingredient-says-where-it-is-kept) | An ingredient says where it is kept | Domain |
| [0071](#0071--a-shop-is-a-name-and-the-cart-is-one-trip-per-shop) | A shop is a name, and the cart is one trip per shop | Product |
| [0072](#0072--the-gesture-keeps-counting-and-holding-a-row-types-the-amount) | The gesture keeps counting, and holding a row types the amount | Product |
| [0073](#0073--an-ingredients-editor-opens-under-the-ingredient) | An ingredient's editor opens under the ingredient | Product |
| [0074](#0074--a-tab-opens-cold) | A tab opens cold | Product |
| [0075](#0075--main-may-not-advertise-a-version-nothing-published) | `main` may not advertise a version nothing published | Deployment |
| [0076](#0076--the-library-travels-as-a-json-file-of-the-apps-own-inputs) | The library travels as a JSON file of the app's own inputs | Product |
| [0077](#0077--the-list-is-where-an-amount-is-changed-too) | The list is where an amount is changed, too | Product |
| [0078](#0078--no-field-is-small-enough-for-ios-to-zoom-at) | No field is small enough for iOS to zoom at | Platform |

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

---

## 0049 — The add-on is cross-compiled here, and never built on the Pi

**Date** 2026-08-09 · **Status** Accepted · **Implements**
[0010](#0010--the-relay-ships-as-a-home-assistant-os-add-on) · **Builds on**
[0048](#0048--the-bundle-is-compiled-into-the-relay-by-a-build-script) ·
**Relates to** [0012](#0012--cloudflare-tunnel-on-an-owned-domain)

**Context.** 0010 settled the shape — a Home Assistant add-on, image built by
CI and pulled by the Supervisor, never compiled on the Raspberry Pi. It left
four things open, and each has one answer that is much better than the others.

**Where the add-on repository lives.** In this repository. The Supervisor reads
`repository.yaml` at a repository's root and then looks for `config.yaml` in
each *top-level* directory; it does not recurse. So `cabas-relay/` sits beside
`crates/` and `ui/` rather than under an `addon/` that would have read more
tidily. The alternative was a second repository, which buys a clean root and
costs a version number kept in step by hand across two places — for a project
where the image and the code are cut from the same commit, that is the wrong
trade.

**How the binary is built.** Cross-compiled on the runner to
`aarch64-unknown-linux-musl`, statically linked, then copied into the image.
Not built inside the Dockerfile under qemu, where compiling this dependency
tree is tens of minutes and occasionally nothing at all.

It needs no C toolchain: every crate in the tree is pure Rust, so `rust-lld`
links musl on its own. That is why the two targets are three lines in
`rust-toolchain.toml` rather than a third devShell — a `rust-std` download,
not a cross-gcc.

The Dockerfile therefore **copies and never runs**. That is load-bearing:
with no `RUN`, `docker buildx --platform linux/arm64` needs no emulation at
all on an x86 runner, so CI installs no qemu. The day someone adds one, the
build fails loudly rather than quietly taking twenty minutes.

**What the image is based on.** `ghcr.io/home-assistant/{arch}-base`, pinned to
Alpine 3.21. The binary is static and would run `FROM scratch` in about 4 MB —
but the base carries s6, which is what puts the relay's log in the Home
Assistant UI and makes a stop a clean one, and it carries a shell, which on an
appliance is the only way to ever look inside. Ten megabytes for the ability to
debug the thing at all.

**What configures it.** Nothing. The relay takes a data directory and a listen
address; inside an add-on the only correct answers are `/data` — the volume
Home Assistant backs up, which is what makes this the recovery point if every
phone is lost — and `0.0.0.0:8787`, the port `config.yaml` maps. Both are
already the defaults, so `options` and `schema` are empty. An add-on with
nothing to fill in has nothing to fill in wrong, and no schema to keep in step
with a struct.

**Consequences.** Publishing has three rules, and they exist because an image
tag is what the Supervisor pulls, so it *is* the release (Rule 12):

- a pull request builds both architectures and publishes nothing;
- a push to `main` publishes, but only a `-dev` version — otherwise the next
  push would silently overwrite the image a release had named;
- an annotated `vX.Y.Z` tag publishes `X.Y.Z` and moves `latest`, and the tag
  must match `config.yaml`'s version or the run fails.

So `0.1.0-dev` is published on every push to `main` and the add-on is
installable from this repository today, before there is any release to cut.

`check-addon` gates the two disagreements that only surface on an appliance:
`config.yaml`'s version against the workspace's, and an architecture offered in
`config.yaml` with no base image in `build.yaml` — an add-on that appears in
the store, installs, and fails to pull.

**amd64 is built alongside aarch64** so the add-on can be installed on a
laptop. Only the Pi has to work; the laptop is where M6's restore drill gets
rehearsed before it is attempted on the machine that holds the only copy.

The one thing none of this proves is the image itself: there is no container
runtime in the devShell, so the Dockerfile is first executed on a runner.
`build-relay` and `check-addon` cover everything up to it.

**Rejected.** **Building the image on the Pi** — 0010's original point, and
still right. **`home-assistant/builder`** — it wraps buildx to do the
cross-compilation dance we no longer need, and adds implicit behaviour over a
Dockerfile that is four lines. **`FROM scratch`** — smaller and elegant, and it
takes away the shell on the one machine where a shell is hardest to get.
**A single multi-arch manifest without `{arch}`** — tidier, and off the path
every add-on document and tool assumes. **An options schema for the log level**
— the first configurable thing is what makes the second one seem reasonable;
Rule 14 says it starts with an entry here.

---

## 0050 — An abandoned family log is forgotten by hand, or not at all

**Date** 2026-08-09 · **Status** Accepted · **Relates to**
[0024](#0024--attribution-is-declarative-not-cryptographic),
[0042](#0042--the-relay-keeps-a-sequenced-log-it-cannot-read),
[0010](#0010--the-relay-ships-as-a-home-assistant-os-add-on)

**Context.** Rotating the family phrase is the whole of revocation (0024):
every device moves to a new family id, and the old log stays on the relay —
sealed, complete, and addressed by an id nobody will ever send again. Nothing
collects it. The ROADMAP has carried this since M5 as an item that had to
become a decision rather than remain an oversight.

The temptation is a sweep: delete a family that has received nothing in *N*
days. It cannot be done, and the reason is the same property that makes the
relay worth trusting. **The relay cannot tell an abandoned family from a quiet
one.** It holds no key, no roster and no calendar of anyone's life. A family
that rotated last spring and a family whose two phones spent the summer
somewhere else are the same directory with an old timestamp. And the log is the
recovery point if every device is lost (0010), so a sweep that guesses wrong
eats the only remaining copy of a family's library. There is no *N* that is
safe, because the quantity being estimated is not on this machine.

**Decision.** No expiry, no sweep, nothing automatic. The relay gains two
subcommands for the person who *does* know which family they abandoned:

- `cabas-relay families` — every family on disk: id, frames handed out over its
  whole life, bytes, and how long since it last received anything. Read-only.
- `cabas-relay forget <id>` — deletes one, named in full.

Ages rather than dates, because the question being answered is "which of these
stopped when I rotated", and "97 days" answers it without arithmetic or a date
library. The stalest is listed first, which puts the candidates at the top.

`forget` takes a whole id and never a prefix, an age or a pattern. The point of
this entry is that the machine cannot judge which of these is finished; it does
not then get to guess at one either. A malformed id and an absent one are
different answers, so a typo cannot read as "already gone".

**Surveying must not open the logs.** `FamilyLog::open` mints an epoch for a
family that has none and rewrites `meta`; doing that while merely counting
would cost every one of that family's devices a full replay (0042). `survey`
reads `meta` and stats the files instead — and it reads the *log* file's
timestamp, not `meta`'s, because `meta` is rewritten on open and would report
when the relay last restarted.

**It is not an HTTP endpoint, and that is the security half.** A family id is
the whole of the relay's access control: `log`'s comment on `open` is that a
stranger cannot mine directories into existence because the ids are
unguessable. A listing served on the port that faces the Cloudflare Tunnel
would hand out precisely the thing that is supposed to be unguessable. These
run for whoever already has a shell on the machine, and for nobody else.

**Consequences.** Running `forget` while the add-on is serving is fine: the
family being forgotten is by definition one no device connects to any more —
that is what abandoned means — so nothing holds it open. Forgetting a *live*
family instead leaves its connections answering "storage failed" until the
process restarts, which is the loud kind of wrong rather than the quiet kind.

Rotating therefore has an operational tail, and it is named in both places it
has to be: the fourth consequence on the phone's rotation screen, before
anything happens, and the procedure in `cabas-relay/DOCS.md` for the person at
the machine. Not doing it costs a directory the size of one family's library,
which is why this is hygiene and not urgency.

And it is worth being plain about what it is *not*. The old log holds nothing
that the holder of the old phrase does not already have on the device that was
lost — that device had the library. Rotation stops the future, not the past;
deleting the old log leaves nothing behind, and rescues nothing. Saying so is
better than implying that revocation cleans up after itself.

**Rejected.** **A TTL on the relay** — the estimate it needs is not on this
machine. **Deleting the old log at rotation, from the device** — the rotating
device knows the old id and could ask; but then any device that ever held a
phrase can order a log destroyed, which makes a lost phone able to erase the
family it was stolen from. The one operation that must not be remote is the
irreversible one. **A retention option in `config.yaml`** — a number the
operator would have to guess, dressed as configuration (0049 rejected
configurability for the same reason). **Listing families over HTTP** — see
above; it publishes the only secret the relay has.

---

## 0051 — The relay pings, because the proxy closes a silent socket

**Date** 2026-08-12 · **Status** Accepted · **Relates to**
[0012](#0012--cloudflare-tunnel-on-an-owned-domain),
[0011](#0011--no-background-sync-no-push-in-v1),
[0042](#0042--the-relay-keeps-a-sequenced-log-it-cannot-read),
[0043](#0043--the-pwas-websocket-lives-in-the-frontend)

**Context.** Every socket this project has ever opened went to a relay on the
same machine or the same wifi, where a connection with nothing to say costs
nothing and lasts forever. The Cloudflare Tunnel (0012) puts a proxy in the
middle, and Cloudflare's own documentation says it "will close a WebSocket
connection when no data is transmitted in either direction for a period of
time" — without publishing the period, and recommending a keepalive.

Neither end sent one. The relay treated ping and pong as "not ours" and
carried on; the frontend had a backoff on `onclose` and nothing else. So the
failure is not that sync breaks — it does not. A closed socket is a
reconnection, the cursor replays, and Rule 6 already means no user action was
waiting on it. The failure is that it happens **every few minutes while the
app sits open in a shop**, that the status on the Settings screen flickers
between connected and reconnecting while it does, and that none of it can be
observed anywhere except behind the tunnel. It would have been discovered by
somebody standing in a supermarket, and attributed to the supermarket's wifi.

**Decision.** The relay sends a WebSocket ping every 30 seconds on every
connection past its `Hello`.

**It is the relay that pings, and that is the whole economy of this entry.** A
browser's `WebSocket` cannot send a ping — the API has no method for it — so a
client-side keepalive would have to be an application message, which means a
new `ClientMessage` variant, which means the wire contract of 0042 gains a
member and every future reader has to ask whether a heartbeat is part of the
protocol. It is not. It is a property of the pipe. A server-sent ping is
answered by the browser inside its own socket implementation, so the page
never learns it happened: no protocol version, no frontend code, no view-model
touched.

Thirty seconds is chosen against a number nobody publishes, which is
uncomfortable and cheap to over-serve: it is well under every proxy timeout
anyone documents, and the cost is two bytes a minute on a socket that only
exists while the app is on screen at all (0011). A tick is unconditional
rather than reset by traffic — activity tracking would be a second clock to
keep honest, and the saving is one frame per thirty seconds on a busy socket.

**No pong is waited for.** This is a keepalive, not a liveness check. Timing a
pong out would make the relay the judge of whether a phone is still there, and
the answer it would act on — close the socket — is one the failing `send`
already produces on its own, without a policy or a second timer. A half-open
connection costs one entry in a broadcast channel until the next write finds
it.

**Consequences.** `crates/relay` gains tokio's `time` feature and a field on
`Relay` that only the test sets, so a case that would otherwise take thirty
seconds runs in fifty milliseconds. The test asserts the thing that is easy to
get wrong by accident: a ping arrives on a connection that has said hello and
then *stopped talking*. A relay that only pinged in response to traffic would
pass every other test in this repository and drop the socket in a shop.

**Rejected.** **A `Ping`/`Pong` pair in the protocol** — a wire change for a
transport concern, and 0042's contract is worth keeping small; the browser
cannot send a control ping anyway, so this buys nothing it does not also cost.
**Client-side reconnect-on-a-timer** — treating the symptom, and it makes the
status flicker deliberate rather than accidental. **Nothing at all** — the
reconnection genuinely does recover, and this was tempting; what it fails is
observability. A permanent low rate of disconnection is a thing every future
network bug gets blamed on. **A configurable period** — a number the operator
would have to guess about somebody else's proxy (0049 and 0050 rejected
configurability for the same reason).

---

## 0052 — The edge must not re-TTL the service worker

**Date** 2026-08-12 · **Status** Accepted · **Relates to**
[0012](#0012--cloudflare-tunnel-on-an-owned-domain),
[0038](#0038--the-service-worker-is-written-by-hand),
[0048](#0048--the-bundle-is-compiled-into-the-relay-by-a-build-script)

**Context.** The relay serves `/sw.js` with `Cache-Control: no-cache` — revalidate
every time — because that file is the entire update mechanism of 0038: a new
build is noticed when the browser fetches the worker and finds it changed.
`assets.rs` has always sent that header and a test asserts what it serves.

Behind the tunnel, it stopped being what arrives. Cloudflare applies a default
four-hour *browser* cache TTL to any cacheable response whose origin sets no
explicit `max-age`, and `no-cache` sets none — so the header reaching the phone
became `max-age=14400`. The hashed files under `/assets/` were untouched,
because they carry their own year-long `max-age`; `/sw.js` was the only
casualty, and it was the worst possible one.

The edge itself behaved: `cf-cache-status` was `REVALIDATED`, so nothing stale
was ever served *from* Cloudflare. What changed was the instruction given to
the browser.

**Decision.** A cache rule on the zone bypasses cache for `/sw.js`, scoped to
the app's hostname:

```
(http.host eq "cabas.cladelabs.com" and http.request.uri.path eq "/sw.js")
```

with cache eligibility set to *Bypass cache*. Verified after deploying: the
header arrives as `no-cache` again, `cf-cache-status` is `DYNAMIC`, and
`/assets/*` still carries `immutable` and still reports `HIT`.

**Why not rely on the browser.** A registration's `updateViaCache` defaults to
`"imports"`, which means the worker script itself already bypasses the HTTP
cache during an update check — so this may well have been harmless. That is
exactly the reason not to leave it: it would have made the update path depend
on a default this project never chose, never wrote down and cannot test, in the
one mechanism whose failure mode is a phone that quietly never updates again.
A rule that costs nothing is cheaper than a property nobody can see.

**Consequences.** There is now a piece of load-bearing configuration that lives
in a dashboard and not in this repository, which no test can reach and no
`nix develop` can check. It is written down in three places for that reason —
here, in the README's tunnel section, and in `cabas-relay/DOCS.md` for whoever
sets this up on another machine. Anyone rebuilding the tunnel from scratch has
to recreate it, and the symptom of forgetting is not an error: it is an app
that updates a few hours later than it should, or on a phone left open, not at
all.

The rule is deliberately narrow. Widening it to `/assets/*` would throw away
the caching that makes a cold start over 4G worth having, and those files are
content-hashed, so they are the one thing that is safe to cache forever.

**Rejected.** **Setting an explicit `max-age=0, must-revalidate` at the
origin** — it would survive the proxy, but it encodes one CDN's defaults into
the relay's own headers, and the next proxy will have different ones; the
origin already states its intent correctly. **Turning off the zone's browser
TTL globally** — it would fix `/sw.js` by making every other file worse.
**Relying on `updateViaCache`** — see above.

---

## 0053 — A cursor must point inside the log, not merely at its epoch

**Date** 2026-08-12 · **Status** Accepted · **Implements**
[0042](#0042--the-relay-keeps-a-sequenced-log-it-cannot-read) · **Relates to**
[0045](#0045--a-cursor-is-not-resumed-on-a-replica-that-never-had-it),
[0009](#0009--zero-knowledge-relay-with-app-layer-e2ee)

**Context.** 0042 gave the log an epoch so a device can tell that the history
its cursor points into still exists. The relay honoured a cursor whenever the
epoch matched and replayed from zero when it did not — which covers a log that
was deleted and remade, because that mints a new epoch.

It does not cover the case M6 is built around. `/data` is the recovery point
precisely because Home Assistant backs it up, and a backup carries `meta` —
so restoring one brings the epoch back *identical*, alongside a log that stops
wherever the backup was taken. Every device still holds a cursor from further
along. The epoch matches, the relay believes the cursor, and "every frame after
412" from a log whose highest frame is 300 is nothing at all.

Nothing reports this. Both devices are online, the relay is not wrong, and the
one thing that would resolve it — a new epoch — is exactly what the restore
undid. New pushes are numbered from 301 again, so they stay below what the
devices claim to have seen, and the family silently stops converging for as
many pushes as the restore rolled back. On two people's shopping that is weeks.

**Decision.** The cursor is honoured only when it names this log's epoch **and**
points inside it — `hello_since < log.next_seq()`. Otherwise it describes
frames that were never handed out, and the connection replays from zero.

**Consequences.** The failure mode after a restore becomes one full replay per
device, which is bounded by design and lands on a replica where merges are
idempotent — the same trade 0045 made for the same reason, and in the same
direction: the cost of being wrong here is a replay, the cost of being wrong
there is a family that never converges again.

It also covers a case nobody would have staged: a `/data` rolled back by a
filesystem snapshot, a half-restored volume, or an add-on reinstalled over an
older backup than the one the phones knew. None of those change the epoch
either.

Found by writing the restore drill rather than by running it — the drill's
central scenario, read against `server.rs`, could not work. `crates/relay/
tests/convergence.rs` now stages it: two devices, a backup, four more pushes, a
restore, and a fifth push that the second device has to receive.

**Rejected.** **Minting a new epoch when the log looks restored** — the relay
cannot tell a restore from a cold start, and guessing wrong rewrites the
identity of a healthy log, costing every device a full replay for nothing.
**Having the operator bump the epoch by hand after a restore** — it makes
correctness depend on remembering a step during the one procedure that is only
ever run under stress. **Recomputing `next_seq` from the frames alone** — it is
already the maximum of the two (`log.rs`), and it is the *device's* cursor that
is ahead here, not the log's own bookkeeping.

## 0054 — A reset cursor voids the shadow, and the answer is a whole replica

**Date** 2026-08-12 · **Status** Accepted · **Implements**
[0042](#0042--the-relay-keeps-a-sequenced-log-it-cannot-read) · **Completes**
[0053](#0053--a-cursor-must-point-inside-the-log-not-merely-at-its-epoch)

**Context.** 0053 made the relay replay from zero when a device's cursor points
past the end of the log, which is what a restored backup produces. That fixes
the direction the relay controls — what it sends. It leaves the other one
untouched.

A device keeps a *shadow*: the version its last acked push reached, meaning
"the relay holds everything up to here". A restore falsifies it, and nothing
the shadow can observe has changed — the epoch is inside `/data`, so it comes
back identical, and 0053's own remedy is invisible to the sender because the
relay never says it refused the cursor. So the device measures its next delta
from a version the log no longer contains.

That delta is causally dangling. A device that missed the rolled-back window
cannot apply it — its dependencies name operations nobody will send again,
since the only device holding them believes they were delivered. The frame is
accepted, the cursor advances, nothing merges, and every later delta is rooted
in the same missing operations. Measured on a real relay: a second device sat
at one ingredient through six pushes, cursor climbing, no error on either side,
converging never again. The rolled-back window is also gone from the log, which
is the recovery point that is supposed to outlive every device.

**Decision.** A session tracks whether the log it is being served holds what
its cursor claimed. Two things say it does not: an epoch that differs, and — the
restored case — a frame whose sequence number is at or below the `since` the
hello asked for, since the relay only ever replays *after* that number. Either
sets `Session::reset`.

A reset session pushes the **whole replica** rather than a delta, ignoring the
shadow, and it owes that push even when nothing changed locally. `SyncSession::
push` decides this in `crates/app`, so both hosts inherit it — the PWA today,
Tauri at M7 — and the frame is the snapshot kind, which the relay is already
allowed to truncate to (0042). The ack discharges the flag: the log holds the
replica whole again, so the shadow describes it truthfully.

**Consequences.** One full document upload per device per restore, against a
family that otherwise stops converging silently and permanently. It is the same
trade as 0045 and 0053, in the third direction: replay is bounded, divergence
is not.

The relay stays unable to detect any of this — it holds ciphertext and cannot
know a delta is dangling — which is why the fix is entirely client-side and
needs no protocol change. Detection by inference rather than by a new field in
`Welcome` keeps the wire format frozen: a phone still running the old bundle
talks to a new relay, and the reverse, exactly as before.

One residue, accepted: a backup taken before the log's very first frame gives a
restore with nothing to replay, and a device with no frame to notice. Its next
push is then a delta the log cannot root. It needs a restore to a point older
than any frame the family ever pushed, which is a window of minutes on the day
the relay is installed.

**Rejected.** **Announcing the replay position in `Welcome`** — the honest
version, and a wire break for a case inference already covers; frozen formats
are worth more than tidy ones here (0042). **Resetting the shadow in each host**
— two hosts, one rule, and the second one arrives at M7 with nobody to notice it
was forgotten. **Always pushing a snapshot** — it turns every connection into a
document upload to spare one per restore.

## 0055 — The running build says its own name, in Settings

**Date** 2026-08-12 · **Status** Accepted · **Relates to**
[0038](#0038--a-hand-written-service-worker-cache-first-over-a-precached-shell),
[0048](#0048--the-bundle-is-compiled-into-the-relay-by-a-build-script)

**Context.** A service worker installs a new build behind the running one and
takes over at the next launch (0038). That is the right behaviour and it makes
one question unanswerable from the device: *is this phone running what the
relay serves?* Until now the answer was to close the app, reopen it, and
believe it — during the restore drill, on the one procedure where being wrong
about the build under test invalidates the result.

Nothing else could answer it either. The relay logs no version, `cabas-relay`
has no `--version`, and two consecutive releases can serve a byte-identical
bundle, so nothing observable from outside distinguishes them. The Supervisor's
add-on page knows, which is a different machine from the one holding the phone.

**Decision.** Settings names the build: `CabasApp.buildVersion()`, from
`CARGO_PKG_VERSION` in the core, rendered at the bottom of the screen with a
line saying an update applies at the next opening.

It comes from the **core**, not from a constant in the frontend or from
`package.json`, because the core is the artifact: the bundle is compiled into
the relay binary (0048), so the version a phone displays is the version of the
relay build that served it, and a wasm module that disagreed with the bundle
around it is exactly what this line would reveal. It is the same string the
Supervisor compares to offer an update and the same one `check-addon` holds the
add-on manifest to (Rule 15) — one version, one meaning, everywhere.

**Consequences.** `ui-test` reads it off the screen and compares it to
`Cargo.toml`, so a stale `build-wasm` fails the suite rather than shipping a
number that describes a different build than the one running.

It is not business state and never enters a view-model (Rule 9): it is a fact
about this binary, like the device identity next to it (0031).

**Rejected.** **A build timestamp or a commit hash** — precise, and unusable in
the one conversation it is for, which is with the Supervisor's version list.
**Showing the relay's version too** — the phone would have to ask for it over
the wire, and it already knows: whatever served the bundle is what the bundle
says it is.

## 0056 — An ingredient is created where it is needed, not in another tab

**Date** 2026-08-12 · **Status** Accepted · **Relates to**
[0039](#0039--the-editor-names-a-recipe-line-before-the-line-exists),
[0018](#0018--scope-cuts-no-pantry-a-single-list-no-ad-hoc-cart-items)

**Context.** An ingredient is the aggregation key of the cart (Rule 3), so
nothing reaches the list or a recipe until the library holds one. The library
had exactly one door: its own tab. Wanting something the library had never
heard of therefore meant leaving the screen, creating it, coming back, and
starting the line again.

That is an annoyance in the list, where a half-typed quantity is lost. In the
recipe editor it is worse than an annoyance: the draft lives in
`Recipes.svelte` and `App.svelte` renders one screen at a time, so switching
to Ingredients **destroys the recipe being written**. The rule was: know every
ingredient before you start writing, or lose what you wrote.

**Decision.** Both ingredient pickers — the list's and each of the editor's
lines — end with an option that is not a value: **« + Nouvel ingrédient »**,
which opens the library's form in place. What it creates is selected in the
picker it was created from, and the line carries on.

It is the **whole** form, in one component (`components/IngredientForm.svelte`)
used by all three screens, rather than a short "name and aisle" version. A
reduced form is a second place to add a field to, and the coefficients are
exactly what a new ingredient wants while a quantity is being typed: a density
is what lets 20 cl of the oil just invented merge with the 300 g asked for
elsewhere (Rule 5).

Two mechanical consequences follow from where it now renders:

- **The panel is not a `<form>`.** It sits inside the list's form and inside
  the one big form the editor is, and a nested `<form>` is not parsed — the
  browser drops it. So every button says `type="button"`, no field is
  `required`, and Enter is caught on each control rather than left to submit
  whatever form happens to be around it.
- **The picker is no longer bound.** Its last option is a door rather than a
  value, so choosing it must leave the line alone *and* put the control back
  where it was — which a binding that never changed would not do.

The id is **minted by the host**, through `mintIngredientId`, exactly as a
recipe line's is (0039). `SaveIngredient` returns a whole new state and not the
id it minted, and the picker has to select what it just created.

**Consequences.** `SaveIngredient` is now sent from three screens. It stands
alone: an ingredient is a library entry, not part of the recipe draft, so
abandoning the recipe afterwards keeps the ingredient. That is the same
ingredient the Ingredients tab would have produced, and deleting it there is
one tap.

Nothing about the scope cut moved (0018, Rule 14): there is still no ad-hoc
cart item, no free text on the list, no second kind of thing that can be
bought. Creating an ingredient from here creates a library ingredient.

`ui-test` drives both paths, and asserts the thing the shape is for — that the
recipe being written is still on screen and still named after the command.

**Rejected.** **A short quick-create form** — two forms to keep in step, and
the fields it drops are the ones a new ingredient most needs. **Inferring the
new id by comparing the library before and after** — three lines, no new API,
and a guess where 0039 already established the fact. **Sending the user to the
Ingredients tab and back** — the status quo, which costs a recipe draft.

## 0057 — Items: an aisle for what is bought whole and never cooked

**Date** 2026-08-12 · **Status** Superseded by
[0069](#0069--the-aisles-are-this-groups-shop-not-a-supermarkets) — the
`Items` aisle retired into `Foyer`, `Soin & santé` and `Artisanat & jardin`,
which say where a thing is found rather than that it is not food. The
argument below about *adding* an aisle costing no schema bump is unchanged
and is what 0069 leans on. · **Relates to**
[0029](#0029--how-the-document-encodes-domain-values),
[0018](#0018--scope-cuts-no-pantry-a-single-list-no-ad-hoc-cart-items)

**Context.** Toilet paper and soap go on the same trip, belong on the same
list, and are not food. They have one thing in common that no food has: they
are bought whole and alone, and no recipe ever calls for one. `Household`
covers the cleaning shelf and reads as such — bleach, sponges, washing powder
— which is a place in the shop rather than that property.

**Decision.** One more `Aisle`: **`Items`**, between `Household` and `Other`
in the declaration order, which *is* the walking order the cart sorts by. The
French for it is "Items", in `labels.ts` with the other ten.

An aisle and nothing more. Nothing stops an ingredient classified this way
from being used in a recipe: the classification is the shopper's word about
where a thing is found and what it is for, not a rule the machine enforces.

**Consequences.** It sits just before `Other`, so the end of the walk — what
is not food, then what has no aisle at all — stays together.

Adding one is **not** a breaking change, and that is a property of the encoding
rather than luck: `store` decodes an unknown aisle as `Other` and refuses an
unknown *unit* (0029), because an aisle only decides sort order while a unit
decides an amount. A phone three weeks out of date reads `"items"` as `Other`
and shows the line at the end of the cart. No schema version bump.

The two hand-written arrays that list every aisle — one in `store::codec`, one
in `app::tags` — are what keep a new variant from reaching a screen untagged;
both had to be widened, which is the point of writing them out.

**Rejected.** **A separate kind of library entry**, an `item` flag invisible to
recipes — it turns "never in a recipe" into a rule the machine enforces, buys
nothing the shopper cannot say by choosing an aisle, and doubles the library
screen and every picker. **Reusing `Household`** — that is the cleaning shelf;
this is about what is bought whole. **Hiding these from the recipe pickers** —
the same enforcement in a cheaper disguise, and a recipe that legitimately
calls for one would have no way to say so.

## 0058 — Anything chosen out of a library is searched for

**Date** 2026-08-15 · **Status** Accepted · **Relates to**
[0056](#0056--an-ingredient-is-created-where-it-is-needed-not-in-another-tab),
[0040](#0040--the-keyboard-is-a-length-not-a-mode),
[0035](#0035--the-app-owns-every-number-the-frontend-owns-every-word)

**Context.** Every ingredient and every sub-recipe was chosen from a native
`<select>`. That works for ten of anything and for nothing else: the options
come in the order they were given and there is no way to narrow them, so
picking the oil out of a family's real library is a scroll through a hundred
names on a phone.

The control was also drawing itself. A `<select>` is as wide as its widest
option — including the `<optgroup>` labels, which are wider than any unit —
and it cannot shrink below that inside a flex row, so the unit dropdown hung
off the right edge of a recipe line. The arrow beside it is the browser's, and
so is its padding. Neither is ours to fix while the control is native.

**Decision.** One component, `components/SearchPicker.svelte`: a text field
that filters, and the matches underneath it. It is what `IngredientPicker`
chooses an ingredient with, what the recipe editor chooses a sub-recipe with,
and what the list chooses a recipe with (0059). The two library screens get
the same search over what they are already showing, through
`components/SearchField.svelte`.

Four things follow from the shape and each of them is load-bearing:

- **The field is a real `<input>`**, so `required` still means required. It
  shows the chosen option's name when closed and the query when open, which is
  why it is not `bind:value`: the value the form validates has to be the name
  of a real option, never a half-typed query.
- **Anywhere else is a way out**, and the listener is in the capture phase.
  Pressing "save" with a half-typed query closes the panel *first*, the field
  goes back to showing the chosen name — empty when nothing was chosen — and
  the browser refuses the submit that follows.
- **A row is held rather than clicked**: `mousedown` is prevented so the
  pointer does not blur the field, close the panel and leave the click landing
  on nothing.
- **Enter chooses and never submits.** The panel opens inside the list's form
  and inside the one big form the recipe editor is, so an Enter that reached
  the form would save a recipe halfway through naming a line — the same trap
  0056 documents for `IngredientForm`, reached through a different control.

Matching is word by word, accent- and case-insensitively, over the name *and*
an ingredient's aliases: "tom cer" finds "Tomates cerises", and somebody
looking for "farine de blé" finds "Farine T55". It lives in `lib/format.ts`
beside `byName`, because searching is presentation for exactly the reason
sorting is (0035) — it answers a question a person is asking about words on a
screen, and the core deals in ids.

Ordering is settled in the same breath: every list of names on screen is
alphabetical in *French*, which is `format.byName` and not the core's sort.
The core sorts by lowercased id-tied name so that two replicas agree; "Œufs"
and "Épinards" land after "Z" there. Stable and legible are different things,
and which one you want depends on whether the reader is a person. So the cart
sorts its lines within each aisle here, and the list sorts its entries here.

**Consequences.** The panel is in flow rather than floating: a phone has no
room for an overlay that can be scrolled out from under, and what is below
simply moves down while the list is open. Like the "@" mention picker it opens
*because* of what was typed, so it inherits 0040's problem — the keyboard is
already up and the list is drawn into the keys — and its answer, `reveal`.

The arrow is ours now, and so is its padding.

`ui-test` drives every picker through the search rather than around it, and
asserts the two failures that are invisible in a unit test: that typing "tom"
narrows the offer to one row, and that a recipe line at 390 px fits the phone
with its parts sharing one right edge.

**Rejected.** **`<datalist>`** — one attribute, and it neither restricts the
value to the list nor renders the same way twice across browsers.
**Keeping the `<select>` and adding a search field above it** — two controls
for one choice, and the overflow stays. **A floating overlay** — it is the
thing that has to be scrolled out from under a keyboard, which is precisely
what cannot be done on iOS (0040).

## 0059 — The list shows what is missing, and a recipe joins it from there

**Date** 2026-08-15 · **Status** Accepted · **Relates to**
[0020](#0020--list-entries-vanish-on-completion-purge-is-deferred),
[0023](#0023--the-staple-flag-and-its-derived-auto-check),
[0058](#0058--anything-chosen-out-of-a-library-is-searched-for)

**Context.** Two things the list screen was making harder than the cart does.

A list entry stays put until every ingredient it contributed is settled, and
then it stays put some more: the purge is deferred to "terminer les courses"
so the trip can be undone (0020). That is right, and it means the screen fills
up with entries that have nothing left to say while the one thing still
missing is somewhere among them.

And a recipe could only reach the list through its own reader. Adding one
therefore meant a detour through Recettes, opening it, and coming back —
which is the wrong way round when the answer to "what are we cooking" is
already known.

**Decision.** Both halves take the shape the cart already has.

A completed entry folds into a `<details>` below the list, under "Terminées",
exactly as a bought line folds under "Acheté" (0023). It is folded and not
gone, because what removes it is "terminer les courses" and that is the step
0020 made undoable.

And the add panel gains a second mode: **Recette**, beside Ingrédient. It
searches the recipes by name (0058), offers the serving count the recipe is
written for, and lets it be changed before the entry is added rather than
after. It sends `AddRecipeToList` — the command the reader already sent, from
the screen the entry lands on.

**Consequences.** The serving count is derived from the chosen recipe with an
override on top, rather than seeded from it: a `$state` seeded from a view
captures the value once and would keep the previous recipe's count when
another is picked. The override carries the recipe it belongs to, so choosing
a different one drops it.

Nothing moved in the core. `AddRecipeToList` is unchanged, `ProgressView`
already carried `complete`, and both halves of this are the frontend
arranging what it was already being handed (Rule 9).

Adding a recipe purges no overlay entry — only adding a bare ingredient does
(Rule 3) — so a staple ticked off earlier in the trip stays ticked when a
recipe that uses it lands on the list. That is the existing rule and this
changes none of it.

**Rejected.** **Hiding completed entries outright** — the trip is undoable
until it is finished, and an entry that vanished would leave no way to see
what has already been covered. **A separate "add a recipe" screen** — a second
place to add something to one list, and the search is the same search.
**Sorting completed entries to the bottom of the same list** — it reads as an
ordering accident rather than as a statement, and the cart had already chosen
the folded section for the same question.

## 0060 — What was searched for is what gets created

**Date** 2026-08-25 · **Status** Accepted · **Relates to**
[0056](#0056--an-ingredient-is-created-where-it-is-needed-not-in-another-tab),
[0058](#0058--anything-chosen-out-of-a-library-is-searched-for)

**Context.** 0056 put the library form where an ingredient is wanted, so that
wanting something the library has never heard of no longer meant leaving the
screen. 0058 then made every library search-first: nothing is scrolled to any
more, it is typed for.

Between the two there was a gap, and it is the same defect 0056 exists to
kill, one step further in. Somebody types "poireau", reads "aucun ingrédient
ne correspond", presses "+ Nouvel ingrédient" — and gets an empty form. The
word they had just typed, twice on a phone keyboard, is gone. The two shelves
were worse: a search that finds nothing leaves a "Nouveau" button sitting in
the header that ignores the search entirely.

**Decision.** **The search is the name.** Whatever was being looked for and
not found is what the creation starts from, at all three places something is
born:

- `SearchPicker` hands its door the trimmed query, and the door **says so** —
  `+ Nouvel ingrédient « poireau »`. The continuity is visible before the
  press rather than discovered after it.
- The ingredients shelf and the recipes shelf grow the same offer **under the
  "nothing matched" message**, seeded with the query, rather than only in the
  header.
- `blankDraft(id, name)` and `blank(name)` take it; empty stays the ordinary
  case, because the header buttons follow no search.

**Consequences.** The query is read **before** `close()`, which clears it.
That ordering is the whole mechanism and it is invisible: the door would
otherwise hand over an empty string with no error anywhere.

`__door` in `ui-test` now asserts both halves — the label names the query and
the form opens carrying it — for every door in the app rather than at each
call site, because every door owes them.

The shelves keep their query after the form opens, so what gets created
matches the search and appears in the list behind it instead of vanishing
into a filter that no longer matches.

Nothing crossed the core. This is the frontend arranging what it already had
(Rule 9), and it is a `fix:` rather than a `feat:` — 0056 already said losing
what was being typed is the defect.

**Rejected.** **Binding the picker's field to the draft's name** — the field
is deliberately not bound (0058), and `required` depends on it showing a real
option's name when closed. **Prefilling verbatim, untrimmed** — the trim is
the only normalisation applied; capitalisation is the person's business, not
ours. **Parameterising the shelves' header button instead** — it is the
furthest thing on the screen from where the eye is when a search fails, and a
header button that changes its own label as you type reads as a glitch. **A
"create" row inside the shelf's result list** — the shelves are not pickers;
the empty state is where the failure is announced and where its answer
belongs.

## 0061 — Purchases are recorded; statistics are derived from them

**Date** 2026-08-25 · **Status** Accepted, scheduled for M9 · **Relates to**
[0018](#0018--scope-cuts-no-pantry-a-single-list-no-ad-hoc-cart-items),
[0019](#0019--the-cart-is-derived-the-overlay-stores-only-explicit-actions),
[0020](#0020--list-entries-vanish-on-completion-purge-is-deferred),
[0023](#0023--the-staple-flag-and-its-derived-auto-check),
[0024](#0024--attribution-is-declarative-not-cryptographic)

**Context.** This reopens a piece of the closed scope on purpose, which is
what Rule 14 asks for: an entry before any code.

What is wanted is a memory of what actually happens. When did we last buy
this, how many times have we, how often per month — or per year, the same
number seen at a different cadence. And the same for recipes, where "we made
it" is defined as **having bought all of its ingredients**.

Nothing in the app records any of it, and the reason is sharper than an
omission. `FinishShopping` is the **only** moment the app knows something was
bought — and it is the same moment the evidence is destroyed: completed
entries leave the list and the overlay is pruned (0020, 0028). One statement
later there is nothing left to read.

The event log is not the place either. It is capped at 200 entries and it is a
courtesy for deletions and edits, not an audit trail (0024) — a history stored
there would be eaten by its own cap within months.

**Decision.** A **purchase is a source**, persisted, synced and sealed like
every other. Five parts:

1. **One record per finished trip and ingredient**, carrying the ingredient,
   the quantities as exact rationals (Rule 4), the moment, the person, and the
   ingredient's name *as it was* — the same reason 0024 copies a label rather
   than resolving it, because the point of a history is to survive the deletion
   of its subject.
2. **One record per recipe completed in the same trip.** Decided at the moment
   of finishing and never re-derived afterwards: a recipe's lines change, the
   list is purged, and an ingredient bought for something else must not make a
   recipe look cooked.
3. **Written by `FinishShopping`, and by nothing else.** There is exactly one
   instant in the app when the real world is known to have happened.
4. **Identifiers are deterministic**, derived from the list entry and the
   ingredient rather than minted. Both people standing in the shop with the app
   open is the normal case, not a corner: two devices ending the same trip must
   write the same key, so the merge collapses them into one record. Random ids
   would union into a permanent double count that nothing could later tell from
   two real trips.
5. **Nothing expires.** The history *is* the feature, and a cap would quietly
   make last year's comparison wrong. Its cost is made visible instead: a
   Settings line reporting what cabas occupies on this device —
   `navigator.storage.estimate()` for the browser's own view of it, and the
   replica's snapshot size from the core.

Statistics themselves are **derived**, as pure functions in `domain` over
those records: last purchase, count, and a rate over a **named** window. The
window is named on screen — "14 fois depuis mars 2026" — rather than
extrapolated into a yearly rate, because there is no retroactive history and
an annual figure says nothing at all until a year has passed.

**Open, deliberately.** Whether an **auto-checked staple counts as a
purchase** is not decided here (0023). It rides along on a trip without having
been bought — the auto-check means "we already have it" — so counting it would
inflate the salt and the flour past any use. It must be settled **before the
first record is written**, because the records are the history and a history
cannot be rewritten. The recipe half is not in question: a recipe whose salt
came from the cupboard was still made.

**Consequences.** Rule 3 is untouched. The cart stays derived and unsynced;
what is persisted here is a source, the trace of an act, exactly like a list
entry.

The relay learns nothing — it is a document container like the rest, sealed
before it leaves (Rule 7).

It is a new top-level container, so an older device **ignores it and records
nothing**: the history has holes until both phones run the same build, with no
error anywhere. That is a reason to update both at once, not a schema break.

It is also the first thing in the document that grows without bound. The order
of magnitude is roughly 1 500 records a year against a library of 154 kB for
200 recipes; `crates/store/tests/document_size.rs` is where that gets measured
before the shape is fixed, not after.

**Rejected.** **Putting purchases in the `EventLog`** — capped by design, and
the cap would eat exactly the entries the feature is about. **Monthly counters
per ingredient** — compact and enough for a rate, but two offline devices
incrementing one counter need counter semantics to avoid losing an increment,
and it throws away the dates that answer "when did we last buy this".
**Deriving "recipe made" after the fact from ingredient purchases** — an
ingredient bought for something else would count, and the answer would change
retroactively every time a recipe is edited. **A pantry or stock model** —
still cut (0018); this records what left the shop, never what is in the
cupboard. **Trimming by age**, at any horizon: chosen against, in favour of
showing the footprint.

## 0062 — A photo is a blob beside the document, never in it

**Date** 2026-08-25 · **Status** Accepted, scheduled for M10 · **Relates to**
[0008](#0008--serialized-snapshots-not-sqlite),
[0009](#0009--zero-knowledge-relay-with-app-layer-e2ee),
[0042](#0042--the-relay-keeps-a-sequenced-log-it-cannot-read),
[0047](#0047--the-qr-is-shown-never-scanned-and-the-encoder-is-ours),
[0050](#0050--an-abandoned-family-log-is-forgotten-by-hand-or-not-at-all),
[0057](#0057--items-an-aisle-for-what-is-bought-whole-and-never-cooked)

**Context.** What is wanted is small to say: take a photo for a recipe, for an
ingredient, for an item. A dish you recognise before reading its name, and a
product you recognise in an aisle — which is the case that decides everything
below, because an aisle is where there is no network.

It reopens the closed scope, so it starts here rather than in code (Rule 14).
And it is not a small feature, for one reason that is invisible from the
screen: **every save writes the whole document.** `App::pending_snapshot`
serialises the entire replica and `Storage::save` replaces it, deliberately —
one blob in, one blob out (0008), atomic because a half-written snapshot is a
destroyed library. Put the photos inside it and ticking an item off in the
shop rewrites the whole photo library.

Measured, x86-64 release, incompressible bytes — a phone in wasm is some
multiple of this, and today's entire library is 154 kB loading in 0.42 ms:

| Photos in the document | Snapshot | Export, on every save | Import, on every cold start |
|---|---|---|---|
| 20 × 150 kB | 5.9 MB | 30 ms | 8 ms |
| 60 × 150 kB | 17.6 MB | 83 ms | 24 ms |
| 200 × 150 kB | 58.6 MB | 310 ms | 98 ms |
| 500 × 80 kB | 78.1 MB | 320 ms | 110 ms |

`crates/store/tests/document_size.rs` already refuses a snapshot above 4 MiB,
which the first row breaks on its own. This is the case 0008 named in advance:
"if data volume ever invalidates the premise, the trait is the seam to
revisit".

**Decision.**

1. **One photo per recipe and one per ingredient** — items included, since an
   item is an ingredient in the `Items` aisle (0057). The document holds an
   **id and nothing else**; the bytes are never in it.
2. **The id is random, minted at capture — not a content hash.** Deduplication
   between two people is worth nothing, and a hash of the plaintext handed to
   the party that also holds the ciphertext turns a guessed photo into a
   confirmed one. Rule 7 is the whole point of the relay's design; an id that
   says something about the bytes gives a piece of it back for a convenience
   nobody asked for.
3. **The bytes live in a store of their own**, one record per photo: a second
   IndexedDB object store on the phone, a directory natively. `Storage` keeps
   its shape and its atomic whole-document write; photos get `PhotoStore` —
   `load`, `save`, `remove`, and `ids`, which is what a prefetch diffs against.
4. **One format, JPEG, with a cap the core enforces.** The frontend downscales
   and encodes, because a canvas is the only image encoder a PWA has; the core
   refuses anything above the cap. The limit is then one number in Rust rather
   than a promise the UI makes to itself. A second format later is a new key,
   never a guess at the bytes.
5. **Capture is `<input type="file" accept="image/*" capture>`, never
   `getUserMedia`** — the same reasoning that made the QR shown and never
   scanned (0047): no permission prompt, no video element, nothing that rots
   quietly in an installed iOS PWA. It also lets an existing photo be chosen,
   which `getUserMedia` cannot.
6. **Photos travel on their own socket**, `/photos`, opened when there is work
   and closed when the queue drains. A `Hello` names the family, one round trip
   reconciles what each side has and wants, and each photo crosses sealed
   (Rule 7). Not on `/sync`: a 200 kB transfer must not sit in front of a list
   edit, and a connection that falls behind the relay's forward buffer is
   disconnected — which a photo transfer would make ordinary.
7. **Every device ends up holding every photo.** The transfer is a background
   prefetch of everything the document references, not a fetch on demand: the
   photo of a product is wanted in the shop, and the shop is a Faraday cage
   (Rule 6). Nothing about it blocks a user action — a photo taken offline is
   attached immediately and uploaded later.
8. **The relay keeps photos until a person forgets them.** It cannot tell that
   an id is unreferenced, because it reads nothing; and "unreferenced on my
   replica" is not "unreferenced" while the other phone has been off for a
   week. So: devices delete their **local** copy of what their own replica no
   longer references — safe, because the relay hands it back if the reference
   returns — and the relay keeps everything, surveyed by `cabas-relay families`
   and forgotten by hand, exactly like an abandoned log (0050). A per-family
   byte cap refuses a push rather than filling the SD card Home Assistant runs
   on.
9. **Additive on both compatibility surfaces.** A `photo` key is ignored by an
   older build on read and never rewritten on save — the same argument that let
   the `Items` aisle ship without a bump (0057) — so `SCHEMA_VERSION` does not
   move. The photo socket is a separate endpoint carrying its own protocol
   byte, so `/sync`'s `PROTOCOL` stays 1 and a phone left in a pocket keeps
   converging, without photos. A feature, therefore: minor.

**Consequences.**

This is the **first content in the app that is not local the moment it
exists**. A photo taken on one phone reaches the other only once both have been
online, so the UI owes a state for "referenced, not here yet" that does not
read as an error — the same discretion the sync indicator has (Rule 6).

The document stays 154 kB and every measurement in ROADMAP M2 stays true. What
grows instead is a store nothing rewrites: adding a photo costs one record, and
a save in the shop costs exactly what it costs today.

Storage on the phone stops being a rounding error. `navigator.storage.persist()`
becomes worth asking for, and the footprint line M9 already owes Settings (0061)
covers photos too — one number for what cabas occupies here.

Home Assistant's backups grow with the photo library, which changes M6's
arithmetic and not its procedure: the drill still reads its marker off the
relay, and a restored `/data` now brings photos back alongside the log.

**Rejected.** **Photos in the document** — measured above; it converts every
tap in a shop into a multi-megabyte write and every launch into an import.
**Small thumbnails in the document instead**, ~20 kB each and nothing else to
build: it is the same curve one order of magnitude to the right, it caps the
product at a thumbnail, and the migration out of it is owed anyway.
**Content-addressed ids** — a confirmation oracle for a party that already
holds the ciphertext (Rule 7), bought for a deduplication two people never hit.
**`GET`/`PUT /photo/<family>/<id>` over HTTP** — simpler, resumable and
cacheable, but it puts the family id, which is the only access control the
relay has, into a URL: request logs, analytics, and every proxy in between.
0047 refused the same move for the phrase. **A new `FrameKind` on the sync
log** — the log is truncated by snapshots (`Snapshot { covers }`, 0042), so a
photo frame would be dropped by the next compaction, and the only way to keep
it would be re-pushing the whole photo library every time. **Relay-side garbage
collection driven by a client's "keep" set** — it needs a last-claim timestamp
per blob and can still delete what a long-offline device references; manual,
like 0050, and visible in `survey`. **Several photos per entity** — an order,
a principal, a management screen, for a product whose whole shape is one
picture per thing. **WebP** — smaller, but canvas encoding of it is the kind of
thing that is present on the desk and absent on the phone. **Encrypting the
local photo store at rest** — the document is not encrypted there either; the
phone's own encryption is that boundary, and a second one here would protect
the photos of a library sitting in plaintext beside them.

## 0063 — A family is called a group

**Date** 2026-08-25 · **Status** Accepted · **Relates to**
[0009](#0009--zero-knowledge-relay-with-app-layer-e2ee),
[0024](#0024--attribution-is-declarative-not-cryptographic),
[0042](#0042--the-sync-protocol-and-what-the-relay-stores),
[0050](#0050--an-abandoned-family-log-is-forgotten-by-hand-or-not-at-all)

**Context.** "Family" was chosen when the product was two people and their
phones, and it described that exactly. It stopped describing the thing the
moment a third device belonged to somebody who is not family — a flatmate, a
parent's spare phone, the two of us plus whoever is cooking this week. The
word also implies a structure the software does not have and must not
acquire: one shared key, no roles, no owner, no head of anything (Rule 7).

The word was everywhere: 33 types and functions across five crates, the
relay's `families` subcommand, a `localStorage` key on two live phones, and
several hundred lines of prose.

**Decision.** **Group**, everywhere a person or a programmer can read it.
`FamilyKey` → `GroupKey`, `FamilyId` → `GroupId`, `FamilyLog` → `GroupLog`,
`cabas-relay families` → `cabas-relay groups`, and every French string.

Three things deliberately **did not** change:

1. **The key derivation.** The symmetric key and the id are the first 48 bytes
   of the phrase's BIP39 seed (0042) — bytes, with no domain-separation string
   in them. So the rename cannot invalidate a phrase, and the twelve words
   written down beside the backup key still open the same group.
2. **What is on disk.** `Meta` is postcard, which is positional: renaming a
   field renames nothing in the file. The relay's directories are named after
   the group id in hex, which the rename does not touch either. An existing
   `/data` is read by the new binary without noticing.
3. **`docs/DECISIONS.md`.** Append-only is a rule (Rule 14), and rewriting
   sixty entries to say "group" would be editing the reasoning of decisions
   that were taken about a family. Every entry before this one still says
   family, and means group.

The one thing that *does* move is the `localStorage` key: `cabas.family`
becomes `cabas.group`, read once from the old name and migrated (`readGroup`).
It carries the phrase, so getting it wrong unpairs a phone in the field.

**Consequences.** A device paired before this build stays paired, and the
migration removes the old key rather than leaving a second copy of the twelve
words in the browser. The cost is one-directional: a build older than the
rename, installed *after* it, would ask for the phrase again. That is a
recoverable state — the phrase is written down (README, "Backups") — and the
alternative was keeping the secret in two places forever.

Two collateral renames had to be undone by hand and are worth naming, because
the next bulk rename will meet them again: `target_family` is a Rust `cfg`,
and `font-family` is CSS. Both matched, both broke everything, and neither had
anything to do with families.

**Rejected.** **Renaming the French only** — the word appears in the code far
more often than on screen, and a codebase whose types disagree with its
screens is where the next person's confusion comes from. **Renaming DECISIONS
too** — Rule 14. **"Household"** — it is already an `Aisle` (0057), and a
second meaning for a word this codebase uses for shelves is a trap.
**"Home"** — collides with Home Assistant in every operational sentence.
**Migrating the relay's directory names** — nothing needed it, and a rename of
live data to fix a word would be the only irreversible part of this change.

## 0064 — The tabs run from the shelves to the trip

**Date** 2026-08-25 · **Status** Accepted · **Relates to**
[0003](#0003--ios-ships-as-a-pwa),
[0059](#0059--the-list-shows-what-is-missing-and-a-recipe-joins-it-from-there)

**Context.** The tab bar was `Courses · Liste · Recettes · Ingrédients ·
Réglages` — the order the milestones were built in, which is the order the
*author* met the screens and not the order anybody uses them. Read left to
right it is the story backwards: the finished trip first, the raw materials
last.

**Decision.** Reverse it: **`Réglages · Ingrédients · Recettes · Liste ·
Courses`**. The shelves an ingredient comes off, then the recipe that uses it,
then the list it is asked for on, then the trip it is bought on. Réglages
takes the far end because it is where a flow starts least often.

**Consequences.** The two screens used while standing in a shop — Liste and
Courses — end up under the right thumb, which is where they should have been.
Nothing about the change is persisted: `session.screen` stores a name, not an
index, so a phone that was left on the cart is still on the cart after the
update.

`ui-test` asserts the whole sequence rather than one position, because a
reversal that goes half wrong passes any single check.

**Rejected.** **Réglages at the far right**, keeping the four content tabs in
flow order — it puts the least-used screen where the most-used one belongs.
**Leaving it alone and calling it habit** — two people use this, both said the
same thing about it.

## 0065 — A photo can be chosen as well as taken

**Date** 2026-08-25 · **Status** Accepted · **Relates to**
[0062](#0062--a-photo-is-a-blob-beside-the-document-never-in-it),
[0047](#0047--the-qr-code-is-shown-and-never-scanned)

**Context.** `PhotoField` had one input, carrying `capture="environment"`.
That attribute is not a preference: on a phone it opens the camera and *only*
the camera. The photo already in the roll — the label photographed in the shop
before cabas was open, the dish someone sent, the picture taken last week — was
unreachable, and the way to attach one was to point the camera at a screen.

**Decision.** Two hidden inputs and two buttons: **"Prendre une photo"** with
`capture`, **"Importer"** without. Everything behind them is unchanged — the
same `encodePhoto`, the same ceiling, the same `putPhoto` (0062).

Two inputs rather than one whose attribute is flipped between clicks:
`capture` is read when the picker opens, so an input that means something
different depending on which button was pressed last is a race with a slow
phone, and the failure is silent.

**Consequences.** The buttons row wraps on a narrow screen rather than
shrinking its labels into initials; with a photo attached there are three of
them beside a thumbnail. On a desktop, where `capture` is ignored, the two
buttons do the same thing — which is honest rather than confusing, since the
machine that has no camera is the one where it does not matter.

`ui-test` drives the import input by the same `DataTransfer` trick as the
camera one, so the second path is covered end to end and not merely present.

**Rejected.** **Dropping `capture` and keeping one button** — the camera is
the common case, and an extra tap through the OS picker for every photo taken
in a shop is the wrong trade. **A single button with a menu** — one OS picker
behind another. **Drag-and-drop** — no phone has it, and it is the platform
this is for.

## 0066 — An ingredient knows how much of it one buys

**Date** 2026-08-25 · **Status** Accepted · **Relates to**
[0015](#0015--no-cross-dimension-conversion-without-an-explicit-coefficient),
[0029](#0029--how-the-document-encodes-domain-values),
[0067](#0067--a-row-goes-on-the-list-by-being-pushed-there)

**Context.** `AddIngredientToList` has always required an amount, and every
caller was a form with a field in it. The gesture in 0067 is not a form: a row
swiped across a screen carries no number, and there is nowhere to put a
question in the middle of it.

Something has to be put on the list. "One piece" is right for tomatoes and
absurd for flour; asking afterwards turns a gesture back into a form.

**Decision.** Two halves.

An ingredient gains an optional **`default_quantity`** — a kilo of flour, six
eggs, a litre of milk. It is a *shopping* amount and nothing else: no recipe
reads it, and it is not a third conversion coefficient (Rule 5 is untouched —
it converts nothing and enables no conversion).

`AddIngredientToList`'s `quantity` becomes **optional**, and absent means "as
much of it as one usually buys": the ingredient's default, or **one piece** if
it has none. The substitution lives in
`cabas_domain::Ingredient::shopping_quantity`, so the rule is in the domain
where a rule belongs, and the frontend sends `null` rather than deciding
anything (Rule 9).

**Consequences.** Additive on the persisted schema, like `photo` before it
(0062) and for the same reason: an older build ignores the key on read and
does not rewrite it on save, so `SCHEMA_VERSION` does not move. A phone three
weeks out of date puts one piece where a current one puts a kilo — a wrong
amount on a shopping list, which is a thing a person corrects in a shop, and
not a document it cannot read.

The view carries it as a `QuantityInput` rather than a rendered
`QuantityView`, because the only thing that displays it is an edit form and a
form that rounds writes the rounded value back on the next save. `None` stays
distinct from "one piece" all the way to the screen: the field is empty when
nobody has said, which is the same distinction the density fields have kept
since 0015.

**Rejected.** **One piece for everything**, no field — it is wrong for
precisely the staples that are swiped most. **Asking for the amount after the
swipe** — that is a form, and the gesture exists to avoid one. **Deriving it
from what has been bought before** — that is M9's data (0061), it does not
exist yet, and a default that changes on its own is one nobody can predict.
**Putting the fallback in the frontend** — business logic, and there would
then be two of it when the Tauri host lands (Rule 9, 0005).

## 0067 — A row goes on the list by being pushed there

**Date** 2026-08-25 · **Status** Accepted · **Extended by**
[0072](#0072--the-gesture-keeps-counting-and-holding-a-row-types-the-amount)
— which keeps everything below and adds the two things it left out: the
gesture goes on counting after the first add, and the long-press rejected at
the end of this entry is reinstated, for reasons that entry gives. ·
**Relates to**
[0059](#0059--the-list-shows-what-is-missing-and-a-recipe-joins-it-from-there),
[0066](#0066--an-ingredient-knows-how-much-of-it-one-buys),
[0020](#0020--a-list-entry-disappears-when-it-is-settled-purge-is-deferred)

**Context.** Putting a known ingredient on the list took a tab change, a
button, a picker, an amount and a submit. The thing being asked for is
"this one, the usual amount", and it was five interactions long.

**Decision.** A horizontal gesture on the shelf rows — ingredients and
recipes. Drag right to a hard stop, let go, and it is on the list with the
amount 0066 decides. The row springs back and **parks short of home**,
leaving "Annuler" uncovered on the left; the text revealed *during* the drag
is "Ajouter à la liste", so the gesture says what it will do before it does
it.

Four decisions inside that, each of which fails quietly if taken the other
way:

1. **Long, and committed on release.** The row has to reach a stop before the
   add happens, and sliding back cancels it. This screen is scrolled far more
   often than it is swiped; a short flick would fire on every scroll that
   started slightly sideways.
2. **`touch-action: pan-y`, and the axis decided once.** Vertical scrolling
   stays the browser's. Without the declaration the browser claims the first
   ambiguous frame as a scroll and the row simply never moves. Ties go to
   vertical.
3. **The parked state is derived, not remembered.** "Annuler" is shown because
   the core's list holds an entry for this row — so it survives leaving the
   screen and a reload, and it disappears by itself when the *other* phone
   takes the entry off the list. A flag set by the gesture would go stale in
   all three cases.
4. **The gesture only ever adds.** Removing is the button. A leftward flick
   over a row that is scrolling is not a deliberate enough act to delete
   something with.

**Consequences.** The click at the end of a drag is swallowed, or every swipe
would also open the ingredient it swiped. The distances are tokens in
`app.css` (Rule 10) read back into the component, because the pointer maths
needs numbers; the spring is a token too, and `prefers-reduced-motion` flattens
it to nothing while the row still lands where it lands.

Nothing is only reachable this way: the list's own add form does both jobs,
which is what keeps a gesture from being a keyboard trap. That is the whole
accessibility argument and it is worth stating, because the honest version is
"this is an accelerator, not an interface".

`setPointerCapture` is attempted and its failure ignored — capture is what
keeps a drag alive when the finger wanders off the row, not what makes it
work.

**Rejected.** **A "+" button on every row** — a fifth control on a line that
already carries a photo, a name, an aisle and a badge, and the row is already
a button. **Swipe left to remove** — see 4. **Long-press** — invisible, and it
fights the OS's own text selection. **Firing at the threshold rather than on
release** — nothing to cancel, and a mis-scroll becomes an edit.

## 0068 — A device joins a group, and then says who is carrying it

**Date** 2026-08-25 · **Status** Accepted · **Relates to**
[0024](#0024--attribution-is-declarative-not-cryptographic),
[0031](#0031--the-device-identity-lives-in-the-host),
[0063](#0063--a-family-is-called-a-group),
[0021](#0021--pairing-by-qr-with-a-12-word-recovery-phrase)

**Context.** A device that joined a group typed the twelve words and then a
first name, and that name minted a **new user**. Every phone that joined
therefore invented a person, whether or not that person was already in the
group. Two phones belonging to Alexis produced two Alexis; the roster grew a
duplicate of somebody it already had, and the journal attributed the same
human's edits to two different names, permanently — no command deleted a user
and nothing merged them.

The information needed to avoid it exists and is already synced: the group's
roster is in the document. What was missing was asking the question *after*
the document arrived instead of before.

**Decision.** Splitting the identity in two. A device knows what it is from
the moment it exists — the replica's peer id derives from the device id — so
`Identity.device` is minted at pairing time as it always was. `Identity.user`
becomes **optional**, and is `None` between the twelve words and the moment
somebody is chosen off the roster or added to it.

The app therefore **opens with no user**: the replica loads, sync connects,
and "Qui êtes-vous ?" is rendered *over* a running app, with the roster
filling in underneath the question as the first frames land. Three commands
serve it — `ChooseUser` (a member the group already has), `CreateUser` (one it
does not), and `NameDevice`.

Consequences that are decisions in their own right:

- **Nothing attributable can be written in that window.** `App::user_id`
  returns `AppError::NoUser`, and every attributed write goes through it. A
  placeholder name would be a row two people have to interpret forever. The
  library is still writable, because creating an ingredient is not attributed.
- **`enrol` writes nothing without a user.** A device record needs an owner,
  so writing one early would mean inventing the owner — the exact ghost this
  entry exists to prevent.
- **`NameDevice` comes before the choice, not after.** The device record
  cannot exist before it has an owner, so naming it first writes nothing and
  naming it afterwards writes the record twice — once with an empty name,
  which is the version the other phone would see.
- **Changing user is the same act.** Settings offers the same picker, and a
  phone handed to somebody else keeps its device record: only the owner moves,
  and `paired_at` is not rewritten, because the device did not join again
  today. What was already added stays attributed to whoever added it (Rule 7).
- **The identity is written back by the host.** `localStorage` holds the only
  durable copy (0031), so `Session.identify` exists as the single door for the
  three commands that move it. Running them through `run` would work perfectly
  until the next launch, which is the worst shape a bug can have.

**Consequences.** `StateView.me` is now `Option<UserView>`, which is the
frontend's cue to ask rather than a sign that nothing loaded. The stored
identity needs **no migration**: an identity written before this change has
both fields present as strings, which is valid under the new shape.

Offline, the roster is empty and "créer" is the only door — the same place the
old flow always landed, and the screen says the list arrives with the first
sync rather than implying the group is empty. Rule 6 holds: nothing waits.

A brand-new group is the same flow with an empty roster, so there is one path
and not two.

**Rejected.** **Matching on the typed name** — "Alexis" and "alexis" and
"Alexis " are three people or one depending on a rule nobody can see, and
guessing wrong merges two humans. **Blocking until the first sync completes**
— it makes the app unusable offline to prevent a duplicate that only matters
online (Rule 6). **Asking only in Settings and keeping the old onboarding** —
the duplicate is created at the moment of joining, which is the one moment
this has to be right. **A "primary" device that approves joiners** — an owner,
a role, and an access-control story the one shared key cannot back (Rule 7).
**Deleting a user to clean up after the old behaviour** — a delete under a
CRDT races a concurrent write to the same person, and the log entries pointing
at them would dangle. The duplicates that exist stay; the roster is where they
are visible, and renaming one is a label change.

## 0069 — The aisles are this group's shop, not a supermarket's

**Date** 2026-08-26 · **Status** Accepted · **Supersedes**
[0057](#0057--items-an-aisle-for-what-is-bought-whole-and-never-cooked) ·
**Relates to** [0029](#0029--how-the-document-encodes-domain-values)

**Context.** The twelve aisles were a generic French supermarket's counters —
`Produce`, `Butcher`, `Fish`, `Deli`, `Dairy`, `Bakery`, `Grocery`, `Frozen`,
`Beverages`, `Household`, `Items`, `Other`. Three of them name meat and fish
counters that nobody in this group has ever walked past, one (`Grocery`) was
doing the work of four, and `Items` classified by what a thing *is not*
rather than by where it is found.

An aisle set is not a taxonomy. Its only job is to sort the cart into the
order a person walks, and an aisle that is never chosen costs a line in every
dropdown while sorting nothing.

**Decision.** Twelve aisles again, chosen by the people who shop:

| Tag | Reads |
|---|---|
| `produce` | Fruits & légumes |
| `bakery` | Pains & pâtisseries |
| `dairy` | Produits laitiers |
| `pantry` | Ingrédients & épices |
| `frozen` | Surgelés & plats cuisinés |
| `staples` | Pâtes, riz & céréales |
| `snacks` | Snacks & friandises |
| `beverages` | Boissons |
| `household` | Foyer |
| `care` | Soin & santé |
| `crafts` | Artisanat & jardin |
| `other` | Autres |

Declaration order is still the walking order, and it is *their* order — the
list was given in the order they walk it, and reordering it into something
more defensible would have been an opinion about somebody else's shop.

**Consequences.** `SCHEMA_VERSION` does **not** move. An aisle decides sort
order and nothing else, so `store::codec` already degraded an unknown tag to
`Other` rather than refusing the document (0029, 0057) — a phone three weeks
out of date reads `staples` as `Other` and shows the line at the end of the
cart, which is a wrong walk and not a lost amount.

What that argument does *not* cover is **retiring** a tag, and that is the
half worth writing down: without a read mapping, every ingredient in the
existing library would have decoded to `Other` on the next launch. The
document would have opened cleanly, every test would have passed, and the
discovery would have happened in a shop. So `codec::aisle` still answers to
the five retired spellings — `grocery` → `Pantry`, `items` → `Household`,
and `butcher`/`fish`/`deli` → `Other`, because there is no shelf left to put
them on and `Other` says "look at this" rather than inventing a
classification. They are read and never written: one save under this build
and an ingredient stops answering to them.

**Rejected.** **Keeping the meat counters "in case".** They are three rows in
every dropdown, on every ingredient anybody will ever create here, for a case
that does not arise; and the code is append-only in `docs/`, not in
`domain/`. **A user-editable aisle list.** Aisles are ordered, and an ordered
list somebody maintains by hand is a settings screen, a drag handle and a
migration — for a set that changes about once a year. **Bumping
`SCHEMA_VERSION`.** It would refuse the *other* phone's document until both
were updated, which is a worse failure than a line at the end of the cart.

## 0070 — An ingredient says where it is kept

**Date** 2026-08-26 · **Status** Accepted · **Relates to**
[0069](#0069--the-aisles-are-this-groups-shop-not-a-supermarkets),
[0029](#0029--how-the-document-encodes-domain-values)

**Context.** The aisle answers "where do I find this", which is the question
in the shop. There is a second question, half an hour later, with the bags on
the counter: does this go in the fridge, the freezer, or a cupboard. Getting
it wrong costs a bag of frozen peas.

The aisle cannot answer it. Crème fraîche and UHT milk share an aisle and not
a shelf; frozen and refrigerated pastry sit two metres apart in the shop and
in two different appliances at home.

**Decision.** A third field on the ingredient: `Keeping`, one of `Ambient`,
`Fridge`, `Freezer`, defaulting to `Ambient`. It travels to the cart line,
because that is the row still on screen when the unpacking happens, and the
UI draws a badge for the two that are not the default — a badge on almost
every row says nothing.

**Consequences.** Additive: the key is absent on every ingredient written
before this, and an absent key decodes as `Ambient`, which is both the
default and the honest reading of silence. `SCHEMA_VERSION` does not move,
for the reason 0069 gives. An unrecognised value decodes as `Ambient` too —
this is a hint, not an amount, and Rule 4's strictness has nothing to say
about it.

It is deliberately **not** an aisle. Merging the two would mean either three
copies of every cold aisle or an ingredient that cannot be both "produits
laitiers" and "frigo", and the whole point is that the two axes are
independent.

**Rejected.** **Deriving it from the aisle.** `frozen` → freezer is right and
everything else is a guess; a guess here is a bag of peas. **A free-text
"where it lives".** Three answers cover it, and a free-text field is a field
that gets sorted by nothing. **Putting it on the recipe too.** A recipe is not
stored anywhere; the dish might be, and that is a different feature.

## 0071 — A shop is a name, and the cart is one trip per shop

**Date** 2026-08-26 · **Status** Accepted · **Relates to**
[0018](#0018--scope-cuts-no-pantry-a-single-list-no-ad-hoc-cart-items),
[0056](#0056--an-ingredient-is-created-where-it-is-needed-not-in-another-tab),
[0060](#0060--what-was-searched-for-is-what-gets-created),
[0022](#0022--instruction-steps-are-segments-referencing-ingredient-usages)

**Context.** One list, one walking order (0018) — and two shops. Half of what
is on the list is only at the market, the other half only at the supermarket,
and the cart sorted them into one continuous walk through a shop that does not
exist.

**Decision.** A **shop** is a name and nothing else: no address, no hours, no
aisle order of its own. Ingredients carry a list of the shops they can be
bought at, and the cart screen offers one chip per shop plus "Tous". Choosing
one keeps what that shop sells, grouped by aisle exactly as before, and folds
the rest away under "Ailleurs".

Called `Shop` in the code and not `Store`, because `cabas-store` is the
persistence crate and a `Store` inside it would be two unrelated things under
one word in the file that maps between them.

Three parts of this carry the weight:

1. **An ingredient with no shop belongs to every shop.** Empty means "nobody
   has said", never "nowhere". The alternative hides a line from the only
   screen that would have prompted somebody to classify it, which is how a
   shopping list quietly loses an item — and every ingredient starts unplaced.

   The same answer covers a second silence that is easy to miss: an
   ingredient whose named shops have **all been forgotten since**. It was
   classified, the classification is gone, and refusing to show it would lose
   it exactly as above. Deciding that needs the shop library, which is why the
   rule is `domain::shop::sold_at(&ingredient, &shops)` and not a method on
   the ingredient — and why `CartLineView::shops` carries the **resolved**
   trips rather than a copy of the ingredient's own list. The screen filters
   by plain membership and holds no rule of its own (Rule 9); writing
   "empty means everywhere" out again on the Svelte side is a second
   implementation that will drift, and it was there for about an hour.
2. **A shop is created where it is typed**, in the field on the ingredient's
   own form: a name that matches nothing offers to become one. Nobody is going
   to visit a settings screen first, and if they do not, "Biocoop" on the
   second ingredient is a different string from "Biocoop" on the first. The id
   is minted by the frontend so the field can select what it just created —
   the same bargain as 0056, for the third time.
3. **The list is ordered, and the order is read.** The first shop on an
   ingredient is the one to file it under when a future screen has to choose
   one. Today nothing needs to; the field would have had to be re-typed if it
   had been a set.

The filter applies to what is still **to buy** and not to the two folded
sections: those are a record of the trip, and a record that changes shape when
you tap a chip is not one.

**Consequences.** A new root container (`shops`) and a new ingredient key,
both additive — `SCHEMA_VERSION` does not move, for the reason 0069 gives.
Two commands, `SaveShop` and `DeleteShop`, because creating a shop is a thing
a person did and not a side effect of saving something else.

**Nothing is recorded in the event log**, unlike every other library write.
The log exists for what the data cannot remember — a deleted recipe leaves no
field behind to hold "and Alexis did this" (0024) — and a shop is a label on a
handful of ingredients: forgetting one changes no amount, breaks no line, and
is undone by typing the name again. Recording it would also mean widening the
log's persisted `subject_kind`, which is a schema change bought for a row
nobody would read.

Forgetting a shop leaves the ingredients that named it alone. Referential
integrity is not enforceable under a CRDT (0022) and here the dangling id is
harmless: it matches no shop, so it filters nothing, and the line goes back to
being sold everywhere. Settings gains a "Magasins" screen for renaming and
forgetting, because a library with no way to fix a typo fills up with
"Biocoop", "biocoop " and "Bicoop".

**Rejected.** **Grouping the cart by shop, all shops at once.** It answers
"what do I buy where" and the question in a shop is "what do I buy *here*";
it also has to invent a rule for an ingredient sold in two places. **A shop
name straight on the ingredient, no library.** Two spellings are two shops,
which is the entire failure this prevents — the same argument as the alias
table. **Per-shop aisle orders.** A second ordered list to maintain per shop,
for a walk that is already approximately right. **Shops on the recipe.** A
recipe is not bought.

## 0072 — The gesture keeps counting, and holding a row types the amount

**Date** 2026-08-26 · **Status** Accepted · **Extends**
[0067](#0067--a-row-goes-on-the-list-by-being-pushed-there) · **Relates to**
[0066](#0066--an-ingredient-knows-how-much-of-it-one-buys),
[0040](#0040--the-keyboard-is-a-length-not-a-mode)

**Context.** 0067 made the first add one gesture and left everything after it
where it was: the amount was invisible on the shelf, changing it meant the
list tab and a form, and the swipe did nothing at all on a row that was
already on the list. "Two of those" was five interactions again.

**Decision.** The parked row carries **what it asks for**, written under
"Annuler", and the same drag keeps working from there: right for one more
notch, left for one less. Left past the last one takes the row off the list,
because one less than the last one is nothing to buy and a row asking for
none of a thing is a row you read twice.

**A notch is the core's rule, never the frontend's arithmetic.** The command
is `NudgeListEntry { entry, steps }` and `steps` is a count: what it is worth
depends on what is on the line — the ingredient's usual shopping quantity
(0066), or **one whole recipe as written**, so a tart for four goes 4 → 8 →
12 rather than 4 → 5. Both live in `domain::list`, which also decides that the
notch is converted into the *line's* unit and not the other way round: type
"500 g" and nudge, and the row stays in grams, because grams is what is being
read in a shop.

**Holding the row opens the exact amount** — a panel with the amount, the unit,
and nothing else. The swipe is deliberately coarse and cannot express "350 g"
or change a unit, so there has to be somewhere the precise answer is typed;
holding is where, because it is the same thumb in the same place and a tap is
already taken.

**This reverses 0067's rejection of long-press**, which said it was invisible
and fought the OS's text selection. Both objections were right and neither
survived contact: it is no longer invisible because the row it opens now
*shows* an amount that visibly wants changing, and `user-select: none` plus
`-webkit-touch-callout: none` settles the second. It is still an accelerator
— the list's own form does every one of these jobs — which is the whole
accessibility argument, and it is the same one 0067 made.

**Consequences.** `SetEntryQuantity` joins `SetEntryServings`, refused on the
wrong kind of entry rather than guessing which one was meant.
`ListItemView::Ingredient` gains a lossless `edit` beside its rendered
`quantity`, for the reason `IngredientView::default_quantity` has one: a form
seeded from a rounded amount writes the rounding back on the next save.

The panel is a fixed overlay and **not** a `<dialog>`: `showModal()` puts the
element in the top layer, positioned against the layout viewport — the one iOS
does not shrink for the keyboard (0040) — where it cannot read
`--keyboard-inset` and ends up under the keys.

A press that has fired swallows the click that follows it, exactly as a drag
does, or holding a row would also open the editor underneath it.

`--press-delay` is a token like every other measurement (Rule 10), and its
**unit has to be read back, not just its number**: it is authored `500ms` and
the CSS minifier ships it as `.5s`, so a bare `parseFloat` gives 0.5. In the
component that would be a press firing instantly; in the test that drives it,
it looked exactly like a press that never fired.

**Rejected.** **"+" and "−" buttons on the parked row.** Two more targets on a
strip 104 px wide, next to a destructive one. **A notch of one unit.** "One
more gram of flour" is not a thing anybody wants, and the amount one buys is
already a decided rule. **Nudging down leaving a zero.** A line asking for
none of something is a line you have to read and dismiss.

## 0073 — An ingredient's editor opens under the ingredient

**Date** 2026-08-26 · **Status** Accepted · **Relates to**
[0056](#0056--an-ingredient-is-created-where-it-is-needed-not-in-another-tab),
[0058](#0058--anything-chosen-out-of-a-library-is-searched-for)

**Context.** Tapping an ingredient opened the library form in a panel at the
top of the screen, above the search field and possibly a scroll away from the
row that was tapped. On a phone, in a library long enough to need the search,
the form appeared somewhere you were not looking and the row it belonged to
was off screen.

**Decision.** The form opens **under its own row**, and only one is open at a
time: tapping another closes the first. Tapping the open one closes it. The
"Nouveau" button still opens a form at the top, because a new ingredient has
no row to sit under.

**Consequences.** The form is rendered outside the swipe wrapper and inside
the `<li>`. Outside, because a form that slid sideways with the row would be
unusable the moment a finger wandered; inside, because it belongs to the row
and the two have to read as one block when a screen is scrolled.

`Ingredients.svelte`'s `li button` styles keep matching only the row button:
Svelte scopes a selector to its own component's markup, and the form's buttons
belong to `IngredientForm`. That is luck rather than design and is worth
knowing before either file moves.

One open editor rather than a set, because two forms over one library are two
answers to "what am I editing" — and the draft they bind to is one object.

**Rejected.** **A modal.** It is the whole library form, which is longer than
a phone screen, and a modal over a scrollable list is where the keyboard
problem of 0040 is worst. **Keeping the panel at the top and scrolling to
it.** It is still not where the finger was, and the row it belongs to is still
off screen. **Several open at once.** See above.

## 0074 — A tab opens cold

**Date** 2026-08-26 · **Status** Accepted · **Supersedes the scroll half of**
[0003](#0003--ios-ships-as-a-pwa) · **Relates to**
[0073](#0073--an-ingredients-editor-opens-under-the-ingredient)

**Context.** Each screen remembered where it had been left — which recipe was
open, how far down it was scrolled — and coming back to a tab put you in the
middle of whatever you had been doing there, possibly hours earlier. The
recipe you read on Tuesday was still open on Thursday, and the shelf came back
scrolled to a row you no longer cared about.

The offsets were also a mechanism with a subtle bug in it: a `scroll` event
arrives a frame after the scrolling, so the outgoing screen's last scroll was
attributed to the incoming one, and a `#settling` flag existed only to
suppress that.

**Decision.** Switching to a tab opens it **cold**: nothing selected, no
search, at the top. Tapping the tab you are already on does the same, which
is also the way out of a recipe without hunting for the close button.

Most of it is free — every screen lives inside an `{#if}` in `App.svelte`, so
switching away destroys the component and its local state. Two things are not,
and `Session.show` handles both: the open recipe is **core** state
(`OpenRecipe`, device-local but persisted), so it is closed with a command;
and the scroll offset belongs to the window, so it is set to zero.

**Consequences.** *Which* screen you were on is still remembered across a cold
launch — that half of 0003 stands, and it is the one that mattered: an iOS
reload mid-shop must not drop you on the cart when you were on the list. What
is dropped is the offset within it, and the whole `#offsets` / `#settling`
mechanism with it.

`history.scrollRestoration` stays `'manual'`, because the browser's own
restoration aims at a document that does not exist yet — this one renders
after the wasm core has loaded, so what it would restore is an offset into a
page that was empty at the time.

**Rejected.** **Resetting the tab as well**, so every launch lands on the
cart. That is the flaw 0003 fixed. **Keeping the offset and only closing the
recipe.** Half of the surprise is the scroll: a shelf that comes back
mid-scroll reads as a shelf that has lost your place, not one that kept it.
**A timeout — cold after an hour.** A rule nobody can see, which is worse than
either behaviour on its own.

## 0075 — `main` may not advertise a version nothing published

**Date** 2026-08-26 · **Status** Accepted · **Relates to**
[0010](#0010--the-relay-ships-as-a-home-assistant-os-add-on),
[0049](#0049--the-add-on-is-cross-compiled-here-and-never-built-on-the-pi)

**Context.** `cabas-relay/config.yaml`'s `version` is not a label. It is the
image tag the Supervisor pulls, and the Supervisor reads it off this
repository's **default branch**, continuously — so a bump landing on `main`
offers that version to every installed add-on from that moment. Publishing an
image, on the other hand, happens only from a `vX.Y.Z` tag (0049).

Nothing connected the two. `0230fdb` bumped the workspace to 0.6.1 and was
never tagged; the Pi was offered 0.6.1 for a day and could not pull it,
because `ghcr.io/…:0.6.1` never existed. The `image` job knew at the time and
said so —

> `::notice::0.6.1 is a release version — publish it from its vX.Y.Z tag, not
> from main`

— in a log nobody reads. That is the actual failure, and it is the same one
this repository has already written down once: *a red gate that goes unread is
worse than an absent one* (ROADMAP's own opening). A green run with a notice in
it is the same thing wearing a better colour.

**Decision.** A job, `main advertises a released version`, on pushes to `main`
only. A `-dev` version passes — it publishes its own image. A release version
must have a `vX.Y.Z` tag or the run is red, with a message naming the command
that fixes it.

Three choices inside that, each of which matters:

1. **It checks the tag, not the registry.** If the tag exists, its own run
   either published the image or went red trying, and *that* run is the one to
   look at. Polling ghcr from here would be a second place to be wrong about
   what shipped, and it would go red for the ten minutes a release takes to
   build.
2. **The tag need not point at HEAD.** A docs commit landing after a release
   leaves the version untouched and already published, which is fine. What
   must never exist is a version with no tag anywhere.
3. **It waits three minutes before failing.** A release is normally two pushes
   seconds apart — the branch, then the tag — so this run can legitimately
   start before the tag ref exists, and failing a correct release is how a
   gate gets ignored. A genuinely stranded version is still stranded three
   minutes later.

**Consequences.** Bumping the version and tagging it become one act; push them
together. A bump that is not ready to ship has to carry `-dev`, which is what
the `image` job already assumed and nothing enforced.

The job needs no Nix and no cache — it is a `grep` and a `git ls-remote` — so
it costs seconds and cannot be the reason somebody stops reading CI.

**Rejected.** **Leaving it as a notice.** That is the state that produced the
bug. **Making `main` always carry `-dev`.** A `-dev` version publishes an
image, so the Pi would be offered development builds continuously — on the
appliance actually used to do the shopping. **Blocking the *tag* run instead**
— by then the branch has already been advertising the version for however long
it took to notice. **A scheduled job that reconciles `main` against ghcr
nightly.** It would find this, a day late, in a place nobody is looking; the
push is when the mistake is made and the push is where it belongs.

## 0076 — The library travels as a JSON file of the app's own inputs

**Date** 2026-08-26 · **Status** Accepted · **Relates to**
[0010](#0010--the-relay-ships-as-a-home-assistant-os-add-on),
[0024](#0024--attribution-is-declarative-not-cryptographic),
[0033](#0033--one-state-pushed-whole-rebuilt-from-the-document),
[0034](#0034--a-broken-reference-is-a-warning-not-an-empty-screen),
[0062](#0062--a-photo-is-a-blob-beside-the-document-never-in-it),
[0071](#0071--a-shop-is-a-name-and-the-cart-is-one-trip-per-shop)

**Context.** There are three copies of the library — both phones and the
relay's log — and they are all *live*. They answer one question, "a device
died", and no other: a recipe deleted by accident reaches all three in
seconds. The dated copy is Home Assistant's backup (0010), which lives on the
appliance, is opaque, is encrypted with a key kept on paper, and restores **the
relay** rather than a library.

Four things nobody could do, and they are not variations of one another:

1. hold a copy of the library themselves, off the Pi, without asking anybody;
2. type or correct a library on a keyboard rather than with a thumb — the
   library is the part of this app that costs hours to enter;
3. carry everything into a **new group**, which has already happened once: M6
   moved both phones to the permanent origin, and starting a new group there
   meant retyping what had been typed;
4. send recipes to somebody outside the group at all.

They turn out to be one file, which is why there is one mechanism and not
four.

**Decision.** A JSON file, `cabas.library`, holding the shops, the ingredients
and the recipes, with the photos optionally beside them. Five choices inside
that, and each of them is the reason the thing is worth having:

1. **The file *is* the app's own inputs.** Every entry is an
   `IngredientInput`, a `RecipeInput` or a `ShopInput` — the shapes
   `SaveIngredient` and its two siblings already take. There is no second
   vocabulary for the same things and therefore no second set of rules to keep
   in step; a field added to a form is in the file the day it is added. It also
   settles what an amount is: text, `"1,5"` or `"1/3"`, exactly as in a form,
   because a JSON number is a float and Rule 4 does not allow one near a
   quantity. Rendering goes through `number::render_lossless` for the reason an
   edit form does — a rounded amount that gets written back is a quantity that
   quietly changed.
2. **Ids travel; names are the fallback.** An entity carries the id it has
   here, so re-importing this group's own file updates in place — that is what
   makes the file a backup rather than a duplicator. A file from *another*
   group carries ids this document has never seen, so anything that resolves to
   nothing by id is resolved by **name**, through the domain's own matchers.
   That is what keeps somebody else's "Farine" from landing beside ours, and it
   is what lets a file be written by hand: name things and leave the ids out.
   The domain grew `Recipe::matches` and `recipe::resolve` for it, beside the
   two that already existed.
3. **An import merges, and never deletes.** What the file holds wins over what
   is here — including the name that matched it — and what it does not mention
   is left exactly as it was. This is not caution. A delete under a CRDT is a
   **group-wide** fact: an import that pruned would reach through the relay and
   take a recipe off the other person's phone, so one person opening a file
   would be deciding for two.
4. **Photos ride in the same file, base64, and only when asked.** Off by
   default, and that default is the design: without them the file is tens of
   kilobytes and opens in a text editor, which is what makes it a document you
   can correct; with them it is a backup measured in tens of megabytes (0062
   carries the arithmetic). A photo this device does not hold is skipped in
   silence — the entity keeps naming it, so the file stays truthful about what
   exists rather than pretending the picture is gone.
5. **Nothing is written until everything is decided, and nothing is written to
   the log.** There is no transaction under the document, so the import parses
   every value and resolves every reference before the first `put` — which is
   the only way "the import failed" can mean "nothing happened". And it records
   no events: the log is capped at 200 entries (0024) and one file can carry
   more ingredients than that, so routing an import through the ordinary save
   would push out every deletion the log exists to remember. "Imported" has no
   subject to be recorded against either, without widening the persisted
   `subject_kind` — a trade `save_shop` already declined for a row nobody
   reads. The receipt is handed straight back to the screen instead.

**Consequences.** `crates/app/src/transfer.rs` holds the format, the export
projection and the reference rewriting; `App::export_library` and
`App::import_library` are the two doors, both synchronous, with the photos
awaited by the host on either side of them — the same split `putPhoto` already
has, and for the same reason (0032). `save_ingredient`, `save_recipe` and
`save_shop` split into a `*_from` that validates and a caller that writes, so
the import reuses every rule and none of the logging. `Photos::restore` stores
under an id minted elsewhere, which is also exactly what M10's transfer half
will need for a photo fetched from the relay.

Two dependencies enter the registry: `serde_json` for the text and `base64`
for the photos. The second is a crate rather than thirty hand-written lines —
unlike the QR encoder of 0047 — because decoding is the half that meets a file
that came from somewhere.

`format_version` is the file's own, separate from the app's version (Rule 15),
because they answer different questions: the app's says which build wrote the
file, this one says whether a build can read it. A file from the future is
refused with a message naming both numbers; an older one is read, which is the
whole point of having it.

**This is not a substitute for Home Assistant's backup.** It carries the
library and nothing else: no list, no cart overlay, no roster, no devices, no
event log, and none of the relay's log or its epoch. Losing the Pi still needs
the appliance's archive and the key beside the twelve words. What this adds is
a copy of the expensive half that a person holds, reads and can correct.

**Rejected.** **The Loro snapshot as the file** — exact, and about ten lines of
code. It is opaque: it cannot be read, cannot be edited, and cannot be merged
into another group, since it carries a history and a set of peers as well as a
library. It answers the first want and none of the other three, and the relay's
side of it is already what an appliance backup holds.

**A preview of what will change, before it lands.** Wanted, and cut for now:
the file picker is already the deliberate act, nothing is ever deleted, and
importing the *right* file afterwards corrects a wrong one — because the file
wins. If picking the wrong file ever costs somebody an evening, this is the
entry to supersede.

**Selecting which recipes to export.** The whole library goes, and a file
trimmed by hand is what sends one recipe. A selection screen is UI for
something done twice a year, and the file being plain JSON is what makes the
manual version possible at all.

**Deleting what the file does not mention** — "make my library look like this
file". It is the one shape of import that can destroy something, and under a
CRDT it destroys it for both people at once.

**A zip carrying the photos beside the JSON.** One file is what a share sheet
moves and what a file picker hands back; a PWA has no unzip it can reach
without another dependency, and the toggle already gives the small readable
file to whoever wants one.

**CSV, for the spreadsheet half of want 2.** A recipe is a tree — lines,
sub-recipes, and prose that points at lines (0022) — and flattening it would
either lose the references or invent a second format to carry them.

## 0077 — The list is where an amount is changed, too

**Date** 2026-08-26 · **Status** Accepted · **Extends**
[0072](#0072--the-gesture-keeps-counting-and-holding-a-row-types-the-amount) ·
**Relates to** [0066](#0066--an-ingredient-knows-how-much-of-it-one-buys),
[0059](#0059--the-list-shows-what-is-missing-and-a-recipe-joins-it-from-there)

**Context.** 0072 put the whole of "how much" on the **shelf**: swipe for a
notch, hold for the exact amount. It left `Liste` as it was, and `Liste` is
where the answer is actually read — it is the screen that says what has been
asked for. A recipe entry there could be rescaled in place ("we are six
tonight"), a bare ingredient could only be *looked* at: changing 500 g to a
kilo meant going back to the shelf, finding the row again, and holding it.
Worse, it was reachable only from the shelf the ingredient is on, so an entry
whose ingredient had been renamed or hidden behind a search was a number
nobody could edit at all.

**Decision.** A bare ingredient's line carries the same two affordances the
shelf row carries, as buttons rather than as a gesture: **"−" and "+" for one
notch**, and **the amount itself is a door to the exact one** — the same
`AmountDialog` the long press opens, seeded from the line's lossless `edit`
rather than its rounded `quantity`.

Both go through the commands 0072 already added — `NudgeListEntry` and
`SetEntryQuantity` — so no core changed, and the arithmetic stays where it
was. In particular **"−" past the last notch takes the row off the list**,
because that is `Nudged::Off` and the frontend does not get a second opinion
(Rule 9). It reads the same as pressing "×" and lands in the log the same way.

The recipe entry keeps its own stepper unchanged: it counts **people**, not
whole recipes, because that is what the list's "− 4 pers. +" has always meant
and it is the question this screen asks.

**This reverses 0072's rejection of "+/− buttons"**, and only where its
reason does not apply. That rejection was about the *parked shelf row*: two
more targets on a strip 104 px wide, beside a destructive one. A list entry is
a full-width card that already carries this exact stepper on its other kind of
line — there is room, and the shape is already there to copy.

**Consequences.** The list screen gains the dialog and the two commands, and
nothing else in the app moves. The amount is a `<button>` and not a field:
in-place editing would need a form, a commit and a cancel on every row, and
the panel already exists.

`AmountDialog` is now opened from three screens, which is what makes it a
component rather than something the shelf owns.

**Rejected.** **Editing the amount inline on the row.** A text field per line
on the one screen that is read at a glance, each needing its own save — and
the unit still would not fit. **Making the recipe row's "4 pers." a door to
the same dialog.** The dialog's serving half is a stepper, which is what the
row already is; the door would open onto itself. **A notch of one unit here,
whatever the shelf does.** Two meanings for "+" in one app, decided by which
screen you are on.

## 0078 — No field is small enough for iOS to zoom at

**Date** 2026-08-26 · **Status** Accepted · **Relates to**
[0003](#0003--ios-ships-as-a-pwa),
[0040](#0040--the-keyboard-is-a-length-not-a-mode),
[0010](#0010--the-relay-ships-as-a-home-assistant-os-add-on)

**Context.** Tapping into any field on the iPhone zoomed the page towards it,
and left it there — iOS magnifies a focused control whose text is under 16px
and does not undo it when the keyboard goes away. Panning back out by hand,
mid-shop, after every search box.

Almost every field in the app was under the line without anybody choosing it:
`app.css` gives controls `font: inherit`, and most of them sit inside a
`<label>` at `--text-sm`, which is 14px. The two fields that were explicitly
`--text-base` — pairing's phrase box, "qui êtes-vous ?" — were the two that
never zoomed, which is exactly the shape of an accident.

**Decision.** `input`, `select` and `textarea` are floored at
`max(var(--text-base), 1em)` in `app.css`. `1em` is the inherited size, so a
control that is deliberately larger stays larger and only the small end is
lifted. It is one rule in the one file that is allowed to hold measurements
(Rule 10), rather than a correction repeated in twenty components.

**The viewport is deliberately not touched.** `maximum-scale=1` and
`user-scalable=no` stop the automatic zoom by forbidding **all** zoom,
pinch included — and pinch-zoom is an accessibility feature that 0003's
`index.html` comment already says this app keeps. Safari has also ignored
those attributes on and off across versions, so it is a fix that is both
harmful and unreliable. The 16px floor is neither: it is what iOS actually
reads.

**Consequences.** Every field in the app is 16px, which is a visible change —
the search boxes, the ingredient form, the recipe editor's lines. The one
place it could have cost something is a recipe line on a 390px screen, where
an amount, a unit and a picker share a row; `ui-test` already asserts that
nothing there overflows sideways and that the three share a right edge, and
it still does.

The label above a field stays at `--text-sm`: the hierarchy is still there,
it is just the other way round from what it was.

**Rejected.** **`maximum-scale=1` in the viewport meta**, for the reason
above. **Bumping `--text-sm` to 16px.** It is a token used for labels, meta
lines and hints all over the app — that is a redesign, and the fields are what
the problem is about. **Scaling the page back with a `focusout` handler.**
Fighting the platform in JavaScript, on the one interaction that happens
hundreds of times per shop.

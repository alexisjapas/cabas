# cabas — session guide

Offline-first shopping list and recipe manager for two people and their
phones. Rust core (domain, storage, sync, relay), Svelte frontend shipping as
an installed PWA on iOS/web and as a Tauri app on Android/Linux.

**Binding rules** in [CONSTITUTION.md](CONSTITUTION.md); **plan and status**
in [ROADMAP.md](ROADMAP.md) ("Resuming work" section); **why every choice was
made** in [docs/DECISIONS.md](docs/DECISIONS.md).

**Current state**: **M0 through M5 complete** — the exit criterion was met on
an iPhone and a Pixel 8, which pair with twelve words and converge both while
both are open and while neither is ever open with the other. **M6 is under
way**: the Svelte bundle is compiled into `cabas-relay` by its build script, so
one binary serves the app and `/sync` on one origin (DECISIONS 0048), and that
binary is cross-compiled to static musl and published by CI as a Home Assistant
add-on image — `repository.yaml` and `cabas-relay/` make this repo an add-on
repository (DECISIONS 0049). The abandoned group log is settled too: forgotten
by hand or not at all, through `cabas-relay groups` / `forget` (DECISIONS
0050). **That add-on runs on the Raspberry Pi and the app is on the internet**,
at `https://cabas.cladelabs.com` through a Cloudflare Tunnel — which is
therefore **the permanent origin** (0012) and the only address a phone may be
installed from. Verified end to end from outside: TLS, the bundle, no `Vary`,
the service worker registering, and a silent socket held open by the relay's
keepalive (0051). One cache rule keeps the edge from re-TTLing `/sw.js`
(0052), and **both phones are installed from that origin**. **Next is the
backup schedule and the restore drill**, which is all that stands between here
and M6's exit criterion — Home Assistant's own backups, scheduled, retained and
stored somewhere that is not the Pi's SD card, with its encryption key written
down beside the twelve words (README, "Backups"). The drill's procedure is
written too (README, "The restore drill"), and writing it found two real bugs,
one per direction of the same restore — a restored log
left every device stranded behind a cursor the epoch could not invalidate
(0.1.1, 0053), and the shadow on the other side left the rolled-back window
unpushable, stranding any device that had missed it (0.1.2, 0054).
`ui-serve` and its hand-rolled certificate authority are development-only from
now on. **M10 — photos — is under way, alongside M6** (ROADMAP says why that
rule is bent): one photo per recipe and per ingredient, **beside the document
and never inside it**, because every save rewrites the whole document and a
photo library in it turns a tick in a shop into a multi-megabyte write
(DECISIONS 0062, which carries the measurement). Half one is in **0.5.0** —
taken, stored, displayed on one device; **0.6.0 adds importing one from the
device** as well as taking it (0065). **Half two was client-side first**: the
photo protocol (`crates/sync/src/photo.rs`,
DECISIONS 0080) and the client that speaks it — `PhotoSession` in
`cabas-sync`, sans-IO, and `PhotoSync` in `app::photos`, which is that client
met with this device's store. **0.11.0 closes it** (0092): the relay serves
`/photos` out of a per-group directory of sealed blobs beside the log
(`crates/relay/src/photos.rs`), and `PhotoTransfer` in
`ui/src/lib/photos.svelte.ts` drives it — a **second socket** beside the sync
one, because a photo is hundreds of kilobytes and a tick in a shop must not
queue behind it. A photo taken on one phone is on the other; `ui-test` asserts
it where it used to assert the opposite. What is left of M10 is local cleanup
and persisted storage, neither of which gates the exit criterion.

**0.6.0 also changed four things about the app's own shape**, none of them on
a milestone. **A family is a group** (0063) — in the code, in the relay's
`groups` subcommand and on screen; the phrase derivation, the relay's `/data`
and this file's DECISIONS history are untouched, so nothing was unpaired and
no log was rewritten, and every entry before 0063 says "family" and means
group. **The tabs run left to right in the order the app is used** —
`Réglages · Ingrédients · Recettes · Liste · Courses` (0064). **An ingredient
carries the quantity one usually buys** (0066), which exists because **a shelf
row goes on the list by being dragged across it** (0067) and a gesture has
nowhere to put an amount. And **a device joins a group and only then says who
is carrying it** (0068) — the one that fixed a defect rather than adding
anything: joining used to mint a *new* user, so two phones belonging to one
person put two of that person in the roster, for good.

**0.7.0 is six more of the same**, all off-milestone, all about the shape of
the app rather than its plumbing — and the first three change the domain:

- **The aisles are this group's shop** (0069). Twelve again, chosen by the
  people who walk them: no butcher, no fishmonger, and `Items` retired into
  `Foyer · Soin & santé · Artisanat & jardin`. `SCHEMA_VERSION` does not move,
  but **`store::codec` still reads the five retired spellings** — without that
  mapping every existing ingredient would silently decode to `Autres` and the
  discovery would happen in a shop.
- **An ingredient says where it is kept** (0070) — frigo, congélateur, or
  neither, on the ingredient and not on its aisle, and shown on the cart line
  because that is the row on screen when the bags are emptied.
- **A shop is a name, and the cart is one trip per shop** (0071). Ingredients
  name the shops they are sold at, `Courses` offers a chip per shop and folds
  the rest under "Ailleurs", and **an ingredient with no shop belongs to every
  shop** — the half that keeps an unclassified line from being lost. Shops are
  created in the field that needs them, on the ingredient's own form.
- **The gesture keeps counting** (0072): the parked row shows what it asks
  for, right adds a notch and left removes one, left past the last one takes
  the row off the list — and **holding a row opens the exact amount**, which
  reverses 0067's rejection of long-press for reasons that entry gives. What a
  notch is worth is `domain::list`'s: the usual shopping quantity, or one whole
  recipe as written (a tart for four goes 4 → 8 → 12).
- **An ingredient's editor opens under its own row** (0073), one at a time.
- **A tab opens cold** (0074) — nothing selected, no search, at the top. The
  persisted *screen* stays (0003); the scroll offset within it is gone, and so
  is the machinery that kept it.

**0.8.0 is one more, also off-milestone: the library travels as a file**
(0076). `Réglages · Données` writes the shops, the ingredients and the recipes
as one readable JSON file — photos with them if the toggle is on — and reads
one back. The file **is the app's own inputs** (`IngredientInput`,
`RecipeInput`, `ShopInput`), so an import is the ordinary save run over what
the file says and there is no second vocabulary; **ids travel and names are
the fallback**, which is what merges somebody else's "Farine" into ours
instead of beside it; and **an import never deletes**, because a delete under
a CRDT is a group-wide fact and one person opening a file must not decide for
two. It is not a substitute for the appliance backup — no list, no roster, no
relay log — but it is the only dated copy that lives off the Pi.

**0.9.0 is two, both off-milestone and neither touching the core.** **The
list is where an amount is changed, too** (0077): a bare ingredient's line
carries "−" and "+" for one notch and the amount itself is a door to the
exact one, which is 0072's shelf gesture rendered as buttons on the screen
that is actually read. It reverses 0072's rejection of "+/−" only where that
reason does not hold — a parked shelf row is 104 px wide, a list card is not
— and it uses `NudgeListEntry` / `SetEntryQuantity` unchanged, so "−" past
the last notch still takes the row off the list because that is the core's
rule. And **no field is small enough for iOS to zoom at** (0078): tapping any
control under 16px magnified the app and left it magnified, so `app.css`
floors `input`, `select` and `textarea` at `max(var(--text-base), 1em)`. The
viewport meta is untouched on purpose — `maximum-scale=1` stops the zoom by
forbidding pinch-zoom as well.

**0.9.1 is what reviewing 0.9.0 found** (0079), and the first of it is in the
core: **changing what a line asks for purges its tick**, the same way adding
it by hand has since 0019. `set_entry_quantity` and `nudge_list_entry` wrote
straight to the document, so asking for more of something already ticked left
it ticked — folded away under "Terminées", bought as far as the cart was
concerned. `ShoppingList::update` is `add`'s mirror and `App::update_entry`
is `add_entry`'s; every command that rewrites a line goes through it,
`SetEntryServings` included. On screen, **a recipe's line is the same control
as an ingredient's**: the same three buttons, one notch being one whole
recipe as written (4 → 8 → 12), and the exact number of people behind the
amount rather than a "−" that clamped at one person while the identical
button on the row above emptied the row. Four smaller things with it — a
refused command is drawn **above** the panel it was refused in (the `--layer-*`
scale in `app.css`), every accessible name on those rows carries the row and
the value it is showing, the controls are `--tapsize` in both directions, and
`.notch` is not `.step` because `RecipeEditor` already has one.

**0.10.0 is the look** (0081), and it is the entry 0026 was written to make
cheap: the vanilla palette is replaced by the Cabas one — a checked
tablecloth as the page, **cream bubbles** carrying what is read, **lilac
tiles** carrying what is filled in, an **anis title band** on every screen,
**pink pills** for every quantity and every photo ring, and **Shrikhand**
leaning at −8° over **Quicksand**, both OFL, self-hosted under
`ui/public/fonts/`, precached by name in `vite.config.ts` and preloaded by
`index.html`. Nothing functional moved. Four rules came with it and each is a
mistake already made: **the cloth never carries a word** (hence the one
global `.bubble` class); **orange, pink, anis and lilac are surfaces, never
inks** (so every `color: var(--accent)` became `--accent-strong`); **the
display face on anis or apricot is `--display-ink`, and `--text` on a lilac
tile**; and **the focus ring is `--text`**. Two shapes are the whole layout
vocabulary — *a form is a lilac tile and its fields are cream*, *a row that is
read is a cream card spaced by `--space-2`* — and **dark mode is dropped
outright** rather than inverted. With it, one layout change with its own entry
and a number behind it: **the keeping badge goes under the name on a cart
line, never beside it** (0082), because beside it the name is left about 43 px
on a 390 px phone.


**0.11.0 is one session of eleven requests**, and every one of them has an
entry. Three are structural and the rest are the shape of the app:

- **Photos reach the other phone** (0092) — M10's half two; see above.
- **Half a recipe goes on the list** (0091). `ListItem::Recipe` gains
  `only: Option<BTreeSet<UsageId>>` — `None` is the whole recipe and is what
  every gesture produces. The entry stays a *recipe* entry: measured in
  people, and **rescaling scales the lines it kept and no others**, which is
  the whole reason it is a field on the entry rather than a handful of bare
  ingredients. `expand_only` is the domain's one new function; the store's
  `only` key is additive, so `SCHEMA_VERSION` does not move and an older
  build buys the whole recipe. The reader's ingredient list is a list of
  toggles, and its primary button says "Ajouter le reste à la liste" while
  some of it is already asked for.
- **An ingredient is usually bought in a unit as well as an amount** (0089),
  which it always was in the document and was not in the form: a unit chosen
  over an empty amount used to be discarded and now means one of it. And the
  list's own add form seeds both fields from it, which is the one place an
  ingredient was asked for by hand and its usual quantity was ignored.
- **A list row is one line, and the unit is behind the amount** (0090): one
  pink pill holding "−", the amount and "+", sharing its line with the meta.
  Three bordered pills on a row of their own cost a third of a card to say
  "500 g". The amount is the door to the exact amount **and its unit** — the
  command always took both, but nothing said so.
- **One cloth per tab** (0084), **quieter than it was** (0085), and **it
  scrolls on Android too** (0083). The last is one line of CSS that iOS
  ignores and Android honours, so the same build had a moving cloth on one
  phone and a still one on the other. Each palette clears 5.2:1 against
  `--display-ink` and stays under about 1.5:1 against itself — the two
  numbers any new cloth has to meet.
- **A dish is shown at the size a dish is chosen by** (0086): a third photo
  size, `--photo-dish`, square rather than round, on the recipe shelf.
- **A saved ingredient says where it went** (0087): the shelf scrolls to it
  and flashes it once, because the editor opens under its row and saving
  collapses a tall panel.
- **The finished bar throws confetti** (0088) — on the *transition* into a
  full bar, never on arriving at a tab where it already is.
**M9 — history and statistics — is scheduled
before M7**: what the
group buys and how often, recorded at `FinishShopping` and derived from
there, kept forever with the footprint shown in Settings (DECISIONS 0061).
The milestone numbers are names, not the order; ROADMAP says why.

`crates/domain` holds the product logic as pure functions (90 tests);
`crates/store` holds the Loro schema, the two-way
mapping, snapshots, compaction and the `Storage` trait over file +
IndexedDB — **plus `PhotoStore`, a second trait over one record per photo**,
which is where photo bytes live because `Storage` is one blob and that blob is
rewritten on every save (DECISIONS 0062); `crates/app` holds the command set,
the view-models and the wasm
binding — **including the sync session** (`app::sync`, and `sync*` on
`CabasApp`), **the photo library** (`app::photos`, and `putPhoto` / `photo`
on `CabasApp`) **and the library's file form** (`app::transfer`, and
`exportLibrary` / `importLibrary`); `crates/sync` holds the E2EE core
(phrase → key, seal/open, the wire protocol, the sans-IO client `Session`);
`crates/relay` is a working
axum broker persisting sealed frames per group **and serving the PWA out of
its own binary**, **and brokering photos on `/photos`** (0092). 290 native
tests plus 22 in
a real browser — 9 over IndexedDB and the photo store, 13 through the app —
and all of them run
in CI. Two tests at replica level carry the two milestones:
`crates/relay/tests/convergence.rs` is M5's — two devices never online
together converge through the relay, sealed end to end — and
`crates/relay/tests/photos.rs` is M10's, the same two devices ending up
holding each other's pictures. The phones then answered for themselves.

`ui/` is a working Svelte 5 app: identity, the cart, the list, the recipes
(list, reader and editor), the ingredient library and settings, driven end to
end by `ui-test` in headless chromium. **Every command is reachable from the
UI**, the app is **installable**, **it opens with the network off**, and the
soft keyboard no longer covers what is being typed into. Since 0.3.0 the
library form opens **where an ingredient is wanted** — the list's picker and
each recipe line's, through `components/IngredientPicker.svelte`, which owns
the field, its door and the form behind it as one mechanism (DECISIONS 0056)
— and there is an `Items` aisle for what is bought whole and never
cooked (0057). Since 0.4.0 **anything chosen out of a library is searched
for**: `components/SearchPicker.svelte` replaces every `<select>` over a
library with a field that filters and the matches under it,
`components/SearchField.svelte` does the same over the two shelf screens, and
every list of names on screen is alphabetical in French (0058). The list
screen took the cart's shape in the same breath — a settled entry folds away
below what is still missing, and a recipe reaches the list *from* the list
(0059). Since 0.4.1 **what was searched for is what gets created**: the
picker's door carries the query into the form and says so on its label, and
both shelves offer the same thing under a search that found nothing (0060).
Since 0.6.0 **a shelf row goes on the list by being dragged across it** —
`components/SwipeToAdd.svelte` wraps one row, and "Annuler" stays uncovered on
its left for as long as the *list* holds the entry, which is where the state
is read from (0067) — and **the first launch asks who you are only once the
roster has arrived**, in `screens/Identify.svelte`, which is also what
Settings' "Changer d'utilisateur" opens (0068).
`ui-serve` serves the built bundle over TLS from a local CA, which is what
makes the app installable
on a phone at all (DECISIONS 0041). **It is installed on the iPhone**, it opens
in airplane mode, its library survives a cold restart, the cold start is
instantaneous and the keyboard behaves as designed — M4's exit criterion, met on
the device. **The sync engine and pairing are in too**: `lib/sync.svelte.ts`
holds the socket, the foreground rule, backoff and the cursor; `screens/
Pairing.svelte` starts or joins a group and `screens/Settings.svelte` shows
the phrase for a second phone. `ui-test` runs the lot against a real relay and
proves the milestone at browser level — a device pushes its library, loses its
replica, gets everything back from the relay alone, another joins by typing the
twelve words, the roster behind Settings shows both, and the journal shows what
each of them did — sealed throughout. **Every screen M5 asked for exists**, and
the milestone is closed on an iPhone and a Pixel 8. See ROADMAP "Next action"
for M6.

## Environment and commands

Everything goes through the nix flake (`cargo` does not exist outside the
devShell):

```sh
nix develop -c cargo nextest run --workspace --exclude cabas-tauri
nix develop -c cargo clippy --workspace --exclude cabas-tauri --all-targets -- -D warnings
nix develop -c cargo fmt --all
nix develop -c wasm-check                     # Rule 8: the 4 shared crates on wasm32
nix develop -c check-wasm-bindgen             # Rule 13: CLI/crate version match
nix develop -c check-addon                    # M6: the add-on manifest vs the workspace
nix develop -c build-relay aarch64            # M6: the static binary the image copies
nix develop .#wasm-test -c wasm-test          # store + app, in headless chromium
nix develop .#android                         # Android SDK/NDK shell + the Tauri CLI (M7)
nix develop .#android -c tauri-check          # M7: cabas-tauri, for aarch64-linux-android

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
nix develop -c ui-serve                       # ui/dist over TLS, for the phone

nix develop .#wasm-test -c node ui/tools/render-icons.mjs   # the PNG icons
```

`ui-serve` is the only way onto the phone: a service worker needs a secure
context, so the LAN address `pnpm dev` prints can display the app and never
install it. It mints a local CA once into `ui/.certs/` (gitignored), signs a
certificate for `<hostname>.local` and the LAN IP, serves `ui/dist` on 8443 and
hands the CA out over plain HTTP on 8080 — the phone cannot fetch it over the
HTTPS it does not trust yet (DECISIONS 0041). It also proxies `/sync` on the
same origin to the relay (`CABAS_RELAY`, default `127.0.0.1:8787`), which is
how a phone reaches a development relay at all (DECISIONS 0044).

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
so run it after touching **any of the seven files that carry `ts(export)`** —
`command.rs`, `view.rs`, `tags.rs`, `platform.rs`, `sync.rs`,
`transfer.rs` and `photos.rs` (which joined the list in 0092, for
`PhotoEvent` and `PhotoStatus`). `grep -rl 'ts(export)' crates/app/src` is
the authoritative list.

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
| `crates/store` | Loro schema, snapshots, `Storage` (the document) and `PhotoStore` (the photos), each over file / IndexedDB |
| `crates/sync` | E2EE, pairing, WebSocket transport |
| `crates/app` | Commands + view-models — the only surface the UI touches |
| `crates/relay` | Sync broker + PWA host, shipped as an HA add-on |
| `cabas-relay/` | The add-on: manifest, base images, Dockerfile, its own docs |
| `src-tauri/` | The Android and Linux host — the same core over `invoke` (0093). A workspace member, deliberately not a `default-member` |

Every crate holds code since M5's first half. `crates/sync` — read
`protocol.rs` first, it is the wire contract and carries the reasoning
(DECISIONS 0042):

| Module | Holds |
|---|---|
| `key` | `GroupKey`, `GroupId` — both derived from the 12-word phrase's BIP39 seed |
| `seal` | XChaCha20-Poly1305 `seal`/`open`, the only cipher anywhere (Rule 7) |
| `protocol` | `ClientMessage`/`ServerMessage`, `FrameKind`, the postcard codec |
| `photo` | The photo protocol (0080) — `PhotoName`, a `Hello` carrying what this device has and wants, a `Welcome` answering with what to upload and what is available, one sealed photo per message after that. Its own version byte, on `/photos` |
| `session` | `Session` — the sans-IO client: cursor, epoch reset, seal/push, one `Event` per wire message |
| `photo_session` | `PhotoSession` — the sans-IO photo client: the hello it offers, the two queues the welcome fills, one `PhotoEvent` per wire message. Stores nothing it did not ask for (0080). Spoken to by `crates/relay` since 0092 |
| `error` | `SyncError` — no vendor type crosses the boundary |

`crates/relay` (binary + lib, never in `wasm-check`): `log.rs` is one
group's persisted sealed log — append, replay, snapshot-truncate, torn-tail
recovery, the minted `epoch` — and `server.rs` is the axum WebSocket side:
replay under the same lock as the subscription, then live forwarding, plus a
30-second ping so the tunnel does not close a socket for having nothing to say
(DECISIONS 0051). A device's cursor is honoured only when it names the log's
epoch **and** points inside it — the second half is what a restored backup
needs (0053). It depends on `cabas-sync` for the protocol types and never
for a key. `photos.rs` is the other half of a group's directory: one file per
`PhotoName` holding sealed bytes, an index of names and sizes read once at
open, and two caps that reject **a photo** rather than the connection
(DECISIONS 0092). It is not a log — no sequence, no epoch, no replay — and
`server.rs`'s `/photos` handler is correspondingly short: a hello, a welcome
carrying two set differences, then one photo per message in whichever
direction asked for it. Naming a file after something off a socket is safe
because a `PhotoName` cannot decode unless it is ASCII letters, digits, `_`
and `-` (0080).
`assets.rs` is the static half — the PWA, served from the same origin as
`/sync`, out of a table `build.rs` wrote by walking `ui/dist` (DECISIONS
0048). It shares nothing with the sync side but the port. `admin.rs` is the
data directory as seen from a shell: `survey` and `forget`, behind
`cabas-relay groups` / `cabas-relay forget <id>`, because an abandoned group
log can only be identified by a person — the relay cannot tell one from a
quiet group, and the log is the recovery point if every device is lost
(DECISIONS 0050). Deliberately **not** an HTTP endpoint: a group id is the
only access control the relay has and the port faces the tunnel. **A missing
`ui/dist` embeds nothing and is not an error**, which is what keeps `cargo
clippy --workspace` working in a fresh checkout; the release image sets
`CABAS_EMBED_UI=required` so an image with no app in it fails on the runner.

`crates/domain`'s modules, bottom-up — each depends only on the ones above it:

| Module | Holds |
|---|---|
| `units` | `Dimension`, `Unit`, exact conversion factors, `convert` |
| `quantity` | `Quantity`, scaling, addition, `ceil_to_whole`, `humanized` |
| `ingredient` | `Ingredient`, `Aisle` (0069), `Keeping` (0070), cross-dimension conversion, `resolve`, `shopping_quantity` (0066) |
| `shop` | `Shop` — a name and nothing else; `resolve` over it, and `sold_at`, the one place "an unplaced ingredient is on every trip" is written (0071). Named `Shop` and not `Store` because `cabas-store` is a crate |
| `recipe` | `Recipe`, usages, `Segment` steps, `dangling_refs`, and `matches` / `resolve` — a recipe recognised by its name, which is what an imported file needs (0076) |
| `expand` | DAG flattening, cycle detection, `MAX_DEPTH` — and `expand_only`, which restricts a list entry to some of a recipe's own lines (0091) |
| `overlay` | `Explicit`, `CheckState`, `resolve` (state derivation) |
| `list` | `ShoppingList`, `ListEntry`, `add`/`update` — both purge the overlay (0019, 0079) — what one notch of the swipe is worth (`nudge_quantity` / `nudge_servings`, 0072), and `ListItem::Recipe::only`, the lines a half-added recipe asks for (0091) |
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
| `schema` | Container and key names, `SCHEMA_VERSION`, the layout diagram — plus `shops` (0071) and a list entry's `only` (0091), both additive like every key since |
| `codec` | `LoroValue` ⇄ primitives: rationals, units, aisles, timestamps |
| `mapping` | Domain struct ⇄ document, one pair per entity |
| `document` | `Document`: lifecycle, reads, writes, snapshots, sync bytes |
| `storage` | `Storage` trait; `MemoryStorage`, `FileStorage` (native), `IndexedDbStorage` (wasm) |
| `photos` | `PhotoStore` — one record per photo, beside the document: `MemoryPhotoStore`, `FilePhotoStore`, `IndexedDbPhotoStore` (a second object store in the same database) |
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
| `platform` | `Platform` (clock + randomness), `SystemPlatform`, `Identity` — whose user half is `None` until somebody is chosen (0068) |
| `sync` | `SyncSession` — `cabas_sync`'s sans-IO client met with the replica: merge inside, seal outside, one `SyncEvent` per wire message |
| `photos` | `Photos` — the bytes the document only names: mint an id, store, read, `restore` one under an id minted elsewhere, and the two diffs a prefetch and a sweep need (0062, 0076) — plus `PhotoSync`, that store met with `cabas_sync`'s `PhotoSession`, which the PWA drives over `/photos` (0080, 0092) |
| `transfer` | `LibraryFile` — the library as a JSON file of the app's own inputs, the reference rewriting an import needs, and `ImportReport` (0076) |
| `wasm` | `CabasApp` — the PWA binding, and nothing but translation |

The shape to keep in mind: **`apply` is synchronous and returns the whole new
state; `persist` writes.** A render never waits on storage, and the borrow is
released before anything is awaited — which is what keeps a second tap from
panicking at the wasm boundary (DECISIONS 0032).

The sync surface has a shape of its own: **the socket belongs to whoever calls
it, and plaintext never comes back out.** `SyncSession::handle` merges an
opened frame into the replica and returns a `SyncEvent` — a `merged` event
already carries the whole new `StateView`, like every other mutation. The
frontend drives it through `syncHello / syncHandle / syncPush / syncSnapshot /
syncVersion / syncStatus / syncClose` on `CabasApp`, where every byte crosses
as an opaque `Uint8Array`. M7's Tauri host will drive the same `SyncSession`
from Rust, which is why the composition lives in `app::sync` and not in
`wasm.rs`.

`ui/` (M4) — plain Vite + Svelte 5, no SvelteKit (DECISIONS 0037). Read
`lib/session.svelte.ts` first, it is the state and the save policy in one
file:

| Path | Holds |
|---|---|
| `lib/bindings/` | Generated from Rust, committed, diffed by CI (0036) |
| `lib/wasm/` | Generated by `build-wasm`, gitignored |
| `lib/core.ts` | The **interface**, and nothing else: `Core`, `Host`, and a re-export of whichever implementation `$core-host` resolves to. Fully asynchronous on purpose (0093) |
| `lib/core.wasm.ts` | The PWA host — the typed edge, the only place a cast meets the wasm `any`, and the identity in `localStorage` (0031) |
| `lib/session.svelte.ts` | The one `$state.raw`, `await run(command)` — which also writes back the identity for the commands `MOVES_IDENTITY` names (0068) — the debounced flush, the persisted screen and its scroll offset |
| `lib/sync.svelte.ts` | The socket and its policy: connect on foreground, backoff, push on change, the cursor and shadow in `localStorage` (0043) |
| `lib/photos.svelte.ts` | The **second** socket, on `/photos`: opens when there may be photos to move, drains its queues one message at a time, closes when done, and bumps a counter every `<Photo>` reads (0092). Nothing is persisted between connections |
| `lib/list.ts` | What the list already holds, keyed by the ingredient or recipe it came from — the whole entry since 0072, because a swipeable shelf now asks how much as well as whether |
| `lib/qr.ts` | A QR encoder, hand-written and fixed to version 6-L — the one payload is a 12-word phrase (0047) |
| `lib/photo.ts` | A picked file into the JPEG the core takes: EXIF orientation, downscale, encode until it fits under `maxPhotoBytes()` (0062) |
| `lib/keyboard.svelte.ts` | The soft keyboard as a length — `--keyboard-inset`, and the scroll CSS cannot do (0040) |
| `lib/labels.ts` | The French for every tag the core sends, and nothing else (0035) |
| `lib/format.ts` | Rendered number meets word: decimal comma, "≈", plurals, relative time, French name order — and `fold`/`matches`, which every search filters through (0058) |
| `app.css` | The tokens, `--layer-*` among them, and the two global classes `.display` and `.bubble` (0081). No component writes a literal value, a `z-index` included (Rule 10, 0079) |
| `screens/`, `components/` | The screens, and what more than one of them needs. `Pairing.svelte` is used twice — the first launch, and Settings on a device that already runs; `People.svelte` is the roster and the only place key rotation is offered; `Events.svelte` is the log; `SearchPicker.svelte` is how anything is chosen out of a library and `SearchField.svelte` how a shelf is narrowed (0058); `IngredientForm.svelte` is the library form and `IngredientPicker.svelte` is that form behind a picker's last row, used wherever an ingredient is chosen (0056); `Photo.svelte` shows one — at `thumb`, `dish` or `full` (0086) — and re-reads it when the transfer lands one (0092), `PhotoField.svelte` takes one and imports one (0062, 0065); `Confetti.svelte` is one burst over the cart's progress bar and nothing else (0088); `SwipeToAdd.svelte` wraps a shelf row, puts it on the list and goes on counting it (0067, 0072) with `AmountDialog.svelte` behind its long press — and behind the amount on a `screens/List.svelte` row too, whichever kind of line it is, which is the third screen that opens it (0077, 0079); `ShopPicker.svelte` is where a shop is chosen and born (0071) and `screens/Shops.svelte` is where one is renamed or forgotten; `Identify.svelte` is "qui êtes-vous ?" — the first launch and Settings' user switch, one screen (0068); `screens/Transfer.svelte` is `Réglages · Données`, the whole of export and import, and it holds the delivery of the file and nothing about its shape (0076) |
| `sw.js` | The service worker: precache, one versioned cache, cache-first (0038) |
| `vite.config.ts` | The build, and the plugin that writes the precache list into the worker — plus `PUBLIC_SHELL`, the handful of `public/` files named by hand because the plugin reads the bundle and never the disk (0081) |
| `public/` | Served verbatim: the manifest, the favicon, the icons — and `fonts/`, the two OFL faces `app.css` declares (0081) |
| `tools/render-icons.mjs` | SVG → the committed PNGs, over CDP. Not part of any loop |
| `tools/serve.mjs` | `ui/dist` over TLS for the phone, plus the CA over plain HTTP (0041) |
| `tests/smoke.mjs` | The vertical in a browser, over CDP, zero dependencies — including sync, against the real relay `ui-test` starts on 8788, which also serves the bundle (0048) |

`screens/Settings.svelte` is six views behind one tab — itself, the roster,
the shops, the log, the file door and the user switch — and it is also where
the running build names itself (0055), and
`screens/Recipes.svelte` is three behind another — the shelf, the one being
read, and the one being written — and the shape is worth knowing before
touching it. Which recipe is *open* is core state (`OpenRecipe`, never
synced); the recipe being *edited* is a draft that lives in `Recipes.svelte`
and is handed to `RecipeEditor` as a `$bindable`, because a form seeded from a
prop captures the value once and ignores the next one. The draft **is** a
`RecipeInput`, so saving is one command and no mapping. `RecipeReader` scales
nothing: `focus.recipe` arrives already rendered at the current servings.

Four shapes worth knowing before editing it. The state is **`$state.raw`**,
because every command returns a whole new tree and a deep proxy would track
mutations that never happen. `run()` returns a boolean, so a form can stay
open when a command is refused. The flush is debounced *and* hooked to
`visibilitychange`/`pagehide`, because a pending timer dies with the page and
iOS backgrounds a PWA whenever it likes. And **`show()` is where a tab is
opened cold** (0074): it closes the open recipe — core state, so nothing else
would — and scrolls to the top. Everything else a screen was in the middle of
dies with the component, because each one sits in an `{#if}` in `App.svelte`.

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
- An ingredient carries **three orthogonal placements**: its `Aisle` (where it
  is found, and the cart's walking order — 0069), its `shops` (which trips it
  belongs to, empty meaning all of them — 0071) and its `Keeping` (where it
  goes at home — 0070). Collapsing any two of them was rejected in each entry
  for the same reason: they answer questions asked in three different places.
- One **notch** of the swipe is the ingredient's `shopping_quantity` or the
  recipe's own `servings` (0072). Nudging down past the last one returns
  `Nudged::Off` and the entry leaves the list; a step that cannot be expressed
  in the line's dimension returns `Refused` rather than a guess (Rule 5).

## Conventions

- Conventional Commits (`feat:`, `fix:`, `chore:`, …), in English — Rule 15.
- **Bump `[workspace.package].version` by ordinary semver, every time** —
  `fix:` → patch, `feat:` → minor, a break in the sync protocol or the
  persisted schema → major, tooling- and docs-only changes bump nothing. It
  is the semver of the *shipped artifact* (the relay image and the bundle it
  serves), not a commit counter. `cabas-relay/config.yaml` carries the same
  string and `check-addon` fails when the two disagree.
- **A version bump and its tag are one act.** `config.yaml`'s version is the
  image tag the Supervisor pulls and it reads it off `main` continuously, so
  a bump that lands untagged offers every installed add-on a version nothing
  published — which is what stranded 0.6.1 for a day. CI's
  `main advertises a released version` job fails a push to `main` carrying a
  release version with no `vX.Y.Z` tag anywhere (DECISIONS 0075); it waits
  three minutes first, because the branch and the tag are normally two pushes
  seconds apart. Push them together.
- **A release is an annotated `vX.Y.Z` tag whose message *is* the
  changelog**, and it is the only thing that ships. `main` publishes a
  `-dev` version and nothing else; a version with no `-dev` on `main`
  publishes nothing at all and says so in the job log. **A change that does
  not move the version never reaches the Pi**: the Supervisor decides an
  add-on has an update by comparing this string to the installed one, so
  republishing a tag an appliance already pulled is a no-op there — which is
  how a fix can be green in CI, present in the registry, and absent from the
  only machine that runs it.
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
- **And so does changing what a line already on the list asks for** — the
  same purge, from the other door (DECISIONS 0079). `set_entry_quantity` and
  `nudge_list_entry` wrote to the document directly and did not, so asking
  for more of something ticked off earlier in the trip left it ticked: the
  cart called it bought, the row stayed folded under "Terminées", and the
  difference turned up at home. Every command that rewrites a line goes
  through **`App::update_entry`** over **`ShoppingList::update`**, which is
  `add_entry` over `add` with the same second half; a new one that calls
  `document.update_list_entry` itself compiles, passes, and loses the rule
  again.
- **Every core call is a promise, and `Session.run` is one too** (DECISIONS
  0093). Tauri's IPC has no synchronous form, so the surface is uniformly
  asynchronous on both hosts rather than differing per platform — which means
  a handler that calls `session.run(...)` without awaiting it compiles, runs,
  and reads a `Promise` as truthy. `if (session.run(cmd))` is then a form that
  closes on a command the core refused. TypeScript catches exactly the sites
  that *read* the result (`--fail-on-warnings` turns the always-truthy
  condition into an error); a bare `session.run(cmd)` in a handler is fine and
  is not a bug, since nothing depends on the answer.
- **The `$core-host` alias is the one conditional in `ui/`, and it lives in
  two files that nothing keeps in step.** `vite.config.ts` resolves it per
  build (`--mode tauri` picks `core.tauri.ts`, everything else picks
  `core.wasm.ts`) and `tsconfig.json`'s `paths` points `svelte-check` at the
  wasm one, because a type-checker cannot know what Vite decided. So the PWA
  implementation is the only one CI type-checks; the Tauri one is checked by
  the Android build and nowhere else. The alias is chosen by Vite's `mode`
  rather than an environment variable on purpose: `vite.config.ts` says at the
  top that it uses no Node API, which is what lets it be checked under the
  same strict browser tsconfig as the app, and `process.env` would be the
  first one.
- **`cargo clippy --workspace` fails, and `--exclude cabas-tauri` is why the
  gates say so.** On Linux the `tauri` crate links the desktop GUI stack
  (dbus, gtk3, libsoup, webkit2gtk), which the everyday shell deliberately
  does not carry — the same argument that keeps the Android SDK in a shell of
  its own. `src-tauri` is a workspace *member* (so Rule 13's registry reaches
  it and one lockfile resolves everything) and **not** a `default-member`, but
  an explicit `--workspace` overrides `default-members`, so every gate that
  names `--workspace` has to name the exclusion too. Cross-compiled to
  `aarch64-linux-android` the crate needs none of those libraries, which is
  what `tauri-check` in the `.#android` shell relies on. **Nothing in CI
  compiles it today**; that gap closes at M8, when a desktop shell has to
  carry the GUI stack anyway.
- **The APK job does not run on a push, and the CI APK is a fresh install
  every time.** `apk` is gated to `workflow_dispatch` and `vX.Y.Z` tags,
  because it realizes the Android SDK and NDK through Nix and then lets
  `gradlew` fetch several more gigabytes — so "is Android green?" is answered
  by asking for a build, not by reading the last push. And what it produces is
  a **debug** APK: a release one is unsigned and will not install, and Gradle
  mints a debug keystore per machine, so each run signs with a different key.
  Installing over a previous build fails, uninstalling is the only way
  through, and uninstalling takes `identity.json` with it — the phone rejoins
  as a new device and leaves a dead peer on the roster, which is 0068 through
  another door. `src-tauri/README.md` has the keystore that closes it.
- **An Android build runs `beforeBuildCommand`, and that used to overwrite the
  PWA.** `crates/relay/build.rs` compiles `ui/dist` into the relay binary
  (0048), and `cargo tauri android build` rebuilds the frontend before it
  packages anything. With one output directory, an Android build replaced the
  bundle the relay was about to ship with a Tauri one — no wasm, no service
  worker — and everything stayed green: the relay builds, CI passes, and the
  phones get a blank page. `vite.config.ts` writes `dist-tauri` in that mode
  for exactly this reason, and `frontendDist` points there. Anything else that
  builds the frontend has to pick a directory on purpose.
- **The Android SDK pins are dictated by the generated project, and the two
  numbers do not match each other.** `platformVersions` has to equal the
  `compileSdk` in `gen/android/app/build.gradle.kts` (36, from CLI 2.11.4);
  `buildToolsVersions` has to equal what that project's Android Gradle Plugin
  (8.11.0) defaults to, which is **35.0.0** and is not the platform's number.
  Getting either wrong fails identically and unhelpfully: Gradle tries to
  install the component itself, cannot, because /nix/store is read-only, and
  says "The SDK directory is not writable" while naming something nobody asked
  for. `android init` has to run before either number can be known, which is
  why M7's first item could not finish before its fifth.
- **The Gradle in the shell does not build the APK, and there is no longer one
  there.** `gradlew` pins its own distribution (8.14.3) and fetches it, plus
  the AGP and Kotlin trees from `google()` and `mavenCentral()` — about 1.5 GB
  into `~/.gradle`, none of it Nix's. The flake used to carry `gradle` on the
  theory that the wrapper could be pointed at it; it cannot, without patching
  a generated file, so the package is gone and the impurity is left visible.
  It also means **the first Android build needs the network** and is slow in a
  way no later one is.
- **`beforeBuildCommand` runs from the repository root, not from
  `src-tauri/`.** `frontendDist` is relative to `tauri.conf.json` and the
  before-commands are not, so `../ui` in one and `ui` in the other is correct
  rather than a typo.
- **Tauri v2 reads `NDK_HOME` and nothing else.**
 The `.#android` shell sets
  `ANDROID_NDK_ROOT` as well, because that is the spelling Google documents
  and other tools read — but a shell with only that one fails at the link
  step complaining about a missing NDK while an NDK is plainly installed.
  Both point at `ndk-bundle` rather than at the versioned directory beside
  it, so bumping `composeAndroidPackages` cannot silently leave one aimed at
  an NDK that is no longer there. The pins are validated as of 2026-09-02
  (SDK platform 36, build-tools 35.0.0, NDK 29.0.14206865, JDK 21,
  `cargo-tauri` 2.11.4) — validated the only way they can be, by building an
  APK, which is what corrected two of them.
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
- **Adding an `Aisle` is cheap; adding a `Unit` is not.** `store::codec`
  decodes an unknown aisle as `Other` and *refuses* an unknown unit, because an
  aisle only decides sort order while a unit decides an amount (DECISIONS 0029,
  0057). So a new aisle needs no `SCHEMA_VERSION` bump — an older phone puts
  the line at the end of the cart — but it does need widening the two
  hand-written arrays that list every variant, one in `store::codec` and one in
  `app::tags`, plus the total `Record` in `labels.ts`.
- **Retiring an aisle is not cheap, and it fails silently.** The graceful
  degradation above cuts the other way: drop a tag and every ingredient
  already filed under it decodes to `Other` on the next launch. The document
  opens, every test passes, and the whole library is at the end of the cart —
  which is discovered in a shop. `codec::aisle` therefore keeps a read-only
  mapping for the five spellings 0069 retired (`grocery`, `items`, `butcher`,
  `fish`, `deli`), and `a_retired_aisle_still_reads_as_the_shelf_it_became`
  is what stops it being deleted as dead code. The same applies to any tag set
  the codec degrades rather than refuses.
- **An ingredient with no shop is sold *everywhere*, not nowhere** — and so is
  one whose shops have all been forgotten since. `domain::sold_at` is the only
  place that rule is written (DECISIONS 0071); it needs the shop library to
  answer at all, which is what makes the orphaned case decidable, and it is
  why `CartLineView.shops` carries the **resolved** trips rather than a copy
  of the ingredient's own list. The frontend filters by plain membership and
  holds no rule. Re-deriving it on the Svelte side — `shops.length === 0 ||
  shops.includes(…)` — is the tempting one-liner, and it is a second copy of a
  business rule that will drift; reading it as "nowhere" instead is a
  one-character change that loses an item off a shopping list.
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
- **chromedriver does not read `CHROME_PATH`.** It searches well-known absolute
  locations first, so on any machine with Google Chrome installed — every GitHub
  runner — it launches `/opt/google/chrome/chrome` rather than the chromium the
  flake pins. The versions then disagree, chromedriver answers `session not
  created`, and the runner carries on using the failed session's id, so every
  later request returns 404: what surfaces is a bare `http status: 404` naming
  neither Chrome nor a version. `wasm-test` pins the browser through
  `goog:chromeOptions.binary`, in a `webdriver.json` it generates and points at
  with `WASM_BINDGEN_TEST_WEBDRIVER_JSON` — the only channel chromedriver
  honours. `ui-test` was never affected, because it launches chromium itself.
- **`#[test]` does not run on wasm32** — browser cases need
  `#[wasm_bindgen_test]`. That is why `wasm-test` names its test targets one
  by one, and why `tests/persistence.rs` and `tests/document_size.rs` are
  `cfg`-gated to native. `crates/app/tests/scenario.rs` is the pattern for a
  file that runs on both: one `async fn` body, two thin wrappers.
- **`getrandom` on wasm32 needs two opt-ins, not one** — the `wasm_js`
  feature *and* `--cfg getrandom_backend="wasm_js"` (in `.cargo/config.toml`).
  Either alone is a `compile_error!`.
- **`chacha20poly1305`'s default features drag in a second `getrandom`** —
  the 0.2 line via `rand_core` 0.6, which needs its own, *different* wasm
  opt-in (`js`) that nothing in this workspace sets. The registry entry turns
  default features off and `sync` draws nonces from `getrandom` 0.3 directly;
  re-enabling them breaks `wasm-check`, not the native build.
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
  checks them.** Touching any of the seven files carrying `ts(export)` —
  `command.rs`, `view.rs`, `tags.rs`, `platform.rs`, `sync.rs`,
  `transfer.rs`, `photos.rs` — means rerunning the export command above. **A doc comment counts**:
  rustdoc prose
  is copied into the generated `.ts`, so reflowing a paragraph over a struct
  that exports is enough to fail the gate while every test still passes. That
  is how 0.6.0's first tag went red, on `platform.rs` — a file the earlier
  version of this note did not even list.
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
- **`ui-test` refuses to start if 8788 or 9222 is already bound**, and that is
  the point: attaching to a browser some earlier run left behind means
  reporting on a page this run never opened. If it says so, `pkill -f
  'remote-debugging-port=9222'` and check `ss -lptn 'sport = :8788'`. The
  script itself no longer leaks — `exec`ing the harness would have replaced
  the shell and skipped its own cleanup trap. It waits on `/` and not
  `/healthz`, because a relay built with no bundle answers the second and
  would leave the suite to fail one screen at a time.
- **The relay's copy of the app is compiled in, so a running one cannot see a
  new `pnpm build`.** `build.rs` declares `rerun-if-changed` on `ui/dist`, so
  the *rebuild* is automatic — but the process already running is not, and
  neither is a container image. `ui-serve` reads from disk and behaves the
  opposite way, which is exactly the confusion to expect: restart the relay.
- **A step can only reference an *ingredient* line.** `Recipe::usage` searches
  `Component::Ingredient` and nothing else, so a segment naming a sub-recipe
  usage renders as `Missing` — not an error, just permanently a warning. The
  `@` picker in `RecipeEditor` filters to ingredient components for that
  reason, and widening it would need a domain change first.
- **A recipe line is named when it is added, not when it is saved**
  (DECISIONS 0039). `mintUsageId()` on the host is why the editor can mention
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
- **`IngredientForm.svelte` is not a `<form>`, and must not become one.** It
  renders inside the list's add form and inside the one big form the recipe
  editor is, and a nested `<form>` is dropped by the parser. Three rules follow
  and anything added to it inherits them: every button says `type="button"`, no
  field is `required`, and Enter is handled on each control rather than left to
  submit the form around it (DECISIONS 0056). **Each** control, including the
  checkbox: Enter on a checkbox submits the surrounding form like any other
  field, so the one that is missing the guard is the one that destroys the
  recipe being written.
- **A `SearchPicker`'s field is not bound, and the reason is `required`.** It
  displays the chosen option's name when closed and the query when open, so a
  binding would put a half-typed query where the form's value should be. Three
  behaviours hold that together and each fails silently on its own (DECISIONS
  0058): the outside-pointer listener is in the **capture** phase, so pressing
  "save" closes the panel *before* the click and the field is back to a real
  name — empty when nothing was chosen, which is what makes `required` refuse;
  a row prevents `mousedown`, or the pointer blurs the field, closes the panel
  and the click lands on nothing; and Enter always `preventDefault`s, because
  the panel opens inside a form that would otherwise save.
  **The panel's state belongs to the picker and dies with it**, which is what
  makes closing the list's add form forget a half-typed ingredient, and
  removing a recipe line take its own and leave every other line's.
  **The door reads the query before `close()` clears it** — that ordering is
  the whole of 0060, and getting it wrong hands the form an empty string with
  no error anywhere.
- **`.picker` in the same document is the recipe editor's "@" mention list.**
  Svelte scopes styles per component but `ui-test` queries the DOM globally, so
  a second component naming its root `.picker` silently changes what
  `__count('.picker button')` counts. `IngredientPicker.svelte` is
  `.ingredient-picker` and `SearchPicker.svelte` is `.search-picker` for that
  reason. It caught `.step` next — the recipe editor's instruction step and
  the list's stepper button, one release apart — which is why the list's is
  `.notch` (0079).
- **A UI test that sets a `<select>` needs the native setter and a dispatched
  event** — assigning `.value` moves the pixel and tells Svelte nothing. Note
  also that `form select:nth-of-type(1)` matches *every* first-select-child in
  the form, not the first select in it; `querySelectorAll(...)[n]` is what you
  meant. Both traps are already handled in `ui/tests/smoke.mjs`.
- **A picker cannot be driven by `focus()` from a headless page**, because the
  page is not the focused window and the focus handler never runs — which looks
  exactly like a panel that failed to render. `__open` in `smoke.mjs` clicks the
  field, which is the event the component listens for anyway; `__choose`,
  `__offered` and `__door` all go through it.
- **A `<fieldset>` is `min-inline-size: min-content` in every browser's own
  stylesheet.** It therefore refuses to be narrower than its widest child,
  which is how one long `<option>` label pushed the unit dropdown past the
  right edge of a recipe line on a phone. `QuantityField` sets `min-width: 0`
  on the fieldset and lets the select shrink (`flex: 0 1 auto`, capped); a flex
  child's floor is its content everywhere else too, so anything new on a line
  inherits the same trap. `ui-test` asserts at 390 px that nothing overflows
  sideways and that a line's rows share one right edge.
- **`Vary` makes the precache miss its own entries.** A server answering
  `Vary: Origin` (Vite's preview does; the relay deliberately does not) makes
  the Cache API match on the
  request's `Origin` header too. The worker precaches with requests that carry
  none; the page then asks for its JS and CSS *with* one, because Vite marks
  both tags `crossorigin`. Every asset cached, every lookup a miss — and
  online it is invisible, because the miss falls through to a network that
  answers. `sw.js` reads through `lookup()`, which passes `ignoreVary: true`,
  and `assets.rs` has a test that nothing it serves ever varies — a tunnel or
  a future proxy can still put one back, which is why both ends hold.
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
- **A tab opens cold, and two of the three things that has to reset are not
  free.** Component-local state dies on its own — each screen sits in an
  `{#if}` in `App.svelte`, so switching away destroys it. The open recipe is
  *core* state and has to be closed with a command, and the scroll offset
  belongs to the window (DECISIONS 0074). `Session.show` does both; anything
  new that a screen remembers outside its own component has to be added there.
- **A duration token has a unit, and `parseFloat` drops it.** `--press-delay`
  is authored `500ms` and the CSS minifier ships it as `.5s`, so a bare
  `parseFloat` gives 0.5 — in `SwipeToAdd` that is a long press firing
  instantly, and in the test that drives it, it looked exactly like a press
  that never fired at all. Both ends read the unit back. The distance tokens
  are safe only because `px` survives minification unchanged.
- **A form control may never be smaller than `--text-base`.** `app.css` floors
  `input`, `select` and `textarea` at `max(var(--text-base), 1em)` because iOS
  zooms the page towards anything under 16px that takes focus, and does not
  zoom back (DECISIONS 0078). A component that sets `font-size: var(--text-sm)`
  on a field wins on specificity — Svelte's scoping class outranks a bare
  element selector — and puts the behaviour back, on that field only, which is
  exactly the shape nobody notices until they are in a shop. Label it small if
  the hierarchy needs it; the field itself stays 16px.
  **`font: inherit` does it too, and looks like nothing**: the shorthand
  resets `font-size` along with everything else, so a scoped
  `textarea { font: inherit }` outranks the floor exactly the same way.
  `app.css` already sets it on every control, so a component repeating it is
  saying nothing and exempting one field (0079).
- **Orange, pink, anis and lilac are surfaces. They are never inks** (DECISIONS
  0081). `color: var(--accent)` on anything light is 2:1 to 2.6:1 and reads as
  a smudge; the ink is `--accent-strong`, and what is written *on* one of those
  surfaces takes its `--on-*`. The trap is that `--accent` still looks like the
  obvious token for "make this word the accent colour", and the result is
  legible enough on a desktop monitor at full brightness to survive review. The
  same shape twice more: `--display-ink` is the olive for the display face **on
  anis or apricot** and `--text` for one on a lilac tile, and the focus ring is
  `--text` because orange on apricot is a ring nobody sees.
- **Nothing is written straight onto the tablecloth.** The page background is a
  40px chequerboard, so any loose paragraph needs a surface under it — the
  global `.bubble` class in `app.css`, added beside the component's own class
  (`class="empty bubble"`). The exception is deliberate and narrow: a heading
  in the display face at `--text-lg` or bigger clears 3:1 against both checks,
  which is what lets an aisle title sit on the cloth. Anything at body size
  does not.
- **Two shapes, and picking the wrong one leaves a field with no edge.** *A
  form is a lilac tile (`--surface-raised`) and its fields are cream
  (`--bubble`)*; *a row that is only read is a cream card, spaced by
  `--space-2`, with the cloth between the cards instead of a hairline*. Cream
  fields inside a cream card vanish into it, which is why `Shops.svelte`'s rows
  are tiles and `People.svelte`'s are cards.
- **A `--radius-pill` button that also sets `background` can be outranked by
  the rule above it.** `.amount button` and `.quantity` are both one class deep,
  so the plain class lost and the list's amount pill came out cream instead of
  pink. It is written `.amount .quantity` for that reason, and anything new
  inside a container that styles `button` inherits the same trap.
- **`.display` is `inline-block`, and that is load-bearing.** A transform does
  not apply to a non-replaced inline box, so the same class on a `<span>`
  inside a `<summary>` would silently not lean. That `<span>` exists so the
  disclosure marker stays upright while the words lean.
- **The two faces are precached by name, and a rename breaks the install.**
  `PUBLIC_SHELL` in `vite.config.ts` lists them because Vite copies `public/`
  straight to `dist/` without passing it through the bundle, and the
  service-worker plugin deliberately reads the bundle rather than the disk. The
  price is that `cache.addAll` rejects **as a whole** if any one entry 404s,
  which leaves the app with no precache at all — so they are committed assets
  and a rename has to be made in both places. `format('woff2')` and not
  `format('woff2-variations')` for the variable Quicksand: an unknown format
  string makes the browser skip the source and fall back to the system face,
  silently, which is the exact failure self-hosting exists to prevent.
- **There is one theme, and five cloths.** `color-scheme: light`, no
  `prefers-color-scheme: dark` block, and nothing may reintroduce one
  piecemeal: the cloth has no night version and a half-inverted palette is
  worse than none. What *does* change per screen is `--check-a` and
  `--check-b`, and nothing else (DECISIONS 0084): `App.svelte` stamps
  `data-screen` on the root element and `app.css` redefines the two under
  `:root[data-screen='…']`. A new tab needs a new palette, and it has to clear
  **5.2:1 against `--display-ink`** — because a heading in the display face
  sits directly on the cloth — while staying under about **1.5:1 against
  itself**, or the weave reads as a chequerboard rather than a texture (0085).
- **`background-attachment: fixed` on the root element is two behaviours, not
  one.** iOS ignores it and Android honours it, so the cloth scrolled on one
  phone and stood still on the other from the same build (DECISIONS 0083). It
  is gone; anything that wants a still background needs a scrolling container,
  and a scrolling container between `<body>` and the app makes the whole
  `--layer-*` scale a lie.
- **A photo travels on its own socket, and nothing about it is persisted.**
  `/photos` shares the origin and the group key with `/sync` and nothing else
  — no cursor, no epoch, no sequence (DECISIONS 0080, 0092). The consequence
  worth knowing is that a transfer cut short costs one round trip and never a
  photo: the next hello re-derives the work from what is on disk at both ends.
  The consequence worth watching is the other one — **`photoHandle` and
  `photoPush` take the session out of the core's cell while they await**, so
  two of them in flight at once find it empty. `PhotoTransfer` chains every
  core call for that reason, and the error says so if the chain is ever
  bypassed.
- **A photo arriving is not a state change anything on screen is watching.**
  It is written into a store beside the document, so no `StateView` moves and
  no component re-renders. `PhotoTransfer.generation` is what closes that
  loop: `Photo.svelte`'s effect reads it, so bumping it re-reads every photo
  on screen and a placeholder becomes a picture. Anything else that shows
  photo bytes has to read it too.
- **The relay's photo cap has to bite before the bytes are in memory.**
  `GroupPhotos::store` refusing an oversized blob is one step too late, so
  `/photos` sets `max_message_size` on the upgrade (0092). A hello's two lists
  and a push's payload are the two unbounded things a stranger who guessed a
  group id can send.
- **Adding a field to `ListItem::Recipe` is cheap; forgetting it in one arm of
  `nudge_list_entry` is not.** `only` (DECISIONS 0091) has to survive every
  command that rewrites a recipe line — `SetEntryServings` and
  `NudgeListEntry` both rebuild the item — and dropping it there silently
  turns half a recipe back into a whole one at the first "+" somebody presses.
  The compiler catches the construction, not the `..existing` that would have
  carried it.
- **An empty `only` is refused, and that is a product decision rather than
  defensiveness.** A recipe entry asking for none of its lines contributes
  nothing, so its progress is 0/0, so it is never complete, so it never leaves
  the list (0020). `chosen_components` refuses it at the command; the frontend
  removes the entry instead, which is what the person meant.
- **A `z-index` is a `--layer-*` token, and the scale lives in `app.css`.**
  The handful in this app all share the root stacking context — nothing
  between them and `<body>` sets a `position`, a `z-index` or a `transform` —
  so each number only means something against the other four, and they were
  written as bare literals in five different files. The error banner sat at
  `3` under the amount panel's scrim at `20`: a refused command explained
  underneath the dimming that hid it, on a panel that stays open precisely so
  the explanation can be read (DECISIONS 0079). Anything new that floats reads
  the scale instead of picking a bigger number — and anything that puts a
  stacking context between `<body>` and one of these makes the whole scale a
  lie.
- **iOS does not resize the page for the keyboard.** The layout viewport keeps
  its height and the keys are drawn over it, so `100dvh`, `position: fixed` and
  `env(safe-area-inset-bottom)` all describe a viewport whose bottom third is
  gone. `--keyboard-inset` is the measured truth, and everything reads it
  through `max()` so that `0px` is the layout that existed before it did
  (DECISIONS 0040). Anything new that sits at the bottom of the screen has to
  read it too.
- **A keyboard cannot be emulated by shrinking the window.** Every DevTools
  command that resizes anything resizes the *layout* viewport, which tells the
  page the truth and so tests nothing; the case that matters is a page that
  still believes it is full height. `__keyboard()` in `smoke.mjs` overrides the
  `VisualViewport.height` accessor and fires `resize`, which is the shape iOS
  actually makes.
- **`scrollIntoView` cannot lift anything out of the keyboard**, because to the
  browser it is already visible — "visible" means inside the layout viewport,
  which is exactly what is covered. `reveal()` scrolls by the measured
  difference instead. The padding and the scroll are one mechanism: the picker
  can only climb out because `Screen`'s `padding-bottom` put scrollable document
  underneath it.
- **A service worker only registers in a secure context**, so the LAN address
  `pnpm dev` prints can show the app on a phone and can never make it
  installable — no worker, no precache, and airplane mode is a blank page. That
  is what `ui-serve` exists for (DECISIONS 0041), and it is why nothing about
  M4's remaining item can be rehearsed over `http://192.168.…`.
- **The relay's epoch is a random `u64`, and JavaScript has no such number.**
  Almost every epoch is above 2^53, and `serde_wasm_bindgen` refuses to round
  it — so `syncStatus` threw on the *first* real connection while every test
  against a hand-made epoch of 0 passed. Identities cross as **text**, counts
  stay numbers (DECISIONS 0046).
- **A restored backup brings the epoch back with it.** The epoch is the guard
  that tells a device its cursor points into a log that no longer exists — and
  it lives in `meta`, inside `/data`, inside the backup. So a restore returns it
  *identical* alongside a log that stops where the backup was taken, every
  device keeps a cursor from further along, and "replay everything after frame
  412" out of a log ending at 300 is nothing at all. No error on either side,
  and it lasts until the log grows back. The relay therefore checks
  `hello_since < log.next_seq()` as well as the epoch (DECISIONS 0053). Anything
  that rolls `/data` back — a filesystem snapshot, a half-restored volume —
  lands in the same place.
- **A restore falsifies the shadow too, and that half is worse.** The shadow
  says "the relay holds everything up to version V"; a restore makes it false
  while changing nothing the shadow can see. A delta measured from it names
  operations the log no longer carries, so a device that missed the rolled-back
  window can never root it — it accepts frame after frame, advances its cursor,
  merges none of them, permanently, while both ends look healthy. A session
  that finds a log it does not recognise therefore ignores its shadow and
  pushes the **whole replica** (`SyncSession::push`, DECISIONS 0054). The relay
  never says it refused a cursor, so the client infers it: a frame at or below
  the `since` the hello asked for cannot belong to the log that cursor was
  reading. Both halves are staged in `convergence.rs`, one test each.
- **The restore drill's marker is read off the relay, not off a phone.** The
  planted ingredient lives in the replica of every phone that saw it, and a
  restore reaches none of them — so a phone showing it afterwards says nothing
  at all. `cabas-relay groups` before and after is the evidence, because the
  relay is the only party whose state a backup rolls back.
- **A sync cursor must never outlive the replica it belongs to.** They are two
  different files — `localStorage` and IndexedDB — and a cursor that survives
  alone leaves the device with an empty library and no error, for good.
  `App::opened_fresh()` is the exact fact, and the engine starts from zero
  when it is true (DECISIONS 0045).
- **The engine's own `pagehide` writes the cursor on the way out**, so a test
  that clears `cabas.sync` and then navigates gets it written straight back by
  the page it is leaving. That is correct behaviour and the reason the sync
  test exercises the harder case instead of fighting it.
- **`console.error` from app code was invisible to `ui-test`.** The harness
  listened to `Log.entryAdded`, which carries what the *browser* complains
  about; anything the app logs arrives as `Runtime.consoleAPICalled` and used
  to fail nothing. Both are collected now, and a timeout prints them — a
  "timed out waiting for X" with no page log is a much longer afternoon.
- **A new build appears one launch late, by design.** The launch that meets it
  keeps showing the old one and installs the new worker behind it; the next
  launch is the one that takes over, because activating earlier would delete
  caches a running page is still loading from (DECISIONS 0038). Observed on the
  phone at M5. So "my change isn't there" is answered by *closing the app and
  reopening it*, not by reloading — and a phone that is never closed stays a
  build behind. **Settings names the build** since 0.2.0, so which one is
  running is read rather than counted (DECISIONS 0055): the string comes from
  the core, which is what the relay compiled in, and `ui-test` holds it against
  `Cargo.toml`.
- **The QR encoder is fixed to version 6, level L.** That is what makes a
  hand-written one safe — one version is one row of the spec's tables instead
  of a wall of them — and it means a longer payload throws rather than draws
  something unreadable. `ui-test` compares every module against `qrencode`, so
  a change that breaks the encoder fails there and not on a phone. The one bug
  it caught while being written: writing the generator polynomial's two terms
  in the wrong order yields a valid polynomial and a symbol no scanner reads
  (DECISIONS 0047).
- **A paired device retries a relay it cannot reach, and the browser logs it.**
  That is the engine working — the network comes back and so does it — but it
  means `pnpm preview` answers `/sync` with the app and the console fills up.
  `ui-test` ignores exactly those network-level entries and still fails on
  anything the *app* logs.
- **A secure context cannot open a `ws:`**, so the TLS that makes the app
  installable is also what stops it reaching a relay listening in plaintext —
  and the relay terminates no TLS, because in production the tunnel does. It is
  invisible until a phone is in hand: `ui-test` runs over `http://localhost`,
  where `ws:` is same-scheme. `ui-serve` proxies `/sync` for that reason
  (DECISIONS 0044), which also keeps development on the single origin
  production has.
- **`<hostname>.local` does not resolve just because avahi is running.** NixOS
  enables it as a resolver and leaves `publish.enable` off, so the host never
  announces its own name; `avahi-resolve -n $(uname -n).local` times out locally,
  which is the fast way to tell. The LAN address is the fallback, and then the
  DHCP lease has to be reserved — whichever address the phone installs from
  becomes the app's identity (DECISIONS 0012, 0041).
- **iOS rejects a server certificate on three silent grounds**: no `serverAuth`
  in the extended key usage, a validity longer than 825 days, or hosts named in
  the common name instead of the SAN. All three surface only as "the connection
  is not private". `ui-serve` produces all three correctly; anything that
  regenerates a certificate by hand has to keep doing so.
- **Installing a root on iOS and trusting it are two different screens.**
  Settings → Profile Downloaded installs it; General → About → Certificate Trust
  Settings is what makes it count. Skipping the second leaves a certificate that
  is installed, listed, and still refused — the single most likely reason the
  phone will not open the app.
- **`ui-test` takes its target from `APP_URL`**, which is how the whole
  end-to-end suite was run against `ui-serve`'s TLS origin without touching it.
  Chromium needs the leaf's SPKI pinned with
  `--ignore-certificate-errors-spki-list` for that — unlike
  `--ignore-certificate-errors`, it leaves the origin a secure context, which is
  the entire property under test.
- **`--window-size` sizes the window, not the viewport**, so a headless
  screenshot comes out a browser frame short. `render-icons.mjs` sets the
  viewport with `Emulation.setDeviceMetricsOverride` instead. An inline `<svg>`
  also needs `display:block`, or its text baseline overflows the viewport and
  puts a scrollbar in the icon.
- **The Supervisor does not recurse into an add-on repository.** It reads
  `repository.yaml` at the root and then looks for `config.yaml` in each
  *top-level* directory, which is the only reason `cabas-relay/` sits beside
  `crates/` and `ui/` instead of under a tidier `addon/`. Moving it does not
  break anything visibly — the add-on simply stops appearing in the store.
- **The add-on's Dockerfile must contain no `RUN`.** It only copies a binary
  that was cross-compiled on the runner, and that is what lets `docker buildx
  --platform linux/arm64` work on an x86 runner with no qemu installed. A
  single `RUN` would not fail the build; it would silently make it need
  emulation, and CI installs none (DECISIONS 0049).
- **`cabas-relay/config.yaml`'s `version` is the image tag the Supervisor
  pulls**, so it has to equal the workspace version in `Cargo.toml`. Drift
  shows up as an add-on that offers an update forever. `check-addon` compares
  them, and also checks that every architecture in `config.yaml` has a base
  image in `build.yaml` — the missing one installs and then fails to pull.
- **Cross-compiling the relay by hand needs the linker named.** The musl
  targets link through `rust-lld`, and a bare `cargo build --target
  aarch64-unknown-linux-musl` fails at the link step looking for a cross-gcc
  that this shell deliberately does not carry. `build-relay <arch>` sets
  `CARGO_TARGET_*_LINKER` along with `CABAS_EMBED_UI=required`; use it.
- **The image runs on the appliance and still cannot be run from here.** There
  is no container runtime in the devShell, so the Dockerfile is executed on a
  runner and the image on the Pi; `build-relay` and `check-addon` cover
  everything up to that line and nothing past it. What *can* be checked
  locally is the artifact rather than its frame, because the binary is static:
  pull the published layer from ghcr with `curl`, untar it, and run
  `usr/bin/cabas-relay` directly — that is how the amd64 image was checked
  before the aarch64 one was started on the Pi. It says nothing about s6,
  `/data` or the Supervisor, which remain the appliance's word alone.
- **The headers `assets.rs` sends are not the headers the phone receives.**
  Cloudflare gives any cacheable response with no explicit `max-age` a default
  four-hour *browser* TTL, which turned the relay's `Cache-Control: no-cache`
  on `/sw.js` into `max-age=14400` — on the one file that decides whether an
  installed app ever notices a new build (0038). A cache rule on the zone
  bypasses cache for that path and restores it (DECISIONS 0052). The lesson
  generalises past this instance: `assets.rs`'s tests assert what the *origin*
  serves, and nothing in this repository can see what survives the edge. A
  header that matters has to be checked through the tunnel, from outside, with
  `curl`.
- **A WebSocket that carries nothing gets closed by whatever proxy is in
  front of it.** Cloudflare documents this and does not publish the period,
  which is why the relay pings every 30 seconds (DECISIONS 0051). There is no
  frontend half: a browser's `WebSocket` cannot send a ping and answers the
  server's without telling the page. None of it reproduces locally — nothing
  between two sockets on one machine times anything out — so the test drives
  the relay's own period down to milliseconds instead of trying to stage a
  tunnel. Anything added here that holds a socket open and quiet inherits the
  same problem.
- **Surveying the data directory must never open a log.** `GroupLog::open`
  mints an epoch for a group that has none and rewrites `meta`, so a
  "read-only" listing built on it would cost every device of every group a
  full replay. `admin::survey` reads `meta` and stats the files instead — and
  it takes the timestamp off the *log* file, not `meta`, because `meta` is
  rewritten on open and would report when the relay last restarted.
- **`forget` is irreversible and takes a whole group id.** No prefix, no age,
  no pattern — the premise of DECISIONS 0050 is that the machine cannot judge
  which group is finished, so it does not get to guess at one either. It is
  safe to run while the relay serves, because an abandoned group is by
  definition one nothing connects to; forgetting a *live* one leaves its
  sockets answering "storage failed" until the process restarts.
- **Every save writes the whole document, so nothing binary may live in it.**
  `App::pending_snapshot` serialises the entire replica and `Storage::save`
  replaces it — deliberately, atomically, on a debounce that fires after every
  tick in a shop. Measured (x86-64 release, against 154 kB and 0.42 ms today):
  20 photos of 150 kB make a 5.9 MB snapshot exported in 30 ms, 60 make 17.6 MB
  in 83 ms, 200 make 58.6 MB in 310 ms — paid on **every** save and again on
  every cold start, times whatever wasm on a phone costs. That is why photos
  are a `PhotoStore` beside the document and not a field in it (DECISIONS
  0062), and `document_size.rs`'s 4 MiB ceiling is what catches a regression.
- **The IndexedDB database is at version 2, and two types open it.**
  `IndexedDbStorage` and `IndexedDbPhotoStore` share one `open()` and therefore
  one `VERSION`; giving either its own would make one of them request a
  downgrade, which the browser refuses outright. The upgrade creates only the
  object stores that are missing — that is what makes the bump non-destructive
  for a phone that already holds a document.
- **A phone photo's rotation lives in EXIF, not in its pixels.** Drawing one
  to a canvas without `imageOrientation: 'from-image'` puts every landscape
  shot on its side, and it is invisible on a desktop file that has no EXIF at
  all. `createImageBitmap` is used precisely because it can apply it; an
  `<img>` cannot be asked to.
- **`Photos::forget_unreferenced` is a cleanup only once the relay holds a
  copy — and since 0092 it usually does.** The precondition it was written
  against is met: the relay keeps a sealed copy of every photo it has been
  pushed and deletes none. What it still lacks is a caller that runs it only
  when *this* device's photos are durable there; sweeping a photo the relay
  has not been given yet is still a deletion. It is implemented and tested;
  nothing calls it yet, on purpose.
- **A photo session's `want` list is not bookkeeping, it is the only thing
  standing between an untrusted relay and a phone's storage.** The relay is
  zero-knowledge and *not* trusted (Rule 7), and `/photos` is the one endpoint
  where it hands bytes back that get written to disk. `PhotoSession` therefore
  checks an incoming `Photo` against what the hello asked for **before** it
  opens it, and narrows the welcome's two lists to what the hello said —
  otherwise a relay answering a fetch with something else, or naming photos
  nobody asked about, writes whatever it likes onto every device in the group.
  Both checks read like redundant defence against a server we run ourselves,
  and both are two lines. The tests that stop them being tidied away are
  `a_photo_nobody_asked_for_is_dropped_without_being_opened` and
  `a_welcome_cannot_widen_what_the_hello_said`.
- **A photo id is not a `PhotoName`, and one bad one must not cost the rest
  their transfer.** `PhotoId::from_raw` takes whatever a document says and an
  imported file writes ids by hand (0076), while a `PhotoName` is checked
  because the relay names a file after it (0080). `app::photos::speakable`
  leaves an id that is not a name out of both the hello's lists instead of
  raising: `PhotoStore` refuses the same shapes, so its bytes were never here
  anyway, and failing `PhotoSync::open` over one malformed reference would
  strand every *other* photo in the library, permanently and silently.
- **An import must not go through `Command::SaveIngredient`, and the reason is
  the event log.** The log is capped at 200 entries (`EventLog::CAP`) and one
  file can carry more ingredients than that, so replaying an import through
  the ordinary save would push out every deletion the log exists to remember —
  silently, and only on the day somebody looks (DECISIONS 0076). That is why
  `save_ingredient`, `save_recipe` and `save_shop` are each split into an
  `App::*_from` that validates and a caller that writes and logs; anything new
  on those paths inherits the split.
- **An import resolves everything before it writes anything.** There is no
  transaction under the document, so a failure halfway through would leave a
  library nobody could describe. `App::import_library` builds every domain
  value first and only then calls `put_*`; adding a write into the resolving
  half quietly removes that guarantee, and the test that catches it is
  `an_import_that_fails_leaves_the_library_exactly_as_it_was`.
- **An export skips a photo this device does not hold, in silence.** The
  entity keeps naming it, so the file stays truthful — but it means an export
  *with photos asked for* can carry fewer than the library names. Since 0092
  that window is usually short: a phone that joined by typing twelve words
  fetches the pictures over `/photos` a moment later. Short is not zero, and
  a test that waited on it would be racing the transfer — `ui-test` puts a
  photo on that device by hand for exactly that reason.
- **`navigator.share` wants the tap it came from, and the export is
  asynchronous.** Safari drops the transient activation across the await that
  builds the file, so the share sheet may refuse on a real phone while working
  everywhere it is not needed. `Transfer.deliver` tries it and falls through to
  a download — and distinguishes a *refusal* from an `AbortError`, because the
  second one means the person already said no and a download after it would be
  the app insisting.
- **`smoke.mjs`'s `HELPERS` is one big template literal.** A backtick anywhere
  in it — including inside a doc comment, which is exactly where prose puts
  them — closes the string, and the failure is a `SyntaxError` at module load
  quoting a word out of the comment rather than anything about the test. Every
  backtick in there is written `\``.
- **`input.files` is read-only**, so a test cannot hand a file to a file input
  by assignment. `__photograph` in `smoke.mjs` builds a real JPEG on a canvas
  and fills a `DataTransfer` — which is also why the photo path is tested
  end to end without any camera or any file on disk.
- **`capture` on a file input is not a hint.** On a phone it opens the camera
  and *only* the camera — the photo already in the roll is unreachable through
  it. That is why `PhotoField` has two hidden inputs and two buttons (0065),
  and why the attribute is not flipped on one input between clicks: it is read
  when the picker opens, so an input whose meaning depends on which button was
  pressed last is a race nobody will reproduce.
- **A bulk rename of "family" hits `target_family` and `font-family`.** Both
  matched when 0063 was done, and both break everything: the first is a Rust
  `cfg` (`cargo build` fails with "unexpected cfg condition name"), the second
  silently unstyles the app. Anything similar needs the same second pass.
  Note also that `docs/DECISIONS.md` is deliberately **excluded** from that
  rename — it is append-only (Rule 14), so every entry before 0063 says
  "family" and means group.
- **`StateView.me` is `null` between joining a group and choosing a member**
  (0068), and that is a normal state with a real replica behind it — not a
  loading state. Anything new that reads `me` has to say what it does then;
  `App.svelte` renders `Identify` over the app instead of the tab bar.
- **A command that moves the identity must be listed in `MOVES_IDENTITY`**
  (`session.svelte.ts`). `choose_user`, `create_user`, `name_device` and
  `rename_user` change what the core thinks this device is, and
  `localStorage` holds the only durable copy (0031); `run` writes it back for
  exactly the commands that set names. Leaving one out works perfectly until
  the next launch, which is the worst shape a bug can have — and a second
  door beside `run` was the first attempt at this and had the same flaw, from
  the other side: `rename_user` never went through it.
- **`enrol` writes nothing when there is no user, and that is load-bearing.**
  A device record needs an owner; writing one before somebody is chosen means
  inventing the owner, which is the duplicate person 0068 exists to prevent.
  For the same reason `name_device` is sent *before* the choice — afterwards
  it would write the record twice, once with an empty name that the other
  phone would see.
- **A synthesised `PointerEvent` is not an active pointer**, so
  `setPointerCapture` throws `NotFoundError` on one — which is how the swipe
  first failed in `ui-test` while working by hand. The component ignores that
  failure, because capture is what keeps a drag alive when the finger leaves
  the row rather than what makes the drag work.
- **A swipe is anchored where its axis is decided, not where the finger went
  down.** `SwipeToAdd` commits to the horizontal after `SLOP` pixels and
  measures from there, so the row does not jump to meet the finger — and a
  test that drives it has to measure from the same place. `__swipe` takes the
  distance travelled *after* the anchor for exactly that reason; measuring
  from the start silently tests the slop instead of the threshold.
- **`.swipe .front` must stay opaque.** It slides over the row underneath it,
  so a transparent background shows the "Ajouter à la liste" strip through the
  row that is not moving.
- **`isIdentity` in `core.wasm.ts` must accept a `null` user.** That is the state a
  device is in between the twelve words and the roster (0068), and it is on
  disk for exactly as long as the "Qui êtes-vous ?" screen is up. A reader
  that insists on strings sends a phone closed on that screen back to pairing
  — new device id, dead peer in the replica's history, twelve words typed
  again. `ui-test` reloads on that screen for this reason.

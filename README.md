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

**Status**: M0–M5 complete — two phones, an iPhone and a Pixel 8, pair with
twelve words and converge through a relay that cannot read a byte of what it
carries. `crates/domain` holds the
whole product logic as pure, tested functions — units and exact conversions,
recipe scaling, the sub-recipe DAG, cart aggregation and check-state
derivation. `crates/store` persists it: a Loro document, snapshots with
history compaction, and a `Storage` trait over a file (native) and IndexedDB
(the PWA). Two replicas that edited while apart converge, including through a
relay they never used at the same time. `crates/app` turns that into the
surface a frontend can use: thirteen commands in, one complete view-model
out, with the TypeScript types generated from the Rust ones. A scripted
shopping trip — build a library, cook for six instead of four, tick things
off, finish, restart — passes natively **and** in a headless browser. 163
tests green plus 10 in chromium — 5 over IndexedDB and 5 through the app, and
since M4 those run in CI too rather than only on a developer machine.

**M4 is done**: `ui/` is a working Svelte 5 app — it mints an identity,
builds a library, writes and reads recipes, derives the cart and survives a
cold restart, all driven end to end in a real browser by `ui-test`. Every one
of the thirteen commands is reachable from the UI. It is **installable and it
opens with the network off**: a hand-written service worker precaches the shell
from Vite's own build manifest, and `ui-test` proves it by turning the network
off in the browser and reading a recipe back
([0038](docs/DECISIONS.md#0038--the-service-worker-is-written-by-hand)). The soft
keyboard is handled as a measured length rather than a mode, so a form's last
field and the recipe editor's mention picker stay above the keys
([0040](docs/DECISIONS.md#0040--the-keyboard-is-a-length-not-a-mode)). And
`ui-serve` hands the built bundle to the phone over TLS, which is what a service
worker needs to exist at all
([0041](docs/DECISIONS.md#0041--the-phone-installs-from-a-local-certificate-authority)).
On the device itself the cold start is instantaneous, which closes the question
of the 713 kB core, and the keyboard behaves as it was designed to.

**M5 is done**: the relay brokers a sealed log it cannot read
([0042](docs/DECISIONS.md#0042--the-relay-keeps-a-sequenced-log-it-cannot-read)),
the PWA drives its own socket
([0043](docs/DECISIONS.md#0043--the-pwas-websocket-lives-in-the-frontend)), and
pairing is twelve words shown with a QR that is never scanned
([0047](docs/DECISIONS.md#0047--the-qr-is-shown-never-scanned-and-the-encoder-is-ours)).
`ui-test` proves the round trip against the real relay binary: a device pushes
its library, loses its replica, gets everything back, and a second one joins by
typing the words — with nothing in the relay's log readable as text. Behind
Settings there is a roster of who is in the family on what, and a journal of
what was edited and deleted; both say plainly that a shared key means these are
names and not permissions (Rule 7). The milestone was closed where M4's was —
on the devices: an iPhone and a Pixel 8 pair with twelve words and converge
both while both are open and while neither is ever open with the other. The
same PWA runs on both, so M7's Android app is a better wrapper rather than a
requirement.

**Next is M6.** All of this lives on one wifi today, behind a certificate
authority installed by hand, with the relay in a terminal — deployment is what
makes it survive the laptop being closed. Resuming work starts at the "Resuming
work" section of the [ROADMAP](ROADMAP.md).

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

The PWA lives in `ui/` — Svelte 5 on plain Vite
([0037](docs/DECISIONS.md#0037--the-pwa-is-a-plain-vite-spa-not-sveltekit)).
Build the wasm core before anything that reads it; the glue it writes is a
build product and is not committed:

```sh
build-wasm [--dev]                        # the core → ui/src/lib/wasm/
pnpm -C ui install                        # once
pnpm -C ui dev                            # dev server, bound to the LAN
pnpm -C ui check                          # types, against the generated bindings
pnpm -C ui build                          # ui/dist
```

### Installing it on a phone

`pnpm dev` is reachable from a phone but the app is **not installable over it**:
a service worker only registers in a secure context, and a LAN address over
plain HTTP is not one. `ui-serve` serves the built bundle over TLS instead,
signing a certificate with a local CA it generates once per machine
([0041](docs/DECISIONS.md#0041--the-phone-installs-from-a-local-certificate-authority)):

```sh
build-wasm && pnpm -C ui build            # what actually gets installed
ui-serve                                  # TLS on 8443, the CA on 8080
```

Both ports have to be reachable from the phone, and a host firewall is the
first thing that silently is not — the symptom is a phone that cannot load
either address while the desktop loads both. On NixOS with the default iptables
firewall, for one session:

```sh
sudo iptables -I nixos-fw -p tcp --dport 8080 -j nixos-fw-accept
sudo iptables -I nixos-fw -p tcp --dport 8443 -j nixos-fw-accept
```

or `networking.firewall.allowedTCPPorts = [ 8080 8443 ];` to keep it.

**`<hostname>.local` does not resolve out of the box.** NixOS enables avahi as a
resolver but not as a publisher (`services.avahi.publish.enable` defaults to
`false`, and `publish.addresses` with it), so the host answers nobody's query
for its own name — `avahi-resolve -n $(uname -n).local` timing out locally is
the quick check. Either turn publishing on:

```nix
services.avahi = {
  enable = true;
  nssmdns4 = true;
  publish = { enable = true; addresses = true; };
};
```

or install from the LAN address instead and make *that* stable, by reserving the
machine's DHCP lease on the router. Both give a permanent origin, which is the
only property that matters here (0012); the name is merely nicer to type.

On the phone, on the same wifi, open the `:8080` address and follow the two
steps it lists — install the certificate, **then** trust it under Settings →
General → About → Certificate Trust Settings. They are separate screens, and
until the second one is done Safari refuses the origin with nothing more
specific than "not private".

Then open the `https://<hostname>.local:8443` address and add it to the home
screen. Prefer that name over the IP: an installed PWA is identified by its
origin, so an app installed from an address DHCP can move loses its stored
library the day it moves
([0012](docs/DECISIONS.md#0012--cloudflare-tunnel-on-an-owned-domain)). The
certificate covers both, and is re-signed automatically when the address
changes — the CA is not, so the phone keeps trusting it.

**A new build shows up one launch late.** Serving a newer bundle does not
change the app that is open: that launch installs the new service worker
behind the one still running, and the *next* launch is the one it takes over
([0038](docs/DECISIONS.md#0038--the-service-worker-is-written-by-hand)). So
after rebuilding, close the app and reopen it — reloading achieves nothing, and
a phone left open stays a build behind.

The CA's private key is a trust anchor on every phone that installed it. It is
generated per machine into `ui/.certs/` (gitignored), never travels, and the
profile is worth removing from the phone once M6's permanent origin exists.

### Reaching a development relay from the phone

`ui-serve` also proxies `/sync` on that same origin to the relay, by default at
`127.0.0.1:8787` and elsewhere with `CABAS_RELAY`
([0044](docs/DECISIONS.md#0044--in-development-the-sync-socket-goes-through-ui-serve)):

```sh
CABAS_RELAY_DATA=.relay cargo run -p cabas-relay    # in another shell
ui-serve                                            # /sync now reaches it
```

The proxy is not a convenience. A page served over `https:` may not open a
`ws:` — the browser blocks it as mixed content — and the relay terminates no
TLS, because in production the tunnel does that in front of it (0012). Without
this the phone, which is the only place M5 can be closed, could not reach a
relay at all. It also keeps development on **one origin**, as production will
be, so the app's default relay URL is the one being exercised.

The relay's own port therefore stays on the loopback and needs no firewall
rule; the phone only ever talks to 8443. If the relay is down, the socket is
refused with a `502` and `ui-serve` prints the command that starts it.

### Two phones, end to end

M5's exit criterion, run by hand: two devices converge, both while they are
online together and while they never are. CI proves this at replica level and
in a browser; this is the part only phones can answer. An iPhone and an Android
is the ideal pair — both install the same PWA, and M7's native Android app is a
nicer wrapper rather than a requirement.

**On the machine**, once:

```sh
nix develop -c build-wasm && nix develop -c pnpm -C ui build
nix develop -c bash -c 'CABAS_RELAY_DATA=.relay cargo run -p cabas-relay' &
nix develop -c ui-serve        # prints both URLs and the CA fingerprint
```

Both phones need to be on the same wifi, and ports **8080 and 8443** open on
the machine (see above). Reserve the machine's DHCP lease on the router first:
the origin is the app's identity on both platforms, so an address that moves
costs both phones their library ([0012](docs/DECISIONS.md#0012--cloudflare-tunnel-on-an-owned-domain)).

**On the iPhone.** Open `http://<address>:8080` and follow the two steps it
lists — install the profile, **then** trust it under Settings → General →
About → Certificate Trust Settings. Skipping the second leaves a certificate
that is installed, listed and still refused. Then open
`https://<address>:8443`, Share → Add to Home Screen, and launch it from there.
Choose **Commencer une famille** and write the twelve words down.

**On the Android.** The phone needs a screen lock before it will accept a
certificate at all — set one first, or the install silently is not offered.
Open `http://<address>:8080`, download `ca.crt`, then Settings → Security →
*More security settings* → Encryption & credentials → Install a certificate →
**CA certificate**, and accept the warning. (That page's own instructions are
written for iOS; the Android path is this one.) Then open
`https://<address>:8443`, Chrome menu → Add to Home screen, launch it, choose
**Rejoindre une famille** and type the twelve words.

Leave **Serveur** empty in Réglages on both. The relay is reached through the
app's own origin, which is the whole point of the proxy above and the shape
production has.

**What to actually watch.** Réglages shows the connection: *Synchronisé* means
the socket is up.

1. **Both open.** Add an ingredient on one; it appears on the other within a
   second. Tick a line in the cart on the second; the first shows it under
   "Acheté", attributed to that person.
2. **Never at the same time.** Force-quit phone A. On B, add a recipe and put
   it on the list, then force-quit B. Open A: the recipe is there. This is the
   case the relay exists for, and the one a broadcast-only server would fail.
3. **Offline, then not.** Airplane mode on A, tick things off, leave the app.
   Turn the network back on and reopen: both agree, and nothing was lost while
   it was away.
4. **The roster.** Réglages → Personnes et appareils on either phone lists both
   people, each with the device they paired.
5. **The journal.** Delete an ingredient on A; on B, Réglages → Journal names
   what went and who did it.
6. **Cold restart.** Force-quit both and reopen: everything is still there, and
   still agrees.

That is M5. What it deliberately does not cover is reaching the relay from
outside the wifi — that is M6's tunnel, which also retires the local CA on both
phones.

Two things need more than the everyday shell, so they get their own — a
browser and an Android SDK are both large downloads that most work never
touches ([DECISIONS 0013](docs/DECISIONS.md#0013--nix-flake-with-a-separate-android-shell)):

```sh
nix develop .#wasm-test -c wasm-test      # store + app, in headless chromium
nix develop .#wasm-test -c ui-test        # the PWA end to end (needs ui/dist)
nix develop .#android                     # SDK/NDK — M7 only
```

`wasm-test` is the only thing that *runs* wasm; `wasm-check` proves the
shared crates compile for it, which is a weaker and much faster claim.
`ui-test` drives the built PWA over the DevTools protocol: mint an identity,
build a library, derive the cart, tick a line, reload from IndexedDB, and open
the whole thing again with the network switched off. It then starts **a real
relay** on 8788 and syncs against it — the app pushes its library, loses its
replica, and gets everything back from the relay alone, with nothing in the
relay's log readable as text.

The home-screen icons are committed PNGs, because iOS reads `apple-touch-icon`
as a bitmap. They are rasterised from the SVG beside them, in the same shell
that owns the browser:

```sh
nix develop .#wasm-test -c node ui/tools/render-icons.mjs
```

The frontend's types are generated from the Rust ones and committed, and CI
fails if they are stale
([0036](docs/DECISIONS.md#0036--typescript-types-are-generated-behind-a-feature)):

```sh
cargo test -p cabas-app --features typescript export_bindings
```

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
| `ui/` | Svelte frontend, and the generated types it is written against | holds no business state; owns every word, no number |

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
| Android | the same PWA today; a Tauri v2 APK at M7 | primary |
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

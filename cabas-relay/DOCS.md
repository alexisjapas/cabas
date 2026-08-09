# cabas

Serves the cabas app and brokers sync between the phones that run it. It holds
no key and can read none of what it stores.

## What it does

Two things, from one address:

- **It serves the app.** The whole Svelte bundle is compiled into the binary,
  so this add-on is what the phones install from and what they load every time
  they open.
- **It brokers sync.** Each device seals its changes on the device itself and
  pushes ciphertext here; the relay appends it to that family's log and
  forwards it to whoever else is connected. A phone that has been offline for a
  week replays the log and catches up. That is the whole reason this is a
  server and not a broadcast — two phones are rarely awake at the same time.

It cannot read any of it. The key is the twelve-word phrase held by the
devices, and it is never sent here.

## Configuration

There is none, and that is deliberate. The relay takes a data directory and a
listen address; inside an add-on the only correct answers are `/data` and
`0.0.0.0:8787`, and both are already the defaults. Nothing to fill in means
nothing to get wrong.

## Installing

1. **Settings → Add-ons → Add-on store → ⋮ → Repositories**, and add
   `https://github.com/alexisjapas/cabas`.
2. Install **cabas** from the store, and start it.
3. Check the log. It says `cabas-relay up` with the number of files it is
   serving. If that number is `0`, this image was built without an app in it —
   report it, do not try to work around it.

Then reach it once over the LAN, at `http://<home-assistant>:8787`, to confirm
the page loads.

## Making it reachable from outside

The point of the app is a phone in a shop, on 4G, so the last step is a
Cloudflare Tunnel onto a domain you own, pointed at this add-on's port 8787.

**The address you choose is permanent.** An installed web app is identified by
its origin: install the phones from `cabas.example.com` and later move to
`cabas.example.net`, and both phones will treat it as a different app — new
icon, empty library, and the twelve words have to be typed again. Pick the
hostname you intend to keep before anyone installs anything.

## Backups

Everything this add-on holds lives in `/data`, which Home Assistant's own
backups cover. That makes it the recovery point if every phone is lost at
once: install the add-on, restore the backup, and pair a phone with the family
phrase — the library comes back from the log.

The phrase is not in the backup, and cannot be. Write it down somewhere else.
A restored `/data` without it is a directory of ciphertext nobody can open.

## Ports

| Port | Why |
|---|---|
| 8787/tcp | The app, the manifest, the wasm core and the `/sync` WebSocket — all of it, because an installed web app cannot have two origins |

`/healthz` on the same port answers `ok` and nothing else. Home Assistant's
watchdog uses it to restart the add-on if the process stops answering.

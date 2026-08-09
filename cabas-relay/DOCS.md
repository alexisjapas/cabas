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

## After changing the family phrase

Changing the phrase is how a lost phone is revoked: every phone moves to a new
family, and **the old one's log stays here**. Nothing collects it, because
nothing here can tell a family that was abandoned from one whose phones have
simply been quiet for a season — and that log is the recovery point if every
phone is lost, so guessing is not on offer.

So it is a thing you do, once, when you have finished re-pairing everyone. From
a shell on the machine (the SSH add-on, then `docker exec` into this one):

```sh
cabas-relay families
```

```
family                              frames       size  last write
5a7bcc53b64acb1c9465f84e1e54ad50       412     154 kB  97 days ago
9f1e2d3c4b5a69788796a5b4c3d2e1f0        26      10 kB  4min ago
```

The abandoned one is the one that stopped when you changed the phrase. Remove
it by naming it in full:

```sh
cabas-relay forget 5a7bcc53b64acb1c9465f84e1e54ad50
```

There is no undo, no prefix matching and no "delete everything older than". You
can run it while the add-on is going: the family you are forgetting is one no
phone connects to any more, which is what abandoned means.

Not doing it costs a directory the size of one family's library. And it is
worth being clear about what it does not do: the old log holds nothing the
holder of the old phrase does not already have on the phone that was lost.
Changing the phrase stops the future, not the past.

## Ports

| Port | Why |
|---|---|
| 8787/tcp | The app, the manifest, the wasm core and the `/sync` WebSocket — all of it, because an installed web app cannot have two origins |

`/healthz` on the same port answers `ok` and nothing else. Home Assistant's
watchdog uses it to restart the add-on if the process stops answering.

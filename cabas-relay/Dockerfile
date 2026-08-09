# The add-on image: a base, one static binary, nothing else (DECISIONS 0049).
#
# There is no build stage here on purpose. `cabas-relay.bin` is cross-compiled
# on the runner by `build-relay`, statically linked against musl, with the
# whole PWA already inside it (DECISIONS 0048) — so this file copies a
# finished artifact and never compiles anything. That is what keeps the arm64
# image off qemu, where building Rust takes tens of minutes and sometimes
# simply stops.
#
# It also means **no `RUN` line belongs here.** Without one, `docker buildx
# --platform linux/arm64` needs no emulation at all on an x86 runner. Adding
# one would not fail the build; it would quietly make it need qemu, which is
# why the workflow deliberately does not install it.
ARG BUILD_FROM
FROM ${BUILD_FROM}

# The binary needs nothing from this image but a kernel. The base is here for
# what surrounds it: s6 as init, so logs reach the Home Assistant UI and a
# stop is a clean one, and a shell — the only way to look inside an add-on on
# an appliance where there is no other way in.
COPY cabas-relay.bin /usr/bin/cabas-relay

# What the Supervisor reads off the image. `io.hass.version` is the one that
# matters: it is compared against `config.yaml`, and a mismatch shows up as an
# add-on that offers an update forever.
#
# The `org.opencontainers.*` set has to be written out too, and not because it
# is missing — the base image supplies its own, so an image that names only
# `source` and `licenses` ends up claiming Alpine's version and Alpine's build
# date as its own. Inherited metadata that describes something else is worse
# than absent metadata.
#
# `created` is the *commit* date rather than the build date, which keeps
# rebuilding a given commit from producing a different answer.
ARG BUILD_VERSION
ARG BUILD_ARCH
ARG BUILD_REVISION
ARG BUILD_CREATED
LABEL \
  io.hass.type="addon" \
  io.hass.version="${BUILD_VERSION}" \
  io.hass.arch="${BUILD_ARCH}" \
  io.hass.name="cabas" \
  io.hass.description="Sync relay and host for the cabas shopping list" \
  org.opencontainers.image.title="cabas" \
  org.opencontainers.image.description="Sync relay and host for the cabas shopping list" \
  org.opencontainers.image.version="${BUILD_VERSION}" \
  org.opencontainers.image.revision="${BUILD_REVISION}" \
  org.opencontainers.image.created="${BUILD_CREATED}" \
  org.opencontainers.image.source="https://github.com/alexisjapas/cabas" \
  org.opencontainers.image.licenses="MIT OR Apache-2.0"

# s6 runs this as the container's program and shuts the container down when it
# exits. The relay reads `CABAS_RELAY_DATA` (default `/data`, which is the
# volume Home Assistant backs up) and `CABAS_RELAY_ADDR` (default
# `0.0.0.0:8787`, which is the port `config.yaml` maps) — both defaults are
# already the right answer here, so there is nothing to pass.
CMD ["/usr/bin/cabas-relay"]

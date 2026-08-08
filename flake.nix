{
  description = "cabas — offline-first shopping list and recipe manager (Rust core, PWA front)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        inherit (pkgs) lib;

        # Stable toolchain driven by rust-toolchain.toml (single source of
        # truth, shared with rustup users). Reproducible through flake.lock.
        # The file also carries the wasm32 target, mandatory at all times
        # (CONSTITUTION Rule 8).
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        # `wasm-bindgen-cli` and the `wasm-bindgen` crate must be the *exact*
        # same version or the generated glue fails at load time with an opaque
        # error. nixpkgs pins the CLI; the Cargo side follows it, never the
        # reverse (CONSTITUTION Rule 13). `check-wasm-bindgen` compares the two
        # and is what the CI calls, so the mismatch is caught by a test rather
        # than by a blank page on an iPhone.
        #
        # Both the declaration *and* the lockfile are checked, because they can
        # disagree: M2 added `loro`, whose transitive `js-sys` pins its own
        # exact `wasm-bindgen`, and that silently resolved the lock to a version
        # the declaration never mentions. The lock is what actually compiles, so
        # a check that only read Cargo.toml would have reported OK on a tree
        # that builds the wrong pair.
        checkWasmBindgen = pkgs.writeShellScriptBin "check-wasm-bindgen" ''
          set -euo pipefail
          cli="$(wasm-bindgen --version | ${pkgs.gawk}/bin/awk '{print $2}')"
          crate="$(${pkgs.gnugrep}/bin/grep -oP '^wasm-bindgen\s*=\s*"=?\K[0-9.]+' Cargo.toml || true)"
          if [ -z "$crate" ]; then
            echo "check-wasm-bindgen: no wasm-bindgen entry in [workspace.dependencies] — skipping the declaration check"
          elif [ "$cli" != "$crate" ]; then
            echo "check-wasm-bindgen: MISMATCH — CLI $cli vs Cargo.toml $crate" >&2
            echo "Align [workspace.dependencies].wasm-bindgen onto the CLI version." >&2
            exit 1
          fi

          # `name = "wasm-bindgen"` followed by its `version = "..."` line.
          locked="$(${pkgs.gnugrep}/bin/grep -A1 '^name = "wasm-bindgen"$' Cargo.lock \
            | ${pkgs.gnugrep}/bin/grep -oP '^version = "\K[0-9.]+' || true)"
          if [ -n "$locked" ] && [ "$cli" != "$locked" ]; then
            echo "check-wasm-bindgen: MISMATCH — CLI $cli vs Cargo.lock $locked" >&2
            echo "The lockfile is what gets compiled. Usually a transitive js-sys" >&2
            echo "pinning a different wasm-bindgen; realign with:" >&2
            echo "  cargo update -p js-sys --precise <version matching $cli>" >&2
            exit 1
          fi
          echo "check-wasm-bindgen: OK ($cli)"
        '';

        # Rule 8 in one command. Scoped to the four crates that genuinely ship
        # on both targets — `relay` is server-side only and would fail the
        # moment axum lands, which would train everyone to ignore this check.
        wasmCheck = pkgs.writeShellScriptBin "wasm-check" ''
          exec cargo check \
            -p cabas-domain -p cabas-store -p cabas-sync -p cabas-app \
            --target wasm32-unknown-unknown "$@"
        '';

        # The wasm core, in the shape the PWA imports it.
        #
        # Two explicit steps rather than `wasm-pack`, for the reason Rule 13
        # exists: this calls *the* `wasm-bindgen` the flake pins, where
        # wasm-pack fetches a CLI of its own choosing and would quietly
        # reintroduce the version skew `check-wasm-bindgen` was written to
        # catch. It also gets the `wasm-release` profile, which wasm-pack has
        # no flag for — on a phone the download is the cost that matters, so
        # the core is built `opt-level = "z"` and then run through wasm-opt.
        #
        # `--target web` emits an ES module the browser loads directly. Vite
        # sees the `.wasm` through `new URL(..., import.meta.url)` and
        # fingerprints it like any other asset.
        #
        # The output is a build product, not a source: `ui/src/lib/wasm/` is
        # gitignored, unlike `ui/src/lib/bindings/`, which is generated *and*
        # committed because CI diffs it (DECISIONS 0036).
        buildWasm = pkgs.writeShellScriptBin "build-wasm" ''
          set -euo pipefail
          cd "$(${pkgs.git}/bin/git rev-parse --show-toplevel)"

          profile="wasm-release"
          artifacts="wasm-release"
          optimise=1
          if [ "''${1:-}" = "--dev" ]; then
            # Seconds instead of a minute, for the edit-reload loop. The
            # module is several times larger and nobody should ship it.
            profile="dev"
            artifacts="debug"
            optimise=0
            shift
          fi

          out="ui/src/lib/wasm"
          cargo build -p cabas-app \
            --target wasm32-unknown-unknown --profile "$profile" "$@"
          wasm-bindgen \
            --target web --out-dir "$out" --out-name cabas \
            "target/wasm32-unknown-unknown/$artifacts/cabas_app.wasm"

          if [ "$optimise" = 1 ]; then
            # The features rustc enables by default for wasm32-unknown-unknown,
            # spelled out because wasm-opt refuses anything it was not told
            # about and the target-features section does not survive
            # wasm-bindgen. Without them this fails with a wall of validator
            # output about `memory.fill` and `i64.trunc_sat_f64_s` rather than
            # anything resembling "add a flag".
            wasm-opt -Oz \
              --enable-sign-ext \
              --enable-mutable-globals \
              --enable-nontrapping-float-to-int \
              --enable-bulk-memory \
              --enable-bulk-memory-opt \
              --enable-multivalue \
              --enable-reference-types \
              --enable-call-indirect-overlong \
              -o "$out/cabas_bg.wasm" "$out/cabas_bg.wasm"
          fi
          echo "build-wasm: $out/cabas_bg.wasm ($(du -h "$out/cabas_bg.wasm" | cut -f1), profile $profile)"
        '';

        # The two things that only a browser can answer, in a real browser.
        #
        # `store`'s IndexedDB backend, because IndexedDB exists nowhere else;
        # and `app`'s scenario, because a missing `getrandom` web backend or a
        # clock that panics on wasm32 are invisible natively and fatal on a
        # phone — a blank page with nothing in the console.
        #
        # Scoped to those two test targets on purpose: the rest of the suite
        # is platform-free and already runs natively in milliseconds, and
        # building it for wasm would only slow the job down to re-prove what
        # `wasm-check` proves in seconds.
        wasmTest = pkgs.writeShellScriptBin "wasm-test" ''
          set -euo pipefail
          export CHROMEDRIVER="${pkgs.chromedriver}/bin/chromedriver"
          # chromedriver launches a browser it finds on PATH; be explicit, so
          # a runner with some other chrome installed cannot change what is
          # being tested.
          export CHROME_PATH="${pkgs.chromium}/bin/chromium"
          cargo test -p cabas-store \
            --target wasm32-unknown-unknown --test indexeddb "$@"
          cargo test -p cabas-app \
            --target wasm32-unknown-unknown --test scenario "$@"
        '';

        # The PWA itself, driven in the same browser, for the same reason.
        #
        # `wasm-test` proves the core survives a browser; this proves the
        # screens do. Everything it covers is invisible anywhere else — the
        # wasm module failing to instantiate under Vite's asset handling, a
        # `null` arriving as `undefined`, an IndexedDB write that never lands.
        # All three look like a blank page on a phone and like nothing at all
        # in a unit test.
        #
        # It expects a built bundle rather than building one: `build-wasm`
        # needs binaryen and the default shell, and a test command that
        # silently rebuilds is a test command nobody can reason about.
        uiTest = pkgs.writeShellScriptBin "ui-test" ''
          set -euo pipefail
          cd "$(${pkgs.git}/bin/git rev-parse --show-toplevel)"

          if [ ! -f ui/dist/index.html ]; then
            echo "ui-test: no ui/dist. Build it first, in the default shell:" >&2
            echo "  nix develop -c build-wasm" >&2
            echo "  nix develop -c pnpm -C ui build" >&2
            exit 1
          fi

          # Refuse to start on a port something else already holds. Without
          # this the harness attaches to *that* browser instead of its own,
          # and then reports on a page this run never opened — a test that
          # lies is worse than a test that fails.
          if ${pkgs.curl}/bin/curl -sf --max-time 2 http://localhost:9222/json/version >/dev/null; then
            echo "ui-test: something is already listening on 9222 (a leaked run?)." >&2
            echo "  pkill -f 'remote-debugging-port=9222'" >&2
            exit 1
          fi
          if ${pkgs.curl}/bin/curl -sf --max-time 2 http://localhost:4173 >/dev/null; then
            echo "ui-test: something is already listening on 4173." >&2
            exit 1
          fi

          profile="$(mktemp -d)"
          pnpm -C ui preview --port 4173 --strictPort >/dev/null 2>&1 &
          server=$!
          chromium --headless --no-sandbox --disable-gpu \
            --remote-debugging-port=9222 --user-data-dir="$profile" \
            --window-size=390,844 about:blank >/dev/null 2>&1 &
          browser=$!

          cleanup() {
            kill "$server" "$browser" 2>/dev/null || true
            wait "$server" "$browser" 2>/dev/null || true
            rm -rf "$profile"
          }
          trap cleanup EXIT INT TERM

          # Both bind their port a moment after forking; polling beats a sleep
          # long enough to be safe on the slowest runner.
          ready=0
          for _ in $(seq 1 100); do
            if ${pkgs.curl}/bin/curl -sf http://localhost:4173 >/dev/null \
              && ${pkgs.curl}/bin/curl -sf http://localhost:9222/json/version >/dev/null; then
              ready=1
              break
            fi
            sleep 0.2
          done
          [ "$ready" = 1 ] || { echo "ui-test: preview or chromium never came up" >&2; exit 1; }

          # Deliberately not `exec`: that would replace this shell and the
          # trap above would never run, leaking a browser and a server that
          # then hold the ports against the next run.
          node ui/tests/smoke.mjs "$@"
        '';

        # Everything the Rust core and the PWA need. Deliberately *not*
        # including the Android SDK: it is a multi-GB download that only M7
        # needs, so it lives in its own shell (`nix develop .#android`).
        webPackages = with pkgs; [
          wasm-pack
          wasm-bindgen-cli
          binaryen # wasm-opt — release size pass on the wasm core
          nodejs_22
          pnpm
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [
            toolchain
            pkgs.pkg-config
          ];

          packages =
            webPackages
            ++ [
              checkWasmBindgen
              wasmCheck
              buildWasm
              pkgs.cargo-nextest
              # Dependency hygiene (Rule 13): `cargo deny check` in CI, and
              # the third-party notices the PWA has to ship.
              pkgs.cargo-deny
              pkgs.cargo-about
            ];

          shellHook = ''
            echo "cabas dev shell — $(rustc --version)"
            echo "  cargo nextest run --workspace         # tests"
            echo "  wasm-check                            # Rule 8: the four shared crates on wasm32"
            echo "  check-wasm-bindgen                    # CLI/crate version match"
            echo "  build-wasm [--dev]                    # the wasm core into ui/src/lib/wasm"
            echo "  nix develop .#wasm-test -c wasm-test   # IndexedDB in headless chromium"
            echo "  nix develop .#android                 # Android SDK/NDK shell (M7)"
            echo "  wasm-bindgen $(wasm-bindgen --version | ${pkgs.gawk}/bin/awk '{print $2}') · node $(node --version)"
          '';
        };

        # A browser is a large download that only the IndexedDB tests need, so
        # it gets its own shell — the same reasoning that keeps the Android
        # SDK out of the everyday one (DECISIONS 0013).
        devShells.wasm-test = pkgs.mkShell {
          nativeBuildInputs = [ toolchain ];
          packages = [
            pkgs.wasm-bindgen-cli
            pkgs.chromium
            pkgs.chromedriver
            wasmTest
            # `ui-test` drives the built PWA in the same browser, so it needs a
            # node to run the harness and a pnpm to serve `ui/dist`.
            pkgs.nodejs_22
            pkgs.pnpm
            uiTest
          ];
          shellHook = ''
            echo "cabas browser shell — the two checks that need a real browser:"
            echo "  wasm-test    # store's IndexedDB and app's scenario, in Rust"
            echo "  ui-test      # the PWA end to end (needs ui/dist)"
          '';
        };

        # M7 only. Kept in a separate shell so the everyday `nix develop` stays
        # a small download. The pinned SDK/NDK versions below are a starting
        # point and are validated at the beginning of M7, not before — nothing
        # depends on them until then.
        devShells.android =
          let
            androidPkgs = import nixpkgs {
              inherit system;
              config = {
                allowUnfree = true;
                android_sdk.accept_license = true;
              };
              overlays = [ rust-overlay.overlays.default ];
            };
            androidSdk = androidPkgs.androidenv.composeAndroidPackages {
              platformVersions = [ "35" ];
              buildToolsVersions = [ "35.0.0" ];
              includeNDK = true;
            };
            androidToolchain = androidPkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          in
          androidPkgs.mkShell {
            nativeBuildInputs = [
              (androidToolchain.override {
                targets = [
                  "wasm32-unknown-unknown"
                  "aarch64-linux-android"
                  "armv7-linux-androideabi"
                ];
              })
              androidPkgs.pkg-config
            ];

            packages =
              webPackages
              ++ (with androidPkgs; [
                cargo-ndk
                jdk21
                androidSdk.androidsdk
              ]);

            ANDROID_HOME = "${androidSdk.androidsdk}/libexec/android-sdk";
            ANDROID_SDK_ROOT = "${androidSdk.androidsdk}/libexec/android-sdk";
            ANDROID_NDK_ROOT = "${androidSdk.androidsdk}/libexec/android-sdk/ndk-bundle";

            shellHook = ''
              echo "cabas android shell (M7) — $(rustc --version)"
              echo "  ANDROID_HOME=$ANDROID_HOME"
              echo "  SDK/NDK versions are unvalidated until M7 starts — see ROADMAP M7."
            '';
          };

        formatter = pkgs.nixfmt-tree;
      }
    );
}

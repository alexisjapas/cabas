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
        checkWasmBindgen = pkgs.writeShellScriptBin "check-wasm-bindgen" ''
          set -euo pipefail
          cli="$(wasm-bindgen --version | ${pkgs.gawk}/bin/awk '{print $2}')"
          crate="$(${pkgs.gnugrep}/bin/grep -oP '^wasm-bindgen\s*=\s*"=?\K[0-9.]+' Cargo.toml || true)"
          if [ -z "$crate" ]; then
            echo "check-wasm-bindgen: no wasm-bindgen entry in [workspace.dependencies] yet (pre-M3) — skipping"
            exit 0
          fi
          if [ "$cli" != "$crate" ]; then
            echo "check-wasm-bindgen: MISMATCH — CLI $cli vs Cargo.toml $crate" >&2
            echo "Align [workspace.dependencies].wasm-bindgen onto the CLI version." >&2
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
            echo "  nix develop .#android                 # Android SDK/NDK shell (M7)"
            echo "  wasm-bindgen $(wasm-bindgen --version | ${pkgs.gawk}/bin/awk '{print $2}') · node $(node --version)"
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

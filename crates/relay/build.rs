//! Embeds the built PWA into the relay binary (DECISIONS 0048).
//!
//! The add-on is one image serving one origin: the static bundle and the
//! sync socket (DECISIONS 0010, 0012). This script walks `ui/dist` and
//! writes a table of `include_bytes!` into `OUT_DIR`, which `assets.rs`
//! includes. There is no dependency and no crate that must exist for the
//! workspace to build: **a missing `ui/dist` produces an empty table**, so
//! `cargo clippy --workspace` works in a fresh checkout, where the bundle
//! is a gitignored build product that nobody has produced yet.
//!
//! Two knobs, both for the release image:
//!
//! - `CABAS_UI_DIST` — where the bundle is, if not `<repo>/ui/dist`. The
//!   Docker build builds the frontend in another stage and puts it
//!   elsewhere.
//! - `CABAS_EMBED_UI=required` — fail the build instead of embedding
//!   nothing. An image that ships a relay with no app in it is the one
//!   failure that must not be quiet, and it is invisible until a phone
//!   asks for the page.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo::rerun-if-env-changed=CABAS_UI_DIST");
    println!("cargo::rerun-if-env-changed=CABAS_EMBED_UI");

    let dist = match std::env::var_os("CABAS_UI_DIST") {
        Some(path) => PathBuf::from(path),
        // `crates/relay` → the repository root.
        None => manifest_dir().join("../../ui/dist"),
    };
    // Additions and removals: `include_bytes!` makes Cargo watch each file
    // it names, but only the directory tells it a *new* file appeared.
    println!("cargo::rerun-if-changed={}", dist.display());

    let mut files = Vec::new();
    if dist.is_dir() {
        collect(&dist, &mut String::new(), &mut files);
        files.sort_by(|a, b| a.0.cmp(&b.0));
    }

    let required = std::env::var("CABAS_EMBED_UI").is_ok_and(|v| v == "required");
    let complete = files.iter().any(|(path, _)| path == "index.html");
    if required && !complete {
        panic!(
            "CABAS_EMBED_UI=required, but {} holds no index.html.\n\
             Build the bundle first: `build-wasm && pnpm -C ui build`.",
            dist.display()
        );
    }
    if !complete {
        println!(
            "cargo::warning=no PWA bundle at {} — this relay will broker sync and serve no app",
            dist.display()
        );
    }

    let mut table = String::from("pub(crate) static ASSETS: &[Asset] = &[\n");
    for (path, file) in &files {
        let bytes = fs::read(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        // `{:?}` writes a Rust string literal, escapes and all — the paths
        // come from a bundler and are plain, but generated code that only
        // works for plain input is a trap for the day one is not.
        let _ = writeln!(
            table,
            "    Asset {{ path: {:?}, etag: {:?}, bytes: include_bytes!({:?}) }},",
            path,
            etag(&bytes),
            file
        );
    }
    table.push_str("];\n");

    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    fs::write(out.join("assets.rs"), table).expect("write assets.rs");
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
}

/// Depth-first walk, accumulating `/`-separated paths relative to the root —
/// the shape a URL asks for, so the lookup is a comparison and never a join.
fn collect(dir: &Path, prefix: &mut String, out: &mut Vec<(String, PathBuf)>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    for entry in entries {
        let entry = entry.expect("read_dir entry");
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            panic!("{}: not UTF-8", entry.path().display());
        };
        let mark = prefix.len();
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(name);

        if entry.path().is_dir() {
            collect(&entry.path(), prefix, out);
        } else {
            out.push((prefix.clone(), entry.path()));
        }
        prefix.truncate(mark);
    }
}

/// FNV-1a over the bytes, plus the length, as the entity tag.
///
/// A validator only has to change when the file does; nothing authenticates
/// anything against it, so this is the same trade the service worker's cache
/// name makes (DECISIONS 0038) — arithmetic instead of a hash dependency.
fn etag(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("\"{:x}-{:016x}\"", bytes.len(), hash)
}

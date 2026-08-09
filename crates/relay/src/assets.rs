//! The static half: the PWA, served from the same origin as `/sync`.
//!
//! One origin is not a convenience, it is the app's identity. An installed
//! PWA is keyed to the origin it was installed from — change it and iOS
//! treats it as a different app, icon gone and IndexedDB dropped (DECISIONS
//! 0012). Serving the bundle from the relay is what makes that one address
//! for good, and it is why the add-on is a single image (DECISIONS 0048).
//!
//! The table comes from `build.rs`, which walked `ui/dist`. It may be
//! empty — a fresh checkout has no bundle — and then every path here is a
//! 404 and the relay is a sync broker and nothing else.
//!
//! Three rules carry the whole module, and each one is a bug that has
//! already been paid for elsewhere in this repo:
//!
//! - **`assets/*` is immutable, everything else revalidates.** Vite puts a
//!   content hash in the name of everything it compiles, so those may be
//!   cached for a year; `index.html`, `sw.js`, the manifest and the icons
//!   have stable names and must not be. `sw.js` above all: the browser
//!   decides there is a new build by fetching that one file and comparing
//!   its bytes (DECISIONS 0038), so a cached copy is an app that can never
//!   update.
//! - **No `Vary`, ever.** A `Vary: Origin` makes the Cache API match on the
//!   request's `Origin` header, and the service worker precaches with
//!   requests that carry none while the page asks for its `crossorigin` JS
//!   and CSS with one. Every asset cached, every lookup a miss — invisible
//!   online, a blank page offline. The worker defends itself with
//!   `ignoreVary`, and the server it was written against should not need it.
//! - **An unknown path is a 404, not the page.** Which screen is open is
//!   core state and never a URL (DECISIONS 0037), so there is no route to
//!   fall back for. Answering `index.html` to a mistyped asset name turns a
//!   missing file into a page that loads and does nothing.

use axum::body::Body;
use axum::http::{HeaderMap, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};

/// One embedded file. `path` has no leading slash and uses `/` separators —
/// the shape a request carries, so matching is a comparison.
pub(crate) struct Asset {
    path: &'static str,
    etag: &'static str,
    bytes: &'static [u8],
}

// Written by `build.rs` into `OUT_DIR`: `static ASSETS: &[Asset]`, sorted by
// path.
include!(concat!(env!("OUT_DIR"), "/assets.rs"));

/// How many files were embedded. The process says so at startup, because
/// "the app does not load" and "there is no app in this build" are the same
/// symptom from a phone and very different afternoons.
pub fn embedded() -> usize {
    ASSETS.len()
}

/// Content-hashed by the bundler, so the URL cannot outlive its contents.
const IMMUTABLE: &str = "public, max-age=31536000, immutable";
/// Cached, and checked every time. With an `ETag` the check is a 304.
const REVALIDATE: &str = "no-cache";

pub(crate) async fn handler(uri: Uri, headers: HeaderMap) -> Response {
    respond(
        ASSETS,
        uri.path(),
        headers
            .get(header::IF_NONE_MATCH)
            .and_then(|value| value.to_str().ok()),
    )
}

/// The whole policy, as a function of a table and a request — which is what
/// lets it be tested without a bundle on disk.
fn respond(assets: &[Asset], path: &str, if_none_match: Option<&str>) -> Response {
    // `/` is the page. Everything else is a file name as the bundler wrote
    // it: ASCII, no escapes, so there is nothing to percent-decode. A name
    // that needed it would simply not be found.
    let wanted = match path.trim_start_matches('/') {
        "" => "index.html",
        rest => rest,
    };

    let Ok(index) = assets.binary_search_by(|asset| asset.path.cmp(wanted)) else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };
    let asset = &assets[index];

    let caching = if asset.path.starts_with("assets/") {
        IMMUTABLE
    } else {
        REVALIDATE
    };
    let unchanged = if_none_match.is_some_and(|header| matches(header, asset.etag));

    let mut response = Response::builder()
        .header(header::ETAG, asset.etag)
        .header(header::CACHE_CONTROL, caching)
        // The bundle carries a `.map` next to every script and the icons are
        // drawn by us, but the relay is on the public internet behind the
        // tunnel and sniffing is never what we meant.
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff");

    if unchanged {
        return response
            .status(StatusCode::NOT_MODIFIED)
            .body(Body::empty())
            .expect("static header values");
    }

    response = response.header(header::CONTENT_TYPE, content_type(asset.path));
    response
        .body(Body::from(asset.bytes))
        .expect("static header values")
}

/// `If-None-Match` is a list, and a proxy is entitled to weaken any of its
/// entries to `W/"…"` on the way through. `*` matches anything that exists.
fn matches(header: &str, etag: &str) -> bool {
    header.split(',').any(|candidate| {
        let candidate = candidate.trim();
        candidate == "*" || candidate.trim_start_matches("W/") == etag
    })
}

/// By extension, over the closed set of things this bundle contains.
///
/// `application/wasm` is the one that is not cosmetic: without it the
/// browser refuses to stream-compile the core and falls back to buffering
/// the whole 1.8 MB first — or, on a stricter engine, refuses outright.
fn content_type(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, ext)| ext) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("webmanifest") => "application/manifest+json; charset=utf-8",
        // Source maps, and anything else the bundler labels JSON.
        Some("json" | "map") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/vnd.microsoft.icon",
        Some("woff2") => "font/woff2",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table shaped like a real bundle, so the tests say nothing about
    /// whether anyone has run `pnpm build`.
    const FIXTURE: &[Asset] = &[
        Asset {
            path: "assets/app-abc123.js",
            etag: "\"3-0000000000000001\"",
            bytes: b"app",
        },
        Asset {
            path: "icons/icon-192.png",
            etag: "\"3-0000000000000002\"",
            bytes: b"png",
        },
        Asset {
            path: "index.html",
            etag: "\"4-0000000000000003\"",
            bytes: b"page",
        },
        Asset {
            path: "sw.js",
            etag: "\"6-0000000000000004\"",
            bytes: b"worker",
        },
    ];

    fn header(response: &Response, name: header::HeaderName) -> Option<&str> {
        response.headers().get(name)?.to_str().ok()
    }

    #[test]
    fn the_root_is_the_page() {
        let response = respond(FIXTURE, "/", None);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            header(&response, header::CONTENT_TYPE),
            Some("text/html; charset=utf-8")
        );
    }

    #[test]
    fn an_unknown_path_is_not_the_page() {
        // The failure this forbids: a typo'd asset name answering 200 with
        // markup, which loads and then does nothing (DECISIONS 0037).
        let response = respond(FIXTURE, "/recipes", None);
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn hashed_assets_are_immutable_and_the_shell_is_not() {
        let hashed = respond(FIXTURE, "/assets/app-abc123.js", None);
        assert_eq!(header(&hashed, header::CACHE_CONTROL), Some(IMMUTABLE));

        // The one that decides whether a new build is ever noticed.
        let worker = respond(FIXTURE, "/sw.js", None);
        assert_eq!(header(&worker, header::CACHE_CONTROL), Some(REVALIDATE));

        let page = respond(FIXTURE, "/", None);
        assert_eq!(header(&page, header::CACHE_CONTROL), Some(REVALIDATE));

        // Names the bundler did not hash: stale forever if this ever moves.
        let icon = respond(FIXTURE, "/icons/icon-192.png", None);
        assert_eq!(header(&icon, header::CACHE_CONTROL), Some(REVALIDATE));
    }

    #[test]
    fn nothing_ever_varies() {
        // Not a style choice: `Vary: Origin` and the worker's precache
        // cannot both be right, and the phone is offline when it matters.
        for path in ["/", "/sw.js", "/assets/app-abc123.js"] {
            let response = respond(FIXTURE, path, None);
            assert_eq!(header(&response, header::VARY), None, "{path}");
        }
    }

    #[test]
    fn a_matching_etag_is_answered_with_nothing() {
        let response = respond(FIXTURE, "/", Some("\"4-0000000000000003\""));
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
        // A 304 still has to carry the validator and the policy, or the next
        // request arrives with no `If-None-Match` and pays for the body.
        assert_eq!(
            header(&response, header::ETAG),
            Some("\"4-0000000000000003\"")
        );
        assert_eq!(header(&response, header::CACHE_CONTROL), Some(REVALIDATE));
    }

    #[test]
    fn a_stale_etag_is_answered_with_the_file() {
        let response = respond(FIXTURE, "/", Some("\"4-000000000000ffff\""));
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn a_weakened_or_listed_etag_still_matches() {
        // Both shapes a proxy is entitled to produce on the way through.
        let weak = respond(FIXTURE, "/sw.js", Some("W/\"6-0000000000000004\""));
        assert_eq!(weak.status(), StatusCode::NOT_MODIFIED);

        let listed = respond(FIXTURE, "/sw.js", Some("\"other\", \"6-0000000000000004\""));
        assert_eq!(listed.status(), StatusCode::NOT_MODIFIED);
    }

    #[test]
    fn the_core_is_labelled_so_it_can_stream() {
        assert_eq!(content_type("assets/cabas_bg-x.wasm"), "application/wasm");
        assert_eq!(
            content_type("manifest.webmanifest"),
            "application/manifest+json; charset=utf-8"
        );
        assert_eq!(content_type("favicon.svg"), "image/svg+xml");
        assert_eq!(content_type("LICENSE"), "application/octet-stream");
    }

    /// `respond` binary-searches, which is a silent lie if `build.rs` ever
    /// stops sorting.
    #[test]
    fn the_embedded_table_is_sorted() {
        assert!(ASSETS.windows(2).all(|pair| pair[0].path < pair[1].path));
    }
}

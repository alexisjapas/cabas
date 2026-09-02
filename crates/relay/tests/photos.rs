//! A photo taken on one phone, seen on the other (DECISIONS 0080, 0092).
//!
//! `convergence.rs` is this file's counterpart for `/sync`, and the two are
//! deliberately separate: they share a port and a group key and nothing else —
//! no cursor, no epoch, no sequence — so a test that drove both would be
//! testing the port.
//!
//! Everything here is real except the clock: real `Photos` over real stores, a
//! real relay writing real files, real WebSockets between them, and every
//! payload sealed. The client loop each scenario drives by hand — hello,
//! welcome, then one fetch or one push at a time until the queues drain — is
//! the loop `ui/src/lib/photos.svelte.ts` drives in the browser;
//! `cabas_app::PhotoSync` is what keeps the two honest about doing it
//! identically.

use cabas_app::{PhotoStatus, PhotoSync, Photos};
use cabas_domain::PhotoId;
use cabas_store::MemoryPhotoStore;

use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

const PHRASE: &str = "abandon abandon abandon abandon abandon abandon \
                      abandon abandon abandon abandon abandon about";

/// A directory that cleans itself up, so the tests need no dev-dep.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "cabas-photo-transfer-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn spawn_relay(root: PathBuf) -> SocketAddr {
    let relay = cabas_relay::Relay::open(root).expect("data dir");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, cabas_relay::router(relay))
            .await
            .expect("serve");
    });
    addr
}

async fn connect(addr: SocketAddr) -> Ws {
    let (ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/photos"))
        .await
        .expect("connect");
    ws
}

/// The next binary message, or a loud failure — a hang here is a protocol
/// bug, and CI deserves a message over a timeout.
async fn recv(ws: &mut Ws) -> Vec<u8> {
    let deadline = std::time::Duration::from_secs(10);
    loop {
        let message = tokio::time::timeout(deadline, ws.next())
            .await
            .expect("a message within 10s")
            .expect("an open socket")
            .expect("a healthy socket");
        match message {
            Message::Binary(bytes) => return bytes,
            Message::Close(_) => panic!("closed while a message was expected"),
            _ => continue,
        }
    }
}

/// A JPEG, as far as anything in this workspace is concerned: the three magic
/// bytes `app::photos` checks, and then whatever. The pictures are not decoded
/// anywhere — the relay cannot even see them.
fn jpeg(fill: u8, len: usize) -> Vec<u8> {
    let mut bytes = vec![0xff, 0xd8, 0xff];
    bytes.resize(len, fill);
    bytes
}

/// One whole photo connection: hello, welcome, then one message at a time
/// until the device says the work is done.
///
/// The pace is the device's and the *end* is the device's, which is the shape
/// of the protocol (DECISIONS 0080): the relay never says "that is all",
/// because it cannot know what this device still wants.
async fn transfer(
    photos: &Photos<MemoryPhotoStore>,
    referenced: &[PhotoId],
    addr: SocketAddr,
) -> PhotoStatus {
    let mut sync = PhotoSync::open(PHRASE, photos, referenced)
        .await
        .expect("the phrase derives");
    let mut ws = connect(addr).await;
    ws.send(Message::Binary(sync.hello().expect("hello")))
        .await
        .expect("send hello");

    loop {
        let wire = recv(&mut ws).await;
        if let cabas_app::PhotoEvent::Refused { reason } =
            sync.handle(photos, &wire).await.expect("a server message")
        {
            panic!("refused: {reason}");
        }
        if sync.done() {
            break;
        }
        // One request in flight at a time, which is what makes a queue a queue
        // rather than a burst a phone has to buffer.
        if let Some(fetch) = sync.fetch().expect("fetch") {
            ws.send(Message::Binary(fetch)).await.expect("send fetch");
            continue;
        }
        if let Some(push) = sync.push(photos).await.expect("push") {
            ws.send(Message::Binary(push)).await.expect("send push");
        }
        // Neither, and not done: an answer is outstanding, so go and read it.
    }

    ws.close(None).await.expect("close");
    sync.status()
}

/// The whole point of the endpoint: a photo taken here turns up there.
///
/// The two devices are never online together, exactly as in `convergence.rs`
/// — which is the case a broadcast relay cannot serve and the reason this one
/// keeps the bytes.
#[tokio::test(flavor = "multi_thread")]
async fn a_photo_taken_on_one_device_reaches_the_other() {
    let dir = TempDir::new("across");
    let addr = spawn_relay(dir.0.clone()).await;

    let taken = jpeg(0xa5, 20_000);

    // The phone that took it.
    let alice = Photos::new(MemoryPhotoStore::new());
    let id = alice
        .put(&cabas_app::SystemPlatform, &taken)
        .await
        .expect("stored locally");
    let status = transfer(&alice, std::slice::from_ref(&id), addr).await;
    assert_eq!(status.sent, 1, "one photo uploaded");
    assert_eq!(status.received, 0);
    assert!(status.done);

    // The phone that has merged the document and so *names* the photo, with
    // no bytes for it. That is an ordinary state and the one this endpoint
    // exists to end (Rule 6).
    let bob = Photos::new(MemoryPhotoStore::new());
    assert_eq!(bob.get(&id).await.expect("read"), None);

    let status = transfer(&bob, std::slice::from_ref(&id), addr).await;
    assert_eq!(status.received, 1, "one photo fetched");
    assert_eq!(status.sent, 0);
    assert_eq!(status.dropped, 0);
    assert_eq!(
        bob.get(&id).await.expect("read"),
        Some(taken.clone()),
        "byte for byte, through a relay that never held the key"
    );

    // And what the relay actually wrote down is not the picture. Rule 7 is
    // asserted rather than assumed: this is the one endpoint where the
    // untrusted party stores something a device will later write to disk.
    let stored = std::fs::read(dir.0.join(hex_group()).join("photos").join(id.to_string()))
        .expect("the relay wrote a file named after the photo");
    assert_ne!(stored, taken, "the relay must hold ciphertext");
    assert!(
        !stored.windows(3).any(|w| w == [0xff, 0xd8, 0xff]),
        "not even a JPEG header survives sealing"
    );
}

/// The second connection has nothing to do, and says so in one round trip.
///
/// This is the ordinary case — a phone opens the app, the library has not
/// changed, and the whole exchange is a hello and a welcome. It is worth a
/// test because the alternative shapes (re-offering everything, streaming
/// everything) are invisible until somebody watches a data allowance.
#[tokio::test(flavor = "multi_thread")]
async fn a_device_that_is_up_to_date_transfers_nothing() {
    let dir = TempDir::new("nothing");
    let addr = spawn_relay(dir.0.clone()).await;

    let alice = Photos::new(MemoryPhotoStore::new());
    let id = alice
        .put(&cabas_app::SystemPlatform, &jpeg(0x11, 4_096))
        .await
        .expect("stored");
    transfer(&alice, std::slice::from_ref(&id), addr).await;

    let status = transfer(&alice, std::slice::from_ref(&id), addr).await;
    assert_eq!((status.sent, status.received, status.pending), (0, 0, 0));
    assert!(status.done);
}

/// A photo the group does not have yet: the welcome says so by leaving it out
/// of `available`, and nothing stalls waiting for bytes nobody holds.
///
/// The state is ordinary — the picture is on a phone that has not opened the
/// app since — and the screen keeps showing a placeholder, which is what
/// `Photo.svelte` renders (Rule 6).
#[tokio::test(flavor = "multi_thread")]
async fn a_photo_nobody_has_uploaded_yet_is_simply_not_offered() {
    let dir = TempDir::new("absent");
    let addr = spawn_relay(dir.0.clone()).await;

    let bob = Photos::new(MemoryPhotoStore::new());
    let wanted = PhotoId::from_raw("pho_0000000000000009");

    let status = transfer(&bob, std::slice::from_ref(&wanted), addr).await;
    assert_eq!((status.sent, status.received, status.dropped), (0, 0, 0));
    assert!(status.done);
    assert_eq!(bob.get(&wanted).await.expect("read"), None);
}

/// Everything a group has, to a phone that has just typed the twelve words.
///
/// The join is the case that moves the most bytes, and the one where a relay
/// that streamed its answer would hand a phone tens of megabytes at once
/// (DECISIONS 0080). Here it is a queue: one fetch, one photo, repeat.
#[tokio::test(flavor = "multi_thread")]
async fn a_phone_that_has_just_joined_fetches_the_whole_library() {
    let dir = TempDir::new("join");
    let addr = spawn_relay(dir.0.clone()).await;

    let alice = Photos::new(MemoryPhotoStore::new());
    let mut ids = Vec::new();
    for n in 0..5u8 {
        ids.push(
            alice
                .put(&cabas_app::SystemPlatform, &jpeg(n, 8_000 + usize::from(n)))
                .await
                .expect("stored"),
        );
    }
    let status = transfer(&alice, &ids, addr).await;
    assert_eq!(status.sent, 5);

    let bob = Photos::new(MemoryPhotoStore::new());
    let status = transfer(&bob, &ids, addr).await;
    assert_eq!(status.received, 5);
    for id in &ids {
        assert_eq!(
            bob.get(id).await.expect("read"),
            alice.get(id).await.expect("read"),
        );
    }
}

/// The group id `PHRASE` derives, in the hex the relay names directories with.
fn hex_group() -> String {
    cabas_sync::GroupKey::from_phrase(PHRASE)
        .expect("the phrase derives")
        .id()
        .to_hex()
}

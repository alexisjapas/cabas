//! M5's exit criterion: two devices converge through the relay, **including
//! when they are never online at the same time**.
//!
//! Everything here is real except the clock: real `App`s over real
//! documents, a real relay appending to a real directory, real WebSockets
//! between them, and every payload sealed — the relay in this test could
//! not cheat if it wanted to, it never holds a key. The client loop each
//! scenario drives by hand (connect, replay, merge, push, persist the
//! cursor) is the loop the PWA wiring will drive from `session.svelte.ts`;
//! `cabas_sync::Session` keeps the two honest about doing it identically.
//!
//! **M10's criterion is at the bottom of this file, and it is here rather
//! than beside it** (DECISIONS 0080): a photo travelling between two devices
//! that are never online together is the same sentence about a different
//! socket, and it needs the same two `App`s, the same relay and the same
//! never-simultaneous choreography. A second file would have been a second
//! copy of all of it, kept in step by hand.

use cabas_app::command::{IngredientInput, QuantityInput};
use cabas_app::tags::{AisleTag, CheckStateTag, KeepingTag, UnitTag};
use cabas_app::view::StateView;
use cabas_app::{App, Command, Identity, Platform};
use cabas_app::{PhotoEvent, PhotoStatus, PhotoSync, Photos};
use cabas_domain::{PhotoId, Timestamp};
use cabas_store::{MemoryPhotoStore, MemoryStorage};
use cabas_sync::protocol::{ClientMessage, FrameKind, encode_client};
use cabas_sync::{Event, GroupKey, Session};

use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// A clock that does not tick and a "random" source that counts from a
/// per-device base — so ids are readable in a failing assertion *and* two
/// devices never mint the same one.
#[derive(Debug)]
struct TestPlatform {
    counter: std::cell::Cell<u64>,
}

impl TestPlatform {
    fn from(base: u64) -> Self {
        TestPlatform {
            counter: std::cell::Cell::new(base),
        }
    }
}

impl Platform for TestPlatform {
    fn now(&self) -> Timestamp {
        Timestamp(1_700_000_000_000)
    }

    fn random_u64(&self) -> cabas_app::Result<u64> {
        self.counter.set(self.counter.get() + 1);
        Ok(self.counter.get())
    }
}

/// A directory that cleans itself up, so the tests need no dev-dep.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "cabas-convergence-{tag}-{}-{:?}",
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

/// One device: a replica, a phrase, and the cursor + shadow the sync loop
/// persists between connections (DECISIONS 0042).
struct Device {
    app: App<MemoryStorage, TestPlatform>,
    /// The bytes the document only names, on this device (DECISIONS 0062).
    /// Empty in every scenario but the last one, which is the point: a photo
    /// is not part of the replica and no amount of syncing moves it.
    photos: Photos<MemoryPhotoStore>,
    /// A second counter, for the ids the camera mints. `App`'s own is busy
    /// numbering ingredients, and a photo id has to be unique across
    /// devices for the same reason every other id does.
    camera: TestPlatform,
    phrase: String,
    epoch: u64,
    since: u64,
    shadow: Vec<u8>,
}

impl Device {
    async fn join(phrase: &str, base: u64, user: &str, name: &str, device: &str) -> Self {
        let identity = Identity {
            user: Some(user.into()),
            user_name: Some(name.into()),
            device: device.into(),
            device_name: format!("{name}'s device"),
        };
        Device {
            app: App::open(MemoryStorage::new(), TestPlatform::from(base), identity)
                .await
                .expect("the app opens"),
            photos: Photos::new(MemoryPhotoStore::new()),
            camera: TestPlatform::from(base + 1_000_000),
            phrase: phrase.to_string(),
            epoch: 0,
            since: 0,
            // The empty shadow: nothing pushed yet, so the first delta is
            // everything this device knows.
            shadow: Vec::new(),
        }
    }

    fn key(&self) -> GroupKey {
        GroupKey::from_phrase(&self.phrase).expect("the phrase derives")
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
    connect_to(addr, "/sync").await
}

async fn connect_to(addr: SocketAddr, path: &str) -> Ws {
    let (ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}{path}"))
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

/// One full sync: connect, replay into the replica, and if the device has
/// local changes, push them and wait for durability. Returns with the
/// cursor persisted and the socket closed — "never online at the same
/// time" is a sequence of these.
async fn sync_once(device: &mut Device, addr: SocketAddr, dirty: bool) {
    let mut ws = connect(addr).await;
    let mut session = Session::new(device.key(), device.epoch, device.since);
    ws.send(Message::Binary(session.hello().expect("hello")))
        .await
        .expect("send hello");

    loop {
        match session
            .handle(&recv(&mut ws).await)
            .expect("a server message")
        {
            Event::Connected | Event::Dropped { .. } => {}
            Event::Merge(plaintext) => {
                device.app.merge(&plaintext).expect("merge");
            }
            Event::CaughtUp => break,
            Event::Acked { .. } => panic!("acked before anything was pushed"),
            Event::Refused { reason } => panic!("refused: {reason}"),
        }
    }

    // `dirty` is the ordinary reason to push. A reset session is the other
    // one, and it owes a push whether or not anything changed here: its
    // shadow describes a log the relay no longer has (DECISIONS 0054). What
    // goes out then is the whole replica, which is what `SyncSession::push`
    // decides for both real hosts — mirrored here rather than shared, because
    // this file drives the protocol a layer below it.
    if dirty || session.reset() {
        let version_after = device.app.version();
        let push = if session.reset() {
            let replica = device.app.changes_since(&[]).expect("export the replica");
            session.snapshot(&replica)
        } else {
            let delta = device
                .app
                .changes_since(&device.shadow)
                .expect("export the delta");
            session.delta(&delta)
        }
        .expect("seal");
        ws.send(Message::Binary(push)).await.expect("send push");
        loop {
            match session
                .handle(&recv(&mut ws).await)
                .expect("a server message")
            {
                Event::Acked { .. } => break,
                Event::Merge(plaintext) => {
                    device.app.merge(&plaintext).expect("merge");
                }
                Event::Dropped { .. } => {}
                other => panic!("expected an ack, got {other:?}"),
            }
        }
        device.shadow = version_after;
    }

    let (epoch, since) = session.cursor();
    device.epoch = epoch;
    device.since = since;
    ws.close(None).await.expect("close");
}

/// The shared state — everything synced, nothing device-local. `me` and
/// `focus` differ between devices by design; `revision` counts renders.
fn shared(view: &StateView) -> String {
    format!(
        "{:?} | {:?} | {:?} | {:?} | {:?}",
        view.cart, view.list, view.recipes, view.ingredients, view.problems
    )
}

fn save_ingredient(name: &str) -> Command {
    Command::SaveIngredient {
        ingredient: IngredientInput {
            id: None,
            name: name.into(),
            aliases: Vec::new(),
            aisle: AisleTag::Produce,
            shops: Vec::new(),
            keeping: KeepingTag::Ambient,
            staple: false,
            density: None,
            unit_weight: None,
            default_quantity: None,
            photo: None,
        },
    }
}

fn ingredient_id(view: &StateView, name: &str) -> String {
    view.ingredients
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("{name} is in the library"))
        .id
        .clone()
}

/// Two replicas that are never online at the same time still converge —
/// the reason the relay persists anything at all (DECISIONS 0009), and the
/// milestone's exit criterion.
#[tokio::test(flavor = "multi_thread")]
async fn never_simultaneous_devices_converge() {
    let dir = TempDir::new("sequential");
    let addr = spawn_relay(dir.0.clone()).await;
    let phrase = GroupKey::generate().expect("generate").phrase().to_string();

    // Alice, at home: five tomatoes on the list. Online alone, then gone.
    let mut alice = Device::join(&phrase, 0, "usr_alice", "Alice", "dev_phone").await;
    let view = alice
        .app
        .dispatch(save_ingredient("Tomates"))
        .await
        .expect("save");
    let tomatoes = ingredient_id(&view, "Tomates");
    alice
        .app
        .dispatch(Command::AddIngredientToList {
            ingredient: tomatoes.clone(),
            quantity: Some(QuantityInput {
                amount: "5".into(),
                unit: UnitTag::Piece,
            }),
        })
        .await
        .expect("add to list");
    sync_once(&mut alice, addr, true).await;

    // A stranger who found the group id but not the phrase appends noise.
    // Nobody merges it: it does not open (DECISIONS 0042).
    {
        let mut ws = connect(addr).await;
        let intruder = Session::new(alice.key(), 0, 0);
        ws.send(Message::Binary(intruder.hello().expect("hello")))
            .await
            .expect("send");
        let push = encode_client(&ClientMessage::Push {
            kind: FrameKind::Delta,
            payload: vec![0xbb; 96], // not sealed by anything
        })
        .expect("encode");
        ws.send(Message::Binary(push)).await.expect("send");
        // Drain until the ack so the append is durably in the log before
        // Bob connects.
        let mut session = Session::new(alice.key(), 0, 0);
        loop {
            if let Event::Acked { .. } = session.handle(&recv(&mut ws).await).expect("msg") {
                break;
            }
        }
        ws.close(None).await.expect("close");
    }

    // Bob, later, from an empty replica: pairs with the phrase, replays,
    // and finds Alice's list — then ticks the tomatoes off in the shop.
    let mut bob = Device::join(&phrase, 5000, "usr_bob", "Bob", "dev_laptop").await;
    sync_once(&mut bob, addr, true).await;
    let view = bob.app.state().expect("state");
    assert_eq!(view.list.len(), 1, "Alice's entry reached Bob");
    assert_eq!(view.cart.to_buy.len(), 1);
    assert_eq!(view.cart.to_buy[0].name, "Tomates");

    bob.app
        .dispatch(Command::ToggleCartItem {
            ingredient: tomatoes.clone(),
        })
        .await
        .expect("check");
    sync_once(&mut bob, addr, true).await;

    // Alice reconnects — Bob is long gone — and sees who bought what.
    sync_once(&mut alice, addr, false).await;
    let alice_view = alice.app.state().expect("state");
    let bob_view = bob.app.state().expect("state");

    assert_eq!(alice_view.cart.bought.len(), 1);
    assert_eq!(alice_view.cart.bought[0].state, CheckStateTag::Checked);
    assert_eq!(
        alice_view.cart.bought[0].checked_by.as_deref(),
        Some("Bob"),
        "attribution crossed the relay as a name (DECISIONS 0024)"
    );
    assert_eq!(
        shared(&alice_view),
        shared(&bob_view),
        "the two replicas converged"
    );
}

/// Both phones in the shop: a push on one socket arrives on the other
/// without either reconnecting. `checked_by` in real time is the feature
/// this buys (DECISIONS 0024).
#[tokio::test(flavor = "multi_thread")]
async fn simultaneous_devices_see_each_other_live() {
    let dir = TempDir::new("live");
    let addr = spawn_relay(dir.0.clone()).await;
    let phrase = GroupKey::generate().expect("generate").phrase().to_string();

    let mut carol = Device::join(&phrase, 0, "usr_carol", "Carol", "dev_a").await;
    let mut dan = Device::join(&phrase, 5000, "usr_dan", "Dan", "dev_b").await;

    // Both connect and stay connected.
    let mut carol_ws = connect(addr).await;
    let mut carol_session = Session::new(carol.key(), 0, 0);
    carol_ws
        .send(Message::Binary(carol_session.hello().expect("hello")))
        .await
        .expect("send");
    loop {
        match carol_session
            .handle(&recv(&mut carol_ws).await)
            .expect("msg")
        {
            Event::CaughtUp => break,
            Event::Merge(p) => {
                carol.app.merge(&p).expect("merge");
            }
            _ => {}
        }
    }

    let mut dan_ws = connect(addr).await;
    let mut dan_session = Session::new(dan.key(), 0, 0);
    dan_ws
        .send(Message::Binary(dan_session.hello().expect("hello")))
        .await
        .expect("send");
    loop {
        match dan_session.handle(&recv(&mut dan_ws).await).expect("msg") {
            Event::CaughtUp => break,
            Event::Merge(p) => {
                dan.app.merge(&p).expect("merge");
            }
            _ => {}
        }
    }

    // Carol adds milk while both are online.
    let view = carol
        .app
        .dispatch(save_ingredient("Lait"))
        .await
        .expect("save");
    let milk = ingredient_id(&view, "Lait");
    carol
        .app
        .dispatch(Command::AddIngredientToList {
            ingredient: milk,
            quantity: Some(QuantityInput {
                amount: "1".into(),
                unit: UnitTag::L,
            }),
        })
        .await
        .expect("add");
    let delta = carol.app.changes_since(&carol.shadow).expect("delta");
    carol_ws
        .send(Message::Binary(carol_session.delta(&delta).expect("seal")))
        .await
        .expect("push");

    // Dan's socket, without reconnecting, produces the frame.
    loop {
        match dan_session.handle(&recv(&mut dan_ws).await).expect("msg") {
            Event::Merge(p) => {
                let view = dan.app.merge(&p).expect("merge");
                if view.list.len() == 1 {
                    break;
                }
            }
            other => panic!("expected a live frame, got {other:?}"),
        }
    }
    let view = dan.app.state().expect("state");
    assert_eq!(
        view.cart.to_buy.iter().filter(|l| l.name == "Lait").count(),
        1
    );
}

/// The relay dies and comes back on the same data directory: nothing is
/// lost, the epoch survives, and a device that never met the first process
/// replays everything from the second (DECISIONS 0009 — the relay is the
/// recovery point).
#[tokio::test(flavor = "multi_thread")]
async fn the_log_outlives_the_relay_process() {
    let dir = TempDir::new("restart");
    let phrase = GroupKey::generate().expect("generate").phrase().to_string();

    let mut eve = Device::join(&phrase, 0, "usr_eve", "Eve", "dev_a").await;
    eve.app
        .dispatch(save_ingredient("Beurre"))
        .await
        .expect("save");

    let first = spawn_relay(dir.0.clone()).await;
    sync_once(&mut eve, first, true).await;
    let epoch_before = eve.epoch;
    // The first process is gone; only the directory remains.

    let second = spawn_relay(dir.0.clone()).await;
    let mut frank = Device::join(&phrase, 5000, "usr_frank", "Frank", "dev_b").await;
    sync_once(&mut frank, second, true).await;

    assert_eq!(
        frank.epoch, epoch_before,
        "the epoch is the log's identity and the log survived"
    );
    assert_eq!(frank.app.state().expect("state").ingredients.len(), 1);
    assert_eq!(
        frank.app.state().expect("state").ingredients[0].name,
        "Beurre"
    );
}

/// Copies a directory tree — the data directory as a backup would carry it.
fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).expect("mkdir");
    for entry in std::fs::read_dir(from).expect("read_dir") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copy");
        }
    }
}

/// M6's recovery point, exercised: `/data` goes back to a backup while both
/// devices keep replicas and cursors from further along than the restored log
/// ever reached.
///
/// The epoch cannot catch this on its own — it is inside the backup, so it
/// comes back identical, and the relay honestly replays "everything after
/// frame N" from a log whose highest frame is far below N. That is nothing, on
/// both sides, with no error anywhere, until the log grows back past N
/// (DECISIONS 0053).
#[tokio::test(flavor = "multi_thread")]
async fn a_restored_backup_does_not_strand_devices_holding_newer_cursors() {
    let dir = TempDir::new("restore");
    let vault = TempDir::new("restore-backup");
    let phrase = GroupKey::generate().expect("generate").phrase().to_string();

    let mut alice = Device::join(&phrase, 0, "usr_alice", "Alice", "dev_a").await;
    let mut bob = Device::join(&phrase, 5000, "usr_bob", "Bob", "dev_b").await;

    let relay = spawn_relay(dir.0.clone()).await;

    alice
        .app
        .dispatch(save_ingredient("Beurre"))
        .await
        .expect("save");
    sync_once(&mut alice, relay, true).await;
    sync_once(&mut bob, relay, false).await;

    // The backup: the data directory exactly as it stands.
    copy_tree(&dir.0, &vault.0);

    // The group carries on, so both cursors move well past the backup.
    for name in ["Farine", "Sucre", "Sel", "Poivre"] {
        alice
            .app
            .dispatch(save_ingredient(name))
            .await
            .expect("save");
        sync_once(&mut alice, relay, true).await;
        sync_once(&mut bob, relay, false).await;
    }
    // Both are now well past the single frame the backup holds.
    assert!(alice.since > 1, "alice's cursor moved: {}", alice.since);
    assert!(bob.since > 1, "bob's cursor moved: {}", bob.since);

    // The restore: the process stops, /data goes back to what was backed up.
    std::fs::remove_dir_all(&dir.0).expect("clear");
    copy_tree(&vault.0, &dir.0);
    let restored = spawn_relay(dir.0.clone()).await;

    // Alice, whose replica was never touched, adds something and pushes it.
    alice
        .app
        .dispatch(save_ingredient("Levure"))
        .await
        .expect("save");
    sync_once(&mut alice, restored, true).await;

    // Bob connects to the restored relay.
    sync_once(&mut bob, restored, false).await;

    let names: Vec<String> = bob
        .app
        .state()
        .expect("state")
        .ingredients
        .iter()
        .map(|i| i.name.clone())
        .collect();
    assert!(
        names.contains(&"Levure".to_string()),
        "bob never received alice's post-restore push: {names:?}"
    );
}

/// The other side of the same restore, and the one 0053 alone does not
/// survive: a device that was **closed** for the window the backup rolls
/// back, so its replica never held that window either.
///
/// The log loses it, Bob never had it, and Alice — who has it — never offers
/// it again, because her shadow says the relay took it. Worse than a gap:
/// every later delta of hers is causally rooted in those operations, so Bob
/// accepts frame after frame, advances his cursor, and applies none of them.
/// Online, no error, and never converging again (DECISIONS 0054).
#[tokio::test(flavor = "multi_thread")]
async fn a_restored_backup_does_not_strand_a_device_that_missed_the_window() {
    let dir = TempDir::new("restore-gap");
    let vault = TempDir::new("restore-gap-backup");
    let phrase = GroupKey::generate().expect("generate").phrase().to_string();

    let mut alice = Device::join(&phrase, 0, "usr_alice", "Alice", "dev_a").await;
    let mut bob = Device::join(&phrase, 5000, "usr_bob", "Bob", "dev_b").await;

    let relay = spawn_relay(dir.0.clone()).await;

    alice
        .app
        .dispatch(save_ingredient("Beurre"))
        .await
        .expect("save");
    sync_once(&mut alice, relay, true).await;
    sync_once(&mut bob, relay, false).await;

    // The backup: the data directory exactly as it stands.
    copy_tree(&dir.0, &vault.0);

    // Alice shops on for a week. Bob's phone is closed the whole time (0011),
    // so this window lives in Alice's replica and in the relay's log, nowhere
    // else.
    for name in ["Farine", "Sucre", "Sel", "Poivre"] {
        alice
            .app
            .dispatch(save_ingredient(name))
            .await
            .expect("save");
        sync_once(&mut alice, relay, true).await;
    }

    // The restore.
    std::fs::remove_dir_all(&dir.0).expect("clear");
    copy_tree(&vault.0, &dir.0);
    let restored = spawn_relay(dir.0.clone()).await;

    // Alice reconnects and adds one more thing — the drill's last step.
    alice
        .app
        .dispatch(save_ingredient("Levure"))
        .await
        .expect("save");
    sync_once(&mut alice, restored, true).await;

    // Bob opens his phone for the first time since before the backup.
    sync_once(&mut bob, restored, false).await;

    let names: Vec<String> = bob
        .app
        .state()
        .expect("state")
        .ingredients
        .iter()
        .map(|i| i.name.clone())
        .collect();

    for expected in ["Beurre", "Farine", "Sucre", "Sel", "Poivre", "Levure"] {
        assert!(
            names.contains(&expected.to_string()),
            "bob is missing {expected}: alice holds it and the restored log \
             does not, so only a whole replica puts it back — {names:?}"
        );
    }
}

// --- M10: the same sentence, on the other socket (DECISIONS 0080) -----------

/// The smallest thing the core accepts as a photo. What is inside a JPEG is
/// nobody's business here — the relay stores ciphertext and this test asserts
/// the bytes come back identical, which is the whole of what a transfer owes.
fn jpeg(tail: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0xff, 0xd8, 0xff];
    bytes.extend_from_slice(tail);
    bytes
}

/// The same save command, carrying a photo — attaching one is the ordinary
/// save and never a second command (DECISIONS 0062).
fn photographed(name: &str, photo: &PhotoId) -> Command {
    let Command::SaveIngredient { mut ingredient } = save_ingredient(name) else {
        unreachable!("save_ingredient builds exactly that")
    };
    ingredient.photo = Some(photo.to_string());
    Command::SaveIngredient { ingredient }
}

/// One photo connection, run to its own end: hello, welcome, then one
/// message at a time until both queues drain.
///
/// This is the loop `sync_once` is for the log, and it is shorter for the
/// reason 0080 gives — there is no cursor to persist and no epoch to check,
/// because the next hello re-derives the work from what is on disk at both
/// ends. **One message in flight**: the device paces the transfer, which is
/// what stops a phone that has just joined being handed the whole library at
/// once.
async fn move_photos(device: &mut Device, addr: SocketAddr) -> PhotoStatus {
    let referenced = device.app.referenced_photos().expect("what is referenced");
    let mut session = PhotoSync::open(&device.phrase, &device.photos, &referenced)
        .await
        .expect("the phrase derives");

    let mut ws = connect_to(addr, "/photos").await;
    ws.send(Message::Binary(session.hello().expect("hello")))
        .await
        .expect("send hello");

    while !session.done() {
        match session
            .handle(&device.photos, &recv(&mut ws).await)
            .await
            .expect("a server message")
        {
            PhotoEvent::Refused { reason } => panic!("refused: {reason}"),
            PhotoEvent::Rejected { id, reason } => panic!("rejected {id}: {reason}"),
            PhotoEvent::Dropped { id } => panic!("dropped {id}"),
            _ => {}
        }
        // Fetches first: a device that is missing pictures is a device with
        // a placeholder on screen, and what it holds is already safe here.
        let next = match session.fetch().expect("fetch") {
            Some(wire) => Some(wire),
            None => session.push(&device.photos).await.expect("push"),
        };
        if let Some(wire) = next {
            ws.send(Message::Binary(wire)).await.expect("send");
        }
    }
    ws.close(None).await.expect("close");
    session.status()
}

/// M10's exit criterion at replica level: **a photo taken on one phone is on
/// the other, and the two are never online at the same time.**
///
/// The mirror of `never_simultaneous_devices_converge`, one socket over. What
/// makes it a different test rather than a longer one is that a photo is not
/// in the replica: Bob's document names it the moment the two merge, and the
/// bytes arrive later, on their own connection, or not at all (DECISIONS
/// 0062).
#[tokio::test(flavor = "multi_thread")]
async fn a_photo_reaches_a_device_that_was_never_online_with_the_one_that_took_it() {
    let dir = TempDir::new("photos");
    let addr = spawn_relay(dir.0.clone()).await;
    let phrase = GroupKey::generate().expect("generate").phrase().to_string();
    let pixels = jpeg(b"a bag of flour, photographed in an aisle");

    // Alice, in the shop: she photographs the flour she buys and attaches it
    // to the ingredient. Both are local the moment they happen (Rule 6).
    let mut alice = Device::join(&phrase, 0, "usr_alice", "Alice", "dev_phone").await;
    let photo = alice
        .photos
        .put(&alice.camera, &pixels)
        .await
        .expect("store the photo");
    alice
        .app
        .dispatch(photographed("Farine", &photo))
        .await
        .expect("save");

    // Two sockets, one after the other: the document, then the bytes.
    sync_once(&mut alice, addr, true).await;
    let sent = move_photos(&mut alice, addr).await;
    assert_eq!(sent.sent, 1, "the photo did not reach the relay");
    assert_eq!(sent.received, 0, "there was nothing for Alice to fetch");

    // The relay holds it, and cannot read it (Rule 7). This is the one place
    // a test can look at what the untrusted party actually has.
    let blob = std::fs::read(
        dir.0
            .join(
                GroupKey::from_phrase(&phrase)
                    .expect("derive")
                    .id()
                    .to_hex(),
            )
            .join("photos")
            .join(photo.as_str()),
    )
    .expect("the relay stored a blob under the photo's own id");
    assert_ne!(blob, pixels, "the relay is holding the photo in the clear");
    assert!(
        !blob.windows(4).any(|w| w == b"flou"),
        "the plaintext is recognisable inside what the relay stored"
    );

    // Bob, later — Alice is long gone. The document reaches him first, so
    // his replica names a photo whose bytes are on nobody's disk but hers.
    let mut bob = Device::join(&phrase, 5000, "usr_bob", "Bob", "dev_laptop").await;
    sync_once(&mut bob, addr, true).await;
    let view = bob.app.state().expect("state");
    let farine = view
        .ingredients
        .iter()
        .find(|i| i.name == "Farine")
        .expect("Alice's ingredient reached Bob");
    assert_eq!(
        farine.photo.as_deref(),
        Some(photo.as_str()),
        "the reference travels with the document"
    );
    assert_eq!(
        bob.photos.get(&photo).await.expect("get"),
        None,
        "and the bytes do not — that is the whole reason /photos exists"
    );

    // Then the other socket, and he has them.
    let got = move_photos(&mut bob, addr).await;
    assert_eq!(got.received, 1);
    assert_eq!(got.sent, 0, "Bob had nothing to offer");
    assert_eq!(
        bob.photos.get(&photo).await.expect("get"),
        Some(pixels.clone()),
        "byte-identical: sealed by Alice, stored verbatim, opened by Bob"
    );

    // And it is his, offline: nothing about reading a photo touches a socket
    // again. Both devices now hold it, so a second connection has no work —
    // the relay never asks for a photo it already has, whoever offers it
    // (DECISIONS 0080's decision 9).
    let again = move_photos(&mut bob, addr).await;
    assert_eq!(
        (again.sent, again.received, again.pending),
        (0, 0, 0),
        "a settled device reconnects, finds nothing to do, and closes"
    );
    let alice_again = move_photos(&mut alice, addr).await;
    assert_eq!(
        alice_again.sent, 0,
        "the relay already holds it: offering it again would be a second upload"
    );
}

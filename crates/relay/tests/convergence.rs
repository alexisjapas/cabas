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

use cabas_app::command::{IngredientInput, QuantityInput};
use cabas_app::tags::{AisleTag, CheckStateTag, UnitTag};
use cabas_app::view::StateView;
use cabas_app::{App, Command, Identity, Platform};
use cabas_domain::Timestamp;
use cabas_store::MemoryStorage;
use cabas_sync::protocol::{ClientMessage, FrameKind, encode_client};
use cabas_sync::{Event, FamilyKey, Session};

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
    phrase: String,
    epoch: u64,
    since: u64,
    shadow: Vec<u8>,
}

impl Device {
    async fn join(phrase: &str, base: u64, user: &str, name: &str, device: &str) -> Self {
        let identity = Identity {
            user: user.into(),
            user_name: name.into(),
            device: device.into(),
            device_name: format!("{name}'s device"),
        };
        Device {
            app: App::open(MemoryStorage::new(), TestPlatform::from(base), identity)
                .await
                .expect("the app opens"),
            phrase: phrase.to_string(),
            epoch: 0,
            since: 0,
            // The empty shadow: nothing pushed yet, so the first delta is
            // everything this device knows.
            shadow: Vec::new(),
        }
    }

    fn key(&self) -> FamilyKey {
        FamilyKey::from_phrase(&self.phrase).expect("the phrase derives")
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
    let (ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/sync"))
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

    if dirty {
        let delta = device
            .app
            .changes_since(&device.shadow)
            .expect("export the delta");
        let version_after = device.app.version();
        ws.send(Message::Binary(session.delta(&delta).expect("seal")))
            .await
            .expect("send push");
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
            staple: false,
            density: None,
            unit_weight: None,
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
    let phrase = FamilyKey::generate()
        .expect("generate")
        .phrase()
        .to_string();

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
            quantity: QuantityInput {
                amount: "5".into(),
                unit: UnitTag::Piece,
            },
        })
        .await
        .expect("add to list");
    sync_once(&mut alice, addr, true).await;

    // A stranger who found the family id but not the phrase appends noise.
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
    let phrase = FamilyKey::generate()
        .expect("generate")
        .phrase()
        .to_string();

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
            quantity: QuantityInput {
                amount: "1".into(),
                unit: UnitTag::L,
            },
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
    let phrase = FamilyKey::generate()
        .expect("generate")
        .phrase()
        .to_string();

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
    let phrase = FamilyKey::generate()
        .expect("generate")
        .phrase()
        .to_string();

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

    // The family carries on, so both cursors move well past the backup.
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

//! The client half of a connection, without a relay.
//!
//! `crates/relay/tests/convergence.rs` proves two replicas converge through a
//! real broker; this proves the piece the PWA actually calls does what that
//! loop assumes — merges inside, seals outside, and moves the cursor exactly
//! once per frame. The relay here is three lines of test code, because what is
//! under test is the client (DECISIONS 0043).
//!
//! Native only. The same surface is exercised in a browser through the wasm
//! binding, in `scenario.rs`, which is where a `Uint8Array` that does not
//! cross would show up.

#![cfg(not(target_family = "wasm"))]

use cabas_app::command::{IngredientInput, QuantityInput};
use cabas_app::sync::{SyncCursor, SyncEvent, SyncSession, mint_phrase, read_phrase};
use cabas_app::tags::{AisleTag, UnitTag};
use cabas_app::{App, Command, Identity, Platform};
use cabas_domain::Timestamp;
use cabas_store::MemoryStorage;
use cabas_sync::protocol::{ClientMessage, FrameKind, ServerMessage, decode_client, encode_server};

/// A fixed clock and counting ids, so a failing assertion is readable. The
/// base separates two devices that must not mint the same id.
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

/// The storage futures here are ready on the first poll, so a real executor
/// would be a dependency spent on nothing. Same helper as `scenario.rs`.
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return value;
        }
    }
}

async fn open(base: u64, user: &str, name: &str) -> App<MemoryStorage, TestPlatform> {
    App::open(
        MemoryStorage::new(),
        TestPlatform::from(base),
        Identity {
            user: user.into(),
            user_name: name.into(),
            device: format!("dev_{user}"),
            device_name: format!("{name}'s phone"),
        },
    )
    .await
    .expect("the app opens")
}

/// The relay's side, in as few lines as the test needs: a push comes off the
/// wire as a client message, and goes back out as the frame the other device
/// receives. Nothing here holds a key — it moves a sealed blob, which is the
/// whole point (Rule 7).
fn forward(push: &[u8], seq: u64) -> Vec<u8> {
    match decode_client(push).expect("a client message") {
        ClientMessage::Push { kind, payload } => {
            encode_server(&ServerMessage::Frame { seq, kind, payload }).expect("encode")
        }
        other => panic!("expected a push, got {other:?}"),
    }
}

fn save(name: &str) -> Command {
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

/// The loop the frontend engine will run: one device seals what it has, the
/// other opens it into its own replica and renders the state that comes back.
#[test]
fn a_sealed_push_from_one_device_becomes_a_state_on_the_other() {
    block_on(async {
        let phrase = mint_phrase().expect("a phrase");
        let mut alice = open(0, "usr_alice", "Alice").await;
        let mut bob = open(5000, "usr_bob", "Bob").await;

        let view = alice.dispatch(save("Tomates")).await.expect("save");
        let tomatoes = view.ingredients[0].id.clone();
        alice
            .dispatch(Command::AddIngredientToList {
                ingredient: tomatoes,
                quantity: QuantityInput {
                    amount: "5".into(),
                    unit: UnitTag::Piece,
                },
            })
            .await
            .expect("add to list");

        // Alice pushes everything she knows: the empty shadow is a device
        // that has never had anything acked.
        let sending = SyncSession::open(&phrase, SyncCursor::default()).expect("open");
        let shadow = alice.version();
        let push = sending.push(&alice, &[]).expect("seal");
        assert_ne!(shadow, Vec::<u8>::new(), "the version is not empty");

        // Bob's session opens it and merges it, in one call.
        let mut receiving = SyncSession::open(&phrase, SyncCursor::default()).expect("open");
        let event = receiving
            .handle(&mut bob, &forward(&push, 7))
            .expect("handle");

        match event {
            SyncEvent::Merged { state } => {
                assert_eq!(state.list.len(), 1, "Alice's entry arrived");
                assert_eq!(state.cart.to_buy[0].name, "Tomates");
            }
            other => panic!("expected a merge, got {other:?}"),
        }
        assert_eq!(
            receiving.status().cursor,
            SyncCursor { epoch: 0, since: 7 },
            "the cursor followed the frame"
        );
        assert_eq!(receiving.status().replayed, 1);
        assert_eq!(receiving.status().dropped, 0);

        // And the replica really holds it, not just the returned view.
        assert_eq!(bob.state().expect("state").list.len(), 1);
    });
}

/// A frame sealed by another family — or by nobody — is stepped over. The
/// cursor still advances: refetching it forever would not make it open (0042).
#[test]
fn a_frame_from_a_stranger_is_dropped_without_touching_the_replica() {
    block_on(async {
        let mut bob = open(0, "usr_bob", "Bob").await;
        let mine = mint_phrase().expect("a phrase");
        let theirs = mint_phrase().expect("another phrase");

        let mut alice = open(5000, "usr_alice", "Alice").await;
        alice.dispatch(save("Tomates")).await.expect("save");
        let stranger = SyncSession::open(&theirs, SyncCursor::default()).expect("open");
        let push = stranger.push(&alice, &[]).expect("seal");

        let mut session = SyncSession::open(&mine, SyncCursor::default()).expect("open");
        let event = session
            .handle(&mut bob, &forward(&push, 3))
            .expect("handle");

        assert_eq!(event, SyncEvent::Dropped { seq: 3 });
        assert_eq!(session.status().dropped, 1);
        assert_eq!(session.status().cursor.since, 3);
        assert!(
            bob.state().expect("state").ingredients.is_empty(),
            "nothing a stranger sent reached the replica"
        );
    });
}

/// A snapshot carries the whole document, so the relay can drop everything it
/// had — a device that merges only the snapshot must end up complete.
#[test]
fn a_snapshot_carries_enough_to_stand_alone() {
    block_on(async {
        let phrase = mint_phrase().expect("a phrase");
        let mut alice = open(0, "usr_alice", "Alice").await;
        alice.dispatch(save("Beurre")).await.expect("save");
        alice.dispatch(save("Lait")).await.expect("save");

        let session = SyncSession::open(&phrase, SyncCursor::default()).expect("open");
        let push = session.snapshot(&alice).expect("seal");
        match decode_client(&push).expect("a client message") {
            ClientMessage::Push { kind, .. } => assert_eq!(
                kind,
                FrameKind::Snapshot { covers: 0 },
                "a snapshot covers what this session applied"
            ),
            other => panic!("expected a push, got {other:?}"),
        }

        let mut fresh = open(5000, "usr_bob", "Bob").await;
        let mut receiving = SyncSession::open(&phrase, SyncCursor::default()).expect("open");
        receiving
            .handle(&mut fresh, &forward(&push, 1))
            .expect("handle");

        let state = fresh.state().expect("state");
        assert_eq!(state.ingredients.len(), 2);
    });
}

/// The epoch is the log's identity: a relay restored from a backup hands back
/// a different one, and the session replays from the beginning rather than
/// from a sequence number that now means something else.
#[test]
fn a_new_epoch_resets_the_cursor() {
    let phrase = mint_phrase().expect("a phrase");
    let mut app = block_on(open(0, "usr_alice", "Alice"));
    let mut session = SyncSession::open(
        &phrase,
        SyncCursor {
            epoch: 4,
            since: 91,
        },
    )
    .expect("open");

    let welcome = encode_server(&ServerMessage::Welcome { epoch: 4 }).expect("encode");
    assert_eq!(
        session.handle(&mut app, &welcome).expect("handle"),
        SyncEvent::Connected
    );
    assert_eq!(
        session.status().cursor,
        SyncCursor {
            epoch: 4,
            since: 91
        }
    );

    let welcome = encode_server(&ServerMessage::Welcome { epoch: 5 }).expect("encode");
    session.handle(&mut app, &welcome).expect("handle");
    assert_eq!(
        session.status().cursor,
        SyncCursor { epoch: 5, since: 0 },
        "old sequence numbers point into a log that is gone"
    );
}

/// The pairing screen's two calls: what a phrase must survive, and what it
/// must be told when it does not.
#[test]
fn a_phrase_is_forgiven_its_formatting_and_refused_its_mistakes() {
    let phrase = mint_phrase().expect("a phrase");
    assert_eq!(phrase.split_whitespace().count(), 12);

    let shouted = format!("  {}  ", phrase.to_uppercase());
    assert_eq!(
        read_phrase(&shouted).expect("case and spacing are forgiven"),
        phrase
    );

    let short = read_phrase("abandon abandon abandon").expect_err("three words is not a phrase");
    assert!(
        short.to_string().contains("got 3"),
        "the message says what was wrong: {short}"
    );
    let misspelled = read_phrase(
        "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon abandonn",
    )
    .expect_err("a word off the list is not a phrase");
    assert!(!misspelled.to_string().is_empty());

    // And a phrase that does not decode cannot open a session at all, which
    // is what keeps the failure on the pairing screen rather than on the
    // first connection.
    assert!(SyncSession::open("not a phrase", SyncCursor::default()).is_err());
}

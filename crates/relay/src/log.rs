//! One family's sealed log: the state that makes the relay worth having.
//!
//! A pure broadcast relay never reconciles two devices that are never
//! online at the same time — the normal case for a phone in a shop and a
//! laptop at home — so the relay persists what it forwards (DECISIONS
//! 0009). What it persists is exactly what it saw: sealed payloads, each
//! under the sequence number it assigned, plus the one number it minted
//! itself — the **epoch**, which is how a device can tell that the log its
//! cursor points into still exists (0042).
//!
//! Layout on disk, under `<data>/<family id in hex>/`:
//!
//! - `meta` — `Meta { epoch, next_seq }`, rewritten atomically (tmp +
//!   rename) so a crash mid-write costs a re-replay, never a misparse.
//! - `log` — length-prefixed records, one per frame, appended and fsynced.
//!   A truncated tail — power loss mid-append — is cut off and forgotten
//!   on load: the device that pushed it never got its ack, so it will
//!   push again.
//!
//! Everything is held in memory too. A family's history between
//! compactions is bounded by the same argument that sized `store`'s
//! snapshots: two people's shopping does not outgrow a Raspberry Pi's RAM.

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

use cabas_sync::protocol::FrameKind;
use serde::{Deserialize, Serialize};

/// A frame as stored and as replayed. The payload is ciphertext end to end;
/// `kind` and the sequence number are the plaintext minimum the log needs
/// to order and truncate (DECISIONS 0042).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StoredFrame {
    pub seq: u64,
    pub kind: FrameKind,
    pub payload: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct Meta {
    pub epoch: u64,
    pub next_seq: u64,
}

pub struct FamilyLog {
    dir: PathBuf,
    epoch: u64,
    next_seq: u64,
    frames: Vec<StoredFrame>,
}

impl FamilyLog {
    /// Opens or creates the log directory. Creation is triggered by the
    /// first `Hello` naming this family — the id is unguessable, so a
    /// stranger cannot mine directories into existence (0042).
    pub fn open(dir: PathBuf) -> std::io::Result<Self> {
        fs::create_dir_all(&dir)?;
        let (epoch, mut next_seq) = match read_meta(&dir.join("meta"))? {
            Some(meta) => (meta.epoch, meta.next_seq),
            None => (mint_epoch()?, 1),
        };
        let frames = read_log(&dir.join("log"))?;
        // The log, not the meta, is authoritative for sequence numbers when
        // the two disagree: handing out a sequence twice is the one mistake
        // devices cannot recover from, since their cursors would silently
        // skip the second frame.
        if let Some(last) = frames.last() {
            next_seq = next_seq.max(last.seq + 1);
        }
        let log = FamilyLog {
            dir,
            epoch,
            next_seq,
            frames,
        };
        log.write_meta()?;
        Ok(log)
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Every frame after `since`, in sequence order — the replay a
    /// reconnecting device asked for.
    pub fn replay(&self, since: u64) -> Vec<StoredFrame> {
        self.frames
            .iter()
            .filter(|f| f.seq > since)
            .cloned()
            .collect()
    }

    /// Appends a frame, durably, and returns it with its assigned sequence.
    ///
    /// A snapshot drops every frame at or below what it declares to cover —
    /// the *device* vouches for that coverage, because only a device can
    /// read what a snapshot contains. The relay checks nothing about the
    /// claim and cannot (0042).
    pub fn append(&mut self, kind: FrameKind, payload: Vec<u8>) -> std::io::Result<StoredFrame> {
        let frame = StoredFrame {
            seq: self.next_seq,
            kind,
            payload,
        };
        self.next_seq += 1;
        self.frames.push(frame.clone());

        match kind {
            FrameKind::Snapshot { covers } => {
                self.frames.retain(|f| f.seq > covers);
                self.rewrite_log()?;
            }
            FrameKind::Delta => {
                let record = encode_record(&frame)?;
                let mut file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(self.dir.join("log"))?;
                file.write_all(&record)?;
                file.sync_data()?;
            }
        }
        self.write_meta()?;
        Ok(frame)
    }

    /// The whole log, rewritten through a rename so no crash leaves a
    /// half-truncated file pretending to be the history.
    fn rewrite_log(&self) -> std::io::Result<()> {
        let tmp = self.dir.join("log.tmp");
        let mut file = File::create(&tmp)?;
        for frame in &self.frames {
            file.write_all(&encode_record(frame)?)?;
        }
        file.sync_all()?;
        fs::rename(&tmp, self.dir.join("log"))
    }

    fn write_meta(&self) -> std::io::Result<()> {
        let meta = Meta {
            epoch: self.epoch,
            next_seq: self.next_seq,
        };
        let bytes = postcard::to_allocvec(&meta).map_err(corrupt)?;
        let tmp = self.dir.join("meta.tmp");
        let mut file = File::create(&tmp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&tmp, self.dir.join("meta"))
    }
}

/// A record is `u32 LE length ‖ postcard(StoredFrame)` — the length prefix
/// exists so a truncated tail is detectable without trusting the codec to
/// fail cleanly on arbitrary bytes.
fn encode_record(frame: &StoredFrame) -> std::io::Result<Vec<u8>> {
    let body = postcard::to_allocvec(frame).map_err(corrupt)?;
    let mut record = Vec::with_capacity(4 + body.len());
    record.extend_from_slice(&(body.len() as u32).to_le_bytes());
    record.extend_from_slice(&body);
    Ok(record)
}

/// Also read by [`crate::admin`], which reports on a log without opening it —
/// surveying every family on disk must not mint an epoch for a directory it
/// is only counting.
pub fn read_meta(path: &Path) -> std::io::Result<Option<Meta>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    match postcard::from_bytes(&bytes) {
        Ok(meta) => Ok(Some(meta)),
        // A torn meta means the epoch is gone; minting a fresh one merely
        // costs every device a full replay, which idempotent merges make
        // verbose rather than wrong (0042).
        Err(_) => Ok(None),
    }
}

fn read_log(path: &Path) -> std::io::Result<Vec<StoredFrame>> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    let mut frames = Vec::new();
    let mut offset = 0usize;
    while bytes.len() - offset >= 4 {
        let len =
            u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("4 bytes")) as usize;
        let start = offset + 4;
        let Some(body) = bytes.get(start..start + len) else {
            break; // torn tail: the append never got acked
        };
        let Ok(frame) = postcard::from_bytes::<StoredFrame>(body) else {
            break;
        };
        frames.push(frame);
        offset = start + len;
    }
    if offset < bytes.len() {
        tracing::warn!(
            dropped = bytes.len() - offset,
            "log has a torn tail — cutting it off; the device that pushed it \
             never got its ack and will push again"
        );
        let file = OpenOptions::new().write(true).open(path)?;
        file.set_len(offset as u64)?;
        file.sync_all()?;
    }
    Ok(frames)
}

/// A fresh epoch, never zero: zero is what a device that has never synced
/// sends, and the two must not be confusable.
fn mint_epoch() -> std::io::Result<u64> {
    loop {
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes).map_err(|e| std::io::Error::other(e.to_string()))?;
        let epoch = u64::from_le_bytes(bytes);
        if epoch != 0 {
            return Ok(epoch);
        }
    }
}

fn corrupt(e: postcard::Error) -> std::io::Error {
    std::io::Error::new(ErrorKind::InvalidData, e.to_string())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A directory that cleans itself up, so the tests need no dev-dep —
    /// the same shape `store`'s file tests use. Shared with `admin`'s tests,
    /// which need the same thing and should not grow a second one.
    pub(crate) struct TempDir(pub(crate) PathBuf);

    impl TempDir {
        pub(crate) fn new(tag: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "cabas-relay-{tag}-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let _ = fs::remove_dir_all(&path);
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn sequences_start_at_one_and_replay_respects_since() {
        let dir = TempDir::new("replay");
        let mut log = FamilyLog::open(dir.0.clone()).expect("open");
        assert!(log.replay(0).is_empty());
        for payload in [b"a".to_vec(), b"b".to_vec(), b"c".to_vec()] {
            log.append(FrameKind::Delta, payload).expect("append");
        }
        assert_eq!(
            log.replay(0).iter().map(|f| f.seq).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(
            log.replay(2).iter().map(|f| f.seq).collect::<Vec<_>>(),
            vec![3]
        );
        assert!(log.replay(3).is_empty());
    }

    #[test]
    fn the_log_survives_a_restart() {
        let dir = TempDir::new("restart");
        let epoch = {
            let mut log = FamilyLog::open(dir.0.clone()).expect("open");
            log.append(FrameKind::Delta, b"first".to_vec())
                .expect("append");
            log.append(FrameKind::Delta, b"second".to_vec())
                .expect("append");
            log.epoch()
        };
        let mut log = FamilyLog::open(dir.0.clone()).expect("reopen");
        assert_eq!(log.epoch(), epoch, "the epoch is the log's identity");
        assert_eq!(log.replay(0).len(), 2);
        let next = log
            .append(FrameKind::Delta, b"third".to_vec())
            .expect("append");
        assert_eq!(next.seq, 3, "sequences continue where they left off");
    }

    #[test]
    fn a_snapshot_truncates_what_it_covers() {
        let dir = TempDir::new("truncate");
        let mut log = FamilyLog::open(dir.0.clone()).expect("open");
        for payload in [b"a".to_vec(), b"b".to_vec(), b"c".to_vec()] {
            log.append(FrameKind::Delta, payload).expect("append");
        }
        log.append(FrameKind::Snapshot { covers: 2 }, b"snap".to_vec())
            .expect("append");
        // Frame 3 was not covered; the snapshot itself is frame 4.
        let seqs: Vec<u64> = log.replay(0).iter().map(|f| f.seq).collect();
        assert_eq!(seqs, vec![3, 4]);

        // And the truncation is durable, not an in-memory illusion.
        let log = FamilyLog::open(dir.0.clone()).expect("reopen");
        let seqs: Vec<u64> = log.replay(0).iter().map(|f| f.seq).collect();
        assert_eq!(seqs, vec![3, 4]);
    }

    #[test]
    fn a_torn_tail_is_cut_off_not_fatal() {
        let dir = TempDir::new("torn");
        {
            let mut log = FamilyLog::open(dir.0.clone()).expect("open");
            log.append(FrameKind::Delta, b"whole".to_vec())
                .expect("append");
        }
        // Power loss mid-append: a length prefix promising more than exists.
        let path = dir.0.join("log");
        let mut file = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open log");
        file.write_all(&[42, 0, 0, 0, 1, 2, 3]).expect("tear");
        drop(file);

        let mut log = FamilyLog::open(dir.0.clone()).expect("reopen");
        assert_eq!(log.replay(0).len(), 1, "the whole record survives");
        let next = log
            .append(FrameKind::Delta, b"after".to_vec())
            .expect("append");
        assert_eq!(next.seq, 2);
        // The torn bytes are gone from disk too, or the next reopen would
        // misparse the record appended after them.
        let log = FamilyLog::open(dir.0.clone()).expect("re-reopen");
        assert_eq!(log.replay(0).len(), 2);
    }

    #[test]
    fn a_lost_meta_changes_the_epoch() {
        let dir = TempDir::new("epoch");
        let epoch = {
            let mut log = FamilyLog::open(dir.0.clone()).expect("open");
            log.append(FrameKind::Delta, b"x".to_vec()).expect("append");
            log.epoch()
        };
        fs::remove_file(dir.0.join("meta")).expect("lose the meta");
        let log = FamilyLog::open(dir.0.clone()).expect("reopen");
        assert_ne!(
            log.epoch(),
            epoch,
            "a log that cannot prove it is the same log must not claim to be"
        );
    }
}

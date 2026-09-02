//! One group's sealed photos: a blob per name, beside its log.
//!
//! The other half of DECISIONS 0062, and the reason the app has a
//! `PhotoStore` separate from its document at all: photos are big, they never
//! change, and there is no order to preserve between them. So this is not a
//! log — there is no sequence, no epoch and no replay. It is a directory of
//! files, and the whole reconciliation is "which of these names do you have"
//! (DECISIONS 0080).
//!
//! Layout, under `<data>/<group id in hex>/photos/`:
//!
//! - one file per photo, **named after the [`PhotoName`]** and holding the
//!   sealed bytes exactly as the device pushed them.
//!
//! Naming a file after something that arrived over a socket is the dangerous
//! part, and it is handled one layer up: a `PhotoName` cannot be decoded
//! unless it is ASCII letters, digits, `_` and `-`, so a `/` or a `..` is a
//! [`cabas_sync::SyncError::Wire`] before this module sees it. The check is
//! restated in a test here anyway — the type is the guard, and a test that
//! would fail if somebody widened it is what keeps the guard load-bearing.
//!
//! # What it holds in memory, and what it does not
//!
//! The **names and their sizes**, read once when the group is opened. That is
//! what a `Welcome` needs and it is a few dozen bytes per photo — a group with
//! a thousand pictures costs the Pi a map of a thousand short strings. The
//! bytes themselves are read off the disk per fetch and never cached: they are
//! sent once to each device and then never again, so a cache would be a
//! megabyte held for a reader that has already gone.
//!
//! # Two caps, and neither closes a socket
//!
//! A single photo may not exceed [`MAX_PHOTO_BYTES`], and one group may not
//! hold more than [`MAX_GROUP_BYTES`]. Both answer with
//! [`Rejection`] against **one photo**, which the protocol turns into a
//! `Rejected` — the rest of the queue is still worth transferring, and a
//! refusal that dropped the connection would be re-offered on the reconnect
//! (DECISIONS 0080).

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use cabas_sync::PhotoName;

/// The most one sealed photo may weigh here.
///
/// The app refuses a photo over 512 kB *before* sealing it
/// (`app::photos::MAX_PHOTO_BYTES`), and sealing adds a 24-byte nonce and a
/// 16-byte tag. This is that number with room to spare rather than the exact
/// sum: the two live in different crates on different release cycles, and a
/// relay that rejected a photo the app had just accepted would be a picture
/// that vanishes between two phones with nothing on either screen to say why.
/// It is a bound on what a stranger who guessed a group id can spend, not a
/// second opinion about what a photo should be.
pub const MAX_PHOTO_BYTES: usize = 1024 * 1024;

/// The most one group may hold.
///
/// A thousand photos at the app's ceiling. Sized against the appliance rather
/// than against the use case: this lives on an SD card that Home Assistant
/// also backs up whole (README, "Backups"), so the number that matters is what
/// a backup can carry rather than how many pictures of jars two people might
/// take. Reaching it is a `Rejected` naming the cap, which is a sentence
/// somebody can act on — deleting a photo in the app frees nothing here, so
/// the action is `cabas-relay forget` and a new group, or a bigger card.
pub const MAX_GROUP_BYTES: u64 = 1024 * 1024 * 1024;

/// Why a photo was not stored. Carries its own sentence, because the device
/// shows it and nothing on this side knows more than this does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejection(pub String);

pub struct GroupPhotos {
    dir: PathBuf,
    /// Name → size on disk. The index a `Welcome` is computed from, and the
    /// running total the group cap is checked against.
    held: BTreeMap<String, u64>,
    bytes: u64,
}

impl GroupPhotos {
    /// Opens or creates the directory and reads what is in it.
    ///
    /// Creating it is triggered by a device saying hello for this group, the
    /// same way the log's is: the id is unguessable, so a stranger cannot mine
    /// directories into existence (DECISIONS 0042).
    ///
    /// A file whose name is not a [`PhotoName`] is ignored rather than raised.
    /// Nothing this process writes can produce one — but a restored backup, a
    /// half-finished copy or somebody's editor swap file all can, and refusing
    /// to open the group over it would take the photos of two people down for
    /// a stray `.DS_Store`.
    pub fn open(dir: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&dir)?;
        let mut held = BTreeMap::new();
        let mut bytes = 0;
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            if !meta.is_file() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if PhotoName::new(&name).is_err() {
                continue;
            }
            bytes += meta.len();
            held.insert(name, meta.len());
        }
        Ok(GroupPhotos { dir, held, bytes })
    }

    pub fn contains(&self, name: &PhotoName) -> bool {
        self.held.contains_key(name.as_str())
    }

    /// Everything in `offered` this group does not hold — the `upload` list.
    pub fn missing(&self, offered: &[PhotoName]) -> Vec<PhotoName> {
        offered
            .iter()
            .filter(|name| !self.contains(name))
            .cloned()
            .collect()
    }

    /// Everything in `wanted` this group does hold — the `available` list.
    pub fn present(&self, wanted: &[PhotoName]) -> Vec<PhotoName> {
        wanted
            .iter()
            .filter(|name| self.contains(name))
            .cloned()
            .collect()
    }

    /// The sealed bytes, or nothing.
    ///
    /// `None` covers a photo forgotten between the welcome and the fetch,
    /// which the protocol answers with `Absent` — `forget` is run by a hand
    /// and a hand can run during a connection (DECISIONS 0050).
    pub fn read(&self, name: &PhotoName) -> io::Result<Option<Vec<u8>>> {
        match fs::read(self.path(name)) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Stores one sealed photo, durably, or says why not.
    ///
    /// Written to a temporary file and renamed, so a crash mid-write leaves
    /// either the old file or none — never a truncated blob that would be
    /// served to the other phone as a photo and fail to open there. The
    /// `Ok(Err(_))` shape separates "this photo may not be stored", which the
    /// device is told about and carries on from, from "this disk is broken",
    /// which ends the connection.
    pub fn store(&mut self, name: &PhotoName, bytes: &[u8]) -> io::Result<Result<(), Rejection>> {
        if bytes.len() > MAX_PHOTO_BYTES {
            return Ok(Err(Rejection(format!(
                "{} kB is over the {} kB one photo may weigh",
                bytes.len() / 1024,
                MAX_PHOTO_BYTES / 1024
            ))));
        }
        // Measured against what this name already costs, so re-pushing a photo
        // the group holds cannot be made to count twice.
        let already = self.held.get(name.as_str()).copied().unwrap_or(0);
        let after = self.bytes - already + bytes.len() as u64;
        if after > MAX_GROUP_BYTES {
            return Ok(Err(Rejection(format!(
                "this group is at its {} MB of photos",
                MAX_GROUP_BYTES / (1024 * 1024)
            ))));
        }

        let final_path = self.path(name);
        let tmp = self.dir.join(format!("{name}.tmp"));
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, &final_path)?;

        self.bytes = after;
        self.held.insert(name.to_string(), bytes.len() as u64);
        Ok(Ok(()))
    }

    fn path(&self, name: &PhotoName) -> PathBuf {
        self.dir.join(name.as_str())
    }

    /// Where a group's photos live, given its directory. One function so the
    /// name of the subdirectory is written once.
    pub fn dir_of(group: &Path) -> PathBuf {
        group.join("photos")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "cabas-photos-{tag}-{}-{:?}",
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

    fn name(raw: &str) -> PhotoName {
        PhotoName::new(raw).expect("a name")
    }

    #[test]
    fn a_stored_photo_comes_back_byte_for_byte() {
        let dir = TempDir::new("roundtrip");
        let mut photos = GroupPhotos::open(dir.0.clone()).expect("open");
        let sealed = vec![7u8; 4096];

        assert!(!photos.contains(&name("pho_0000000000000001")));
        photos
            .store(&name("pho_0000000000000001"), &sealed)
            .expect("write")
            .expect("accepted");

        assert!(photos.contains(&name("pho_0000000000000001")));
        assert_eq!(
            photos.read(&name("pho_0000000000000001")).expect("read"),
            Some(sealed)
        );
    }

    /// The index is the directory, so a relay that restarts knows what it has
    /// without being told. Without this a reconnecting device would be offered
    /// every photo it already sent, on every launch of the relay.
    #[test]
    fn what_is_on_disk_is_found_again_after_a_restart() {
        let dir = TempDir::new("reopen");
        {
            let mut photos = GroupPhotos::open(dir.0.clone()).expect("open");
            photos
                .store(&name("pho_0000000000000002"), &[1, 2, 3])
                .expect("write")
                .expect("accepted");
        }
        let photos = GroupPhotos::open(dir.0.clone()).expect("reopen");
        assert!(photos.contains(&name("pho_0000000000000002")));
        assert_eq!(photos.bytes, 3);
    }

    #[test]
    fn the_two_lists_a_welcome_needs_are_set_differences() {
        let dir = TempDir::new("welcome");
        let mut photos = GroupPhotos::open(dir.0.clone()).expect("open");
        photos
            .store(&name("pho_held"), &[0; 16])
            .expect("write")
            .expect("accepted");

        let offered = vec![name("pho_held"), name("pho_new")];
        assert_eq!(photos.missing(&offered), vec![name("pho_new")]);

        let wanted = vec![name("pho_held"), name("pho_nobody_has")];
        assert_eq!(photos.present(&wanted), vec![name("pho_held")]);
    }

    #[test]
    fn a_photo_over_the_ceiling_is_rejected_and_not_written() {
        let dir = TempDir::new("cap");
        let mut photos = GroupPhotos::open(dir.0.clone()).expect("open");
        let huge = vec![0u8; MAX_PHOTO_BYTES + 1];

        let outcome = photos.store(&name("pho_huge"), &huge).expect("no io error");
        assert!(outcome.is_err(), "over the ceiling");
        assert!(!photos.contains(&name("pho_huge")));
        assert_eq!(photos.read(&name("pho_huge")).expect("read"), None);
    }

    /// Re-pushing a photo the group already holds replaces it rather than
    /// counting twice against the cap — otherwise two devices that both offer
    /// the same picture would spend it twice.
    #[test]
    fn storing_the_same_name_twice_does_not_double_the_total() {
        let dir = TempDir::new("twice");
        let mut photos = GroupPhotos::open(dir.0.clone()).expect("open");
        for _ in 0..3 {
            photos
                .store(&name("pho_same"), &[9; 1000])
                .expect("write")
                .expect("accepted");
        }
        assert_eq!(photos.bytes, 1000);
        assert_eq!(photos.held.len(), 1);
    }

    /// The traversal guard, restated where the file is actually written.
    ///
    /// `PhotoName` is what enforces it — a hostile name is a wire error before
    /// any of this runs (DECISIONS 0080) — and this is the test that fails if
    /// somebody ever widens the character set "just for a dot".
    #[test]
    fn a_name_that_could_escape_the_directory_is_not_a_name() {
        for hostile in ["../../etc/passwd", "a/b", "..", "", "with space", "dot.jpg"] {
            assert!(
                PhotoName::new(hostile).is_err(),
                "{hostile:?} must not be nameable"
            );
        }
    }

    /// A directory holding something this process did not write opens anyway.
    #[test]
    fn a_file_that_is_not_a_photo_is_ignored_rather_than_fatal() {
        let dir = TempDir::new("stray");
        fs::create_dir_all(&dir.0).expect("mkdir");
        fs::write(dir.0.join(".DS_Store"), b"junk").expect("write");
        fs::write(dir.0.join("pho_real"), b"sealed").expect("write");

        let photos = GroupPhotos::open(dir.0.clone()).expect("open");
        assert!(photos.contains(&name("pho_real")));
        assert_eq!(photos.held.len(), 1);
        assert_eq!(photos.bytes, 6);
    }
}

//! One group's photos: a directory of sealed blobs, and the two policies the
//! protocol left here.
//!
//! Nothing in this file resembles [`crate::log`], and that is the point
//! (DECISIONS 0080). A photo library has no order, so there is no sequence to
//! assign, no epoch to mint, no cursor to honour and nothing to truncate: a
//! photo is a blob under a name, written once and read back whole. What the
//! relay does with them is put, get, list and refuse.
//!
//! Layout on disk, under `<data>/<group id in hex>/photos/`: one file per
//! photo, named after the [`PhotoName`] the device sent. That is safe because
//! a name is checked as it is *decoded* — `cabas-sync` owns that rule
//! precisely so the party naming a file after one cannot forget it (0080's
//! decision 7). It also puts the photos **inside** the group's directory,
//! which is what makes [`crate::admin::forget`] take them with the log
//! without knowing they exist.
//!
//! The directory is created by the first photo that is stored, never by a
//! connection: a hello alone must not mine a group directory into existence,
//! for the reason `log::GroupLog::open` gives about unguessable ids.
//!
//! # The two policies (DECISIONS 0080, 0081)
//!
//! Both are the relay's and neither is the protocol's, because both are about
//! this machine's disk rather than about what the two ends mean to each
//! other:
//!
//! - **What one photo may weigh here** ([`MAX_BLOB_BYTES`]) — deliberately
//!   above the app's own ceiling, so a blob that trips it is not something an
//!   honest device sends.
//! - **What one group's photos may weigh** ([`DEFAULT_CAP`]) — the SD card
//!   Home Assistant runs on is the thing being protected (0062's decision 8).
//!
//! Both answer with [`Kept::Rejected`], which the connection turns into
//! words against that one photo. Neither drops the socket: the queue behind
//! a refused photo has nothing wrong with it.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use cabas_sync::PhotoName;

/// The most a single sealed blob may weigh, and a backstop rather than the
/// rule anyone is expected to meet.
///
/// The app's own ceiling is half that (`cabas_app::MAX_PHOTO_BYTES`), checked
/// where a photo is encoded and again where it is stored — this number is not
/// that one and must not be confused with it. It is here so that a device
/// which somehow offers something enormous is answered **by name**, with a
/// reason, while the socket's own limit (`server::MAX_MESSAGE`) sits higher
/// still and only ever fires on a message no version of this app sends. Words
/// for an honest mistake, a dropped socket for a stranger.
pub const MAX_BLOB_BYTES: u64 = 1024 * 1024;

/// What one group's photos may weigh before a push is refused.
///
/// A thousand photos at the app's own half-megabyte ceiling. The number that
/// matters is not how many pictures two people take — it is that a Raspberry
/// Pi's SD card and every Home Assistant backup grow with this directory
/// (DECISIONS 0062), so the failure has to be a refusal a device can report
/// rather than a full filesystem that takes the sync log down with it.
pub const DEFAULT_CAP: u64 = 512 * 1024 * 1024;

/// What became of a pushed photo — the answer the connection sends back.
#[derive(Debug, PartialEq, Eq)]
pub enum Kept {
    /// Durable, or already here. Both mean the same thing to the device: it
    /// is no longer the only copy, and it may stop offering this photo.
    Stored,
    /// Not stored, and pushing it again now would not help. The reason is
    /// sent verbatim, because the device shows it to a person.
    Rejected(String),
}

/// One group's sealed blobs.
///
/// The index — every name and its weight — is held in memory, so the welcome
/// that starts every connection is a set difference over a map rather than a
/// directory walk per name. It is rebuilt on open, which is the only place
/// this type reads the directory listing.
///
/// The file I/O is blocking and the caller holds a lock across it, which is
/// what [`crate::log`] does too and for the same reason: a group is two
/// people, and one of them waiting half a millisecond for the other's photo
/// to be read is not a problem worth a thread pool. What would be a problem
/// is two connections writing against one index, which is what the lock is
/// there for.
pub struct GroupPhotos {
    dir: PathBuf,
    sizes: BTreeMap<String, u64>,
    weight: u64,
    cap: u64,
}

impl GroupPhotos {
    /// Reads what is on disk for this group. A directory that does not exist
    /// is an empty library, not a failure — it is what a group that has never
    /// sent a photo looks like, and creating one here would put a directory
    /// under `/data` for every hello.
    pub fn open(dir: PathBuf, cap: u64) -> io::Result<Self> {
        let mut photos = GroupPhotos {
            dir,
            sizes: BTreeMap::new(),
            weight: 0,
            cap,
        };
        photos.index()?;
        Ok(photos)
    }

    fn index(&mut self) -> io::Result<()> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        for entry in entries {
            let entry = entry?;
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            // Anything that is not a name this relay would have written is
            // something it did not write: a scratch file an interrupted push
            // left behind, or a stray. The scratch files are swept, because
            // nothing else ever will and each one weighs as much as a photo;
            // anything else is left exactly where it is, the way `admin`
            // leaves what is not a group id alone.
            if PhotoName::new(&name).is_err() {
                if name.ends_with(SCRATCH) {
                    let _ = fs::remove_file(entry.path());
                }
                continue;
            }
            let weight = entry.metadata()?.len();
            self.weight += weight;
            self.sizes.insert(name, weight);
        }
        Ok(())
    }

    /// Whether this relay holds the bytes of a photo — the question the
    /// welcome asks of every name in a hello, in both directions.
    pub fn holds(&self, name: &PhotoName) -> bool {
        self.sizes.contains_key(name.as_str())
    }

    /// The sealed bytes, exactly as they were pushed — or `None`, which is
    /// what a photo forgotten between the welcome and the fetch looks like
    /// (DECISIONS 0080's decision 5).
    pub fn load(&self, name: &PhotoName) -> io::Result<Option<Vec<u8>>> {
        match fs::read(self.dir.join(name.as_str())) {
            Ok(bytes) => Ok(Some(bytes)),
            // Not an error even though the index said otherwise: a file can
            // go under a running process, and answering `Absent` keeps the
            // device's queue moving where raising would end its connection.
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Stores one sealed blob, durably, or says why it will not.
    ///
    /// **The first write wins.** Ids are minted random at capture (0062), so
    /// two devices pushing the same name are pushing the same photo and the
    /// second push is a no-op — while overwriting would mean anyone who
    /// learned the group id could replace a photo with something else. The
    /// cheap answer and the safe one are the same answer.
    pub fn store(&mut self, name: &PhotoName, payload: &[u8]) -> io::Result<Kept> {
        if self.holds(name) {
            return Ok(Kept::Stored);
        }
        let weight = payload.len() as u64;
        if weight > MAX_BLOB_BYTES {
            return Ok(Kept::Rejected(format!(
                "{} is over the {} one photo may weigh here",
                crate::admin::bytes(weight),
                crate::admin::bytes(MAX_BLOB_BYTES)
            )));
        }
        if self.weight + weight > self.cap {
            return Ok(Kept::Rejected(format!(
                "this group is at its {} of photos",
                crate::admin::bytes(self.cap)
            )));
        }

        fs::create_dir_all(&self.dir)?;
        // Beside the target and renamed over it, and fsynced before the
        // rename: `Stored` is a promise of durability the device acts on by
        // sweeping its own copy later (0062's decision 8), so a photo that
        // exists only in the page cache would be a promise this relay cannot
        // keep. A scratch file left by a crash mid-write is swept by the next
        // `open` — nothing else would ever collect it.
        let scratch = self.dir.join(format!("{name}{SCRATCH}"));
        let mut file = fs::File::create(&scratch)?;
        file.write_all(payload)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&scratch, self.dir.join(name.as_str()))?;

        self.weight += weight;
        self.sizes.insert(name.to_string(), weight);
        Ok(Kept::Stored)
    }
}

/// The suffix a half-written photo carries. It cannot collide with a real
/// name: a [`PhotoName`] is letters, digits, `_` and `-`, so nothing valid
/// contains a `.`.
const SCRATCH: &str = ".part";

/// Where one group's photos live, given that group's directory.
pub(crate) fn dir_of(group: &Path) -> PathBuf {
    group.join("photos")
}

/// What one group's photos are, counted **without opening anything**.
///
/// [`crate::admin`] reports on a data directory it must leave exactly as it
/// found it — the same rule that keeps `survey` from opening a log and
/// minting an epoch for a group it is only counting. [`GroupPhotos::open`]
/// sweeps scratch files, which is right for a relay about to serve and wrong
/// for a listing, so this walks the directory on its own.
pub(crate) fn held(group: &Path) -> io::Result<Held> {
    let mut held = Held { count: 0, bytes: 0 };
    let entries = match fs::read_dir(dir_of(group)) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(held),
        Err(e) => return Err(e),
    };
    for entry in entries {
        let entry = entry?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if PhotoName::new(&name).is_err() {
            continue;
        }
        held.count += 1;
        held.bytes += entry.metadata()?.len();
    }
    Ok(held)
}

/// One group's photos, as reported by [`held`].
pub(crate) struct Held {
    pub(crate) count: u64,
    /// What the photos themselves weigh — a subset of the group's total,
    /// which also carries the log.
    pub(crate) bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::tests::TempDir;

    fn name(raw: &str) -> PhotoName {
        PhotoName::new(raw).expect("a name")
    }

    fn photos(dir: &TempDir, cap: u64) -> GroupPhotos {
        GroupPhotos::open(dir_of(&dir.0), cap).expect("open")
    }

    #[test]
    fn a_group_that_has_never_sent_a_photo_is_empty_and_makes_no_directory() {
        let dir = TempDir::new("photos-empty");
        let photos = photos(&dir, DEFAULT_CAP);
        let held = held(&dir.0).expect("held");
        assert_eq!((held.count, held.bytes), (0, 0));
        assert!(!photos.holds(&name("pho_0000000000000001")));
        assert!(
            !dir_of(&dir.0).exists(),
            "a connection must not mine a directory into existence"
        );
    }

    #[test]
    fn a_photo_is_stored_and_read_back_verbatim() {
        let dir = TempDir::new("photos-roundtrip");
        let mut photos = photos(&dir, DEFAULT_CAP);
        let id = name("pho_0000000000000001");
        let sealed = vec![7u8; 4096];

        assert_eq!(photos.store(&id, &sealed).expect("store"), Kept::Stored);
        assert!(photos.holds(&id));
        let held = held(&dir.0).expect("held");
        assert_eq!((held.count, held.bytes), (1, 4096));
        assert_eq!(photos.load(&id).expect("load"), Some(sealed));
        assert_eq!(
            photos.load(&name("pho_0000000000000002")).expect("load"),
            None,
            "a photo nobody pushed is absent, not an error"
        );
    }

    #[test]
    fn the_index_survives_a_restart() {
        let dir = TempDir::new("photos-restart");
        {
            let mut photos = photos(&dir, DEFAULT_CAP);
            photos
                .store(&name("pho_0000000000000001"), &[1u8; 64])
                .expect("store");
            photos
                .store(&name("pho_0000000000000002"), &[2u8; 128])
                .expect("store");
        }
        let photos = photos(&dir, DEFAULT_CAP);
        assert!(photos.holds(&name("pho_0000000000000001")));
        assert!(photos.holds(&name("pho_0000000000000002")));
        let held = held(&dir.0).expect("held");
        assert_eq!((held.count, held.bytes), (2, 64 + 128));
    }

    /// Ids are minted random at capture, so the same name twice is the same
    /// photo twice — and the relay is reachable by anyone who learns the
    /// group id (DECISIONS 0012). Keeping the first copy is what stops a
    /// second push replacing a photo with something else.
    #[test]
    fn the_first_copy_of_a_photo_is_the_one_that_stays() {
        let dir = TempDir::new("photos-first");
        let mut photos = photos(&dir, DEFAULT_CAP);
        let id = name("pho_0000000000000001");

        photos.store(&id, b"the photo").expect("store");
        assert_eq!(
            photos
                .store(&id, b"something else entirely")
                .expect("store"),
            Kept::Stored,
            "a device that pushes again is told the photo is safe"
        );
        assert_eq!(
            photos.load(&id).expect("load").as_deref(),
            Some(&b"the photo"[..])
        );
        assert_eq!(held(&dir.0).expect("held").bytes, b"the photo".len() as u64);
    }

    #[test]
    fn a_blob_beyond_what_a_photo_may_weigh_is_refused_by_name() {
        let dir = TempDir::new("photos-fat");
        let mut photos = photos(&dir, DEFAULT_CAP);
        let id = name("pho_0000000000000001");

        let Kept::Rejected(reason) = photos
            .store(&id, &vec![0u8; MAX_BLOB_BYTES as usize + 1])
            .expect("store")
        else {
            panic!("an oversized blob was stored");
        };
        assert!(reason.contains("one photo may weigh"), "{reason}");
        assert!(!photos.holds(&id));
        assert_eq!(held(&dir.0).expect("held").count, 0);
    }

    #[test]
    fn a_group_at_its_cap_refuses_a_push_rather_than_filling_the_disk() {
        let dir = TempDir::new("photos-cap");
        let mut photos = photos(&dir, 4096);
        photos
            .store(&name("pho_0000000000000001"), &[1u8; 3000])
            .expect("store");

        let Kept::Rejected(reason) = photos
            .store(&name("pho_0000000000000002"), &[2u8; 3000])
            .expect("store")
        else {
            panic!("the cap let a push through");
        };
        assert!(reason.contains("at its"), "{reason}");
        assert_eq!(
            held(&dir.0).expect("held").count,
            1,
            "the group keeps what it already had"
        );

        // And the refusal is against that photo, not against the group: one
        // that fits still goes.
        assert_eq!(
            photos
                .store(&name("pho_0000000000000003"), &[3u8; 512])
                .expect("store"),
            Kept::Stored
        );
    }

    /// A push interrupted by a power cut leaves a scratch file weighing as
    /// much as a photo, under a name nothing will ever ask for. Nothing else
    /// collects it — the relay deletes nothing on its own (DECISIONS 0050) —
    /// so the next open does.
    #[test]
    fn an_interrupted_push_leaves_nothing_behind() {
        let dir = TempDir::new("photos-scratch");
        fs::create_dir_all(dir_of(&dir.0)).expect("dir");
        let scratch = dir_of(&dir.0).join(format!("pho_0000000000000001{SCRATCH}"));
        fs::write(&scratch, [0u8; 999]).expect("half a push");
        // Something that is not ours and not a scratch file: left alone.
        // A name with a `.` in it, because anything a `PhotoName` accepts is
        // indistinguishable from a photo and would be counted as one.
        fs::write(dir_of(&dir.0).join("notes.txt"), b"not a photo").expect("stray");

        let photos = photos(&dir, DEFAULT_CAP);
        assert!(!photos.holds(&name("pho_0000000000000001")));
        let held = held(&dir.0).expect("held");
        assert_eq!(
            (held.count, held.bytes),
            (0, 0),
            "a torn write is not counted as a photo"
        );
        assert!(!scratch.exists(), "the scratch file was not swept");
        assert!(
            dir_of(&dir.0).join("notes.txt").exists(),
            "a stray was removed"
        );
    }
}

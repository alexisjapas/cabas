//! What is on disk, and how to remove one of it (DECISIONS 0050).
//!
//! Rotating the group phrase is the only way to revoke a lost device
//! (0024): every device moves to a new group id, and the old log stays
//! here — sealed, complete, and addressed by an id nobody will ever send
//! again. Nothing collects it, and nothing can: **the relay cannot tell an
//! abandoned group from a quiet one.** It holds no key, no roster and no
//! calendar of anyone's life; a group that rotated last spring and a
//! group whose two phones spent the summer elsewhere are the same
//! directory with an old timestamp.
//!
//! So there is no expiry and no sweep. What there is, is a person who knows
//! which one they abandoned, and two commands for them:
//!
//! - [`survey`] — what is here, how much of it, and when each group last
//!   received anything. Read-only, and it does **not** open the logs: doing
//!   so would mint an epoch for a group it is merely counting, which would
//!   cost every one of that group's devices a full replay. It counts the
//!   photos separately (DECISIONS 0080), because they are the half that
//!   grows without bound: a log is compacted by every snapshot a device
//!   pushes, and a photo is never rewritten and collected by nothing.
//! - [`forget`] — delete one, named in full. Photos included: they live
//!   inside the group's directory, which is what makes that true without
//!   this file knowing they exist.
//!
//! **Not an HTTP endpoint, and that is the security part.** A group id is
//! the whole of the relay's access control — `log`'s comment on `open` is
//! that a stranger cannot mine directories into existence because the ids
//! are unguessable. A listing served over the port that faces the tunnel
//! would hand out exactly the thing that is meant to be unguessable. These
//! run for whoever already has a shell on the machine, and for nobody else.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::log::read_meta;
use crate::photos;

/// One group's directory, as reported without opening it.
#[derive(Debug)]
pub struct Group {
    /// The directory name: a group id in hex, as it arrived in a `Hello`.
    pub id: String,
    /// Sequence numbers handed out over this group's whole life —
    /// including those a snapshot has since truncated away.
    pub frames: u64,
    /// Sealed photos this relay holds for the group (DECISIONS 0080), and
    /// what they weigh. Reported separately from `bytes` because they are
    /// the part that grows without bound — the log is compacted by every
    /// snapshot a device pushes, and a photo is never rewritten and never
    /// collected but by hand.
    pub photos: u64,
    pub photo_bytes: u64,
    /// Everything under the directory: the log, the meta, and the photos.
    pub bytes: u64,
    /// When this group last *received* something. `None` for one that said
    /// hello and never pushed.
    ///
    /// Read from the log file, which only an append or a snapshot rewrite
    /// touches — `meta` is rewritten on open too, so its timestamp would say
    /// "when the relay last restarted" and be useless for the one question
    /// this exists to answer.
    pub last_write: Option<SystemTime>,
}

/// Every group under `root`, oldest activity first — which puts the
/// candidates for [`forget`] at the top.
pub fn survey(root: &Path) -> io::Result<Vec<Group>> {
    let mut groups = Vec::new();
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        // A relay that has never been connected to has no directory yet, and
        // that is a fact to report rather than an error to raise.
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(groups),
        Err(e) => return Err(e),
    };

    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let Some(id) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !is_group_id(&id) {
            continue;
        }
        groups.push(inspect(&entry.path(), id)?);
    }

    // `None` — never wrote anything — sorts first: it is the emptiest thing
    // here and the least costly mistake to remove.
    groups.sort_by(|a, b| a.last_write.cmp(&b.last_write).then(a.id.cmp(&b.id)));
    Ok(groups)
}

/// Deletes one group's directory, irreversibly.
///
/// Named in full and never matched by prefix or by age: the whole point of
/// this module is that the machine cannot judge which of these is finished,
/// so it does not get to guess at one either.
///
/// The photos go with it, without this function naming them: they are a
/// subdirectory of the group's own directory, which is the whole reason
/// [`crate::photos`] puts them there (DECISIONS 0080).
///
/// Safe to run while the relay is serving. The group being forgotten is by
/// definition one no device connects to any more — that is what abandoned
/// means — so nothing holds it open. Forgetting a *live* group instead
/// would leave its connections answering "storage failed" until the process
/// restarts, which is the loud kind of wrong.
pub fn forget(root: &Path, id: &str) -> io::Result<Group> {
    if !is_group_id(id) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{id:?} is not a group id — 32 hex characters, as `groups` prints them"),
        ));
    }
    let dir = root.join(id);
    if !dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no group {id} under {}", root.display()),
        ));
    }
    let group = inspect(&dir, id.to_owned())?;
    fs::remove_dir_all(&dir)?;
    Ok(group)
}

fn inspect(dir: &Path, id: String) -> io::Result<Group> {
    let meta = read_meta(&dir.join("meta"))?;
    let log = dir.join("log");
    let held = photos::held(dir)?;
    Ok(Group {
        id,
        // `next_seq` is the number about to be handed out, so one less is
        // the count handed out so far.
        frames: meta.map(|m| m.next_seq.saturating_sub(1)).unwrap_or(0),
        photos: held.count,
        photo_bytes: held.bytes,
        bytes: weigh(dir)?,
        last_write: fs::metadata(&log).and_then(|m| m.modified()).ok(),
    })
}

fn weigh(dir: &Path) -> io::Result<u64> {
    let mut total = 0;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        total += if meta.is_dir() {
            weigh(&entry.path())?
        } else {
            meta.len()
        };
    }
    Ok(total)
}

/// A `GroupId` is 16 bytes rendered as hex, and the directory is named
/// after it. Anything else under the data root belongs to somebody else and
/// is left alone — including, deliberately, whatever a future version puts
/// there.
fn is_group_id(name: &str) -> bool {
    name.len() == 32 && name.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The survey as text, for a terminal.
///
/// Ages rather than dates: the question this answers is "which of these
/// stopped when I rotated", and "97 days" answers it without anyone doing
/// arithmetic on a timestamp — or this file gaining a date library to
/// render one.
pub fn render(groups: &[Group], now: SystemTime) -> String {
    if groups.is_empty() {
        return "no groups — nothing has ever synced through this relay\n".to_string();
    }

    let mut out = format!(
        "{:<32}  {:>8}  {:>16}  {:>9}  {}\n",
        "group", "frames", "photos", "size", "last write"
    );
    let mut total = 0;
    let mut total_photos = 0;
    for group in groups {
        total += group.bytes;
        total_photos += group.photos;
        let age = match group.last_write {
            Some(at) => now
                .duration_since(at)
                .map(|d| format!("{} ago", humanize(d)))
                .unwrap_or_else(|_| "in the future".to_string()),
            None => "never".to_string(),
        };
        out.push_str(&format!(
            "{:<32}  {:>8}  {:>16}  {:>9}  {}\n",
            group.id,
            group.frames,
            photo_column(group),
            bytes(group.bytes),
            age
        ));
    }
    out.push_str(&format!(
        "\n{} group{}, {} photo{}, {} on disk\n",
        groups.len(),
        if groups.len() == 1 { "" } else { "s" },
        total_photos,
        if total_photos == 1 { "" } else { "s" },
        bytes(total)
    ));
    out
}

/// The count and the weight in one column, because they answer one question:
/// this is the part of a group that grows without bound, and a person
/// deciding whether to `forget` it wants both halves at once.
fn photo_column(group: &Group) -> String {
    if group.photos == 0 {
        // Not "0 · 0 B": a group with no photos is a row to skip over, and
        // the arithmetic is noise in a column being scanned.
        return "—".to_string();
    }
    format!("{} · {}", group.photos, bytes(group.photo_bytes))
}

fn humanize(d: Duration) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;
    let s = d.as_secs();
    match s {
        0..MINUTE => format!("{s}s"),
        MINUTE..HOUR => format!("{}min", s / MINUTE),
        HOUR..DAY => format!("{}h", s / HOUR),
        _ => format!("{} days", s / DAY),
    }
}

/// A byte count as a person reads it. `pub(crate)` because it is the one
/// place in this crate that renders one, and [`crate::photos`] tells a device
/// its group's cap in the same words this prints it in.
pub(crate) fn bytes(n: u64) -> String {
    match n {
        0..1024 => format!("{n} B"),
        1024..1_048_576 => format!("{:.0} kB", n as f64 / 1024.0),
        _ => format!("{:.1} MB", n as f64 / 1_048_576.0),
    }
}

/// Where the data lives, resolved the same way the server resolves it.
pub fn data_dir() -> PathBuf {
    PathBuf::from(std::env::var("CABAS_RELAY_DATA").unwrap_or_else(|_| "/data".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::GroupLog;
    use crate::log::tests::TempDir;
    use cabas_sync::protocol::FrameKind;

    const A: &str = "0123456789abcdef0123456789abcdef";
    const B: &str = "fedcba9876543210fedcba9876543210";

    fn group(root: &Path, id: &str, frames: usize) {
        let mut log = GroupLog::open(root.join(id)).expect("open");
        for i in 0..frames {
            log.append(FrameKind::Delta, vec![i as u8; 64])
                .expect("append");
        }
    }

    /// Photos for a group, through the type the relay uses — so a change to
    /// where they live cannot leave this file testing a directory nothing
    /// writes to any more.
    fn pictures(root: &Path, id: &str, count: usize, each: usize) {
        let mut photos =
            photos::GroupPhotos::open(photos::dir_of(&root.join(id)), photos::DEFAULT_CAP)
                .expect("open");
        for i in 0..count {
            let name = cabas_sync::PhotoName::new(format!("pho_{i:016x}")).expect("a name");
            photos.store(&name, &vec![i as u8; each]).expect("store");
        }
    }

    #[test]
    fn a_relay_nobody_has_used_reports_nothing() {
        let dir = TempDir::new("admin-empty");
        // Not even created: `survey` is run on a fresh add-on too.
        let groups = survey(&dir.0).expect("survey");
        assert!(groups.is_empty());
        assert!(render(&groups, SystemTime::now()).contains("nothing has ever synced"));
    }

    #[test]
    fn a_survey_counts_without_opening() {
        let dir = TempDir::new("admin-survey");
        fs::create_dir_all(&dir.0).expect("root");
        group(&dir.0, A, 3);
        group(&dir.0, B, 1);

        // The epochs each group already minted. A survey that opened the
        // logs would mint new ones and cost every device a full replay.
        let before: Vec<u64> = [A, B]
            .iter()
            .map(|id| {
                read_meta(&dir.0.join(id).join("meta"))
                    .expect("meta")
                    .expect("some")
                    .epoch
            })
            .collect();

        let groups = survey(&dir.0).expect("survey");
        assert_eq!(groups.len(), 2);
        let a = groups.iter().find(|f| f.id == A).expect("A");
        assert_eq!(a.frames, 3);
        assert!(a.bytes > 0);
        assert!(a.last_write.is_some());

        let after: Vec<u64> = [A, B]
            .iter()
            .map(|id| {
                read_meta(&dir.0.join(id).join("meta"))
                    .expect("meta")
                    .expect("some")
                    .epoch
            })
            .collect();
        assert_eq!(before, after, "surveying must not touch an epoch");
    }

    #[test]
    fn anything_that_is_not_a_group_is_left_alone() {
        let dir = TempDir::new("admin-strangers");
        fs::create_dir_all(dir.0.join("not-hex")).expect("dir");
        fs::create_dir_all(dir.0.join("deadbeef")).expect("short");
        fs::write(dir.0.join(A), b"a file, not a group").expect("file");
        group(&dir.0, B, 1);

        let groups = survey(&dir.0).expect("survey");
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, B);
    }

    #[test]
    fn forgetting_takes_a_whole_id_and_nothing_less() {
        let dir = TempDir::new("admin-forget-guard");
        fs::create_dir_all(&dir.0).expect("root");
        group(&dir.0, A, 2);

        // A prefix, an empty string and a traversal are all "not an id" —
        // there is no matching here on purpose (DECISIONS 0050).
        for wrong in ["0123456789abcdef", "", "..", "../../etc"] {
            let e = forget(&dir.0, wrong).expect_err("refused");
            assert_eq!(e.kind(), io::ErrorKind::InvalidInput, "{wrong:?}");
        }
        // Well-formed and absent is a different answer from malformed.
        let e = forget(&dir.0, B).expect_err("absent");
        assert_eq!(e.kind(), io::ErrorKind::NotFound);

        assert!(dir.0.join(A).is_dir(), "nothing was removed");
    }

    #[test]
    fn forgetting_removes_one_group_and_reports_what_went() {
        let dir = TempDir::new("admin-forget");
        fs::create_dir_all(&dir.0).expect("root");
        group(&dir.0, A, 5);
        group(&dir.0, B, 2);

        let gone = forget(&dir.0, A).expect("forget");
        assert_eq!(gone.id, A);
        assert_eq!(gone.frames, 5);
        assert!(gone.bytes > 0);

        assert!(!dir.0.join(A).exists());
        let left = survey(&dir.0).expect("survey");
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].id, B);
    }

    #[test]
    fn the_stalest_group_is_listed_first() {
        let dir = TempDir::new("admin-order");
        fs::create_dir_all(&dir.0).expect("root");
        // Said hello, never pushed: no log file, so no last write at all.
        GroupLog::open(dir.0.join(A)).expect("open");
        group(&dir.0, B, 1);

        let groups = survey(&dir.0).expect("survey");
        assert_eq!(groups[0].id, A);
        assert!(groups[0].last_write.is_none());

        let text = render(&groups, SystemTime::now());
        assert!(text.contains("never"), "{text}");
        assert!(text.contains("2 groups"), "{text}");
    }

    #[test]
    fn ages_read_as_a_person_would_say_them() {
        assert_eq!(humanize(Duration::from_secs(12)), "12s");
        assert_eq!(humanize(Duration::from_secs(3 * 60 + 4)), "3min");
        assert_eq!(humanize(Duration::from_secs(5 * 3600)), "5h");
        assert_eq!(humanize(Duration::from_secs(97 * 24 * 3600)), "97 days");
        assert_eq!(bytes(512), "512 B");
        assert_eq!(bytes(2048), "2 kB");
        assert_eq!(bytes(3 * 1_048_576), "3.0 MB");
    }

    /// The count and the weight of the photos, which is the half of a group
    /// that grows without bound (DECISIONS 0080) — and the reason `groups`
    /// grew a column at all.
    #[test]
    fn a_survey_counts_the_photos_and_weighs_them() {
        let dir = TempDir::new("admin-photos");
        fs::create_dir_all(&dir.0).expect("root");
        group(&dir.0, A, 2);
        pictures(&dir.0, A, 3, 1000);
        group(&dir.0, B, 1);

        let groups = survey(&dir.0).expect("survey");
        let a = groups.iter().find(|g| g.id == A).expect("A");
        assert_eq!(a.photos, 3);
        assert_eq!(a.photo_bytes, 3000);
        assert!(
            a.bytes > a.photo_bytes,
            "the total carries the log as well as the photos"
        );

        let b = groups.iter().find(|g| g.id == B).expect("B");
        assert_eq!((b.photos, b.photo_bytes), (0, 0));

        let text = render(&groups, SystemTime::now());
        assert!(text.contains("3 · 3 kB"), "{text}");
        assert!(
            text.contains("—"),
            "a group with no photos reads as a dash: {text}"
        );
        assert!(text.contains("3 photos"), "{text}");
    }

    /// A survey must leave the directory exactly as it found it — the same
    /// rule that keeps it from opening a log and minting an epoch. Opening
    /// the photos would sweep an interrupted push, which is a write.
    #[test]
    fn a_survey_does_not_touch_the_photos_either() {
        let dir = TempDir::new("admin-photos-readonly");
        fs::create_dir_all(&dir.0).expect("root");
        group(&dir.0, A, 1);
        let scratch = photos::dir_of(&dir.0.join(A)).join("pho_0000000000000001.part");
        fs::create_dir_all(scratch.parent().expect("dir")).expect("dir");
        fs::write(&scratch, [0u8; 32]).expect("half a push");

        let groups = survey(&dir.0).expect("survey");
        assert_eq!(groups[0].photos, 0, "a torn write is not a photo");
        assert!(scratch.exists(), "the survey swept something");
    }

    /// Photos live inside the group's directory precisely so this holds
    /// without `forget` knowing about them.
    #[test]
    fn forgetting_a_group_takes_its_photos_with_it() {
        let dir = TempDir::new("admin-forget-photos");
        fs::create_dir_all(&dir.0).expect("root");
        group(&dir.0, A, 1);
        pictures(&dir.0, A, 2, 512);

        let gone = forget(&dir.0, A).expect("forget");
        assert_eq!(gone.photos, 2);
        assert_eq!(gone.photo_bytes, 1024);
        assert!(!photos::dir_of(&dir.0.join(A)).exists());
        assert!(!dir.0.join(A).exists());
    }
}

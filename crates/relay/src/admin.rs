//! What is on disk, and how to remove one of it (DECISIONS 0050).
//!
//! Rotating the family phrase is the only way to revoke a lost device
//! (0024): every device moves to a new family id, and the old log stays
//! here — sealed, complete, and addressed by an id nobody will ever send
//! again. Nothing collects it, and nothing can: **the relay cannot tell an
//! abandoned family from a quiet one.** It holds no key, no roster and no
//! calendar of anyone's life; a family that rotated last spring and a
//! family whose two phones spent the summer elsewhere are the same
//! directory with an old timestamp.
//!
//! So there is no expiry and no sweep. What there is, is a person who knows
//! which one they abandoned, and two commands for them:
//!
//! - [`survey`] — what is here, how much of it, and when each family last
//!   received anything. Read-only, and it does **not** open the logs: doing
//!   so would mint an epoch for a family it is merely counting, which would
//!   cost every one of that family's devices a full replay.
//! - [`forget`] — delete one, named in full.
//!
//! **Not an HTTP endpoint, and that is the security part.** A family id is
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

/// One family's directory, as reported without opening it.
#[derive(Debug)]
pub struct Family {
    /// The directory name: a family id in hex, as it arrived in a `Hello`.
    pub id: String,
    /// Sequence numbers handed out over this family's whole life —
    /// including those a snapshot has since truncated away.
    pub frames: u64,
    /// Everything under the directory.
    pub bytes: u64,
    /// When this family last *received* something. `None` for one that said
    /// hello and never pushed.
    ///
    /// Read from the log file, which only an append or a snapshot rewrite
    /// touches — `meta` is rewritten on open too, so its timestamp would say
    /// "when the relay last restarted" and be useless for the one question
    /// this exists to answer.
    pub last_write: Option<SystemTime>,
}

/// Every family under `root`, oldest activity first — which puts the
/// candidates for [`forget`] at the top.
pub fn survey(root: &Path) -> io::Result<Vec<Family>> {
    let mut families = Vec::new();
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        // A relay that has never been connected to has no directory yet, and
        // that is a fact to report rather than an error to raise.
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(families),
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
        if !is_family_id(&id) {
            continue;
        }
        families.push(inspect(&entry.path(), id)?);
    }

    // `None` — never wrote anything — sorts first: it is the emptiest thing
    // here and the least costly mistake to remove.
    families.sort_by(|a, b| a.last_write.cmp(&b.last_write).then(a.id.cmp(&b.id)));
    Ok(families)
}

/// Deletes one family's directory, irreversibly.
///
/// Named in full and never matched by prefix or by age: the whole point of
/// this module is that the machine cannot judge which of these is finished,
/// so it does not get to guess at one either.
///
/// Safe to run while the relay is serving. The family being forgotten is by
/// definition one no device connects to any more — that is what abandoned
/// means — so nothing holds it open. Forgetting a *live* family instead
/// would leave its connections answering "storage failed" until the process
/// restarts, which is the loud kind of wrong.
pub fn forget(root: &Path, id: &str) -> io::Result<Family> {
    if !is_family_id(id) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{id:?} is not a family id — 32 hex characters, as `families` prints them"),
        ));
    }
    let dir = root.join(id);
    if !dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no family {id} under {}", root.display()),
        ));
    }
    let family = inspect(&dir, id.to_owned())?;
    fs::remove_dir_all(&dir)?;
    Ok(family)
}

fn inspect(dir: &Path, id: String) -> io::Result<Family> {
    let meta = read_meta(&dir.join("meta"))?;
    let log = dir.join("log");
    Ok(Family {
        id,
        // `next_seq` is the number about to be handed out, so one less is
        // the count handed out so far.
        frames: meta.map(|m| m.next_seq.saturating_sub(1)).unwrap_or(0),
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

/// A `FamilyId` is 16 bytes rendered as hex, and the directory is named
/// after it. Anything else under the data root belongs to somebody else and
/// is left alone — including, deliberately, whatever a future version puts
/// there.
fn is_family_id(name: &str) -> bool {
    name.len() == 32 && name.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The survey as text, for a terminal.
///
/// Ages rather than dates: the question this answers is "which of these
/// stopped when I rotated", and "97 days" answers it without anyone doing
/// arithmetic on a timestamp — or this file gaining a date library to
/// render one.
pub fn render(families: &[Family], now: SystemTime) -> String {
    if families.is_empty() {
        return "no families — nothing has ever synced through this relay\n".to_string();
    }

    let mut out = format!(
        "{:<32}  {:>8}  {:>9}  {}\n",
        "family", "frames", "size", "last write"
    );
    let mut total = 0;
    for family in families {
        total += family.bytes;
        let age = match family.last_write {
            Some(at) => now
                .duration_since(at)
                .map(|d| format!("{} ago", humanize(d)))
                .unwrap_or_else(|_| "in the future".to_string()),
            None => "never".to_string(),
        };
        out.push_str(&format!(
            "{:<32}  {:>8}  {:>9}  {}\n",
            family.id,
            family.frames,
            bytes(family.bytes),
            age
        ));
    }
    out.push_str(&format!(
        "\n{} famil{}, {} on disk\n",
        families.len(),
        if families.len() == 1 { "y" } else { "ies" },
        bytes(total)
    ));
    out
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

fn bytes(n: u64) -> String {
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
    use crate::log::FamilyLog;
    use crate::log::tests::TempDir;
    use cabas_sync::protocol::FrameKind;

    const A: &str = "0123456789abcdef0123456789abcdef";
    const B: &str = "fedcba9876543210fedcba9876543210";

    fn family(root: &Path, id: &str, frames: usize) {
        let mut log = FamilyLog::open(root.join(id)).expect("open");
        for i in 0..frames {
            log.append(FrameKind::Delta, vec![i as u8; 64])
                .expect("append");
        }
    }

    #[test]
    fn a_relay_nobody_has_used_reports_nothing() {
        let dir = TempDir::new("admin-empty");
        // Not even created: `survey` is run on a fresh add-on too.
        let families = survey(&dir.0).expect("survey");
        assert!(families.is_empty());
        assert!(render(&families, SystemTime::now()).contains("nothing has ever synced"));
    }

    #[test]
    fn a_survey_counts_without_opening() {
        let dir = TempDir::new("admin-survey");
        fs::create_dir_all(&dir.0).expect("root");
        family(&dir.0, A, 3);
        family(&dir.0, B, 1);

        // The epochs each family already minted. A survey that opened the
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

        let families = survey(&dir.0).expect("survey");
        assert_eq!(families.len(), 2);
        let a = families.iter().find(|f| f.id == A).expect("A");
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
    fn anything_that_is_not_a_family_is_left_alone() {
        let dir = TempDir::new("admin-strangers");
        fs::create_dir_all(dir.0.join("not-hex")).expect("dir");
        fs::create_dir_all(dir.0.join("deadbeef")).expect("short");
        fs::write(dir.0.join(A), b"a file, not a family").expect("file");
        family(&dir.0, B, 1);

        let families = survey(&dir.0).expect("survey");
        assert_eq!(families.len(), 1);
        assert_eq!(families[0].id, B);
    }

    #[test]
    fn forgetting_takes_a_whole_id_and_nothing_less() {
        let dir = TempDir::new("admin-forget-guard");
        fs::create_dir_all(&dir.0).expect("root");
        family(&dir.0, A, 2);

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
    fn forgetting_removes_one_family_and_reports_what_went() {
        let dir = TempDir::new("admin-forget");
        fs::create_dir_all(&dir.0).expect("root");
        family(&dir.0, A, 5);
        family(&dir.0, B, 2);

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
    fn the_stalest_family_is_listed_first() {
        let dir = TempDir::new("admin-order");
        fs::create_dir_all(&dir.0).expect("root");
        // Said hello, never pushed: no log file, so no last write at all.
        FamilyLog::open(dir.0.join(A)).expect("open");
        family(&dir.0, B, 1);

        let families = survey(&dir.0).expect("survey");
        assert_eq!(families[0].id, A);
        assert!(families[0].last_write.is_none());

        let text = render(&families, SystemTime::now());
        assert!(text.contains("never"), "{text}");
        assert!(text.contains("2 families"), "{text}");
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
}

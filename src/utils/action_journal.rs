// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Paul <abonnementspaul (at) gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! An append-only record of everything SamRewritten changes. One JSON object
//! per line, grouped into operations by a batch id: an operation is one thing
//! the user did, and the unit the undo works in.

use crate::utils::app_paths::get_app_cache_dir;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// v2 names the change an undo put back by id, not by position in the batch,
/// which a trim could shift. v1 lines still load.
pub const FORMAT_VERSION: u32 = 2;

const FILE_NAME: &str = "action_journal.jsonl";

pub const ENABLED_KEY: &str = "action-journal-enabled";

const KEEP_ENTRIES: usize = 5_000;
const TRIM_ABOVE: usize = 6_000;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    ManualToggle,
    StatEdit,
    StagedUnlock,
    TimedUnlock,
    CopyTiming,
    ResetApp,
    BulkUnlock,
    BulkLock,
    Import,
    Revert,
}

impl Op {
    pub fn restores_stats(self) -> bool {
        matches!(self, Op::ResetApp | Op::Revert)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Change {
    Achievement {
        id: String,
        #[serde(default)]
        name: String,
        before: bool,
        after: bool,
    },
    IntStat {
        id: String,
        #[serde(default)]
        name: String,
        before: i32,
        after: i32,
    },
    FloatStat {
        id: String,
        #[serde(default)]
        name: String,
        before: f32,
        after: f32,
    },
    /// A library-wide operation: nothing here to put back. `detail` is an
    /// untranslated message id.
    Opaque { detail: String },
}

impl Change {
    pub fn key(&self) -> String {
        match self {
            Change::Achievement { id, .. } => format!("ach:{id}"),
            Change::IntStat { id, .. } => format!("int:{id}"),
            Change::FloatStat { id, .. } => format!("float:{id}"),
            Change::Opaque { detail } => format!("opaque:{detail}"),
        }
    }

    pub fn label(&self) -> &str {
        let (id, name) = match self {
            Change::Achievement { id, name, .. }
            | Change::IntStat { id, name, .. }
            | Change::FloatStat { id, name, .. } => (id, name),
            Change::Opaque { detail } => return detail,
        };
        if name.is_empty() { id } else { name }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reverses {
    pub batch: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Entry {
    pub v: u32,
    pub batch: u64,
    /// Unix seconds.
    pub at: u64,
    #[serde(default)]
    pub account: u32,
    pub app_id: u32,
    #[serde(default)]
    pub app_name: String,
    pub op: Op,
    pub change: Change,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverses: Option<Reverses>,
}

impl Entry {
    pub fn undoable(&self) -> bool {
        match self.change {
            Change::Achievement { .. } => true,
            Change::IntStat { .. } | Change::FloatStat { .. } => self.op.restores_stats(),
            Change::Opaque { .. } => false,
        }
    }
}

static ENABLED: AtomicBool = AtomicBool::new(false);
static ACCOUNT: AtomicU32 = AtomicU32::new(0);

/// Seeded from the clock, with the process id in the low bits: SamRewritten is
/// `NON_UNIQUE`, and two windows sharing the file must not mint the same ids.
static NEXT_BATCH: LazyLock<AtomicU64> =
    LazyLock::new(|| AtomicU64::new((now_millis() << 16) | u64::from(std::process::id() as u16)));

/// `(lines, bytes)` as this process last left the file, and the write lock. The
/// byte count is what says the tally still holds.
static LINES: Mutex<Option<(usize, u64)>> = Mutex::new(None);

pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub fn set_account(account: u32) {
    ACCOUNT.store(account, Ordering::Relaxed);
}

pub fn account() -> u32 {
    ACCOUNT.load(Ordering::Relaxed)
}

pub fn path() -> PathBuf {
    get_app_cache_dir().join(FILE_NAME)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Debug, Clone)]
pub struct Batch {
    id: u64,
    op: Op,
    app_id: u32,
    app_name: String,
    reverses: Option<Reverses>,
}

impl Batch {
    pub fn new(op: Op, app_id: u32, app_name: impl Into<String>) -> Self {
        Self {
            id: NEXT_BATCH.fetch_add(1, Ordering::Relaxed),
            op,
            app_id,
            app_name: app_name.into(),
            reverses: None,
        }
    }

    pub fn across(op: Op) -> Self {
        Self::new(op, 0, "")
    }

    pub fn reversing(app_id: u32, app_name: impl Into<String>, reverses: Reverses) -> Self {
        Self {
            reverses: Some(reverses),
            ..Self::new(Op::Revert, app_id, app_name)
        }
    }

    pub fn record(&self, changes: Vec<Change>) {
        let app_id = self.app_id;
        let app_name = self.app_name.clone();
        self.record_per_app(
            changes
                .into_iter()
                .map(|change| (app_id, app_name.clone(), change))
                .collect(),
        );
    }

    pub fn record_per_app(&self, changes: Vec<(u32, String, Change)>) {
        // An undo is written even with recording off: it is bookkeeping on a
        // history already on screen, and dropping it would leave the row it
        // undoes forever offering to be undone again.
        if changes.is_empty() || (!is_enabled() && self.op != Op::Revert) {
            return;
        }
        let at = now_secs();
        let account = account();
        let entries: Vec<Entry> = changes
            .into_iter()
            .map(|(app_id, app_name, change)| Entry {
                v: FORMAT_VERSION,
                batch: self.id,
                at,
                account,
                app_id,
                app_name,
                op: self.op,
                change,
                reverses: self.reverses,
            })
            .collect();
        append(&entries);
    }
}

/// On a sidecar, not the file itself: a trim renames a new file over the old
/// one, so a lock on the journal's own inode would stop meaning anything.
struct FileGuard(std::fs::File);

impl FileGuard {
    fn acquire(path: &Path) -> Option<Self> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path.with_extension("jsonl.lock"))
            .ok()?;
        file.lock().ok()?;
        Some(Self(file))
    }
}

impl Drop for FileGuard {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

fn append(entries: &[Entry]) {
    let mut buffer = String::new();
    let mut written_lines = 0usize;
    for entry in entries {
        match serde_json::to_string(entry) {
            Ok(line) => {
                buffer.push_str(&line);
                buffer.push('\n');
                written_lines += 1;
            }
            Err(e) => eprintln!("[CLIENT] Could not serialize a journal entry: {e}"),
        }
    }
    if buffer.is_empty() {
        return;
    }

    let path = path();
    // Always taken before `LINES`, never after: taking the two in opposite
    // orders in two places deadlocks the main loop against a worker.
    let _guard = FileGuard::acquire(&path);
    let mut lines = LINES.lock().unwrap_or_else(|e| e.into_inner());
    let file = OpenOptions::new().create(true).append(true).open(&path);
    let mut file = match file {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[CLIENT] Could not open {}: {e}", path.display());
            return;
        }
    };
    if let Err(e) = file.write_all(buffer.as_bytes()) {
        eprintln!("[CLIENT] Could not write to {}: {e}", path.display());
        return;
    }
    drop(file);

    let written = buffer.len() as u64;
    let counted = match *lines {
        Some((counted, bytes)) if file_len(&path) == Some(bytes + written) => {
            counted + written_lines
        }
        _ => count_lines(&path),
    };
    let counted = if counted > TRIM_ABOVE {
        trim(&path).unwrap_or(counted)
    } else {
        counted
    };
    *lines = file_len(&path).map(|bytes| (counted, bytes));
}

fn file_len(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|m| m.len())
}

/// Read whole rather than streamed: a line iterator stops at the first
/// unreadable byte, which in `trim` would truncate the file it rewrites.
fn read_lines(path: &Path) -> Vec<Vec<u8>> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    bytes
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .map(<[u8]>::to_vec)
        .collect()
}

fn count_lines(path: &Path) -> usize {
    read_lines(path).len()
}

fn trim(path: &Path) -> Option<usize> {
    let all = read_lines(path);
    if all.len() <= KEEP_ENTRIES {
        return Some(all.len());
    }
    let kept = &all[all.len() - KEEP_ENTRIES..];
    let temporary = path.with_extension("jsonl.tmp");
    let mut out = std::fs::File::create(&temporary).ok()?;
    for line in kept {
        if out
            .write_all(line)
            .and_then(|()| out.write_all(b"\n"))
            .is_err()
        {
            let _ = std::fs::remove_file(&temporary);
            return None;
        }
    }
    drop(out);
    if let Err(e) = std::fs::rename(&temporary, path) {
        eprintln!("[CLIENT] Could not trim {}: {e}", path.display());
        let _ = std::fs::remove_file(&temporary);
        return None;
    }
    Some(kept.len())
}

pub fn clear() {
    let path = path();
    let _guard = FileGuard::acquire(&path);
    let mut lines = LINES.lock().unwrap_or_else(|e| e.into_inner());
    match std::fs::remove_file(&path) {
        Ok(()) => *lines = Some((0, 0)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => *lines = Some((0, 0)),
        Err(e) => eprintln!("[CLIENT] Could not delete {}: {e}", path.display()),
    }
}

pub fn load() -> Vec<Entry> {
    let path = path();
    // Before the guard: never leave a lock file for a feature nobody turned on.
    if !path.exists() {
        return Vec::new();
    }
    let _guard = FileGuard::acquire(&path);
    let raw = read_lines(&path);
    let count = raw.len();
    let mut entries = Vec::new();
    for line in raw {
        match serde_json::from_slice::<Entry>(&line) {
            Ok(entry) => entries.push(entry),
            Err(e) => eprintln!("[CLIENT] Skipping an unreadable journal line: {e}"),
        }
    }
    *LINES.lock().unwrap_or_else(|e| e.into_inner()) = file_len(&path).map(|bytes| (count, bytes));
    entries
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordedChange {
    pub change: Change,
    pub app_id: u32,
    pub app_name: String,
    pub undoable: bool,
    pub reverted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Operation {
    pub batch: u64,
    pub at: u64,
    pub account: u32,
    pub app_id: u32,
    pub app_name: String,
    pub apps: Vec<(u32, String)>,
    pub op: Op,
    pub changes: Vec<RecordedChange>,
    pub reverses: Option<Reverses>,
    pub reverted: bool,
}

impl Operation {
    pub fn undoable(&self) -> bool {
        !self.reverted && self.changes.iter().any(|c| c.undoable && !c.reverted)
    }
}

pub fn group(entries: Vec<Entry>) -> Vec<Operation> {
    let mut operations: Vec<Operation> = Vec::new();
    let mut index_of: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();

    for entry in entries {
        let position = *index_of.entry(entry.batch).or_insert_with(|| {
            operations.push(Operation {
                batch: entry.batch,
                at: entry.at,
                account: entry.account,
                app_id: entry.app_id,
                app_name: entry.app_name.clone(),
                apps: Vec::new(),
                op: entry.op,
                changes: Vec::new(),
                reverses: entry.reverses,
                reverted: false,
            });
            operations.len() - 1
        });

        if let Some(target) = entry.reverses {
            mark_reverted(
                &mut operations,
                &index_of,
                target.batch,
                entry.app_id,
                &entry.change.key(),
                true,
                0,
            );
        }

        let undoable = entry.undoable();
        let operation = &mut operations[position];
        if !operation.apps.iter().any(|(id, _)| *id == entry.app_id) {
            operation.apps.push((entry.app_id, entry.app_name.clone()));
        }
        operations[position].changes.push(RecordedChange {
            change: entry.change,
            app_id: entry.app_id,
            app_name: entry.app_name,
            undoable,
            reverted: false,
        });
    }

    for operation in &mut operations {
        operation.reverted = operation.changes.iter().any(|c| c.undoable)
            && operation.changes.iter().all(|c| !c.undoable || c.reverted);
    }

    operations.reverse();
    operations
}

const MAX_REVERT_DEPTH: usize = 32;

/// Mark one change of `batch` as put back, or with `reverted` false as standing
/// again. It walks further than the entry names: if what was just taken back was
/// itself an undo, whatever *that* one restored is live once more.
fn mark_reverted(
    operations: &mut [Operation],
    index_of: &std::collections::HashMap<u64, usize>,
    batch: u64,
    app_id: u32,
    key: &str,
    reverted: bool,
    depth: usize,
) {
    let Some(&position) = index_of.get(&batch) else {
        return;
    };

    let mut hit = false;
    for change in operations[position].changes.iter_mut() {
        if change.app_id == app_id && change.change.key() == key {
            change.reverted = reverted;
            hit = true;
        }
    }

    if !hit || depth >= MAX_REVERT_DEPTH {
        return;
    }
    if operations[position].op == Op::Revert
        && let Some(inner) = operations[position].reverses
    {
        mark_reverted(
            operations,
            index_of,
            inner.batch,
            app_id,
            key,
            !reverted,
            depth + 1,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn achievement(id: &str, before: bool, after: bool) -> Change {
        Change::Achievement {
            id: id.to_string(),
            name: String::new(),
            before,
            after,
        }
    }

    fn entry(batch: u64, op: Op, change: Change, reverses: Option<Reverses>) -> Entry {
        Entry {
            v: FORMAT_VERSION,
            batch,
            at: 1_000,
            account: 7,
            app_id: 730,
            app_name: "Test".to_string(),
            op,
            change,
            reverses,
        }
    }

    #[test]
    fn an_entry_survives_a_round_trip_through_json() {
        let original = entry(
            1,
            Op::ManualToggle,
            achievement("WIN_ONE", false, true),
            None,
        );
        let line = serde_json::to_string(&original).unwrap();
        assert_eq!(serde_json::from_str::<Entry>(&line).unwrap(), original);
    }

    #[test]
    fn a_stat_is_undoable_only_as_part_of_an_app_reset() {
        let change = Change::IntStat {
            id: "kills".to_string(),
            name: "Kills".to_string(),
            before: 40,
            after: 0,
        };
        assert!(!entry(1, Op::StatEdit, change.clone(), None).undoable());
        assert!(entry(1, Op::ResetApp, change, None).undoable());
    }

    #[test]
    fn a_bulk_action_records_itself_but_offers_nothing_back() {
        let change = Change::Opaque {
            detail: "everything unlocked".to_string(),
        };
        assert!(!entry(1, Op::BulkUnlock, change, None).undoable());
    }

    #[test]
    fn undoing_one_change_leaves_the_rest_of_the_operation_standing() {
        let operations = group(vec![
            entry(1, Op::StagedUnlock, achievement("A", false, true), None),
            entry(1, Op::StagedUnlock, achievement("B", false, true), None),
            entry(
                2,
                Op::Revert,
                achievement("A", true, false),
                Some(Reverses { batch: 1 }),
            ),
        ]);

        let staged = operations.iter().find(|o| o.batch == 1).unwrap();
        assert!(staged.changes[0].reverted);
        assert!(!staged.changes[1].reverted);
        assert!(!staged.reverted);
        assert!(staged.undoable());
    }

    #[test]
    fn undoing_an_undo_puts_the_first_operation_back_on_offer() {
        let operations = group(vec![
            entry(1, Op::ManualToggle, achievement("A", false, true), None),
            entry(
                2,
                Op::Revert,
                achievement("A", true, false),
                Some(Reverses { batch: 1 }),
            ),
            entry(
                3,
                Op::Revert,
                achievement("A", false, true),
                Some(Reverses { batch: 2 }),
            ),
        ]);

        let first = operations.iter().find(|o| o.batch == 1).unwrap();
        let undo = operations.iter().find(|o| o.batch == 2).unwrap();
        assert!(
            !first.reverted,
            "the original must not still read as undone"
        );
        assert!(first.undoable());
        assert!(undo.reverted);
        assert!(!undo.undoable());
    }

    #[test]
    fn undoing_part_of_an_undo_puts_back_only_that_part() {
        let operations = group(vec![
            entry(1, Op::StagedUnlock, achievement("A", false, true), None),
            entry(1, Op::StagedUnlock, achievement("B", false, true), None),
            entry(
                2,
                Op::Revert,
                achievement("A", true, false),
                Some(Reverses { batch: 1 }),
            ),
            entry(
                2,
                Op::Revert,
                achievement("B", true, false),
                Some(Reverses { batch: 1 }),
            ),
            entry(
                3,
                Op::Revert,
                achievement("A", false, true),
                Some(Reverses { batch: 2 }),
            ),
        ]);

        let staged = operations.iter().find(|o| o.batch == 1).unwrap();
        assert!(!staged.changes[0].reverted, "A is unlocked again");
        assert!(staged.changes[1].reverted, "B is still locked");
        assert!(!staged.reverted);
        assert!(staged.undoable());
    }

    #[test]
    fn undoing_a_whole_operation_closes_it() {
        let operations = group(vec![
            entry(1, Op::StagedUnlock, achievement("A", false, true), None),
            entry(1, Op::StagedUnlock, achievement("B", false, true), None),
            entry(
                2,
                Op::Revert,
                achievement("A", true, false),
                Some(Reverses { batch: 1 }),
            ),
            entry(
                2,
                Op::Revert,
                achievement("B", true, false),
                Some(Reverses { batch: 1 }),
            ),
        ]);

        let staged = operations.iter().find(|o| o.batch == 1).unwrap();
        assert!(staged.reverted);
        assert!(!staged.undoable());
    }

    #[test]
    fn an_undo_closes_only_the_change_it_names() {
        let operations = group(vec![
            entry(1, Op::StagedUnlock, achievement("A", false, true), None),
            entry(1, Op::StagedUnlock, achievement("B", false, true), None),
            entry(
                2,
                Op::Revert,
                achievement("A", true, false),
                Some(Reverses { batch: 1 }),
            ),
        ]);

        let staged = operations.iter().find(|o| o.batch == 1).unwrap();
        assert!(staged.changes[0].reverted, "A was put back");
        assert!(!staged.changes[1].reverted, "B was not");
        assert!(!staged.reverted);
        assert!(staged.undoable());
    }

    #[test]
    fn a_stat_edit_is_never_offered_back_even_whole() {
        let operations = group(vec![entry(
            1,
            Op::StatEdit,
            Change::FloatStat {
                id: "distance".to_string(),
                name: "Distance".to_string(),
                before: 1.5,
                after: 9.0,
            },
            None,
        )]);
        assert!(!operations[0].undoable());
        assert!(!operations[0].reverted);
    }

    #[test]
    fn the_newest_operation_comes_first() {
        let operations = group(vec![
            entry(1, Op::ManualToggle, achievement("A", false, true), None),
            entry(2, Op::ManualToggle, achievement("B", false, true), None),
        ]);
        assert_eq!(operations[0].batch, 2);
        assert_eq!(operations[1].batch, 1);
    }

    #[test]
    fn trimming_keeps_the_newest_entries() {
        let path = std::env::temp_dir().join("samrewritten-journal-trim-test.jsonl");
        let mut file = std::fs::File::create(&path).unwrap();
        for i in 0..(TRIM_ABOVE + 10) {
            writeln!(file, r#"{{"batch":{i}}}"#).unwrap();
        }
        drop(file);

        assert_eq!(trim(&path), Some(KEEP_ENTRIES));

        let kept: Vec<String> = read_lines(&path)
            .into_iter()
            .map(|l| String::from_utf8(l).unwrap())
            .collect();
        assert_eq!(kept.len(), KEEP_ENTRIES);
        assert_eq!(
            kept.last().unwrap(),
            &format!(r#"{{"batch":{}}}"#, TRIM_ABOVE + 9)
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn an_undo_of_something_already_trimmed_away_is_ignored() {
        let operations = group(vec![entry(
            9,
            Op::Revert,
            achievement("A", true, false),
            Some(Reverses { batch: 1 }),
        )]);
        assert_eq!(operations.len(), 1);
        assert!(operations[0].undoable());
    }
}

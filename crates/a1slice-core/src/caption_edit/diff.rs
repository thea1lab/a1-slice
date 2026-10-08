//! Line diff between the old captions and the edited ones.

use super::types::{DiffKind, DiffRow};

pub fn diff_caption_lines<A: AsRef<str>, B: AsRef<str>>(before: &[A], after: &[B]) -> Vec<DiffRow> {
    let n = before.len();
    let m = after.len();
    let mut scores = vec![vec![0i64; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            scores[i][j] = if before[i].as_ref() == after[j].as_ref() {
                scores[i + 1][j + 1] + 1
            } else {
                scores[i + 1][j].max(scores[i][j + 1])
            };
        }
    }
    let mut rows = Vec::new();
    let mut i = 0;
    let mut j = 0;
    while i < n && j < m {
        if before[i].as_ref() == after[j].as_ref() {
            rows.push(DiffRow {
                kind: DiffKind::Same,
                text: before[i].as_ref().to_string(),
            });
            i += 1;
            j += 1;
        } else if scores[i + 1][j] >= scores[i][j + 1] {
            rows.push(DiffRow {
                kind: DiffKind::Remove,
                text: before[i].as_ref().to_string(),
            });
            i += 1;
        } else {
            rows.push(DiffRow {
                kind: DiffKind::Add,
                text: after[j].as_ref().to_string(),
            });
            j += 1;
        }
    }
    while i < n {
        rows.push(DiffRow {
            kind: DiffKind::Remove,
            text: before[i].as_ref().to_string(),
        });
        i += 1;
    }
    while j < m {
        rows.push(DiffRow {
            kind: DiffKind::Add,
            text: after[j].as_ref().to_string(),
        });
        j += 1;
    }
    rows
}

pub fn compact_diff(rows: &[DiffRow], context: i64) -> Vec<DiffRow> {
    let mut keep = vec![false; rows.len()];
    for (index, row) in rows.iter().enumerate() {
        if row.kind == DiffKind::Same {
            continue;
        }
        let last = rows.len() as i64 - 1;
        let from = (index as i64 - context).max(0);
        let to = (index as i64 + context).min(last);
        if to < from {
            continue;
        }
        for cursor in from..=to {
            keep[cursor as usize] = true;
        }
    }
    let mut compact = Vec::new();
    let mut skipped = false;
    for (index, row) in rows.iter().enumerate() {
        if !keep[index] {
            skipped = true;
            continue;
        }
        if skipped {
            compact.push(DiffRow {
                kind: DiffKind::Gap,
                text: "…".to_string(),
            });
            skipped = false;
        }
        compact.push(row.clone());
    }
    compact
}

pub fn diff_summary(rows: &[DiffRow]) -> String {
    let removed = rows
        .iter()
        .filter(|row| row.kind == DiffKind::Remove)
        .count();
    let added = rows.iter().filter(|row| row.kind == DiffKind::Add).count();
    if removed == 0 && added == 0 {
        return "The agent left the words as they were.".to_string();
    }
    let mut parts = Vec::new();
    if removed > 0 {
        parts.push(if removed == 1 {
            "Removed 1 line.".to_string()
        } else {
            format!("Removed {removed} lines.")
        });
    }
    if added > 0 {
        parts.push(if added == 1 {
            "Added 1 line.".to_string()
        } else {
            format!("Added {added} lines.")
        });
    }
    parts.join(" ")
}

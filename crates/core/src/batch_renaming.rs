// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
//! Captured rename previews with compatibility masks and stale-source rejection.
use crate::{JobControl, moving::move_entries_controlled};

// Recovered from original BatchRenameDialog tokens/preview and BatchRenameJob.
// See NOTICE.md and evidence/workflows/W05-batch-rename/contract.md.
use crate::{CopyIssue, CopyIssueKind, CopyPolicy, CopyRequest, MoveRequest, is_valid_leaf_name};
use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(default)]
/// User-editable rename mask/counter settings validated before a preview is captured.
pub struct BatchRenameOptions {
    /// Rename template interpreted using the documented compatibility tokens.
    pub mask: String,
    /// Literal substring replaced before rename case handling.
    pub search: String,
    /// Literal replacement for the configured search substring.
    pub replacement: String,
    /// Compatibility case mode index in the validated range zero through four.
    pub case_mode: usize,
    /// User-entered initial counter, parsed during preview construction.
    pub counter_start: String,
    /// User-entered signed counter increment, parsed during preview construction.
    pub counter_step: String,
    /// Validated minimum counter width between one and five digits.
    pub digits: usize,
}
impl Default for BatchRenameOptions {
    fn default() -> Self {
        Self {
            mask: "[N].[E]".into(),
            search: String::new(),
            replacement: String::new(),
            case_mode: 0,
            counter_start: "1".into(),
            counter_step: "1".into(),
            digits: 1,
        }
    }
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
/// An original direct-child name paired with its proposed replacement.
pub struct BatchRenameRow {
    /// Direct entry name, independent of a display path or target resolution.
    pub name: String,
    /// Proposed replacement direct-child name; validated before mutation.
    pub new_name: String,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
/// Ordered rename proposals and conflict checks used before capturing a mutation.
pub struct BatchRenamePreview {
    /// Rename proposals in the selected source order.
    pub rows: Vec<BatchRenameRow>,
    /// Whether proposed replacement names collide with each other.
    pub duplicate_names: bool,
    /// Whether a proposed name collides with another selected original name.
    pub old_names_conflict: bool,
    /// Whether preview conflict checks permit applying any changes.
    pub rename_enabled: bool,
    /// Number of proposed names that differ from their source names.
    pub changed_count: usize,
}
impl BatchRenamePreview {
    /// Check whether the preview has changes and every resulting name is a valid direct child.
    pub fn can_apply(&self) -> bool {
        self.rename_enabled
            && self.changed_count > 0
            && self
                .rows
                .iter()
                .all(|row| is_valid_leaf_name(&row.new_name))
    }
}
#[derive(Clone, Debug, Serialize)]
/// The result of one attempted rename, independent of later items.
pub struct BatchRenameOutcome {
    /// Direct entry name, independent of a display path or target resolution.
    pub name: String,
    /// Proposed replacement direct-child name; validated before mutation.
    pub new_name: String,
    /// Whether this item met the operation success contract; inspect detailed issues separately.
    pub success: bool,
}
#[derive(Clone, Debug, Default)]
/// Ordered rename outcomes and failures, including cancellation after partial progress.
pub struct BatchRenameReport {
    /// Whether cooperative cancellation stopped or prevented the remaining work.
    pub cancelled: bool,
    /// Ordered results for selected top-level items, including partial-operation outcomes.
    pub outcomes: Vec<BatchRenameOutcome>,
    /// Detailed per-path failures, skips, unsupported cases or cancellation notices.
    pub issues: Vec<CopyIssue>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Fingerprint {
    length: u64,
    modified: Option<SystemTime>,
    directory: bool,
    link: bool,
    readonly: bool,
    #[cfg(unix)]
    identity: (u64, u64, u32),
}
fn fingerprint(path: &Path) -> io::Result<Option<Fingerprint>> {
    match fs::symlink_metadata(path) {
        Ok(m) => Ok(Some(Fingerprint {
            length: m.len(),
            modified: m.modified().ok(),
            directory: m.is_dir(),
            link: m.is_symlink(),
            readonly: m.permissions().readonly(),
            #[cfg(unix)]
            identity: (m.dev(), m.ino(), m.mode()),
        })),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}
#[derive(Clone, Debug)]
/// An immutable reviewed preview tied to source fingerprints and validated options.
pub struct BatchRenamePlan {
    directory: PathBuf,
    preview: BatchRenamePreview,
    names: Vec<String>,
    options: BatchRenameOptions,
    fingerprints: Vec<Option<Fingerprint>>,
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
#[derive(Debug)]
enum Token {
    Literal(Vec<u16>),
    Part {
        kind: u16,
        start: i32,
        end: i32,
        count: i32,
    },
    Counter {
        current: i32,
        step: i32,
        digits: usize,
    },
    Date(Vec<u16>),
}
fn take_char(text: &[u16], position: &mut usize) -> u16 {
    let result = text.get(*position).copied().unwrap_or(0);
    if *position < text.len() {
        *position += 1;
    }
    result
}
fn take_int(text: &[u16], position: &mut usize, default: i32) -> i32 {
    let start = *position;
    while *position < text.len()
        && ((48..=57).contains(&text[*position]) || (text[*position] == 45 && *position == start))
    {
        *position += 1;
    }
    String::from_utf16_lossy(&text[start..*position])
        .parse()
        .unwrap_or(default)
}
fn compile(options: &BatchRenameOptions) -> io::Result<Vec<Token>> {
    if !(1..=5).contains(&options.digits) || options.case_mode > 4 || options.mask.len() > 4096 {
        return Err(invalid("Unsupported batch options"));
    }
    let text: Vec<u16> = options.mask.encode_utf16().collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < text.len() {
        if text[i] != 91 {
            result.push(Token::Literal(vec![text[i]]));
            i += 1;
            continue;
        }
        let Some(end) = ((i + 1)..text.len()).find(|&j| text[j] == 93) else {
            result.push(Token::Literal(text[i..].to_vec()));
            break;
        };
        let raw = &text[i + 1..end];
        i = end + 1;
        if raw.is_empty() {
            continue;
        }
        let mut pos = 1;
        result.push(match raw[0] {
            78 | 69 | 80 => {
                let start = take_int(raw, &mut pos, 0);
                let sep = take_char(raw, &mut pos);
                let end = if sep == 45 {
                    take_int(raw, &mut pos, 999)
                } else {
                    0
                };
                let count = if sep == 44 {
                    take_int(raw, &mut pos, 0)
                } else {
                    0
                };
                Token::Part {
                    kind: raw[0],
                    start,
                    end,
                    count,
                }
            }
            67 => {
                let current = take_int(raw, &mut pos, options.counter_start.parse().unwrap_or(0));
                let mut step = options.counter_step.parse().unwrap_or(0);
                let mut digits = options.digits as i32;
                if take_char(raw, &mut pos) == 44 {
                    step = take_int(raw, &mut pos, step);
                    if take_char(raw, &mut pos) == 44 {
                        digits = take_int(raw, &mut pos, digits);
                    }
                }
                if digits > 12 {
                    return Err(invalid("Counter padding supports at most 12 digits"));
                }
                Token::Counter {
                    current,
                    step,
                    digits: digits.max(0) as usize,
                }
            }
            89 | 77 | 68 | 104 | 109 | 115 => Token::Date(raw.to_vec()),
            91 => Token::Literal(vec![91]),
            _ => Token::Literal([&[91], raw, &[93]].concat()),
        });
    }
    Ok(result)
}
fn part(text: &str, mut start: i32, mut end: i32, count: i32) -> Vec<u16> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let length = units.len() as i32;
    if start < 0 {
        start = length.wrapping_add(start).wrapping_add(1);
        if start < 1 {
            start = 1;
        }
    }
    if end < 0 {
        end = length.wrapping_add(end).wrapping_add(1);
    }
    if start > 0 {
        if count > 0 {
            end = start.wrapping_add(count).wrapping_sub(1);
        } else if end == 0 {
            end = start;
        }
        if start <= end && start - 1 < length {
            return units[(start - 1) as usize..end.min(length) as usize].to_vec();
        }
        return Vec::new();
    }
    units
}
fn generated_name(
    name: &str,
    parent: &str,
    modified_ms: i64,
    tokens: &mut [Token],
    options: &BatchRenameOptions,
) -> io::Result<String> {
    let mut units = Vec::new();
    for token in tokens {
        units.extend(match token {
            Token::Literal(value) => value.clone(),
            Token::Part {
                kind,
                start,
                end,
                count,
            } => {
                let (stem, ext) = name.rsplit_once('.').unwrap_or((name, ""));
                part(
                    match kind {
                        78 => stem,
                        69 => ext,
                        _ => parent,
                    },
                    *start,
                    *end,
                    *count,
                )
            }
            Token::Counter {
                current,
                step,
                digits,
            } => {
                let n = i64::from(*current);
                let sign = if n < 0 { "-" } else { "" };
                let value = format!("{sign}{:0width$}", n.abs(), width = *digits);
                *current = current.wrapping_add(*step);
                value.encode_utf16().collect()
            }
            Token::Date(raw) => {
                let date = chrono::DateTime::from_timestamp_millis(modified_ms)
                    .filter(|date| (1970..=9999).contains(&date.year()))
                    .ok_or_else(|| invalid("UTC date tokens support years 1970–9999"))?;
                let mut value = String::new();
                for ch in raw {
                    match ch {
                        89 => value.push_str(&format!("{:04}", date.year())),
                        77 => value.push_str(&format!("{:02}", date.month())),
                        68 => value.push_str(&format!("{:02}", date.day())),
                        104 => value.push_str(&format!("{:02}", date.hour())),
                        109 => value.push_str(&format!("{:02}", date.minute())),
                        115 => value.push_str(&format!("{:02}", date.second())),
                        _ => {}
                    }
                }
                value.encode_utf16().collect()
            }
        });
    }
    let mut text =
        String::from_utf16(&units).map_err(|_| invalid("Name range splits a Unicode character"))?;
    if !options.search.is_empty() {
        text = text.replace(&options.search, &options.replacement);
    }
    if text.ends_with('.') {
        text.pop();
    }
    if options.case_mode != 0 && !text.is_ascii() {
        return Err(invalid(
            "Case conversion supports ASCII; use unchanged case for Unicode names",
        ));
    }
    Ok(match options.case_mode {
        1 => text.to_ascii_lowercase(),
        2 => text.to_ascii_uppercase(),
        3 => {
            let Some(first) = text.get(..1) else {
                return Err(invalid("First-uppercase mode needs a nonempty name"));
            };
            format!(
                "{}{}",
                first.to_ascii_uppercase(),
                text[1..].to_ascii_lowercase()
            )
        }
        4 => {
            let mut after_space = true;
            text.chars()
                .map(|c| {
                    if c == ' ' {
                        after_space = true;
                        c
                    } else if after_space {
                        after_space = false;
                        c.to_ascii_uppercase()
                    } else {
                        c
                    }
                })
                .collect()
        }
        _ => text,
    })
}
impl BatchRenamePlan {
    /// Canonical directory captured for this plan.
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    /// Immutable preview whose names match the captured evidence.
    pub fn preview(&self) -> &BatchRenamePreview {
        &self.preview
    }

    /// Capture source fingerprints and a validated ordered batch-rename preview.
    ///
    /// # Errors
    /// Returns invalid-preview or changed-source errors; completed individual renames are not rolled back.
    pub fn new(
        directory: &Path,
        names: &[String],
        options: &BatchRenameOptions,
    ) -> io::Result<Self> {
        let unique: HashSet<_> = names.iter().collect();
        if names.is_empty()
            || names.len() != unique.len()
            || names.iter().any(|name| !is_valid_leaf_name(name))
        {
            return Err(invalid("Select distinct direct children for batch rename"));
        }
        let directory = fs::canonicalize(directory)?;
        if !directory.is_dir() {
            return Err(invalid("Batch rename requires a folder"));
        }
        let parent = directory
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| invalid("Unsupported parent folder name"))?;
        let mut tokens = compile(options)?;
        let mut rows = Vec::new();
        let mut fingerprints = Vec::new();
        for name in names {
            let path = directory.join(name);
            fingerprints.push(fingerprint(&path)?);
            let modified = fs::metadata(&path)
                .ok()
                .and_then(|m| m.modified().ok())
                .map_or(0, |d| {
                    d.duration_since(UNIX_EPOCH)
                        .map_or(-1, |value| value.as_millis().min(i64::MAX as u128) as i64)
                });
            rows.push(BatchRenameRow {
                name: name.clone(),
                new_name: generated_name(name, parent, modified, &mut tokens, options)?,
            });
        }
        let mut seen = HashSet::new();
        let mut duplicate_names = false;
        let mut old_names_conflict = false;
        for row in &rows {
            if seen.contains(&row.new_name) {
                duplicate_names = true;
                break;
            }
            if row.new_name != row.name && unique.contains(&row.new_name) {
                old_names_conflict = true;
                break;
            }
            seen.insert(row.new_name.clone());
        }
        let changed_count = rows.iter().filter(|row| row.name != row.new_name).count();
        Ok(Self {
            directory,
            preview: BatchRenamePreview {
                rows,
                duplicate_names,
                old_names_conflict,
                rename_enabled: !duplicate_names && !old_names_conflict,
                changed_count,
            },
            names: names.to_vec(),
            options: options.clone(),
            fingerprints,
        })
    }
}

/// Apply the reviewed ordered plan. Additional guards reject stale/unsafe plans;
/// stable owned local trees remain required, as with W04. No race-safety claim.
pub fn apply_batch_rename(plan: &BatchRenamePlan) -> io::Result<BatchRenameReport> {
    apply_batch_rename_with_control(plan, &JobControl::default())
}

/// Apply the reviewed preview with cancellation between individual renames.
///
/// # Errors
/// Returns invalid-preview or changed-source errors; completed individual renames are not rolled back.
pub fn apply_batch_rename_with_control(
    plan: &BatchRenamePlan,
    control: &JobControl,
) -> io::Result<BatchRenameReport> {
    control.total(
        plan.preview
            .rows
            .iter()
            .filter(|row| row.name != row.new_name)
            .count(),
    );
    if !control.checkpoint() {
        return Ok(BatchRenameReport {
            cancelled: true,
            ..Default::default()
        });
    }
    if !plan.preview.can_apply() {
        return Err(invalid(
            "Resolve conflicts, invalid names or unchanged names before applying",
        ));
    }
    let current = BatchRenamePlan::new(&plan.directory, &plan.names, &plan.options)?;
    if current.preview != plan.preview || current.fingerprints != plan.fingerprints {
        return Err(invalid(
            "Selected files changed after preview; review a fresh plan",
        ));
    }
    let mut result = BatchRenameReport::default();
    for row in plan
        .preview
        .rows
        .iter()
        .filter(|row| row.name != row.new_name)
    {
        if !control.checkpoint() {
            break;
        }
        let report = move_entries_controlled(
            &MoveRequest {
                transfer: CopyRequest {
                    source_directory: plan.directory.clone(),
                    destination_directory: plan.directory.clone(),
                    names: vec![row.name.clone()],
                    new_name: Some(row.new_name.clone()),
                    policy: CopyPolicy::Skip,
                },
                rename_mode: true,
            },
            control,
            false,
        )?;
        if report.outcomes.is_empty() {
            break;
        }
        result.outcomes.push(BatchRenameOutcome {
            name: row.name.clone(),
            new_name: row.new_name.clone(),
            success: report.outcomes[0].success,
        });
        result.issues.extend(report.issues);
        if control.is_cancelled() {
            break;
        }
        control.completed();
    }
    result.cancelled = control.is_cancelled();
    if result.cancelled
        && !result
            .issues
            .iter()
            .any(|issue| issue.kind == CopyIssueKind::Cancelled)
    {
        result.issues.push(CopyIssue {
            path: plan.directory.clone(),
            kind: CopyIssueKind::Cancelled,
            message: "Batch rename cancelled; completed renames remain".to_owned(),
        });
    }
    Ok(result)
}

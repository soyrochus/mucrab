// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
//! Bounded, read-only local search. Filesystem trees must remain stable during IO.
use crate::{BrowserPane, JobControl, NameSorter, PinnedDirectory, filename_extension};
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
/// Read-only local search filters; invalid ranges are rejected by plan construction.
pub struct SearchQuery {
    /// Captured local search root; traversal does not follow directory links.
    pub root: PathBuf,
    /// Wildcard pattern using star, question mark and escaped literal characters.
    pub pattern: String,
    /// Whether wildcard and extension comparisons preserve letter case.
    pub case_sensitive: bool,
    /// Optional final filename extension, excluding empty or leading-dot suffixes.
    pub extension: String,
    /// Regular-file inclusion or count, according to the enclosing query/report.
    pub files: bool,
    /// Directory inclusion or count, according to the enclosing query/report.
    pub folders: bool,
    /// Symbolic-link inclusion/count or inode link count, as identified by the enclosing type.
    pub links: bool,
    /// Whether dot-prefixed names participate in search.
    pub include_hidden: bool,
    /// Optional inclusive minimum byte size for regular-file filtering.
    pub min_size: Option<u64>,
    /// Optional inclusive maximum byte size for regular-file filtering.
    pub max_size: Option<u64>,
    /// Optional inclusive UTC epoch-millisecond lower time bound.
    pub from_ms: Option<i64>,
    /// Optional exclusive UTC epoch-millisecond upper time bound.
    pub to_ms: Option<i64>,
}
impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            root: PathBuf::new(),
            pattern: "*".into(),
            case_sensitive: false,
            extension: String::new(),
            files: true,
            folders: true,
            links: true,
            include_hidden: false,
            min_size: None,
            max_size: None,
            from_ms: None,
            to_ms: None,
        }
    }
}
#[derive(Clone, Debug)]
enum Token {
    Star,
    Any,
    Literal(char),
}
#[derive(Clone, Debug)]
/// Validated compiled wildcard query against a captured local directory.
pub struct SearchPlan {
    query: SearchQuery,
    root: PinnedDirectory,
    tokens: Vec<Token>,
}
fn invalid(s: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, s)
}
impl SearchPlan {
    /// Validate search ranges and compile a bounded wildcard query against a pinned root.
    ///
    /// # Errors
    /// Returns validation, changed-source or filesystem errors; search reports retain bounded partial failures.
    pub fn new(mut query: SearchQuery) -> io::Result<Self> {
        if query
            .min_size
            .zip(query.max_size)
            .is_some_and(|(a, b)| a > b)
        {
            return Err(invalid("Minimum size exceeds maximum size"));
        }
        if query.from_ms.zip(query.to_ms).is_some_and(|(a, b)| a >= b) {
            return Err(invalid("From must precede the exclusive To time"));
        }
        if !query.files && !query.folders && !query.links {
            return Err(invalid("Choose at least one file type"));
        }
        let mut chars = query.pattern.chars();
        let mut tokens = Vec::new();
        while let Some(c) = chars.next() {
            tokens.push(match c {
                '*' => Token::Star,
                '?' => Token::Any,
                '\\' => Token::Literal(
                    chars
                        .next()
                        .ok_or_else(|| invalid("Trailing wildcard escape"))?,
                ),
                _ => Token::Literal(c),
            });
            if tokens.len() > 256 {
                return Err(invalid("Pattern exceeds 256 tokens"));
            }
        }
        let root = PinnedDirectory::new(&query.root)?;
        query.root = root.path.clone();
        Ok(Self {
            query,
            root,
            tokens,
        })
    }
    /// Borrow the validated query captured by this search plan.
    pub fn query(&self) -> &SearchQuery {
        &self.query
    }
    /// Match one name against the compiled wildcard and extension filters.
    pub fn matches_name(&self, name: &str) -> bool {
        let chars: Vec<char> = name.chars().collect();
        let mut row = vec![false; chars.len() + 1];
        row[0] = true;
        for token in &self.tokens {
            let mut next = vec![false; row.len()];
            for j in 0..row.len() {
                next[j] = match token {
                    Token::Star => row[j] || (j > 0 && next[j - 1]),
                    Token::Any => j > 0 && row[j - 1],
                    Token::Literal(c) => {
                        j > 0
                            && row[j - 1]
                            && if self.query.case_sensitive {
                                *c == chars[j - 1]
                            } else {
                                c.to_lowercase().eq(chars[j - 1].to_lowercase())
                            }
                    }
                };
            }
            row = next;
        }
        row[chars.len()]
            && (self.query.extension.is_empty()
                || if self.query.case_sensitive {
                    filename_extension(name) == Some(self.query.extension.as_str())
                } else {
                    filename_extension(name).unwrap_or("").to_lowercase()
                        == self.query.extension.to_lowercase()
                })
    }
    /// Search within default bounds, emitting bounded batches and retaining partial results.
    ///
    /// # Errors
    /// Returns validation, changed-source or filesystem errors; search reports retain bounded partial failures.
    pub fn run(
        &self,
        control: &JobControl,
        emit: impl FnMut(Vec<SearchHit>) -> bool,
    ) -> io::Result<SearchReport> {
        self.run_with_limits(control, SearchLimits::default(), emit)
    }
    /// Search within explicit bounds and stop when cancelled or the consumer declines a batch.
    ///
    /// # Errors
    /// Returns validation, changed-source or filesystem errors; search reports retain bounded partial failures.
    pub fn run_with_limits(
        &self,
        control: &JobControl,
        limits: SearchLimits,
        emit: impl FnMut(Vec<SearchHit>) -> bool,
    ) -> io::Result<SearchReport> {
        self.root.validate()?;
        if limits.hits == 0 || limits.visited == 0 || limits.depth == 0 {
            return Err(invalid("Search limits must be positive"));
        }
        let mounts = crate::everyday::mounted_roots()?;
        self.walk(control, limits, mounts, emit)
    }
    fn walk(
        &self,
        control: &JobControl,
        limits: SearchLimits,
        mounts: Vec<PathBuf>,
        mut emit: impl FnMut(Vec<SearchHit>) -> bool,
    ) -> io::Result<SearchReport> {
        #[cfg(unix)]
        let root_device = fs::symlink_metadata(&self.root.path)?.dev();
        let mut report = SearchReport::default();
        let mut stack = vec![(self.root.clone(), fs::read_dir(&self.root.path)?, 0usize)];
        let mut batch = Vec::with_capacity(64);
        while !stack.is_empty() {
            if !control.checkpoint() {
                report.cancelled = true;
                break;
            }
            let (dir, iter, depth) = stack.last_mut().expect("nonempty stack");
            if let Err(e) = dir.validate() {
                report.issue(&dir.path, &e.to_string());
                stack.pop();
                continue;
            }
            let entry = match iter.next() {
                None => {
                    stack.pop();
                    continue;
                }
                Some(Err(e)) => {
                    report.issue(&dir.path, &e.to_string());
                    continue;
                }
                Some(Ok(e)) => e,
            };
            let depth = *depth;
            if report.visited >= limits.visited.min(1_000_000) {
                report.issue(&entry.path(), "Visited-entry limit reached");
                break;
            }
            report.visited += 1;
            let path = entry.path();
            if !control.entry(&path, None) {
                report.cancelled = true;
                break;
            }
            control.completed();
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                report.issue(&path, "Non-Unicode name omitted");
                continue;
            };
            // Use the same visible spelling as local pane listings on macOS.
            #[cfg(target_os = "macos")]
            let name = icu_normalizer::ComposingNormalizerBorrowed::new_nfc()
                .normalize(&name)
                .into_owned();
            if !self.query.include_hidden && name.starts_with('.') {
                continue;
            }
            let meta = match fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(e) => {
                    report.issue(&path, &e.to_string());
                    continue;
                }
            };
            let kind = kind(&meta);
            let modified = meta.modified().ok().and_then(millis);
            let enabled = match kind {
                SearchKind::File => self.query.files,
                SearchKind::Folder => self.query.folders,
                SearchKind::Link => self.query.links,
                SearchKind::Other => false,
            };
            let size_enabled = self.query.min_size.is_some() || self.query.max_size.is_some();
            let time_enabled = self.query.from_ms.is_some() || self.query.to_ms.is_some();
            if time_enabled && modified.is_none() {
                report.issue(&path, "Modification time unavailable");
            }
            if enabled
                && self.matches_name(&name)
                && (!size_enabled
                    || (kind == SearchKind::File
                        && self.query.min_size.is_none_or(|v| meta.len() >= v)
                        && self.query.max_size.is_none_or(|v| meta.len() <= v)))
                && (!time_enabled
                    || modified.is_some_and(|m| {
                        self.query.from_ms.is_none_or(|v| m >= v)
                            && self.query.to_ms.is_none_or(|v| m < v)
                    }))
            {
                let hit = SearchHit {
                    relative_path: path
                        .strip_prefix(&self.root.path)
                        .expect("child")
                        .to_owned(),
                    path: path.clone(),
                    kind,
                    size: meta.len(),
                    modified_ms: modified,
                    identity: Identity::new(&meta),
                };
                report.hits.push(hit.clone());
                batch.push(hit);
                if batch.len() == 64 && !emit(std::mem::take(&mut batch)) {
                    report.cancelled = true;
                    break;
                }
                if report.hits.len() >= limits.hits.min(10_000) {
                    report.issue(&path, "Result limit reached");
                    break;
                }
            }
            if kind == SearchKind::Folder {
                let same_device = {
                    #[cfg(unix)]
                    {
                        root_device == meta.dev()
                    }
                    #[cfg(not(unix))]
                    {
                        true
                    }
                };
                if mounts.contains(&path) || !same_device {
                    report.issue(&path, "Mount boundary omitted");
                    continue;
                }
                if depth + 1 >= limits.depth.min(256) {
                    report.issue(&path, "Depth limit reached");
                    continue;
                }
                match PinnedDirectory::new(&path)
                    .and_then(|p| fs::read_dir(&p.path).map(|r| (p, r, depth + 1)))
                {
                    Ok(next) => stack.push(next),
                    Err(e) => report.issue(&path, &e.to_string()),
                }
            }
        }
        if !batch.is_empty() && !emit(batch) {
            report.cancelled = true;
        }
        report.cancelled |= control.is_cancelled();
        report
            .hits
            .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        Ok(report)
    }
}
#[derive(Clone, Copy, Debug)]
/// Positive traversal bounds with production upper caps applied during search.
pub struct SearchLimits {
    /// Maximum nested directory depth for traversal.
    pub depth: usize,
    /// Visited-entry count or traversal limit, according to the enclosing type.
    pub visited: usize,
    /// Result count bound or collected matching entries, according to the enclosing type.
    pub hits: usize,
}
impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            depth: 256,
            visited: 1_000_000,
            hits: 10_000,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
/// An entry classification that does not follow directory or file links.
pub enum SearchKind {
    #[doc = "A regular file without following a symbolic-link entry."]
    File,
    #[doc = "A directory entry without following a symbolic link."]
    Folder,
    #[doc = "A symbolic-link entry, including missing or external targets."]
    Link,
    #[doc = "An object outside regular-file, directory and symbolic-link classifications."]
    Other,
}
fn kind(m: &fs::Metadata) -> SearchKind {
    if m.is_symlink() {
        SearchKind::Link
    } else if m.is_dir() {
        SearchKind::Folder
    } else if m.is_file() {
        SearchKind::File
    } else {
        SearchKind::Other
    }
}
fn millis(t: SystemTime) -> Option<i64> {
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => i64::try_from(d.as_millis()).ok(),
        Err(e) => i64::try_from(e.duration().as_nanos().div_ceil(1_000_000))
            .ok()
            .map(|v| -v),
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct Identity {
    #[cfg(unix)]
    dev: u64,
    #[cfg(unix)]
    ino: u64,
    kind: SearchKind,
    size: u64,
    modified: Option<SystemTime>,
}
impl Identity {
    fn new(m: &fs::Metadata) -> Self {
        Self {
            #[cfg(unix)]
            dev: m.dev(),
            #[cfg(unix)]
            ino: m.ino(),
            kind: kind(m),
            size: m.len(),
            modified: m.modified().ok(),
        }
    }
}
#[derive(Clone, Debug, Serialize)]
/// One search result tied to the identity required for open-containing revalidation.
pub struct SearchHit {
    /// Path relative to the captured search root, used for stable result ordering.
    pub relative_path: PathBuf,
    /// Captured or reported filesystem/remote path; inspect the enclosing operation contract.
    pub path: PathBuf,
    /// Typed category independent of user-facing diagnostic wording.
    pub kind: SearchKind,
    /// File size in bytes; compatibility listing metadata can use -1 for unknown size.
    pub size: u64,
    /// Signed milliseconds relative to the UTC Unix epoch.
    pub modified_ms: Option<i64>,
    #[serde(skip)]
    identity: Identity,
}
impl SearchHit {
    /// Revalidate a search hit and navigate its pane to the containing folder.
    ///
    /// # Errors
    /// Returns validation, changed-source or filesystem errors; search reports retain bounded partial failures.
    pub fn open_containing(&self, pane: &mut BrowserPane, sorter: &NameSorter) -> io::Result<()> {
        let meta = fs::symlink_metadata(&self.path)?;
        if Identity::new(&meta) != self.identity {
            return Err(invalid("Result changed; search again"));
        }
        let parent = self
            .path
            .parent()
            .ok_or_else(|| invalid("Result has no parent"))?;
        let name = self
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| invalid("Invalid result name"))?;
        let pinned = PinnedDirectory::new(parent)?;
        pinned.validate()?;
        let retained_marks = if pane.archive().is_none()
            && pane.location().map(|p| p.as_path()) == Some(pinned.path.as_path())
        {
            pane.selection.snapshot(&pane.entries, &pane.order).marked
        } else {
            Vec::new()
        };
        if pane.browsing_deferred() {
            pane.queue_browse(parent.to_owned(), true, None);
            pane.focus_after_browse(name);
            return Ok(());
        }
        let mut candidate = BrowserPane {
            options: pane.options,
            show_hidden: pane.show_hidden,
            history: pane.history.clone(),
            ..Default::default()
        };
        if !candidate.open(&pinned.path, sorter) {
            return Err(io::Error::other(
                candidate
                    .error
                    .clone()
                    .unwrap_or_else(|| "Cannot open containing folder".into()),
            ));
        }
        pinned.validate()?;
        if Identity::new(&fs::symlink_metadata(&self.path)?) != self.identity {
            return Err(invalid("Result changed while opening its folder"));
        }
        if !candidate.focus_name(name) {
            return Err(invalid(
                "Result is hidden or disappeared; no file was selected",
            ));
        }
        for (position, index) in candidate.order.iter().enumerate() {
            if retained_marks.contains(&candidate.entries[*index].name) {
                candidate.selection.set_marked(
                    position
                        + usize::from(
                            candidate
                                .location()
                                .map(|p| p.as_path())
                                .and_then(|p| p.parent())
                                .is_some(),
                        ),
                    true,
                    &candidate.order,
                );
            }
        }
        *pane = candidate;
        Ok(())
    }
}
#[derive(Clone, Debug, Default, Serialize)]
/// Collected bounded hits and partial traversal diagnostics, with no mutation effects.
pub struct SearchReport {
    /// Result count bound or collected matching entries, according to the enclosing type.
    pub hits: Vec<SearchHit>,
    /// Visited-entry count or traversal limit, according to the enclosing type.
    pub visited: usize,
    /// Total encountered traversal errors, even when stored details are capped.
    pub error_count: usize,
    /// Detailed per-path failures, skips, unsupported cases or cancellation notices.
    pub issues: Vec<String>,
    /// Whether cooperative cancellation stopped or prevented the remaining work.
    pub cancelled: bool,
}
impl SearchReport {
    fn issue(&mut self, path: &Path, message: &str) {
        self.error_count += 1;
        if self.issues.len() < 256 {
            self.issues.push(format!("{}: {message}", path.display()));
        }
    }
    /// Report whether cancellation or encountered errors make the totals incomplete.
    pub fn partial(&self) -> bool {
        self.cancelled || self.error_count > 0
    }
}
/// UTC date-only To boundaries include the entire day; timestamps stay exclusive.
pub fn search_time_bound(text: &str, end: bool) -> Result<Option<i64>, String> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    if let Ok(date) = chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d") {
        let date = if end {
            date.succ_opt().ok_or("Date overflow")?
        } else {
            date
        };
        return Ok(Some(
            date.and_hms_opt(0, 0, 0)
                .ok_or("Invalid date")?
                .and_utc()
                .timestamp_millis(),
        ));
    }
    chrono::DateTime::parse_from_rfc3339(text)
        .map(|d| Some(d.timestamp_millis()))
        .map_err(|_| "Use YYYY-MM-DD or an RFC3339 UTC timestamp".into())
}

#[cfg(test)]
mod mount_tests {
    use super::*;
    #[test]
    fn discovered_mount_roots_are_reported_without_entering_them() {
        let root = std::env::temp_dir().join(format!(
            "mucrab-search-mount-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("mounted")).unwrap();
        // Match SearchPlan's canonical root (macOS /var aliases /private/var).
        let root = fs::canonicalize(root).unwrap();
        fs::write(root.join("mounted/omitted"), b"x").unwrap();
        let plan = SearchPlan::new(SearchQuery {
            root: root.clone(),
            ..Default::default()
        })
        .unwrap();
        let r = plan
            .walk(
                &JobControl::default(),
                SearchLimits::default(),
                vec![root.join("mounted")],
                |_| true,
            )
            .unwrap();
        assert!(r.partial());
        assert_eq!(r.hits.len(), 1);
        assert_eq!(r.hits[0].relative_path, Path::new("mounted"));
        assert!(r.issues[0].contains("Mount boundary"));
        fs::remove_dir_all(root).unwrap();
    }
}

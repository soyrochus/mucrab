// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

// Recovered from muCommander 1.6.2 FileComparator and FileTableModel binaries.
// See NOTICE.md and evidence/workflows/W01-local-browse-sort/ for upstream
// attribution, original-bytecode fingerprints, and scoped comparison evidence.

use crate::FileEntry;
use icu_collator::{Collator, CollatorBorrowed, options::CollatorOptions, options::Strength};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SortCriterion {
    #[default]
    Name,
    Size,
    Date,
    Extension,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SortMode {
    #[default]
    Natural,
    Lexicographic,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SortOptions {
    #[serde(default)]
    pub criterion: SortCriterion,
    pub ascending: bool,
    pub directories_first: bool,
    pub mode: SortMode,
}

impl Default for SortOptions {
    fn default() -> Self {
        Self {
            criterion: SortCriterion::Name,
            ascending: true,
            directories_first: true,
            mode: SortMode::Natural,
        }
    }
}

pub struct NameSorter {
    tertiary: CollatorBorrowed<'static>,
    primary: CollatorBorrowed<'static>,
}

impl NameSorter {
    /// W01 supports one explicit locale, independent of the host environment.
    pub fn en_us() -> Result<Self, String> {
        let make = |strength| {
            let mut options = CollatorOptions::default();
            options.strength = Some(strength);
            Collator::try_new(icu_locale_core::locale!("en-US").into(), options)
                .map_err(|error| error.to_string())
        };
        Ok(Self {
            tertiary: make(Strength::Tertiary)?,
            primary: make(Strength::Primary)?,
        })
    }

    pub fn compare(&self, left: &FileEntry, right: &FileEntry, options: SortOptions) -> Ordering {
        // FileComparator returns here BEFORE applying descending inversion.
        if options.directories_first && left.is_directory != right.is_directory {
            return right.is_directory.cmp(&left.is_directory);
        }
        let primary = match options.criterion {
            SortCriterion::Name => Ordering::Equal,
            SortCriterion::Size => {
                let size = |entry: &FileEntry| {
                    if entry.is_directory {
                        return 0;
                    }
                    match entry.metadata.as_ref().map_or(-1, |metadata| metadata.size) {
                        -1 => i64::MAX,
                        value => value,
                    }
                };
                size(left).wrapping_sub(size(right)).cmp(&0)
            }
            SortCriterion::Date => left
                .metadata
                .as_ref()
                .map_or(0, |m| m.modified_ms)
                .wrapping_sub(right.metadata.as_ref().map_or(0, |m| m.modified_ms))
                .cmp(&0),
            SortCriterion::Extension => compare_extensions(
                left.metadata.as_ref().and_then(|m| m.extension.as_deref()),
                right.metadata.as_ref().and_then(|m| m.extension.as_deref()),
            ),
        };
        let order = primary.then_with(|| self.compare_names(&left.name, &right.name, options.mode));
        if options.ascending {
            order
        } else {
            order.reverse()
        }
    }

    fn compare_names(&self, left: &str, right: &str, mode: SortMode) -> Ordering {
        if mode == SortMode::Natural
            && let (Some((lp, ln)), Some((rp, rn))) = (first_number(left), first_number(right))
            && lp == rp
        {
            // Equal prefixes also imply equal Java UTF-16 numeric start offsets.
            let ln = ln.trim_start_matches('0');
            let rn = rn.trim_start_matches('0');
            let numeric = ln.len().cmp(&rn.len()).then_with(|| ln.cmp(rn));
            if numeric != Ordering::Equal {
                return numeric;
            }
        }
        let order = self.tertiary.compare(left, right);
        if order == Ordering::Equal {
            self.primary.compare(left, right)
        } else {
            order
        }
    }

    /// Recover the original FileTableModel index permutation, including ties.
    /// An explicit stack avoids recursive stack overflow on large equal sets.
    pub fn sorted_indices(&self, entries: &[FileEntry], options: SortOptions) -> Vec<usize> {
        let mut indices: Vec<_> = (0..entries.len()).collect();
        self.sort_indices(entries, options, &mut indices);
        indices
    }

    /// Subsequent table sorts start from the existing index permutation.
    /// Metadata must satisfy FileMetadata::is_sortable; arbitrary extreme
    /// synthetic values can make the original table sorter non-terminating.
    pub fn sort_indices(&self, entries: &[FileEntry], options: SortOptions, indices: &mut [usize]) {
        if indices.len() < 2 {
            return;
        }
        let mut pending = vec![(0, indices.len() - 1)];
        while let Some((low, high)) = pending.pop() {
            if low >= high {
                continue;
            }
            if low + 1 == high {
                if self
                    .compare(&entries[indices[low]], &entries[indices[high]], options)
                    .is_gt()
                {
                    indices.swap(low, high);
                }
                continue;
            }
            let mut lo = low;
            let mut hi = high;
            let mid = (lo + hi) / 2;
            let pivot = indices[mid];
            indices.swap(mid, hi);
            while lo < hi {
                while lo < hi
                    && !self
                        .compare(&entries[indices[lo]], &entries[pivot], options)
                        .is_gt()
                {
                    lo += 1;
                }
                while lo < hi
                    && !self
                        .compare(&entries[pivot], &entries[indices[hi]], options)
                        .is_gt()
                {
                    hi -= 1;
                }
                if lo < hi {
                    indices.swap(lo, hi);
                }
            }
            indices[high] = indices[hi];
            indices[hi] = pivot;
            if hi < high {
                pending.push((hi + 1, high));
            }
            if lo > low {
                pending.push((low, lo - 1));
            }
        }
    }
}

fn compare_extensions(left: Option<&str>, right: Option<&str>) -> Ordering {
    fn fold(character: char) -> u32 {
        let code = character as u32;
        let data = crate::java_case_data::JAVA_SIMPLE_FOLD;
        data.binary_search_by_key(&code, |(source, _)| *source)
            .map_or(code, |index| data[index].1)
    }
    match (left, right) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(left), Some(right)) => left.chars().map(fold).cmp(right.chars().map(fold)),
    }
}

fn first_number(name: &str) -> Option<(&str, &str)> {
    // Java Pattern.compile("\\d+") has no UNICODE_CHARACTER_CLASS flag.
    let start = name.as_bytes().iter().position(u8::is_ascii_digit)?;
    let length = name.as_bytes()[start..]
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    Some((&name[..start], &name[start..start + length]))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn file(name: &str, directory: bool) -> FileEntry {
        FileEntry {
            name: name.into(),
            is_directory: directory,
            is_symlink: false,
            metadata: None,
        }
    }
    #[test]
    fn directories_stay_first_when_descending() {
        let sorter = NameSorter::en_us().unwrap();
        let options = SortOptions {
            ascending: false,
            ..Default::default()
        };
        assert_eq!(
            sorter.compare(&file("z", true), &file("a", false), options),
            Ordering::Less
        );
        assert_eq!(
            sorter.compare(&file("file2", false), &file("file10", false), options),
            Ordering::Greater
        );
    }
    #[test]
    fn first_group_rule_and_collation_ties_match_recovered_behavior() {
        let sorter = NameSorter::en_us().unwrap();
        assert_eq!(
            sorter.compare_names("x1y10", "x1y2", SortMode::Natural),
            Ordering::Less
        );
        assert_eq!(
            sorter.compare_names("file2", "file10", SortMode::Natural),
            Ordering::Less
        );
        assert_eq!(
            sorter.compare_names("é", "e\u{301}", SortMode::Natural),
            Ordering::Equal
        );
        let equal = vec![file("a", false); 4];
        // Original quicksort moves equal pivots; std::slice::sort is not equivalent.
        assert_eq!(
            sorter.sorted_indices(&equal, SortOptions::default()),
            vec![0, 2, 3, 1]
        );
    }

    #[test]
    fn metadata_ties_unknown_sizes_and_java_unicode_case_rules() {
        use crate::FileMetadata;
        let sorter = NameSorter::en_us().unwrap();
        let entry = |name: &str, size, date, extension: Option<&str>, directory| FileEntry {
            metadata: Some(FileMetadata {
                size,
                modified_ms: date,
                extension: extension.map(str::to_owned),
            }),
            ..file(name, directory)
        };
        let options = SortOptions {
            criterion: SortCriterion::Size,
            directories_first: false,
            ..Default::default()
        };
        let directory = entry("a-folder", 999, 0, None, true);
        let zero = entry("z-file", 0, 0, None, false);
        assert_eq!(sorter.compare(&directory, &zero, options), Ordering::Less);
        assert_eq!(
            sorter.compare(
                &entry("a", -1, 0, None, false),
                &entry("z", 5, 0, None, false),
                options
            ),
            Ordering::Greater
        );
        let ext = SortOptions {
            criterion: SortCriterion::Extension,
            ..options
        };
        assert_eq!(
            sorter.compare(
                &entry("file2", 0, 0, Some("İ"), false),
                &entry("file10", 0, 0, Some("ı"), false),
                ext
            ),
            Ordering::Less
        );
        // Java simple case does not expand sharp S into the two-letter "SS".
        assert_eq!(compare_extensions(Some("ß"), Some("SS")), Ordering::Greater);
        assert_eq!(compare_extensions(Some("𐐀"), Some("𐐨")), Ordering::Equal);
        assert_eq!(compare_extensions(None, Some("")), Ordering::Less);
        let date = SortOptions {
            criterion: SortCriterion::Date,
            ascending: false,
            ..options
        };
        let extreme = entry("x", 0, i64::MIN, None, false);
        assert!(!extreme.metadata.as_ref().unwrap().is_sortable());
        // Preserve the original pair signs without feeding this non-order into quicksort.
        assert_eq!(sorter.compare(&extreme, &zero, date), Ordering::Greater);
        assert_eq!(sorter.compare(&zero, &extreme, date), Ordering::Greater);
    }
}

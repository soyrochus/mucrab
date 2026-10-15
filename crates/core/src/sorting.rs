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
pub enum SortMode {
    #[default]
    Natural,
    Lexicographic,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SortOptions {
    pub ascending: bool,
    pub directories_first: bool,
    pub mode: SortMode,
}

impl Default for SortOptions {
    fn default() -> Self {
        Self {
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
        let order = self.compare_names(&left.name, &right.name, options.mode);
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
        if entries.len() < 2 {
            return indices;
        }
        let mut pending = vec![(0, entries.len() - 1)];
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
        indices
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
}

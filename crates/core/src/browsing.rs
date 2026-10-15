// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

use crate::{NameSorter, SortOptions};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FileEntry {
    pub name: String,
    pub is_directory: bool,
    pub is_symlink: bool,
}

pub fn list_directory(path: &Path) -> io::Result<Vec<FileEntry>> {
    fs::read_dir(path)?
        .map(|item| {
            let item = item?;
            let name = item.file_name().into_string().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "W01 does not support non-Unicode file names",
                )
            })?;
            Ok(FileEntry {
                name,
                is_directory: item.path().is_dir(),
                is_symlink: item.file_type()?.is_symlink(),
            })
        })
        .collect()
}

#[derive(Default)]
pub struct BrowserPane {
    pub location: Option<PathBuf>,
    pub path_input: String,
    pub entries: Vec<FileEntry>,
    pub order: Vec<usize>,
    pub options: SortOptions,
    pub error: Option<String>,
}

impl BrowserPane {
    pub fn open(&mut self, path: &Path, sorter: &NameSorter) -> bool {
        let result = (|| {
            // Store a real location so Parent works after input such as child/..
            // instead of returning to the lexical child component.
            let path = fs::canonicalize(path)?;
            if path.to_str().is_none() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "W01 requires a Unicode path",
                ));
            }
            let entries = list_directory(&path)?;
            Ok((path, entries))
        })();
        match result {
            Ok((path, entries)) => {
                self.path_input = path.to_string_lossy().into_owned();
                self.location = Some(path);
                self.entries = entries;
                self.resort(sorter);
                self.error = None;
                true
            }
            Err(error) => {
                self.error = Some(error.to_string());
                false
            }
        }
    }

    pub fn resort(&mut self, sorter: &NameSorter) {
        self.order = sorter.sorted_indices(&self.entries, self.options);
    }

    pub fn parent(&mut self, sorter: &NameSorter) {
        if let Some(parent) = self
            .location
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
        {
            self.open(&parent, sorter);
        }
    }

    pub fn refresh(&mut self, sorter: &NameSorter) {
        if let Some(path) = self.location.clone() {
            self.open(&path, sorter);
        }
    }
}

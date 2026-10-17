// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn nonce() -> String {
    // macOS clock resolution does not guarantee distinct timestamps for
    // simultaneous fixture creation. The process-local counter does.
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

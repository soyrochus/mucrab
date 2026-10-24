// SPDX-License-Identifier: GPL-3.0-only
//! Bounded deterministic fuzz smoke runner; the same entry points accept fuzzer bytes.
use mucrab_core::fuzzing;

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10_000)
        .min(1_000_000);
    let mut seed = 0x1234_5678_9abc_def0_u64;
    for i in 0..iterations {
        let mut input = vec![0; i % 2048];
        for byte in &mut input {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            *byte = seed as u8;
        }
        let _ = fuzzing::sftp_packet(&input);
        let _ = fuzzing::zip_directory(&input);
        let _ = fuzzing::zip_path(&input);
    }
    println!("Exercised {iterations} inputs across three bounded parser boundaries");
}

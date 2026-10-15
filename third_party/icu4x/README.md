# ICU4X collation

W01 uses `icu_collator` **2.2.1** with its compiled `icu_collator_data` **2.2.0**
for native Rust collation. Exact transitive versions and registry checksums are
recorded in `Cargo.lock`; the application does not use the host's ICU library
or a Java subprocess for sorting.

`LICENSE` is copied verbatim from the downloaded `icu_collator-2.2.1` Cargo
package. It contains Unicode License V3 and the ICU adaptation attribution.
These notices are independent of muCrab's GPLv3 license. Dependency source and
complete notice packaging remain required for a distributable release.

Primary API documentation: [Collator](https://docs.rs/icu_collator/2.2.1/icu_collator/struct.Collator.html).
The matching claim is empirical and limited to the declared en-US W01 suite;
it does not assert identical behavior for all Unicode versions or locales.

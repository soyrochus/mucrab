# W12 contract frozen before Rust implementation

Search matches leaf filenames using `*` (zero or more Unicode scalar values),
`?` (exactly one) and backslash escaping the next literal scalar. A trailing escape
is invalid. Case-insensitive literal comparisons use per-scalar Unicode lowercase;
no canonical-equivalence guarantee. Pattern cap256 tokens. Extension matching uses
the existing last-extension rule. Size bounds inclusive, UTC mtime [from,to).
Invalid ranges reject before traversal. Links use lstat metadata, never traverse;
non-file kinds do not match enabled size criteria. Unknown mtime under date filters
is excluded with a diagnostic count. Default kind selection files/folders/links.

Hidden folders are pruned when excluded. Descendant mount roots are skipped.
Depth256, visited1,000,000, hits10,000; hitting a bound is visibly partial. At most
256 detailed errors are retained with a separate total count. Emission batches64
and bounded UI queue8; cancellation checks prevent a full queue blocking close.
Sort hits by root-relative path; no mutations are allowed from the result list.
Replacement waits for acknowledgement; stale generations cannot mix results.
Open Containing Folder targets the tab captured when invoked and revalidates the
entry before navigating or focusing it. Source trees and unrelated marks stay intact.

Exact executable generator: tools/w11_w12_fixtures.py, cases.json and reserved.json
frozen before Rust implementation. Tree fixtures and deterministic failure/cancel
cases are specified by the OpenSpec tasks and must be tested separately. Original
matcher differences must remain visible, not be normalized away. Mac and real
platform/IME checks remain separate gates.

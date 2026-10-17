# W11 contract frozen before Rust implementation

The OpenSpec proposal/design/specs are the intended contract. New Tab uses Home;
Duplicate copies location/options/focus/history independently but clears marks and
quick-find. At least one and at most32 tabs per pane. Stable IDs never substitute
for each other, including same-location tabs. Reviewed paths and origin IDs are
captured before execution. Shared hidden filtering reconciles every tab. Bookmarks
are structured local/ZIP locations, version1, max256, labels1..128 scalar values,
no duplicate ID or identical location+label. Failed/malformed loads are not silently
overwritten. Changes save atomically before becoming persisted state.

Exact executable generator: tools/w11_w12_fixtures.py, with cases.json and
reserved.json generated before implementation. Events freeze state/failure cases;
runtime binary-recovery and adapter boundaries are recorded separately. No full
original-tab UI parity or Mac verification is implied by this contract.

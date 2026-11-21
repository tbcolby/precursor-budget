# Budget Ledger — agent notes

Rebuilt 2026-11-20 on `precursor-appkit` (strategy R from the portfolio
remediation plan). Read `CLAUDE.md` for layout. Invariants with tests in
`core/`:

- Money is `u64`/`i64` minor units. Parsing never wraps; amounts above
  999,999,999.99 are refused. No `f32`/`f64` anywhere.
- Month totals, category totals and month navigation roll over correctly
  (December to January, leap years).
- Record keys are random and allocated only on save; `seq` (ordering) is
  derived from stored records, never from a separate counter.
- Storage effects are confirmed by the app before the UI commits; failed
  saves roll back.
- Every screen exits on Backspace or F4.

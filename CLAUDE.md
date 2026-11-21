# Budget Ledger — Build Notes

## Architecture
- `core/` (`budget-core`, no Xous deps, host-tested): `money` (integer
  cents, parse/format, no floats), `date` (civil-day math), `ledger`
  (transactions, month and category totals, record JSON), `settings`,
  and the screen state machine `Ui` returning `Effect`s.
- `src/main.rs` routes GAM messages (template pattern); `src/app.rs` maps
  keys, performs effects via `appkit::store::Store`, draws with
  `appkit::screen::Screen`. Today's date from `llio::LocalTime`.
- PDDB dict `budget.ledger`: one key per transaction (`t.<16 random hex>`,
  TRNG via `appkit::rng::Trng::try_u32`, fail closed) plus `settings`.
  Delete removes the key. Reload on basis change; nothing is rewritten on
  focus loss.

## Test / gate
```bash
cd core && cargo test
cd ../xous-dev-toolkit && bin/integrate --xous-root <xous-core> --grade -v ../precursor-budget
```

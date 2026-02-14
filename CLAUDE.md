# Budget Ledger — Agent Instructions

## App Identity
- **Package**: `budget`
- **SERVER_NAME**: `_Budget Ledger_`
- **APP_NAME**: `Budget`
- **TCP Port**: 7883

## Architecture
- **State Machine**: 5 states (Summary, TransactionList, AddTransaction, ViewTransaction, ConfirmDelete)
- **PDDB**: 1 dictionary (`budget.data`), 2 keys (transactions as JSON array, config)
- **Currency**: All amounts stored as `u32` cents (no floating point)

## Key Decisions
1. **Cents not floats** — amounts stored as u32 cents to avoid float precision issues on the RISC-V soft float
2. **Newest first** — transactions inserted at index 0, so list shows most recent at top
3. **All-in-one key** — unlike contacts (index + individual), all transactions stored in a single JSON array. Simpler for sequential data.
4. **Category cycling** — C key cycles through categories, no sub-menu needed

## Build
```bash
cargo build -p budget --target riscv32imac-unknown-xous-elf
```

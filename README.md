# Precursor Budget Ledger

**Every dollar accounted for, encrypted at rest.**

```
"Your spending habits are a fingerprint.
Where you shop, how much you spend,
what you prioritize — this data builds
a profile more intimate than your diary.
Keep it on a device you own."
```

---

## What This Is

A personal income and expense tracker for Precursor. Add transactions with amounts, categories, and descriptions. View your balance, income, and expenses at a glance. All data stored in PDDB hardware-encrypted flash.

---

## Features

- **Transaction types**: Income and Expense
- **Categories**: Food, Housing, Transport, Health, Shopping, Bills, Salary, Freelance, Gift, Other
- **Summary dashboard**: Balance, total income, total expenses, recent transactions
- **Transaction list**: Scrollable, newest-first, with type/amount/category display
- **Amount entry**: Dollar.cents format with live parsing
- **PDDB persistence**: All data encrypted at rest

---

## Keyboard Reference

### Summary
| Key | Action |
|-----|--------|
| I | Add income |
| E | Add expense |
| L/Enter | View transactions |
| Q | Quit |

### Transaction List
| Key | Action |
|-----|--------|
| Up/Down | Navigate |
| Enter | View details |
| I | Add income |
| E | Add expense |
| D | Delete |
| Q | Back to summary |

---

## Architecture

```
src/
  main.rs      — Xous event loop
  app.rs       — State machine (Summary, List, Add, View, ConfirmDelete)
  ledger.rs    — Transaction model, categories, summaries, currency formatting
  ui.rs        — All rendering
  storage.rs   — PDDB persistence
```

---

## Build

```bash
cargo build -p budget --target riscv32imac-unknown-xous-elf
```

## License

Apache 2.0 — see LICENSE file.

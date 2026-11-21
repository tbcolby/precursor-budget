# precursor-budget: Budget Ledger

A personal income and expense ledger for the [Precursor](https://precursor.dev). Record transactions with an amount, type, category, date and note; review them month by month with totals and a spending breakdown by category. Data is stored in the PDDB, which encrypts it at rest.

## Features

- **Income and expenses** with categories per type. Expense: Food, Housing, Transport, Health, Shopping, Bills, Other. Income: Salary, Freelance, Gift, Other.
- **Dated transactions.** New entries default to today's date (from the device clock); the date can be moved a day at a time.
- **Monthly summary**: income, expenses and net for the month, spending by category (largest first), and the all-time balance. ← → move between months, rolling over years.
- **Transaction list** for the month, newest first. View, edit in place, or delete.
- **Exact money.** Amounts are integer cents (`u64`), typed as `12`, `12.5` or `12.50`, up to 999,999,999.99. Nothing uses floating point and nothing can overflow; out-of-range input is refused with a message.
- **Currency symbol** setting ($, €, £ or none). Display only; amounts always have two decimal places.

## Keys

| Screen | Key | Action |
|---|---|---|
| Summary | ← → | Previous / next month |
| | T | Jump to the current month |
| | I / E | Add income / expense |
| | Enter or L | Transaction list |
| | F2 or S | Settings |
| | Backspace / F4 | Leave the app |
| List | ↑ ↓ | Move |
| | ← → | Previous / next month |
| | Enter | View the transaction |
| | I / E | Add income / expense |
| | D | Delete (asks Y to confirm) |
| | Backspace / F4 | Back to summary |
| View | E | Edit |
| | D | Delete (asks Y to confirm) |
| | Enter, Backspace / F4 | Back to list |
| Add / edit | ↑ ↓ | Move between fields |
| | ← → or Enter | Type: toggle. Category: cycle. Date: −/+ one day |
| | digits, `.` | Amount |
| | type | Note (up to 30 characters) |
| | F1, or Enter on [Save] | Save |
| | Backspace | Delete a character; on an empty or non-text field, cancel |
| | F4 | Cancel |
| Delete | Y | Delete; any other key cancels |
| Settings | ← → | Change currency symbol |
| | Enter, Backspace / F4 | Done |

The menu key `∴` opens the system menu; the app never sees it.

## Storage

PDDB dictionary `budget.ledger`:

- One key per transaction, `t.<16 random hex>`, holding JSON `{"version":1, "seq", "kind", "amount", "category", "date":{"year","month","day"}, "description"}`. `amount` is in cents. The key name comes from the TRNG; if the TRNG fails, nothing is saved ("TRNG error: not saved"). `seq` orders same-day entries and is derived from the stored records, so there is no separate counter to fall out of step.
- `settings`: `{"version":1, "currency"}`.

Behaviour:

- Each change writes only the affected record (delete, recreate, sync). Nothing is rewritten when the app loses focus.
- New records go to the most recently unlocked basis, shown on the add form ("Saves to basis"). Edits rewrite a record in its own basis; the view screen shows which basis holds it.
- When a basis is locked or unlocked the ledger is reloaded, so transactions from a locked basis leave memory and the screen, and any form in progress is discarded.
- A record that can't be parsed, fails validation, or was written by a newer version is counted as unreadable (shown on screen) and left untouched. Unreadable settings are never overwritten.
- Delete removes the record's PDDB key and syncs. The PDDB does not overwrite old flash pages immediately, so this is not secure erasure.

## Limitations

- If the device clock (time zone) has not been set, there is no "today": the summary opens on the month of the newest transaction (or January 2026) and new entries default to the first of the shown month. Set the time in the Precursor preferences and the app picks it up next time it comes to the front.
- No CSV or other export, no budgets/limits, no recurring transactions, no search.
- Category lists are fixed.
- The 0.1.0 keys (`budget.data` / `transactions`, `config`) are not migrated: that version never compiled, so no device has such data.
- Renode capture not yet run. `capture.py` is written but not executed; verification so far is the hosted build, clippy, lint and the host tests in `core/`.

## Build and test

```bash
cd core && cargo test                                   # host tests
cd xous-dev-toolkit && bin/integrate --xous-root ~/xous-core --grade ../precursor-budget
cd ~/xous-core && cargo xtask run budget                 # hosted mode
```

Layout: `core/` (`budget-core`: money, dates, ledger totals, record formats, screen state machine; no Xous deps), `src/main.rs` (message routing), `src/app.rs` (drawing, PDDB, clock, TRNG on `precursor-appkit`).

---

Made by Tyler Colby, [Colby's Data Movers, LLC](https://colbysdatamovers.com). Issues: [GitHub](https://github.com/tbcolby/precursor-budget/issues).

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).

# Budget Ledger — Agent Evolution Report

## Agents Used
1. **ideation.md** — Feature design, category selection
2. **architecture.md** — State machine, cents-based currency
3. **graphics.md** — Summary dashboard, field editing, list rendering
4. **storage.md** — Single-array PDDB pattern
5. **build.md** — Standard Cargo.toml
6. **review.md** — Standards compliance

## New Patterns
- **Cents-based currency**: Store amounts as u32 cents, format with `format_cents()` for display. Avoids floating-point entirely — important on RISC-V soft-float where float ops are emulated.
- **Amount string parsing**: User types "12.50" as string, parsed to 1250 cents. Handles partial input ("12.", "12.5") gracefully.
- **Summary dashboard**: Aggregate view showing totals + recent items, with quick-add shortcuts.

## Metrics
| Metric | Value |
|--------|-------|
| Source files | 5 |
| Estimated LOC | ~1,550 |
| States | 5 |
| Toolkit agents used | 6 of 12 |

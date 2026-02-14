//! UI rendering for Budget Ledger.

use crate::app::{AddField, AppState, BudgetApp};
use crate::ledger::{self, TxnType};

use gam::*;

const SCREEN_WIDTH: isize = 336;
const HEADER_HEIGHT: isize = 30;
const FOOTER_HEIGHT: isize = 46;
const CONTENT_TOP: isize = HEADER_HEIGHT;
const CONTENT_BOTTOM: isize = 536 - FOOTER_HEIGHT;
const REGULAR_HEIGHT: isize = 15;
const LINE_GAP: isize = 4;
const LINE_HEIGHT: isize = REGULAR_HEIGHT + LINE_GAP;

pub fn draw(app: &BudgetApp, gam: &Gam, canvas: graphics_server::Gid) {
    let clear_rect = graphics_server::Rectangle::new_coords_with_style(
        0, 0, SCREEN_WIDTH, 536,
        graphics_server::DrawStyle::new(
            graphics_server::PixelColor::Light,
            graphics_server::PixelColor::Light, 0,
        ),
    );
    gam.draw_rectangle(canvas, clear_rect).ok();

    match app.state {
        AppState::Summary => draw_summary(app, gam, canvas),
        AppState::TransactionList => draw_list(app, gam, canvas),
        AppState::AddTransaction => draw_add(app, gam, canvas),
        AppState::ViewTransaction => draw_view(app, gam, canvas),
        AppState::ConfirmDelete => draw_confirm(app, gam, canvas),
    }

    gam.redraw().ok();
}

fn draw_header(gam: &Gam, canvas: graphics_server::Gid, title: &str) {
    let bg = graphics_server::Rectangle::new_coords_with_style(
        0, 0, SCREEN_WIDTH, HEADER_HEIGHT,
        graphics_server::DrawStyle::new(
            graphics_server::PixelColor::Dark,
            graphics_server::PixelColor::Dark, 0,
        ),
    );
    gam.draw_rectangle(canvas, bg).ok();
    let mut tv = TextView::new(canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            8, 2, SCREEN_WIDTH - 8, HEADER_HEIGHT - 2,
        )));
    tv.style = GlyphStyle::Bold;
    tv.invert = true;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "{}", title).ok();
    gam.post_textview(&mut tv).ok();
}

fn draw_footer(gam: &Gam, canvas: graphics_server::Gid, text: &str) {
    let y = CONTENT_BOTTOM;
    let sep = graphics_server::Line::new_with_style(
        Point::new(0, y), Point::new(SCREEN_WIDTH, y),
        graphics_server::DrawStyle::new(
            graphics_server::PixelColor::Dark,
            graphics_server::PixelColor::Dark, 1,
        ),
    );
    gam.draw_line(canvas, sep).ok();
    let mut tv = TextView::new(canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            8, y + 4, SCREEN_WIDTH - 8, y + FOOTER_HEIGHT - 4,
        )));
    tv.style = GlyphStyle::Small;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "{}", text).ok();
    gam.post_textview(&mut tv).ok();
}

fn draw_text(gam: &Gam, canvas: graphics_server::Gid,
    x: isize, y: isize, w: isize, text: &str, style: GlyphStyle) {
    let mut tv = TextView::new(canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            x, y, x + w, y + LINE_HEIGHT,
        )));
    tv.style = style;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "{}", text).ok();
    gam.post_textview(&mut tv).ok();
}

// ─── Summary Screen ─────────────────────────────────────────────────────────

fn draw_summary(app: &BudgetApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_header(gam, canvas, "Budget Ledger");

    let summary = app.summary();
    let mut y = CONTENT_TOP + 20;

    // Balance (big)
    let balance_str = ledger::format_cents(summary.balance);
    let mut tv = TextView::new(canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            16, y, SCREEN_WIDTH - 16, y + 30,
        )));
    tv.style = GlyphStyle::Bold;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "Balance: {}", balance_str).ok();
    gam.post_textview(&mut tv).ok();
    y += 40;

    // Separator
    let sep = graphics_server::Line::new_with_style(
        Point::new(16, y), Point::new(SCREEN_WIDTH - 16, y),
        graphics_server::DrawStyle::new(
            graphics_server::PixelColor::Dark,
            graphics_server::PixelColor::Dark, 1,
        ),
    );
    gam.draw_line(canvas, sep).ok();
    y += 12;

    // Income
    let income_str = ledger::format_cents(summary.total_income);
    draw_text(gam, canvas, 16, y, 140, "Income:", GlyphStyle::Regular);
    draw_text(gam, canvas, 160, y, 160, &income_str, GlyphStyle::Bold);
    y += LINE_HEIGHT + 6;

    // Expenses
    let expense_str = ledger::format_cents(-summary.total_expense);
    draw_text(gam, canvas, 16, y, 140, "Expenses:", GlyphStyle::Regular);
    draw_text(gam, canvas, 160, y, 160, &expense_str, GlyphStyle::Bold);
    y += LINE_HEIGHT + 6;

    // Count
    let count_str = alloc::format!("{} transactions", summary.count);
    draw_text(gam, canvas, 16, y, 300, &count_str, GlyphStyle::Small);

    // Recent transactions preview
    y += LINE_HEIGHT + 20;
    if !app.transactions.is_empty() {
        draw_text(gam, canvas, 16, y, 300, "Recent:", GlyphStyle::Small);
        y += LINE_HEIGHT + 2;

        let show = app.transactions.len().min(5);
        for i in 0..show {
            let txn = &app.transactions[i];
            draw_text(gam, canvas, 20, y, 300, &txn.list_line(), GlyphStyle::Small);
            y += LINE_HEIGHT;
        }
    }

    draw_footer(gam, canvas, "I:Income E:Expense L:List Q:Quit");
}

// ─── Transaction List ───────────────────────────────────────────────────────

fn draw_list(app: &BudgetApp, gam: &Gam, canvas: graphics_server::Gid) {
    let title = alloc::format!("Transactions ({})", app.transactions.len());
    draw_header(gam, canvas, &title);

    let mut y = CONTENT_TOP + 4;

    if app.transactions.is_empty() {
        draw_text(gam, canvas, 16, y + 20, 300,
            "No transactions yet.", GlyphStyle::Regular);
        draw_text(gam, canvas, 16, y + 50, 300,
            "Press I for income, E for expense.", GlyphStyle::Bold);
    } else {
        let end = (app.list_scroll + 18).min(app.transactions.len());
        for i in app.list_scroll..end {
            let txn = &app.transactions[i];
            let selected = i == app.list_cursor;

            if selected {
                let hl = graphics_server::Rectangle::new_coords_with_style(
                    4, y - 1, SCREEN_WIDTH - 4, y + LINE_HEIGHT + 1,
                    graphics_server::DrawStyle::new(
                        graphics_server::PixelColor::Dark,
                        graphics_server::PixelColor::Dark, 0,
                    ),
                );
                gam.draw_rectangle(canvas, hl).ok();
            }

            let mut tv = TextView::new(canvas,
                TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                    8, y, SCREEN_WIDTH - 8, y + LINE_HEIGHT,
                )));
            tv.style = GlyphStyle::Small;
            tv.invert = selected;
            tv.draw_border = false;
            tv.margin = Point::new(0, 0);
            write!(tv, "{}", txn.list_line()).ok();
            gam.post_textview(&mut tv).ok();

            y += LINE_HEIGHT + 2;
        }
    }

    draw_footer(gam, canvas, "Enter:View I:Income E:Expense D:Del Q:Back");
}

// ─── Add Transaction ────────────────────────────────────────────────────────

fn draw_add(app: &BudgetApp, gam: &Gam, canvas: graphics_server::Gid) {
    let txn = match &app.edit_txn {
        Some(t) => t,
        None => return,
    };

    let title = alloc::format!("Add {}", txn.txn_type.label());
    draw_header(gam, canvas, &title);

    let fields = AddField::all();
    let mut y = CONTENT_TOP + 10;

    for (i, field) in fields.iter().enumerate() {
        let selected = i == app.edit_field_cursor;
        let is_editing = selected && app.editing_field;
        let marker = if selected { ">" } else { " " };

        match field {
            AddField::Save => {
                y += 8;
                if selected {
                    let hl = graphics_server::Rectangle::new_coords_with_style(
                        16, y, 120, y + LINE_HEIGHT + 4,
                        graphics_server::DrawStyle::new(
                            graphics_server::PixelColor::Dark,
                            graphics_server::PixelColor::Dark, 0,
                        ),
                    );
                    gam.draw_rectangle(canvas, hl).ok();
                }
                let mut tv = TextView::new(canvas,
                    TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
                        20, y + 2, 116, y + LINE_HEIGHT + 2,
                    )));
                tv.style = GlyphStyle::Bold;
                tv.invert = selected;
                tv.draw_border = false;
                tv.margin = Point::new(0, 0);
                write!(tv, "  SAVE").ok();
                gam.post_textview(&mut tv).ok();
            }
            AddField::Type => {
                let label = alloc::format!("{} {}: {} (Enter to toggle)",
                    marker, field.label(), txn.txn_type.label());
                draw_text(gam, canvas, 12, y, 310, &label,
                    if selected { GlyphStyle::Bold } else { GlyphStyle::Regular });
                y += LINE_HEIGHT + 6;
            }
            AddField::Category => {
                let label = alloc::format!("{} {}: {} (C to cycle)",
                    marker, field.label(), txn.category.label());
                draw_text(gam, canvas, 12, y, 310, &label,
                    if selected { GlyphStyle::Bold } else { GlyphStyle::Regular });
                y += LINE_HEIGHT + 6;
            }
            AddField::Amount => {
                let label = alloc::format!("{} {}:", marker, field.label());
                draw_text(gam, canvas, 12, y, 310, &label,
                    if selected { GlyphStyle::Bold } else { GlyphStyle::Small });
                y += LINE_HEIGHT;

                let box_style = if is_editing {
                    graphics_server::DrawStyle::new(
                        graphics_server::PixelColor::Dark,
                        graphics_server::PixelColor::Light, 2,
                    )
                } else {
                    graphics_server::DrawStyle::new(
                        graphics_server::PixelColor::Dark,
                        graphics_server::PixelColor::Light, 1,
                    )
                };
                let rect = graphics_server::Rectangle::new_coords_with_style(
                    20, y, 200, y + LINE_HEIGHT + 4, box_style);
                gam.draw_rectangle(canvas, rect).ok();

                let display = if is_editing {
                    alloc::format!("${}|", app.amount_buffer)
                } else {
                    txn.amount_display()
                };
                draw_text(gam, canvas, 24, y + 2, 170, &display, GlyphStyle::Regular);
                y += LINE_HEIGHT + 10;
            }
            AddField::Description => {
                let label = alloc::format!("{} {}:", marker, field.label());
                draw_text(gam, canvas, 12, y, 310, &label,
                    if selected { GlyphStyle::Bold } else { GlyphStyle::Small });
                y += LINE_HEIGHT;

                let box_style = if is_editing {
                    graphics_server::DrawStyle::new(
                        graphics_server::PixelColor::Dark,
                        graphics_server::PixelColor::Light, 2,
                    )
                } else {
                    graphics_server::DrawStyle::new(
                        graphics_server::PixelColor::Dark,
                        graphics_server::PixelColor::Light, 1,
                    )
                };
                let rect = graphics_server::Rectangle::new_coords_with_style(
                    20, y, SCREEN_WIDTH - 16, y + LINE_HEIGHT + 4, box_style);
                gam.draw_rectangle(canvas, rect).ok();

                let display = if is_editing {
                    alloc::format!("{}|", txn.description)
                } else {
                    txn.description.clone()
                };
                draw_text(gam, canvas, 24, y + 2, 284, &display, GlyphStyle::Regular);
                y += LINE_HEIGHT + 10;
            }
        }
    }

    let hint = if app.editing_field {
        "Type value  Enter:Done  Bksp:Delete"
    } else {
        "Arrows:Navigate  Enter:Edit/Save  C:Category  Q:Cancel"
    };
    draw_footer(gam, canvas, hint);
}

// ─── View Transaction ───────────────────────────────────────────────────────

fn draw_view(app: &BudgetApp, gam: &Gam, canvas: graphics_server::Gid) {
    let txn = match app.transactions.get(app.viewed_index) {
        Some(t) => t,
        None => {
            draw_header(gam, canvas, "Transaction");
            draw_text(gam, canvas, 16, CONTENT_TOP + 20, 300,
                "Not found.", GlyphStyle::Regular);
            draw_footer(gam, canvas, "Any key:Back");
            return;
        }
    };

    draw_header(gam, canvas, txn.txn_type.label());

    let mut y = CONTENT_TOP + 20;

    // Amount (big)
    let amount = alloc::format!("{}{}", txn.txn_type.symbol(), txn.amount_display());
    let mut tv = TextView::new(canvas,
        TextBounds::BoundingBox(graphics_server::Rectangle::new_coords(
            16, y, SCREEN_WIDTH - 16, y + 30,
        )));
    tv.style = GlyphStyle::Bold;
    tv.draw_border = false;
    tv.margin = Point::new(0, 0);
    write!(tv, "{}", amount).ok();
    gam.post_textview(&mut tv).ok();
    y += 36;

    let cat_line = alloc::format!("Category: {}", txn.category.label());
    draw_text(gam, canvas, 16, y, 300, &cat_line, GlyphStyle::Regular);
    y += LINE_HEIGHT + 6;

    if !txn.description.is_empty() {
        draw_text(gam, canvas, 16, y, 300, "Description:", GlyphStyle::Small);
        y += LINE_HEIGHT;
        draw_text(gam, canvas, 24, y, 296, &txn.description, GlyphStyle::Regular);
    }

    draw_footer(gam, canvas, "D:Delete  Any other key:Back");
}

// ─── Confirm Delete ─────────────────────────────────────────────────────────

fn draw_confirm(app: &BudgetApp, gam: &Gam, canvas: graphics_server::Gid) {
    draw_header(gam, canvas, "Delete Transaction?");

    let desc = app.transactions.get(app.viewed_index)
        .map(|t| t.list_line())
        .unwrap_or_else(|| alloc::string::String::from("Unknown"));

    let mut y = CONTENT_TOP + 60;
    draw_text(gam, canvas, 16, y, 300, "Delete this transaction?", GlyphStyle::Regular);
    y += LINE_HEIGHT + 12;
    draw_text(gam, canvas, 24, y, 290, &desc, GlyphStyle::Bold);
    y += 40;
    draw_text(gam, canvas, 16, y, 300, "This cannot be undone.", GlyphStyle::Small);

    draw_footer(gam, canvas, "Y:Delete  Any other key:Cancel");
}

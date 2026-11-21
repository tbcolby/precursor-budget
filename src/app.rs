//! Drawing, storage and clock plumbing for Budget Ledger. All decisions are
//! made by `budget_core::Ui`; this file maps keys to it, carries out its
//! effects against the PDDB, and draws its state.

use budget_core::ledger::{category_totals, is_txn_key, month_totals, totals};
use budget_core::money::format_amount;
use budget_core::settings::SETTINGS_KEY;
use budget_core::ui::FIELDS;
use budget_core::{Date, Effect, Entropy, EntropyError, Entry, Field, Input, Kind, Screen, Settings, Transaction, Ui};
use gam::menu::Point;
use gam::GlyphStyle;
use precursor_appkit::keys::Key;
use precursor_appkit::layout::ListState;
use precursor_appkit::rng::Trng;
use precursor_appkit::screen::{Screen as Canvas, MARGIN};
use precursor_appkit::store::Store;

/// PDDB dictionary: one key per transaction (`t.<16 hex>`) plus `settings`.
pub const DICT: &str = "budget.ledger";

#[derive(PartialEq, Eq)]
pub enum Action {
    Stay,
    Leave,
}

/// The hardware TRNG as a fallible entropy source (names new records).
struct TrngSource<'a>(&'a Trng);

impl Entropy for TrngSource<'_> {
    fn try_u32(&mut self) -> Result<u32, EntropyError> { self.0.try_u32().map_err(|_| EntropyError) }
}

fn to_input(key: Key) -> Input {
    match key {
        Key::Up => Input::Up,
        Key::Down => Input::Down,
        Key::Left => Input::Left,
        Key::Right => Input::Right,
        Key::Enter => Input::Enter,
        Key::Backspace => Input::Backspace,
        Key::F(n) => Input::F(n),
        Key::Char(c) => Input::Char(c),
    }
}

pub struct App {
    canvas: Canvas,
    store: Store,
    trng: Trng,
    clock: llio::LocalTime,
    ui: Ui,
    list: ListState,
    visible_rows: usize,
}

impl App {
    pub fn new(xns: &xous_names::XousNames, canvas: Canvas, store: Store) -> Self {
        let trng = Trng::new(xns).expect("can't connect to TRNG");
        let mut clock = llio::LocalTime::new();
        let today = clock.get_local_time_ms().map(Date::from_epoch_ms);
        let mut app =
            Self { canvas, store, trng, clock, ui: Ui::new(today), list: ListState::default(), visible_rows: 1 };
        app.reload();
        app
    }

    pub fn screen(&self) -> &Canvas { &self.canvas }

    /// (Re)load every record. Runs at start and whenever a basis is locked or
    /// unlocked, so data from a locked basis leaves memory and the screen.
    pub fn reload(&mut self) {
        let mut entries = Vec::new();
        let mut unreadable = 0;
        match self.store.keys() {
            Ok(names) => {
                for name in names.into_iter().filter(|n| is_txn_key(n)) {
                    match self.store.read_bytes(&name) {
                        Ok(Some(bytes)) => match Transaction::from_json(&bytes) {
                            Ok(txn) => entries.push(Entry { key: name, txn }),
                            Err(e) => {
                                log::warn!("record {} unreadable: {:?}", name, e);
                                unreadable += 1;
                            }
                        },
                        Ok(None) => {}
                        Err(e) => {
                            log::warn!("couldn't read {}: {:?}", name, e.kind());
                            unreadable += 1;
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("couldn't list records: {:?}", e.kind());
                self.ui.status = "Couldn't read the ledger".into();
            }
        }
        self.ui.set_entries(entries, unreadable);
        let settings = match self.store.read_bytes(SETTINGS_KEY) {
            Ok(None) => Ok(None),
            Ok(Some(b)) => Settings::from_json(&b).map(Some).map_err(|e| log::warn!("settings unreadable: {:?}", e)),
            Err(e) => {
                log::warn!("couldn't read settings: {:?}", e.kind());
                Err(())
            }
        };
        self.ui.set_settings(settings);
    }

    pub fn on_key(&mut self, key: Key) -> Action {
        match self.ui.on_key(to_input(key), &mut TrngSource(&self.trng)) {
            Effect::None => {}
            Effect::Leave => return Action::Leave,
            Effect::Persist(k) => {
                let result = match self.ui.entry(&k) {
                    Some(e) => self.store.write_bytes(&e.key, &e.txn.to_json()),
                    None => Err(std::io::Error::new(std::io::ErrorKind::NotFound, "record gone")),
                };
                match result {
                    Ok(()) => self.ui.on_persisted(Ok(())),
                    Err(e) => {
                        log::error!("persist failed: {:?}", e.kind());
                        self.ui.on_persisted(Err(&e.kind().to_string()));
                    }
                }
            }
            Effect::Delete(k) => match self.store.delete(&k) {
                Ok(()) => self.ui.on_deleted(&k),
                Err(e) => {
                    log::error!("delete failed: {:?}", e.kind());
                    self.ui.on_delete_failed(&e.kind().to_string());
                }
            },
            Effect::SaveSettings => match self.store.write_bytes(SETTINGS_KEY, &self.ui.settings.to_json()) {
                Ok(()) => self.ui.on_settings_saved(Ok(())),
                Err(e) => {
                    log::error!("settings save failed: {:?}", e.kind());
                    self.ui.on_settings_saved(Err(&e.kind().to_string()));
                }
            },
        }
        Action::Stay
    }

    /// Re-read the clock (it may have been set, or the date changed) and redraw.
    pub fn on_foreground(&mut self) {
        let today = self.clock.get_local_time_ms().map(Date::from_epoch_ms);
        self.ui.set_today(today);
        self.canvas.refresh_size();
        self.draw();
    }

    fn money(&self, minor: i64) -> String { format_amount(minor, self.ui.currency()) }

    fn footer_and_status(&self, hint: &str) -> isize {
        let mut bottom = self.canvas.footer(hint);
        let h = self.canvas.line_height(GlyphStyle::Small);
        if !self.ui.status.is_empty() {
            self.canvas.text(bottom - h, GlyphStyle::Small, &self.ui.status);
            bottom -= h;
        }
        if self.ui.unreadable > 0 {
            let msg = format!("{} unreadable record(s) left untouched", self.ui.unreadable);
            self.canvas.text(bottom - h, GlyphStyle::Small, &msg);
            bottom -= h;
        }
        bottom
    }

    pub fn draw(&mut self) {
        self.canvas.clear();
        match self.ui.screen {
            Screen::Summary => self.draw_summary(),
            Screen::List => self.draw_list(),
            Screen::View => self.draw_view(),
            Screen::Form => self.draw_form(),
            Screen::ConfirmDelete => self.draw_confirm(),
            Screen::Settings => self.draw_settings(),
        }
        self.canvas.flush();
    }

    fn draw_summary(&self) {
        let month = self.ui.month;
        let mut y = self.canvas.header(&format!("Budget · {month}"));
        let bottom = self.footer_and_status("←→ month · I/E add · ⏎ list · F2 set · ⌫ quit");
        let txns = self.ui.entries.iter().map(|e| &e.txn);
        let t = month_totals(txns.clone(), month);
        for (label, v) in [("Income", t.income), ("Expenses", t.expense), ("Net", t.net())] {
            y += self.canvas.text(y, GlyphStyle::Regular, &format!("{label:<10}{}", self.money(v)));
        }
        y += self.canvas.text(y, GlyphStyle::Small, &format!("{} transaction(s) this month", t.count));
        y += 6;
        let cats = category_totals(txns.clone(), month, Kind::Expense);
        if !cats.is_empty() {
            y += self.canvas.text(y, GlyphStyle::Bold, "Spending by category");
            let h = self.canvas.line_height(GlyphStyle::Regular);
            let all_h = self.canvas.line_height(GlyphStyle::Small) + 6;
            for (cat, sum) in cats {
                if y + h > bottom - all_h {
                    break;
                }
                y += self.canvas.text(y, GlyphStyle::Regular, &format!("  {:<10}{}", cat.label(), self.money(sum)));
            }
        }
        let all = totals(txns);
        let line = format!("All-time balance {}", self.money(all.net()));
        self.canvas.text(bottom - self.canvas.line_height(GlyphStyle::Small), GlyphStyle::Small, &line);
    }

    fn draw_list(&mut self) {
        let idx = self.ui.month_entries();
        let top = self.canvas.header(&format!("{} ({})", self.ui.month, idx.len()));
        let bottom = self.footer_and_status("↑↓ · ←→ month · ⏎ view · I/E add · D del · ⌫");
        if idx.is_empty() {
            let w = self.canvas.size().x;
            self.canvas.text_in(
                Point::new(MARGIN, top + 10),
                Point::new(w - MARGIN, bottom),
                GlyphStyle::Regular,
                "No transactions this month. I adds income, E adds an expense.",
            );
            return;
        }
        let labels: Vec<String> = idx
            .iter()
            .map(|&i| {
                let t = &self.ui.entries[i].txn;
                format!(
                    "{:02} {}{} {} {}",
                    t.date.day,
                    t.kind.sign(),
                    self.money(t.amount as i64),
                    t.category.label(),
                    t.description
                )
            })
            .collect();
        self.list.selected = self.ui.cursor;
        self.list.clamp(labels.len(), self.visible_rows);
        self.visible_rows = self.canvas.list(top, bottom, &labels, &self.list).max(1);
    }

    fn draw_view(&self) {
        let mut y = self.canvas.header("Transaction");
        self.footer_and_status("E edit · D delete · ⌫ back");
        let Some(e) = self.ui.selected() else { return };
        let t = &e.txn;
        let rows = [
            ("Type", t.kind.label().to_string()),
            ("Amount", self.money(t.amount as i64)),
            ("Category", t.category.label().to_string()),
            ("Date", t.date.to_string()),
            ("Note", if t.description.is_empty() { "-".into() } else { t.description.clone() }),
        ];
        for (label, value) in rows {
            y += self.canvas.text(y, GlyphStyle::Regular, &format!("{label:<10}{value}"));
        }
        if let Ok(Some(basis)) = self.store.basis_of(&e.key) {
            self.canvas.text(y + 6, GlyphStyle::Small, &format!("Stored in basis: {basis}"));
        }
    }

    fn draw_form(&self) {
        let f = &self.ui.form;
        let title = match (&f.editing, f.kind) {
            (Some(_), _) => "Edit transaction",
            (None, Kind::Income) => "New income",
            (None, Kind::Expense) => "New expense",
        };
        let mut y = self.canvas.header(title);
        self.footer_and_status("↑↓ field · ←→ change · F1 save · F4 cancel");
        for field in FIELDS {
            let value = match field {
                Field::Kind => f.kind.label().to_string(),
                Field::Amount => format!("{}{}", self.ui.currency(), f.amount),
                Field::Category => f.category.label().to_string(),
                Field::Date => f.date.to_string(),
                Field::Description => f.description.clone(),
                Field::Save => String::new(),
            };
            let here = f.field == field;
            let marker = if here { "▸" } else { " " };
            let cursor = if here && matches!(field, Field::Amount | Field::Description) { "_" } else { "" };
            let line = if field == Field::Save {
                format!("{marker} {}", field.label())
            } else {
                format!("{marker} {:<9}{value}{cursor}", field.label())
            };
            y += self.canvas.text(y, GlyphStyle::Regular, &line);
        }
        y += 8;
        let basis = match &f.editing {
            Some(k) => self.store.basis_of(k).ok().flatten(),
            None => self.store.target_basis(),
        };
        let basis = basis.unwrap_or_else(|| "?".into());
        self.canvas.text(y, GlyphStyle::Small, &format!("Saves to basis: {basis}"));
    }

    fn draw_confirm(&self) {
        let mut y = self.canvas.header("Delete transaction?");
        self.footer_and_status("Y delete · any other key cancels");
        let Some(e) = self.ui.selected() else { return };
        let t = &e.txn;
        let line = format!("{} {}{} {}", t.date, t.kind.sign(), self.money(t.amount as i64), t.category.label());
        y += self.canvas.text(y, GlyphStyle::Regular, &line);
        self.canvas.text(y + 6, GlyphStyle::Small, "Removes the transaction's PDDB record.");
    }

    fn draw_settings(&self) {
        let mut y = self.canvas.header("Settings");
        self.footer_and_status("←→ change · ⏎/⌫ done");
        let line = format!("▸ Currency  ‹ {} ›", self.ui.settings.currency_label());
        y += self.canvas.text(y, GlyphStyle::Regular, &line);
        self.canvas.text(y + 6, GlyphStyle::Small, "Display only. Amounts always have two decimal places.");
    }
}

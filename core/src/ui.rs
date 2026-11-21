//! Screen state machine. Pure: the app crate maps device keys to [`Input`],
//! performs the returned [`Effect`] against the PDDB, reports the outcome
//! with `on_persisted` / `on_deleted` / `on_delete_failed` /
//! `on_settings_saved`, and draws the public state.
//!
//! Every screen leaves on Backspace or F4 (the menu key `∴` never reaches
//! apps). On text fields Backspace deletes a character and cancels only once
//! the field is empty; F4 always cancels.

use crate::date::{Date, YearMonth};
use crate::entropy::Entropy;
use crate::ledger::{newest_first, new_key, Category, Kind, Transaction, MAX_DESC, RECORD_VERSION};
use crate::money::{amount_input_ok, amount_to_input, parse_amount};
use crate::settings::Settings;

/// Month shown when the clock isn't set and there is no data.
pub const FALLBACK_MONTH: YearMonth = YearMonth { year: 2026, month: 1 };

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Backspace,
    F(u8),
    Char(char),
}

impl Input {
    fn is_back(self) -> bool { matches!(self, Input::Backspace | Input::F(4)) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Summary,
    List,
    View,
    Form,
    ConfirmDelete,
    Settings,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    Leave,
    /// Write the entry with this key (`Ui::entry`), then call `on_persisted`.
    Persist(String),
    /// Delete this PDDB key, then call `on_deleted` / `on_delete_failed`.
    Delete(String),
    /// Write `Ui::settings`, then call `on_settings_saved`.
    SaveSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: String,
    pub txn: Transaction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Kind,
    Amount,
    Category,
    Date,
    Description,
    Save,
}

pub const FIELDS: [Field; 6] = [Field::Kind, Field::Amount, Field::Category, Field::Date, Field::Description, Field::Save];

impl Field {
    pub fn label(self) -> &'static str {
        match self {
            Field::Kind => "Type",
            Field::Amount => "Amount",
            Field::Category => "Category",
            Field::Date => "Date",
            Field::Description => "Note",
            Field::Save => "[ Save ]",
        }
    }

    fn index(self) -> usize { FIELDS.iter().position(|&f| f == self).unwrap_or(0) }
}

/// The add / edit form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    /// Key of the entry being edited; `None` for a new transaction.
    pub editing: Option<String>,
    pub kind: Kind,
    pub amount: String,
    pub category: Category,
    pub date: Date,
    pub description: String,
    pub field: Field,
}

impl Form {
    fn new(kind: Kind, date: Date) -> Self {
        Form {
            editing: None,
            kind,
            amount: String::new(),
            category: kind.default_category(),
            date,
            description: String::new(),
            field: Field::Amount,
        }
    }

    fn from_entry(e: &Entry) -> Self {
        Form {
            editing: Some(e.key.clone()),
            kind: e.txn.kind,
            amount: amount_to_input(e.txn.amount),
            category: e.txn.category,
            date: e.txn.date,
            description: e.txn.description.clone(),
            field: Field::Amount,
        }
    }
}

enum Pending {
    New(String),
    Edit(Entry),
    Settings(Settings),
}

pub struct Ui {
    pub screen: Screen,
    /// All loaded transactions, newest first.
    pub entries: Vec<Entry>,
    /// Month shown on the summary and list screens.
    pub month: YearMonth,
    /// Today's local date, if the device clock is set.
    pub today: Option<Date>,
    /// Selection within [`Ui::month_entries`].
    pub cursor: usize,
    pub form: Form,
    pub status: String,
    /// Stored records that couldn't be read (left untouched).
    pub unreadable: usize,
    pub settings: Settings,
    /// Settings exist but couldn't be read; never overwrite them.
    pub settings_locked: bool,
    /// Key shown by View / ConfirmDelete.
    selected: Option<String>,
    form_from: Screen,
    confirm_from: Screen,
    settings_before: Option<Settings>,
    pending: Option<Pending>,
}

impl Ui {
    pub fn new(today: Option<Date>) -> Self {
        let month = today.map(|d| d.year_month()).unwrap_or(FALLBACK_MONTH);
        Ui {
            screen: Screen::Summary,
            entries: Vec::new(),
            month,
            today,
            cursor: 0,
            form: Form::new(Kind::Expense, month.first_day()),
            status: String::new(),
            unreadable: 0,
            settings: Settings::default(),
            settings_locked: false,
            selected: None,
            form_from: Screen::Summary,
            confirm_from: Screen::List,
            settings_before: None,
            pending: None,
        }
    }

    /// Update today's date (call on start and on foreground). If the clock
    /// just became available, jump to the current month.
    pub fn set_today(&mut self, today: Option<Date>) {
        if self.today.is_none() {
            if let Some(d) = today {
                self.month = d.year_month();
                self.cursor = 0;
            }
        }
        self.today = today;
    }

    /// Replace all transactions (start, or after a basis change). Anything
    /// in progress is dropped so nothing from a locked basis lingers.
    pub fn set_entries(&mut self, mut entries: Vec<Entry>, unreadable: usize) {
        entries.sort_by(|a, b| newest_first(&a.txn, &b.txn));
        self.entries = entries;
        self.unreadable = unreadable;
        self.pending = None;
        self.selected = None;
        if self.today.is_none() && self.month == FALLBACK_MONTH {
            if let Some(e) = self.entries.first() {
                self.month = e.txn.date.year_month();
            }
        }
        if !matches!(self.screen, Screen::Summary | Screen::List | Screen::Settings) {
            self.form = Form::new(Kind::Expense, self.month.first_day());
            self.screen = Screen::Summary;
        }
        self.clamp_cursor();
    }

    /// Settings as loaded: `Ok(None)` = none saved, `Err` = unreadable (kept
    /// read-only so the stored copy is never replaced).
    pub fn set_settings(&mut self, loaded: Result<Option<Settings>, ()>) {
        self.settings_before = None;
        match loaded {
            Ok(s) => {
                self.settings = s.unwrap_or_default();
                self.settings_locked = false;
            }
            Err(()) => {
                self.settings = Settings::default();
                self.settings_locked = true;
            }
        }
    }

    pub fn currency(&self) -> &str { &self.settings.currency }

    /// Indices into `entries` for the shown month, newest first.
    pub fn month_entries(&self) -> Vec<usize> {
        (0..self.entries.len()).filter(|&i| self.month.contains(self.entries[i].txn.date)).collect()
    }

    pub fn entry(&self, key: &str) -> Option<&Entry> { self.entries.iter().find(|e| e.key == key) }

    /// The entry under the list cursor.
    pub fn cursor_entry(&self) -> Option<&Entry> { self.month_entries().get(self.cursor).map(|&i| &self.entries[i]) }

    /// The entry shown by View / ConfirmDelete.
    pub fn selected(&self) -> Option<&Entry> { self.selected.as_deref().and_then(|k| self.entry(k)) }

    fn clamp_cursor(&mut self) { self.cursor = self.cursor.min(self.month_entries().len().saturating_sub(1)); }

    fn set_month(&mut self, m: YearMonth) {
        self.month = m;
        self.cursor = 0;
    }

    fn default_date(&self) -> Date {
        match self.today {
            Some(d) if self.month.contains(d) => d,
            _ => self.month.first_day(),
        }
    }

    fn start_add(&mut self, kind: Kind) {
        self.form = Form::new(kind, self.default_date());
        self.form_from = self.screen;
        self.screen = Screen::Form;
        self.status.clear();
    }

    pub fn on_key<E: Entropy + ?Sized>(&mut self, input: Input, entropy: &mut E) -> Effect {
        if self.pending.is_some() {
            return Effect::None; // waiting for the app to report a storage result
        }
        if !matches!(input, Input::Char(_)) || self.screen != Screen::Form {
            self.status.clear();
        }
        match self.screen {
            Screen::Summary => self.key_summary(input),
            Screen::List => self.key_list(input),
            Screen::View => self.key_view(input),
            Screen::Form => self.key_form(input, entropy),
            Screen::ConfirmDelete => self.key_confirm(input),
            Screen::Settings => self.key_settings(input),
        }
    }

    fn key_summary(&mut self, input: Input) -> Effect {
        match input {
            i if i.is_back() => return Effect::Leave,
            Input::Left => self.set_month(self.month.prev()),
            Input::Right => self.set_month(self.month.next()),
            Input::Enter | Input::Char('l') | Input::Char('L') => {
                self.clamp_cursor();
                self.screen = Screen::List;
            }
            Input::Char('i') | Input::Char('I') => self.start_add(Kind::Income),
            Input::Char('e') | Input::Char('E') => self.start_add(Kind::Expense),
            Input::Char('t') | Input::Char('T') => match self.today {
                Some(d) => self.set_month(d.year_month()),
                None => self.status = "Clock not set".into(),
            },
            Input::F(2) | Input::Char('s') | Input::Char('S') => {
                self.settings_before = Some(self.settings.clone());
                self.screen = Screen::Settings;
            }
            _ => {}
        }
        Effect::None
    }

    fn key_list(&mut self, input: Input) -> Effect {
        let n = self.month_entries().len();
        match input {
            i if i.is_back() => self.screen = Screen::Summary,
            Input::Up => self.cursor = self.cursor.saturating_sub(1),
            Input::Down => {
                if self.cursor + 1 < n {
                    self.cursor += 1;
                }
            }
            Input::Left => self.set_month(self.month.prev()),
            Input::Right => self.set_month(self.month.next()),
            Input::Enter => {
                if let Some(k) = self.cursor_entry().map(|e| e.key.clone()) {
                    self.selected = Some(k);
                    self.screen = Screen::View;
                }
            }
            Input::Char('i') | Input::Char('I') => self.start_add(Kind::Income),
            Input::Char('e') | Input::Char('E') => self.start_add(Kind::Expense),
            Input::Char('d') | Input::Char('D') => {
                if let Some(k) = self.cursor_entry().map(|e| e.key.clone()) {
                    self.selected = Some(k);
                    self.confirm_from = Screen::List;
                    self.screen = Screen::ConfirmDelete;
                }
            }
            _ => {}
        }
        Effect::None
    }

    fn key_view(&mut self, input: Input) -> Effect {
        match input {
            i if i.is_back() => self.screen = Screen::List,
            Input::Enter => self.screen = Screen::List,
            Input::Char('e') | Input::Char('E') => {
                if let Some(e) = self.selected() {
                    self.form = Form::from_entry(e);
                    self.form_from = Screen::View;
                    self.screen = Screen::Form;
                }
            }
            Input::Char('d') | Input::Char('D') => {
                self.confirm_from = Screen::View;
                self.screen = Screen::ConfirmDelete;
            }
            _ => {}
        }
        Effect::None
    }

    fn key_confirm(&mut self, input: Input) -> Effect {
        match input {
            Input::Char('y') | Input::Char('Y') => match self.selected.clone() {
                Some(k) if self.entry(&k).is_some() => Effect::Delete(k),
                _ => {
                    self.screen = Screen::List;
                    Effect::None
                }
            },
            _ => {
                self.screen = self.confirm_from;
                Effect::None
            }
        }
    }

    fn key_settings(&mut self, input: Input) -> Effect {
        match input {
            Input::Left | Input::Right => {
                if self.settings_locked {
                    self.status = "Saved settings unreadable; left unchanged".into();
                } else {
                    self.settings.cycle_currency(if input == Input::Left { -1 } else { 1 });
                }
                Effect::None
            }
            i if i.is_back() || i == Input::Enter => {
                self.screen = Screen::Summary;
                match self.settings_before.take() {
                    Some(old) if old != self.settings && !self.settings_locked => {
                        self.pending = Some(Pending::Settings(old));
                        Effect::SaveSettings
                    }
                    _ => Effect::None,
                }
            }
            _ => Effect::None,
        }
    }

    fn cancel_form(&mut self) {
        self.screen = if self.form.editing.is_some() { Screen::View } else { self.form_from };
    }

    fn key_form<E: Entropy + ?Sized>(&mut self, input: Input, entropy: &mut E) -> Effect {
        let f = &mut self.form;
        match input {
            Input::F(4) => self.cancel_form(),
            Input::F(1) => return self.save_form(entropy),
            Input::Up => f.field = FIELDS[f.field.index().saturating_sub(1)],
            Input::Down => f.field = FIELDS[(f.field.index() + 1).min(FIELDS.len() - 1)],
            Input::Backspace => match f.field {
                Field::Amount if !f.amount.is_empty() => {
                    f.amount.pop();
                }
                Field::Description if !f.description.is_empty() => {
                    f.description.pop();
                }
                _ => self.cancel_form(),
            },
            Input::Left | Input::Right => {
                let d: isize = if input == Input::Left { -1 } else { 1 };
                match f.field {
                    Field::Kind => toggle_kind(f),
                    Field::Category => f.category = f.category.cycle(f.kind, d),
                    Field::Date => f.date = f.date.add_days(d as i64),
                    _ => {}
                }
            }
            Input::Enter => match f.field {
                Field::Kind => toggle_kind(f),
                Field::Category => f.category = f.category.cycle(f.kind, 1),
                Field::Save => return self.save_form(entropy),
                _ => f.field = FIELDS[f.field.index() + 1],
            },
            Input::Char(c) => match f.field {
                Field::Amount => {
                    let mut candidate = f.amount.clone();
                    candidate.push(c);
                    if amount_input_ok(&candidate) {
                        f.amount = candidate;
                    }
                }
                Field::Description => {
                    if !c.is_control() && f.description.chars().count() < MAX_DESC {
                        f.description.push(c);
                    }
                }
                _ => {}
            },
            Input::F(_) => {}
        }
        Effect::None
    }

    fn save_form<E: Entropy + ?Sized>(&mut self, entropy: &mut E) -> Effect {
        let amount = match parse_amount(&self.form.amount) {
            Ok(0) => {
                self.status = "Amount must be more than 0".into();
                self.form.field = Field::Amount;
                return Effect::None;
            }
            Ok(a) => a,
            Err(e) => {
                self.status = e.message().into();
                self.form.field = Field::Amount;
                return Effect::None;
            }
        };
        let f = self.form.clone();
        let key = match &f.editing {
            Some(k) => {
                let Some(idx) = self.entries.iter().position(|e| &e.key == k) else {
                    self.status = "That transaction is gone".into();
                    self.screen = Screen::List;
                    return Effect::None;
                };
                let old = self.entries[idx].clone();
                let t = &mut self.entries[idx].txn;
                t.kind = f.kind;
                t.amount = amount;
                t.category = f.category;
                t.date = f.date;
                t.description = f.description.clone();
                self.pending = Some(Pending::Edit(old));
                k.clone()
            }
            None => {
                let Some(key) = self.fresh_key(entropy) else {
                    self.status = "TRNG error: not saved".into();
                    return Effect::None;
                };
                let seq = self.entries.iter().map(|e| e.txn.seq).max().map_or(1, |m| m.saturating_add(1));
                self.entries.push(Entry {
                    key: key.clone(),
                    txn: Transaction {
                        version: RECORD_VERSION,
                        seq,
                        kind: f.kind,
                        amount,
                        category: f.category,
                        date: f.date,
                        description: f.description.clone(),
                    },
                });
                self.pending = Some(Pending::New(key.clone()));
                key
            }
        };
        self.entries.sort_by(|a, b| newest_first(&a.txn, &b.txn));
        self.month = f.date.year_month();
        self.cursor = self.month_entries().iter().position(|&i| self.entries[i].key == key).unwrap_or(0);
        self.screen = Screen::List;
        Effect::Persist(key)
    }

    /// A random key not already in use. A repeat means the TRNG is stuck
    /// (or astronomically unlucky); after a few tries, give up rather than
    /// overwrite an existing record.
    fn fresh_key<E: Entropy + ?Sized>(&self, entropy: &mut E) -> Option<String> {
        for _ in 0..4 {
            let k = new_key(entropy).ok()?;
            if self.entry(&k).is_none() {
                return Some(k);
            }
        }
        None
    }

    /// Result of an `Effect::Persist`. On failure the change is rolled back
    /// and the form is reopened with the user's input intact.
    pub fn on_persisted(&mut self, result: Result<(), &str>) {
        let Some(p) = self.pending.take() else { return };
        match result {
            Ok(()) => self.status = "Saved".into(),
            Err(why) => {
                match p {
                    Pending::New(k) => self.entries.retain(|e| e.key != k),
                    Pending::Edit(old) => {
                        if let Some(e) = self.entries.iter_mut().find(|e| e.key == old.key) {
                            *e = old;
                        }
                    }
                    Pending::Settings(_) => {}
                }
                self.entries.sort_by(|a, b| newest_first(&a.txn, &b.txn));
                self.clamp_cursor();
                self.status = format!("Save failed: {why}");
                self.screen = Screen::Form;
            }
        }
    }

    /// The PDDB key is gone: drop the entry.
    pub fn on_deleted(&mut self, key: &str) {
        self.entries.retain(|e| e.key != key);
        self.selected = None;
        self.screen = Screen::List;
        self.status = "Deleted".into();
        self.clamp_cursor();
    }

    pub fn on_delete_failed(&mut self, why: &str) {
        self.screen = self.confirm_from;
        self.status = format!("Delete failed: {why}");
    }

    /// Result of `Effect::SaveSettings`. On failure the old settings return.
    pub fn on_settings_saved(&mut self, result: Result<(), &str>) {
        if let Some(Pending::Settings(old)) = self.pending.take() {
            if let Err(why) = result {
                self.settings = old;
                self.status = format!("Settings not saved: {why}");
            }
        }
    }
}

fn toggle_kind(f: &mut Form) {
    f.kind = f.kind.toggled();
    if !f.category.valid_for(f.kind) {
        f.category = f.kind.default_category();
    }
}

#[cfg(test)]
mod tests;

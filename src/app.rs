//! Application state machine for Budget Ledger.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::ledger::{self, Category, Transaction, TxnType};
use crate::storage::Storage;

const KEY_UP: char = '\u{2191}';
const KEY_DOWN: char = '\u{2193}';
const KEY_LEFT: char = '\u{2190}';
const KEY_RIGHT: char = '\u{2192}';
const KEY_ENTER: char = '\r';
const KEY_BACKSPACE: char = '\u{0008}';
const KEY_MENU: char = '\u{2234}';

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Summary,
    TransactionList,
    AddTransaction,
    ViewTransaction,
    ConfirmDelete,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AddField {
    Type,
    Amount,
    Category,
    Description,
    Save,
}

impl AddField {
    pub fn all() -> &'static [AddField] {
        &[AddField::Type, AddField::Amount, AddField::Category,
          AddField::Description, AddField::Save]
    }

    pub fn label(&self) -> &'static str {
        match self {
            AddField::Type => "Type",
            AddField::Amount => "Amount",
            AddField::Category => "Category",
            AddField::Description => "Description",
            AddField::Save => "[SAVE]",
        }
    }
}

const LIST_VISIBLE: usize = 18;

pub struct BudgetApp {
    pub state: AppState,
    pub needs_redraw: bool,

    // Transactions
    pub transactions: Vec<Transaction>,
    pub list_cursor: usize,
    pub list_scroll: usize,

    // Add transaction
    pub edit_txn: Option<Transaction>,
    pub edit_field_cursor: usize,
    pub editing_field: bool,
    pub amount_buffer: String,  // typed amount as string ("12.50")

    // View
    pub viewed_index: usize,

    // IDs
    pub next_id: u32,

    // Storage
    pub storage: Option<Storage>,
}

impl BudgetApp {
    pub fn new() -> Self {
        Self {
            state: AppState::Summary,
            needs_redraw: true,
            transactions: Vec::new(),
            list_cursor: 0,
            list_scroll: 0,
            edit_txn: None,
            edit_field_cursor: 0,
            editing_field: false,
            amount_buffer: String::new(),
            viewed_index: 0,
            next_id: 1,
            storage: None,
        }
    }

    pub fn init_storage(&mut self) {
        if let Ok(mut stor) = Storage::new() {
            self.transactions = stor.load_transactions();
            self.next_id = stor.load_next_id();
            self.storage = Some(stor);
        }
    }

    pub fn save_state(&mut self) {
        if let Some(ref mut stor) = self.storage {
            stor.save_transactions(&self.transactions);
            stor.save_next_id(self.next_id);
        }
    }

    pub fn summary(&self) -> ledger::LedgerSummary {
        ledger::summarize(&self.transactions)
    }

    fn clamp_scroll(&mut self) {
        if self.list_cursor < self.list_scroll {
            self.list_scroll = self.list_cursor;
        }
        if self.list_cursor >= self.list_scroll + LIST_VISIBLE {
            self.list_scroll = self.list_cursor + 1 - LIST_VISIBLE;
        }
    }

    pub fn handle_key(&mut self, key: char) -> bool {
        self.needs_redraw = true;

        match self.state {
            AppState::Summary => self.handle_summary_key(key),
            AppState::TransactionList => self.handle_list_key(key),
            AppState::AddTransaction => self.handle_add_key(key),
            AppState::ViewTransaction => self.handle_view_key(key),
            AppState::ConfirmDelete => self.handle_confirm_delete(key),
        }
    }

    // ─── Summary ────────────────────────────────────────────────────────────

    fn handle_summary_key(&mut self, key: char) -> bool {
        match key {
            'l' | 'L' | KEY_ENTER => {
                self.state = AppState::TransactionList;
            }
            'i' | 'I' => {
                self.start_add(TxnType::Income);
            }
            'e' | 'E' => {
                self.start_add(TxnType::Expense);
            }
            'q' | 'Q' | KEY_MENU => return false,
            _ => {}
        }
        true
    }

    // ─── Transaction List ───────────────────────────────────────────────────

    fn handle_list_key(&mut self, key: char) -> bool {
        match key {
            KEY_UP => {
                if self.list_cursor > 0 {
                    self.list_cursor -= 1;
                    self.clamp_scroll();
                }
            }
            KEY_DOWN => {
                if self.list_cursor + 1 < self.transactions.len() {
                    self.list_cursor += 1;
                    self.clamp_scroll();
                }
            }
            KEY_ENTER => {
                if self.list_cursor < self.transactions.len() {
                    self.viewed_index = self.list_cursor;
                    self.state = AppState::ViewTransaction;
                }
            }
            'i' | 'I' => {
                self.start_add(TxnType::Income);
            }
            'e' | 'E' => {
                self.start_add(TxnType::Expense);
            }
            'd' | 'D' => {
                if self.list_cursor < self.transactions.len() {
                    self.viewed_index = self.list_cursor;
                    self.state = AppState::ConfirmDelete;
                }
            }
            'q' | 'Q' | KEY_BACKSPACE | KEY_MENU => {
                self.state = AppState::Summary;
            }
            _ => {}
        }
        true
    }

    // ─── Add Transaction ────────────────────────────────────────────────────

    fn start_add(&mut self, txn_type: TxnType) {
        let mut txn = Transaction::new(self.next_id, txn_type);
        self.next_id += 1;
        self.edit_txn = Some(txn);
        self.edit_field_cursor = 1; // start on Amount
        self.editing_field = true;
        self.amount_buffer.clear();
        self.state = AppState::AddTransaction;
    }

    fn handle_add_key(&mut self, key: char) -> bool {
        let fields = AddField::all();

        if self.editing_field {
            let field = fields[self.edit_field_cursor];

            match field {
                AddField::Amount => {
                    match key {
                        '0'..='9' => {
                            if self.amount_buffer.len() < 10 {
                                self.amount_buffer.push(key);
                                self.parse_amount();
                            }
                        }
                        '.' => {
                            if !self.amount_buffer.contains('.') && self.amount_buffer.len() < 9 {
                                self.amount_buffer.push('.');
                            }
                        }
                        KEY_BACKSPACE => {
                            self.amount_buffer.pop();
                            self.parse_amount();
                        }
                        KEY_ENTER => {
                            self.editing_field = false;
                        }
                        _ => {}
                    }
                }
                AddField::Description => {
                    match key {
                        KEY_ENTER => {
                            self.editing_field = false;
                        }
                        KEY_BACKSPACE => {
                            if let Some(ref mut txn) = self.edit_txn {
                                txn.description.pop();
                            }
                        }
                        c if !c.is_control() && c != KEY_UP && c != KEY_DOWN
                            && c != KEY_LEFT && c != KEY_RIGHT => {
                            if let Some(ref mut txn) = self.edit_txn {
                                if txn.description.len() < 30 {
                                    txn.description.push(c);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {
                    self.editing_field = false;
                }
            }
        } else {
            match key {
                KEY_UP => {
                    if self.edit_field_cursor > 0 {
                        self.edit_field_cursor -= 1;
                    }
                }
                KEY_DOWN | '\t' => {
                    if self.edit_field_cursor + 1 < fields.len() {
                        self.edit_field_cursor += 1;
                    }
                }
                KEY_ENTER | ' ' => {
                    let field = fields[self.edit_field_cursor];
                    match field {
                        AddField::Save => {
                            self.save_transaction();
                            self.state = AppState::TransactionList;
                        }
                        AddField::Type => {
                            if let Some(ref mut txn) = self.edit_txn {
                                txn.txn_type = match txn.txn_type {
                                    TxnType::Income => TxnType::Expense,
                                    TxnType::Expense => TxnType::Income,
                                };
                            }
                        }
                        AddField::Category => {
                            if let Some(ref mut txn) = self.edit_txn {
                                txn.category = txn.category.next();
                            }
                        }
                        AddField::Amount | AddField::Description => {
                            self.editing_field = true;
                        }
                    }
                }
                'c' | 'C' => {
                    if let Some(ref mut txn) = self.edit_txn {
                        txn.category = txn.category.next();
                    }
                }
                'q' | 'Q' | KEY_MENU => {
                    self.edit_txn = None;
                    self.state = AppState::Summary;
                }
                _ => {}
            }
        }
        true
    }

    fn parse_amount(&mut self) {
        if let Some(ref mut txn) = self.edit_txn {
            // Parse "12.50" -> 1250 cents
            let parts: Vec<&str> = self.amount_buffer.split('.').collect();
            let dollars: u32 = parts.get(0)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let cents: u32 = if parts.len() > 1 {
                let c = parts[1];
                match c.len() {
                    0 => 0,
                    1 => c.parse::<u32>().unwrap_or(0) * 10,
                    _ => c[..2].parse::<u32>().unwrap_or(0),
                }
            } else {
                0
            };
            txn.amount_cents = dollars * 100 + cents;
        }
    }

    fn save_transaction(&mut self) {
        if let Some(txn) = self.edit_txn.take() {
            if txn.amount_cents > 0 {
                self.transactions.insert(0, txn); // newest first
                self.save_state();
            }
        }
    }

    // ─── View Transaction ───────────────────────────────────────────────────

    fn handle_view_key(&mut self, key: char) -> bool {
        match key {
            'd' | 'D' => {
                self.state = AppState::ConfirmDelete;
            }
            _ => {
                self.state = AppState::TransactionList;
            }
        }
        true
    }

    // ─── Confirm Delete ─────────────────────────────────────────────────────

    fn handle_confirm_delete(&mut self, key: char) -> bool {
        match key {
            'y' | 'Y' => {
                if self.viewed_index < self.transactions.len() {
                    self.transactions.remove(self.viewed_index);
                    self.save_state();
                    if self.list_cursor > 0 && self.list_cursor >= self.transactions.len() {
                        self.list_cursor -= 1;
                    }
                }
                self.state = AppState::TransactionList;
            }
            _ => {
                self.state = AppState::TransactionList;
            }
        }
        true
    }
}

//! Ledger data model — transactions, categories, summaries.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use serde::{Serialize, Deserialize};

/// Transaction types.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TxnType {
    Income,
    Expense,
}

impl TxnType {
    pub fn label(&self) -> &'static str {
        match self {
            TxnType::Income => "Income",
            TxnType::Expense => "Expense",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            TxnType::Income => "+",
            TxnType::Expense => "-",
        }
    }
}

/// Expense/income categories.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Category {
    Food,
    Housing,
    Transport,
    Health,
    Shopping,
    Bills,
    Salary,
    Freelance,
    Gift,
    Other,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Category::Food => "Food",
            Category::Housing => "Housing",
            Category::Transport => "Transport",
            Category::Health => "Health",
            Category::Shopping => "Shopping",
            Category::Bills => "Bills",
            Category::Salary => "Salary",
            Category::Freelance => "Freelance",
            Category::Gift => "Gift",
            Category::Other => "Other",
        }
    }

    pub fn short(&self) -> &'static str {
        match self {
            Category::Food => "FD",
            Category::Housing => "HO",
            Category::Transport => "TR",
            Category::Health => "HE",
            Category::Shopping => "SH",
            Category::Bills => "BI",
            Category::Salary => "SA",
            Category::Freelance => "FR",
            Category::Gift => "GI",
            Category::Other => "OT",
        }
    }

    pub fn next(&self) -> Category {
        match self {
            Category::Food => Category::Housing,
            Category::Housing => Category::Transport,
            Category::Transport => Category::Health,
            Category::Health => Category::Shopping,
            Category::Shopping => Category::Bills,
            Category::Bills => Category::Salary,
            Category::Salary => Category::Freelance,
            Category::Freelance => Category::Gift,
            Category::Gift => Category::Other,
            Category::Other => Category::Food,
        }
    }

    pub fn expense_categories() -> &'static [Category] {
        &[Category::Food, Category::Housing, Category::Transport,
          Category::Health, Category::Shopping, Category::Bills, Category::Other]
    }

    pub fn income_categories() -> &'static [Category] {
        &[Category::Salary, Category::Freelance, Category::Gift, Category::Other]
    }
}

/// A single transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: u32,
    pub txn_type: TxnType,
    pub amount_cents: u32,   // stored as cents to avoid float
    pub category: Category,
    pub description: String, // max 30 chars
}

impl Transaction {
    pub fn new(id: u32, txn_type: TxnType) -> Self {
        let default_cat = match txn_type {
            TxnType::Income => Category::Salary,
            TxnType::Expense => Category::Food,
        };
        Self {
            id,
            txn_type,
            amount_cents: 0,
            category: default_cat,
            description: String::new(),
        }
    }

    /// Format amount as "$XX.XX"
    pub fn amount_display(&self) -> String {
        let dollars = self.amount_cents / 100;
        let cents = self.amount_cents % 100;
        format!("${}.{:02}", dollars, cents)
    }

    /// Signed amount in cents.
    pub fn signed_cents(&self) -> i64 {
        match self.txn_type {
            TxnType::Income => self.amount_cents as i64,
            TxnType::Expense => -(self.amount_cents as i64),
        }
    }

    /// Short display line for list.
    pub fn list_line(&self) -> String {
        let sym = self.txn_type.symbol();
        let amt = self.amount_display();
        if self.description.is_empty() {
            format!("{}{} [{}]", sym, amt, self.category.short())
        } else {
            format!("{}{} {} [{}]", sym, amt, self.description, self.category.short())
        }
    }
}

/// Summary stats for a set of transactions.
pub struct LedgerSummary {
    pub total_income: i64,   // in cents
    pub total_expense: i64,  // in cents (positive)
    pub balance: i64,        // income - expense
    pub count: usize,
}

pub fn summarize(transactions: &[Transaction]) -> LedgerSummary {
    let mut income = 0i64;
    let mut expense = 0i64;

    for txn in transactions {
        match txn.txn_type {
            TxnType::Income => income += txn.amount_cents as i64,
            TxnType::Expense => expense += txn.amount_cents as i64,
        }
    }

    LedgerSummary {
        total_income: income,
        total_expense: expense,
        balance: income - expense,
        count: transactions.len(),
    }
}

/// Format cents as dollar string.
pub fn format_cents(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    let dollars = abs / 100;
    let c = abs % 100;
    format!("{}${}.{:02}", sign, dollars, c)
}

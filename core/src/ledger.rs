//! Transactions, categories, per-month and per-category totals, and the
//! stored record format (one PDDB key per transaction).

use serde::{Deserialize, Serialize};

use crate::date::{Date, YearMonth};
use crate::entropy::{Entropy, EntropyError};
use crate::money::MAX_AMOUNT;

/// Bump when the record format changes. Records with a newer version are
/// left untouched.
pub const RECORD_VERSION: u32 = 1;
/// Prefix of transaction keys in the PDDB dictionary.
pub const KEY_PREFIX: &str = "t.";
/// Maximum description length, in characters.
pub const MAX_DESC: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Income,
    Expense,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Income => "Income",
            Kind::Expense => "Expense",
        }
    }

    pub fn sign(self) -> &'static str {
        match self {
            Kind::Income => "+",
            Kind::Expense => "-",
        }
    }

    pub fn toggled(self) -> Kind {
        match self {
            Kind::Income => Kind::Expense,
            Kind::Expense => Kind::Income,
        }
    }

    /// Categories offered for this kind, in cycling order.
    pub fn categories(self) -> &'static [Category] {
        match self {
            Kind::Income => &[Category::Salary, Category::Freelance, Category::Gift, Category::Other],
            Kind::Expense => &[
                Category::Food,
                Category::Housing,
                Category::Transport,
                Category::Health,
                Category::Shopping,
                Category::Bills,
                Category::Other,
            ],
        }
    }

    pub fn default_category(self) -> Category { self.categories()[0] }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
    pub fn label(self) -> &'static str {
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

    /// Step `delta` places through `kind`'s categories, wrapping. A category
    /// that isn't in `kind`'s list moves to the list's first entry.
    pub fn cycle(self, kind: Kind, delta: isize) -> Category {
        let list = kind.categories();
        match list.iter().position(|&c| c == self) {
            Some(i) => {
                let n = list.len() as isize;
                list[((i as isize + delta) % n + n) as usize % list.len()]
            }
            None => kind.default_category(),
        }
    }

    pub fn valid_for(self, kind: Kind) -> bool { kind.categories().contains(&self) }
}

/// A transaction as stored (JSON) under its own PDDB key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub version: u32,
    /// Creation order, for ordering same-day entries. Derived from the
    /// stored records (max + 1), so it can't drift from the data.
    pub seq: u64,
    pub kind: Kind,
    /// Minor units (cents), 1..=MAX_AMOUNT.
    pub amount: u64,
    pub category: Category,
    pub date: Date,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordError {
    Parse,
    NewerVersion(u32),
    /// Parsed but out of range (bad date, zero or huge amount, wrong category).
    Invalid,
}

impl Transaction {
    pub fn to_json(&self) -> Vec<u8> { serde_json::to_vec(self).expect("transaction serializes") }

    pub fn from_json(bytes: &[u8]) -> Result<Self, RecordError> {
        #[derive(Deserialize)]
        struct Version {
            version: u32,
        }
        let v: Version = serde_json::from_slice(bytes).map_err(|_| RecordError::Parse)?;
        if v.version > RECORD_VERSION {
            return Err(RecordError::NewerVersion(v.version));
        }
        let t: Transaction = serde_json::from_slice(bytes).map_err(|_| RecordError::Parse)?;
        let ok = t.date.is_valid()
            && (1..=MAX_AMOUNT).contains(&t.amount)
            && t.category.valid_for(t.kind)
            && t.description.chars().count() <= MAX_DESC;
        if ok {
            Ok(t)
        } else {
            Err(RecordError::Invalid)
        }
    }

    /// Signed minor units: income positive, expense negative.
    pub fn signed(&self) -> i64 {
        // amount <= MAX_AMOUNT, far below i64::MAX
        match self.kind {
            Kind::Income => self.amount as i64,
            Kind::Expense => -(self.amount as i64),
        }
    }
}

pub fn is_txn_key(name: &str) -> bool {
    name.len() == KEY_PREFIX.len() + 16
        && name.starts_with(KEY_PREFIX)
        && name[KEY_PREFIX.len()..].bytes().all(|b| b.is_ascii_hexdigit())
}

/// A fresh random key name. Random (not a counter), so a new record can't
/// collide with one stored in a basis that is currently locked.
pub fn new_key<E: Entropy + ?Sized>(e: &mut E) -> Result<String, EntropyError> {
    let hi = e.try_u32()?;
    let lo = e.try_u32()?;
    Ok(format!("{KEY_PREFIX}{hi:08x}{lo:08x}"))
}

/// Newest first: by date, then by creation order.
pub fn newest_first(a: &Transaction, b: &Transaction) -> core::cmp::Ordering {
    b.date.cmp(&a.date).then(b.seq.cmp(&a.seq))
}

/// Totals over a set of transactions, in minor units.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Totals {
    pub income: i64,
    /// Positive.
    pub expense: i64,
    pub count: usize,
}

impl Totals {
    pub fn net(&self) -> i64 { self.income.saturating_sub(self.expense) }

    fn add(&mut self, t: &Transaction) {
        match t.kind {
            Kind::Income => self.income = self.income.saturating_add(t.amount as i64),
            Kind::Expense => self.expense = self.expense.saturating_add(t.amount as i64),
        }
        self.count += 1;
    }
}

pub fn totals<'a>(txns: impl IntoIterator<Item = &'a Transaction>) -> Totals {
    let mut t = Totals::default();
    for x in txns {
        t.add(x);
    }
    t
}

/// Totals for one calendar month.
pub fn month_totals<'a>(txns: impl IntoIterator<Item = &'a Transaction>, month: YearMonth) -> Totals {
    totals(txns.into_iter().filter(|t| month.contains(t.date)))
}

/// Per-category totals of one kind in one month, largest first (ties by
/// category order). Categories with nothing are omitted.
pub fn category_totals<'a>(
    txns: impl IntoIterator<Item = &'a Transaction>,
    month: YearMonth,
    kind: Kind,
) -> Vec<(Category, i64)> {
    let mut out: Vec<(Category, i64)> = Vec::new();
    for t in txns.into_iter().filter(|t| t.kind == kind && month.contains(t.date)) {
        match out.iter_mut().find(|(c, _)| *c == t.category) {
            Some((_, sum)) => *sum = sum.saturating_add(t.amount as i64),
            None => out.push((t.category, t.amount as i64)),
        }
    }
    out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entropy::testing::{Broken, Seq};

    pub(crate) fn txn(seq: u64, kind: Kind, amount: u64, cat: Category, date: (i32, u8, u8)) -> Transaction {
        Transaction {
            version: RECORD_VERSION,
            seq,
            kind,
            amount,
            category: cat,
            date: Date::new(date.0, date.1, date.2).unwrap(),
            description: String::new(),
        }
    }

    fn nov() -> YearMonth { YearMonth::new(2026, 11).unwrap() }

    #[test]
    fn month_totals_only_count_that_month() {
        let txns = vec![
            txn(1, Kind::Income, 300_000, Category::Salary, (2026, 11, 1)),
            txn(2, Kind::Expense, 1_250, Category::Food, (2026, 11, 30)),
            txn(3, Kind::Expense, 9_999, Category::Food, (2026, 12, 1)),
            txn(4, Kind::Expense, 500, Category::Bills, (2026, 10, 31)),
            txn(5, Kind::Expense, 100, Category::Food, (2025, 11, 15)),
        ];
        let t = month_totals(&txns, nov());
        assert_eq!(t, Totals { income: 300_000, expense: 1_250, count: 2 });
        assert_eq!(t.net(), 298_750);
        let dec = month_totals(&txns, nov().next());
        assert_eq!(dec, Totals { income: 0, expense: 9_999, count: 1 });
        let all = totals(&txns);
        assert_eq!(all.net(), 300_000 - 1_250 - 9_999 - 500 - 100);
    }

    #[test]
    fn rollover_from_december_to_january() {
        let txns = vec![
            txn(1, Kind::Expense, 100, Category::Food, (2026, 12, 31)),
            txn(2, Kind::Expense, 200, Category::Food, (2027, 1, 1)),
        ];
        let dec = YearMonth::new(2026, 12).unwrap();
        assert_eq!(month_totals(&txns, dec).expense, 100);
        assert_eq!(month_totals(&txns, dec.next()).expense, 200);
    }

    #[test]
    fn category_totals_sorted_and_scoped() {
        let txns = vec![
            txn(1, Kind::Expense, 500, Category::Food, (2026, 11, 2)),
            txn(2, Kind::Expense, 2_000, Category::Housing, (2026, 11, 3)),
            txn(3, Kind::Expense, 700, Category::Food, (2026, 11, 4)),
            txn(4, Kind::Expense, 1_200, Category::Bills, (2026, 11, 5)),
            txn(5, Kind::Income, 9_000, Category::Other, (2026, 11, 5)),
            txn(6, Kind::Expense, 9_000, Category::Bills, (2026, 12, 5)),
        ];
        assert_eq!(
            category_totals(&txns, nov(), Kind::Expense),
            vec![(Category::Housing, 2_000), (Category::Food, 1_200), (Category::Bills, 1_200)]
        );
        assert_eq!(category_totals(&txns, nov(), Kind::Income), vec![(Category::Other, 9_000)]);
    }

    #[test]
    fn totals_saturate() {
        let big: Vec<Transaction> =
            (0..200_000).map(|i| txn(i, Kind::Income, MAX_AMOUNT, Category::Salary, (2026, 1, 1))).collect();
        let t = totals(&big);
        assert_eq!(t.income, 200_000 * MAX_AMOUNT as i64);
        assert!(t.net() > 0);
    }

    /// Audit regression: `next()` cycled all ten categories for both kinds,
    /// so income could be tagged "Housing".
    #[test]
    fn categories_cycle_within_kind() {
        let mut c = Kind::Income.default_category();
        for _ in 0..20 {
            c = c.cycle(Kind::Income, 1);
            assert!(c.valid_for(Kind::Income), "{c:?}");
        }
        assert_eq!(Category::Salary.cycle(Kind::Income, -1), Category::Other);
        assert_eq!(Category::Other.cycle(Kind::Expense, 1), Category::Food);
        assert_eq!(Category::Housing.cycle(Kind::Income, 1), Category::Salary);
    }

    #[test]
    fn record_round_trip_and_validation() {
        let mut t = txn(7, Kind::Expense, 1_250, Category::Food, (2024, 2, 29));
        t.description = "Lunch, \"deli\"".into();
        assert_eq!(Transaction::from_json(&t.to_json()), Ok(t.clone()));

        let mut bad = t.clone();
        bad.amount = 0;
        assert_eq!(Transaction::from_json(&bad.to_json()), Err(RecordError::Invalid));
        bad = t.clone();
        bad.date = Date { year: 2025, month: 2, day: 29 };
        assert_eq!(Transaction::from_json(&bad.to_json()), Err(RecordError::Invalid));
        bad = t.clone();
        bad.category = Category::Salary;
        assert_eq!(Transaction::from_json(&bad.to_json()), Err(RecordError::Invalid));
        bad = t.clone();
        bad.version = 2;
        assert_eq!(Transaction::from_json(&bad.to_json()), Err(RecordError::NewerVersion(2)));
        assert_eq!(Transaction::from_json(b"[{]"), Err(RecordError::Parse));
    }

    #[test]
    fn keys() {
        let k = new_key(&mut Seq(vec![0xdead_beef, 1], 0)).unwrap();
        assert_eq!(k, "t.deadbeef00000001");
        assert!(is_txn_key(&k));
        assert!(!is_txn_key("settings"));
        assert!(!is_txn_key("t.xyz"));
        assert!(new_key(&mut Broken).is_err());
    }

    #[test]
    fn newest_first_orders_by_date_then_seq() {
        let mut v = vec![
            txn(1, Kind::Expense, 1, Category::Food, (2026, 11, 1)),
            txn(3, Kind::Expense, 1, Category::Food, (2026, 10, 1)),
            txn(2, Kind::Expense, 1, Category::Food, (2026, 11, 1)),
        ];
        v.sort_by(newest_first);
        let seqs: Vec<u64> = v.iter().map(|t| t.seq).collect();
        assert_eq!(seqs, vec![2, 1, 3]);
    }
}

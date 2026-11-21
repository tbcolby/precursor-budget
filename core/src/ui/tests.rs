use super::*;
use crate::entropy::testing::{Broken, Stuck, XorShift};
use crate::ledger::RECORD_VERSION;

fn d(y: i32, m: u8, day: u8) -> Date { Date::new(y, m, day).unwrap() }

fn ui() -> Ui { Ui::new(Some(d(2026, 11, 20))) }

thread_local! {
    // One generator per test thread, so successive saves get distinct keys.
    static RNG: std::cell::RefCell<XorShift> = std::cell::RefCell::new(XorShift(0x2545_f491));
}

fn key(u: &mut Ui, i: Input) -> Effect { RNG.with(|r| u.on_key(i, &mut *r.borrow_mut())) }

fn typ(u: &mut Ui, s: &str) {
    for c in s.chars() {
        assert_eq!(key(u, Input::Char(c)), Effect::None);
    }
}

fn entry(key: &str, seq: u64, kind: Kind, amount: u64, cat: Category, date: Date) -> Entry {
    Entry {
        key: key.into(),
        txn: Transaction { version: RECORD_VERSION, seq, kind, amount, category: cat, date, description: String::new() },
    }
}

/// Add an expense of `amount` text on the current form and persist it OK.
fn add_expense(u: &mut Ui, amount: &str) -> String {
    key(u, Input::Char('e'));
    assert_eq!(u.screen, Screen::Form);
    typ(u, amount);
    let Effect::Persist(k) = key(u, Input::F(1)) else { panic!("no persist: {}", u.status) };
    u.on_persisted(Ok(()));
    k
}

const BACK_KEYS: [Input; 2] = [Input::Backspace, Input::F(4)];

#[test]
fn every_screen_exits_on_backspace_and_f4() {
    for back in BACK_KEYS {
        let mut u = ui();
        let k = add_expense(&mut u, "5");
        // List -> Summary -> Leave
        assert_eq!(u.screen, Screen::List);
        assert_eq!(key(&mut u, back), Effect::None);
        assert_eq!(u.screen, Screen::Summary);
        assert_eq!(key(&mut u, back), Effect::Leave);
        // View -> List
        key(&mut u, Input::Enter);
        key(&mut u, Input::Enter);
        assert_eq!(u.screen, Screen::View);
        assert_eq!(u.selected().unwrap().key, k);
        key(&mut u, back);
        assert_eq!(u.screen, Screen::List);
        // Confirm -> List
        key(&mut u, Input::Char('d'));
        assert_eq!(u.screen, Screen::ConfirmDelete);
        key(&mut u, back);
        assert_eq!(u.screen, Screen::List);
        // Form (new, empty amount) -> List
        key(&mut u, Input::Char('i'));
        assert_eq!(u.screen, Screen::Form);
        key(&mut u, back);
        assert_eq!(u.screen, Screen::List);
        // Settings -> Summary
        key(&mut u, back);
        key(&mut u, Input::F(2));
        assert_eq!(u.screen, Screen::Settings);
        key(&mut u, back);
        assert_eq!(u.screen, Screen::Summary);
    }
}

#[test]
fn f4_cancels_form_even_with_text() {
    let mut u = ui();
    key(&mut u, Input::Char('e'));
    typ(&mut u, "12");
    key(&mut u, Input::Backspace);
    assert_eq!(u.form.amount, "1");
    assert_eq!(u.screen, Screen::Form);
    key(&mut u, Input::F(4));
    assert_eq!(u.screen, Screen::Summary);
    assert!(u.entries.is_empty());
}

#[test]
fn add_uses_todays_date_and_returns_to_its_month() {
    let mut u = ui();
    let k = add_expense(&mut u, "12.50");
    let e = u.entry(&k).unwrap();
    assert_eq!(e.txn.amount, 1250);
    assert_eq!(e.txn.date, d(2026, 11, 20));
    assert_eq!(e.txn.category, Category::Food);
    assert_eq!(u.status, "Saved");
    assert_eq!(u.cursor_entry().unwrap().key, k);
}

/// Audit regression: a zero amount was silently dropped.
#[test]
fn zero_amount_is_refused_with_message() {
    let mut u = ui();
    key(&mut u, Input::Char('e'));
    typ(&mut u, "0.00");
    assert_eq!(key(&mut u, Input::F(1)), Effect::None);
    assert_eq!(u.screen, Screen::Form);
    assert_eq!(u.status, "Amount must be more than 0");
    key(&mut u, Input::F(4));
    key(&mut u, Input::Char('e'));
    assert_eq!(key(&mut u, Input::F(1)), Effect::None);
    assert_eq!(u.status, "Enter an amount");
}

#[test]
fn amount_field_rejects_bad_keystrokes() {
    let mut u = ui();
    key(&mut u, Input::Char('e'));
    typ(&mut u, "1x2.3.45");
    assert_eq!(u.form.amount, "12.34");
    let mut u = ui();
    key(&mut u, Input::Char('e'));
    typ(&mut u, "50000000000");
    assert_eq!(u.form.amount, "500000000");
}

/// Audit regression: an ID was consumed by every cancelled add. Keys are now
/// random and allocated only on save, and `seq` is derived from the data.
#[test]
fn cancelled_add_allocates_nothing_and_seq_follows_data() {
    let mut u = ui();
    u.set_entries(vec![entry("t.0000000000000009", 41, Kind::Expense, 100, Category::Food, d(2026, 11, 1))], 0);
    key(&mut u, Input::Char('e'));
    key(&mut u, Input::F(4));
    let k = add_expense(&mut u, "1");
    assert_eq!(u.entry(&k).unwrap().txn.seq, 42);
}

#[test]
fn trng_failure_saves_nothing() {
    let mut u = ui();
    key(&mut u, Input::Char('e'));
    typ(&mut u, "3");
    assert_eq!(u.on_key(Input::F(1), &mut Broken), Effect::None);
    assert_eq!(u.status, "TRNG error: not saved");
    assert!(u.entries.is_empty());
    assert_eq!(u.screen, Screen::Form);
}

#[test]
fn failed_save_rolls_back_and_reopens_form() {
    let mut u = ui();
    key(&mut u, Input::Char('e'));
    typ(&mut u, "3");
    assert!(matches!(key(&mut u, Input::F(1)), Effect::Persist(_)));
    // keys are ignored until the result arrives
    assert_eq!(key(&mut u, Input::F(4)), Effect::None);
    u.on_persisted(Err("full"));
    assert!(u.entries.is_empty());
    assert_eq!(u.screen, Screen::Form);
    assert_eq!(u.form.amount, "3");
    assert_eq!(u.status, "Save failed: full");
}

#[test]
fn edit_in_place_keeps_key_and_rolls_back_on_failure() {
    let mut u = ui();
    let k = add_expense(&mut u, "10");
    key(&mut u, Input::Enter);
    key(&mut u, Input::Char('e'));
    assert_eq!(u.form.amount, "10.00");
    for _ in 0..5 {
        key(&mut u, Input::Backspace);
    }
    typ(&mut u, "7.5");
    assert_eq!(key(&mut u, Input::F(1)), Effect::Persist(k.clone()));
    assert_eq!(u.entry(&k).unwrap().txn.amount, 750);
    u.on_persisted(Ok(()));
    assert_eq!(u.entries.len(), 1);

    key(&mut u, Input::Enter);
    key(&mut u, Input::Char('e'));
    typ(&mut u, "9");
    key(&mut u, Input::F(1));
    u.on_persisted(Err("io"));
    assert_eq!(u.entry(&k).unwrap().txn.amount, 750);
}

#[test]
fn edit_moving_date_follows_to_new_month() {
    let mut u = ui();
    let k = add_expense(&mut u, "1");
    key(&mut u, Input::Enter);
    key(&mut u, Input::Char('e'));
    for _ in 0..2 {
        key(&mut u, Input::Down);
    }
    assert_eq!(u.form.field, Field::Date);
    for _ in 0..20 {
        key(&mut u, Input::Left);
    }
    assert_eq!(u.form.date, d(2026, 10, 31));
    key(&mut u, Input::F(1));
    u.on_persisted(Ok(()));
    assert_eq!(u.month, YearMonth::new(2026, 10).unwrap());
    assert_eq!(u.cursor_entry().unwrap().key, k);
}

#[test]
fn delete_waits_for_storage() {
    let mut u = ui();
    let a = add_expense(&mut u, "1");
    let b = add_expense(&mut u, "2");
    assert_eq!(u.cursor_entry().unwrap().key, b);
    key(&mut u, Input::Down);
    key(&mut u, Input::Char('d'));
    assert_eq!(key(&mut u, Input::Char('y')), Effect::Delete(a.clone()));
    // failure keeps the entry
    u.on_delete_failed("io");
    assert!(u.entry(&a).is_some());
    assert_eq!(u.screen, Screen::List);
    key(&mut u, Input::Char('d'));
    assert_eq!(key(&mut u, Input::Char('y')), Effect::Delete(a.clone()));
    u.on_deleted(&a);
    assert!(u.entry(&a).is_none());
    assert_eq!(u.cursor, 0);
    assert_eq!(u.screen, Screen::List);
}

#[test]
fn cancel_delete_returns_to_origin() {
    let mut u = ui();
    add_expense(&mut u, "1");
    key(&mut u, Input::Enter);
    key(&mut u, Input::Char('d'));
    key(&mut u, Input::Char('n'));
    assert_eq!(u.screen, Screen::View);
}

#[test]
fn form_type_toggle_keeps_category_valid() {
    let mut u = ui();
    key(&mut u, Input::Char('e'));
    key(&mut u, Input::Up);
    assert_eq!(u.form.field, Field::Kind);
    key(&mut u, Input::Enter);
    assert_eq!(u.form.kind, Kind::Income);
    assert_eq!(u.form.category, Category::Salary);
    key(&mut u, Input::Down);
    key(&mut u, Input::Down);
    for _ in 0..9 {
        key(&mut u, Input::Right);
        assert!(u.form.category.valid_for(Kind::Income));
    }
}

#[test]
fn description_limited_by_characters() {
    let mut u = ui();
    key(&mut u, Input::Char('e'));
    for _ in 0..3 {
        key(&mut u, Input::Down);
    }
    assert_eq!(u.form.field, Field::Description);
    typ(&mut u, &"é".repeat(40));
    assert_eq!(u.form.description.chars().count(), MAX_DESC);
}

#[test]
fn month_navigation_rolls_over_years() {
    let mut u = Ui::new(Some(d(2026, 12, 31)));
    key(&mut u, Input::Right);
    assert_eq!(u.month, YearMonth::new(2027, 1).unwrap());
    key(&mut u, Input::Left);
    key(&mut u, Input::Left);
    assert_eq!(u.month, YearMonth::new(2026, 11).unwrap());
    key(&mut u, Input::Char('t'));
    assert_eq!(u.month, YearMonth::new(2026, 12).unwrap());
    // adding while viewing another month dates it on that month's 1st
    key(&mut u, Input::Left);
    let k = add_expense(&mut u, "1");
    assert_eq!(u.entry(&k).unwrap().txn.date, d(2026, 11, 1));
}

#[test]
fn list_only_shows_the_month() {
    let mut u = ui();
    u.set_entries(
        vec![
            entry("t.0000000000000001", 1, Kind::Expense, 100, Category::Food, d(2026, 10, 31)),
            entry("t.0000000000000002", 2, Kind::Expense, 200, Category::Food, d(2026, 11, 1)),
            entry("t.0000000000000003", 3, Kind::Income, 300, Category::Gift, d(2026, 11, 2)),
        ],
        0,
    );
    let keys: Vec<&str> = u.month_entries().iter().map(|&i| u.entries[i].key.as_str()).collect();
    assert_eq!(keys, vec!["t.0000000000000003", "t.0000000000000002"]);
    key(&mut u, Input::Enter);
    key(&mut u, Input::Down);
    key(&mut u, Input::Down);
    assert_eq!(u.cursor, 1);
}

#[test]
fn no_clock_starts_on_latest_data_then_jumps_when_clock_appears() {
    let mut u = Ui::new(None);
    assert_eq!(u.month, FALLBACK_MONTH);
    u.set_entries(vec![entry("t.0000000000000001", 1, Kind::Expense, 1, Category::Food, d(2025, 6, 3))], 0);
    assert_eq!(u.month, YearMonth::new(2025, 6).unwrap());
    key(&mut u, Input::Char('t'));
    assert_eq!(u.status, "Clock not set");
    u.set_today(Some(d(2026, 11, 20)));
    assert_eq!(u.month, YearMonth::new(2026, 11).unwrap());
    u.set_today(Some(d(2026, 11, 21)));
    key(&mut u, Input::Left);
    u.set_today(Some(d(2026, 11, 21)));
    assert_eq!(u.month, YearMonth::new(2026, 10).unwrap());
}

#[test]
fn reload_drops_in_progress_form() {
    let mut u = ui();
    key(&mut u, Input::Char('e'));
    typ(&mut u, "99");
    u.set_entries(Vec::new(), 2);
    assert_eq!(u.screen, Screen::Summary);
    assert!(u.form.amount.is_empty());
    assert_eq!(u.unreadable, 2);
}

#[test]
fn settings_save_only_when_changed() {
    let mut u = ui();
    key(&mut u, Input::F(2));
    assert_eq!(key(&mut u, Input::Enter), Effect::None);
    key(&mut u, Input::F(2));
    key(&mut u, Input::Right);
    assert_eq!(u.currency(), "€");
    assert_eq!(key(&mut u, Input::Backspace), Effect::SaveSettings);
    u.on_settings_saved(Err("io"));
    assert_eq!(u.currency(), "$");
    assert_eq!(u.status, "Settings not saved: io");
}

#[test]
fn unreadable_settings_are_never_overwritten() {
    let mut u = ui();
    u.set_settings(Err(()));
    key(&mut u, Input::F(2));
    key(&mut u, Input::Right);
    assert_eq!(u.currency(), "$");
    assert_eq!(key(&mut u, Input::F(4)), Effect::None);
}

#[test]
fn stuck_trng_never_overwrites_a_record() {
    let mut u = ui();
    key(&mut u, Input::Char('e'));
    typ(&mut u, "1");
    assert!(matches!(u.on_key(Input::F(1), &mut Stuck(5)), Effect::Persist(_)));
    u.on_persisted(Ok(()));
    key(&mut u, Input::Char('e'));
    typ(&mut u, "2");
    assert_eq!(u.on_key(Input::F(1), &mut Stuck(5)), Effect::None);
    assert_eq!(u.status, "TRNG error: not saved");
    assert_eq!(u.entries.len(), 1);
}

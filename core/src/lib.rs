//! Pure logic for Budget Ledger: integer-cent money parsing and formatting,
//! calendar dates, transactions with per-month and per-category totals, the
//! stored record formats, and the screen state machine. No Xous
//! dependencies; unit-tested on the host.

pub mod date;
pub mod entropy;
pub mod ledger;
pub mod money;
pub mod settings;
pub mod ui;

pub use date::{Date, YearMonth};
pub use entropy::{Entropy, EntropyError};
pub use ledger::{Category, Kind, Totals, Transaction};
pub use settings::Settings;
pub use ui::{Effect, Entry, Field, Input, Screen, Ui};

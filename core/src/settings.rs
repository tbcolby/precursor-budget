//! User settings (stored under the `settings` key).

use serde::{Deserialize, Serialize};

pub const SETTINGS_VERSION: u32 = 1;
pub const SETTINGS_KEY: &str = "settings";

/// Currency symbols offered. Amounts always have two decimal places.
pub const CURRENCIES: &[&str] = &["$", "€", "£", ""];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub version: u32,
    pub currency: String,
}

impl Default for Settings {
    fn default() -> Self { Self { version: SETTINGS_VERSION, currency: CURRENCIES[0].into() } }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsError {
    Parse,
    NewerVersion(u32),
}

impl Settings {
    pub fn to_json(&self) -> Vec<u8> { serde_json::to_vec(self).expect("settings serialize") }

    pub fn from_json(bytes: &[u8]) -> Result<Self, SettingsError> {
        let s: Settings = serde_json::from_slice(bytes).map_err(|_| SettingsError::Parse)?;
        if s.version > SETTINGS_VERSION {
            return Err(SettingsError::NewerVersion(s.version));
        }
        Ok(s)
    }

    /// Step through [`CURRENCIES`]; an unknown symbol moves to the first.
    pub fn cycle_currency(&mut self, delta: isize) {
        let n = CURRENCIES.len() as isize;
        let next = match CURRENCIES.iter().position(|c| *c == self.currency) {
            Some(i) => ((i as isize + delta) % n + n) % n,
            None => 0,
        };
        self.currency = CURRENCIES[next as usize].into();
    }

    pub fn currency_label(&self) -> &str {
        if self.currency.is_empty() {
            "(none)"
        } else {
            &self.currency
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_and_wraps() {
        let mut s = Settings::default();
        s.cycle_currency(-1);
        assert_eq!(s.currency, "");
        assert_eq!(s.currency_label(), "(none)");
        s.cycle_currency(1);
        assert_eq!(s.currency, "$");
        s.currency = "?".into();
        s.cycle_currency(1);
        assert_eq!(s.currency, "$");
    }

    #[test]
    fn json() {
        let s = Settings { version: 1, currency: "€".into() };
        assert_eq!(Settings::from_json(&s.to_json()), Ok(s));
        assert_eq!(Settings::from_json(br#"{"version":9,"currency":"$"}"#), Err(SettingsError::NewerVersion(9)));
        assert_eq!(Settings::from_json(b"nope"), Err(SettingsError::Parse));
    }
}

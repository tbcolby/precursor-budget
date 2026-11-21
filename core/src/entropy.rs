//! Fallible randomness, re-exported from `precursor-appkit-core`. Used only
//! to name new PDDB keys; a TRNG failure means "not saved", never a
//! predictable key. On the device the source is `precursor_appkit::rng::Trng`.

pub use precursor_appkit_core::entropy::{Entropy, EntropyError};

#[cfg(test)]
pub(crate) use precursor_appkit_core::entropy::testing;

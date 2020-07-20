//! Some primitive types re-exported to the entire crate.
//!
//! As long as these are the same in kusama/polkadot/your-chain, then we are good.

/// The account id type.
pub type AccountId = sp_core::crypto::AccountId32;
/// The balance type.
pub type Balance = u128;
/// The hash type.
pub type Hash = sp_core::hash::H256;
/// The block number type
pub type BlockNumber = u32;
/// Re-exported hashing types.
pub use sp_core::hashing::{blake2_256, twox_128};

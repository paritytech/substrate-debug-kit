pub use node_primitives::{AccountId, Balance, Block, BlockNumber, Hash, Nonce};
pub use sp_core::hashing::{blake2_256, twox_128};

#[cfg(feature = "kusama")]
pub use kusama_runtime as runtime;
#[cfg(feature = "polkadot")]
pub use polkadot_runtime as runtime;

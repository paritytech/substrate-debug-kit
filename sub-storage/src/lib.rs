//! # Sub-Storage.
//!
//! A thing wrapper around substrate's RPC calls that work with storage. This module is an
//! equivalent ot the polkadojs-api, in Rust.
//!
//! The base functions of this crate make no assumption about the runtime. Some runtime-dependent
//! functions are provided under the `helpers` module.

use codec::Decode;
use frame_support::StorageHasher;
use jsonrpsee::{
	common::{to_value as to_json_value, Params},
	Client,
};
use node_primitives::Hash;
use sp_core::hashing::twox_128;
use std::fmt::Debug;

/// Helper's module.
#[cfg(feature = "helpers")]
pub mod helpers;

// re-export all the primitives.
pub use node_primitives as primitives;
// re-export some stuff from sp-core.
pub use sp_core::storage::{StorageData, StorageKey};

/// create key for a simple value.
pub fn value_key(module: &[u8], storage: &[u8]) -> StorageKey {
	StorageKey(module_prefix_raw(module, storage))
}

/// create key for a map.
pub fn map_key<H: StorageHasher>(module: &[u8], storage: &[u8], encoded_key: &[u8]) -> StorageKey {
	let prefix = module_prefix_raw(module, storage);
	let key = H::hash(encoded_key);
	let mut final_key = Vec::with_capacity(prefix.len() + key.as_ref().len());
	final_key.extend_from_slice(&prefix);
	final_key.extend_from_slice(key.as_ref());
	StorageKey(final_key)
}

/// create key for a double map.
pub fn double_map_key<H1: StorageHasher, H2: StorageHasher>(
	module: &[u8],
	storage: &[u8],
	encoded_key_1: &[u8],
	encoded_key_2: &[u8],
) -> StorageKey {
	let prefix = module_prefix_raw(module, storage);
	let key1 = H1::hash(encoded_key_1);
	let key2 = H2::hash(encoded_key_2);
	let mut final_key =
		Vec::with_capacity(prefix.len() + key1.as_ref().len() + key2.as_ref().len());
	final_key.extend_from_slice(&prefix);
	final_key.extend_from_slice(key1.as_ref());
	final_key.extend_from_slice(key2.as_ref());
	StorageKey(final_key)
}

/// create key prefix for a map
pub fn map_prefix_key(module: &[u8], storage: &[u8]) -> StorageKey {
	StorageKey(module_prefix_raw(module, storage))
}

/// create key prefix for a module as vec bytes. Basically twox128 hash of the given values.
/// TODO: can't we use stuff from frame_support::storage directly here? This is for now a duplicate.
pub fn module_prefix_raw(module: &[u8], storage: &[u8]) -> Vec<u8> {
	let module_key = twox_128(module);
	let storage_key = twox_128(storage);
	let mut final_key = Vec::with_capacity(module_key.len() + storage_key.len());
	final_key.extend_from_slice(&module_key);
	final_key.extend_from_slice(&storage_key);
	final_key
}

/// Read from a raw key regardless of the type. This can be used in combination with the key
/// generation methods above and read any data from storage, regardless of its type.
pub async fn read<T: Decode>(key: StorageKey, client: &Client, at: Hash) -> Option<T> {
	let serialized_key = to_json_value(key).expect("StorageKey serialization infallible");
	let at = to_json_value(at).expect("Block hash serialization infallible");
	let raw: Option<StorageData> = client
		.request("state_getStorage", Params::Array(vec![serialized_key, at]))
		.await
		.expect("Storage request failed");
	let encoded = raw.map(|d| d.0)?;
	<T as Decode>::decode(&mut encoded.as_slice()).ok()
}

pub async fn get_pairs(
	prefix: StorageKey,
	client: &Client,
	at: Hash,
) -> Vec<(StorageKey, StorageData)> {
	let serialized_prefix = to_json_value(prefix).expect("StorageKey serialization infallible");
	let at = to_json_value(at).expect("Block hash serialization infallible");
	client
		.request("state_getPairs", Params::Array(vec![serialized_prefix, at]))
		.await
		.expect("Storage state_getPairs failed")
}

/// Enumerate all keys and values in a storage map.
pub async fn enumerate_map<K, V>(
	module: &[u8],
	storage: &[u8],
	client: &Client,
	at: Hash,
) -> Result<Vec<(K, V)>, &'static str>
where
	K: Decode + Debug + Clone + AsRef<[u8]>,
	V: Decode + Clone + Debug,
{
	let prefix = map_prefix_key(module.clone(), storage.clone());
	let raw = get_pairs(prefix, client, at).await;

	raw.into_iter()
		.map(|(k, v)| {
			let mut full_key = k.0;
			let full_len = full_key.len();
			let key = full_key.drain(full_len - 32..).collect::<Vec<_>>();
			(key, v.0)
		})
		.map(|(raw_key, raw_value)| {
			let key = <K as Decode>::decode(&mut raw_key.as_slice());
			let value = <V as Decode>::decode(&mut raw_value.as_slice());
			match (key, value) {
				(Ok(key), Ok(value)) => Ok((key, value)),
				_ => Err("failed to decode map prefix"),
			}
		})
		.collect::<Result<Vec<(K, V)>, &'static str>>()
}

/// Get the latest finalized head of the chain.
///
/// This is technically not a storage operation but RPC, but we will keep it here since it is very
/// useful in lots of places.
pub async fn get_head(client: &Client) -> Hash {
	let data: Option<StorageData> = client
		.request("chain_getFinalizedHead", Params::None)
		.await
		.expect("get chain finalized head request failed");
	let now_raw = data.expect("Should always get the head hash").0;
	<Hash as Decode>::decode(&mut &*now_raw).expect("Block hash should decode")
}

/// Get the metadata of a chain.
pub async fn get_metadata(client: &Client, at: Hash) -> sp_core::Bytes {
	let at = to_json_value(at).expect("Block hash serialization infallible");
	let data: Option<sp_core::Bytes> = client
		.request("state_getMetadata", Params::Array(vec![at]))
		.await
		.expect("Failed to decode block");

	data.unwrap()
}

/// Get the runtime version at the given block.
pub async fn get_runtime_version(client: &Client, at: Hash) -> sp_version::RuntimeVersion {
	let at = to_json_value(at).expect("Block hash serialization infallible");
	let data: Option<sp_version::RuntimeVersion> = client
		.request("state_getRuntimeVersion", Params::Array(vec![at]))
		.await
		.expect("Failed to decode block");

	data.unwrap()
}

/// Get the size of a storage map.
pub async fn get_storage_size(key: StorageKey, client: &Client, at: Hash) -> Option<u64> {
	let at = to_json_value(at).expect("Block hash serialization infallible");
	let key = to_json_value(key).expect("extrinsic serialization infallible");
	client
		.request("state_getStorageSize", Params::Array(vec![key, at]))
		.await
		.unwrap()
}

#[cfg(test)]
mod tests {
	use super::*;
	use async_std::task::block_on;
	use jsonrpsee::{raw::RawClient, transport::ws::WsTransportClient, Client};

	use frame_system::AccountInfo;
	use node_primitives::{Balance, Nonce};
	use pallet_balances::AccountData;

	const TEST_URI: &'static str = "wss://kusama-rpc.polkadot.io/";

	async fn build_client() -> Client {
		let transport = WsTransportClient::new(TEST_URI)
			.await
			.expect("Failed to connect to client");
		RawClient::new(transport).into()
	}

	#[test]
	fn storage_value_read_works() {
		let client = block_on(build_client());
		let at = block_on(get_head(&client));
		let key = value_key(b"Balances", b"TotalIssuance");
		let issuance = block_on(read::<Balance>(key, &client, at));
		assert!(issuance.is_some());
	}

	#[test]
	fn storage_map_read_works() {
		let client = block_on(build_client());
		let at = block_on(get_head(&client));
		// web3 foundation technical account.
		let account =
			hex_literal::hex!["8a0e42d190d3ecaebf11d3834f4b992e0fab469e6bf17056d402cb172b827a22"];
		let data = block_on(read::<AccountInfo<Nonce, AccountData<Balance>>>(
			map_key::<frame_support::Blake2_128Concat>(b"System", b"Account", &account),
			&client,
			at,
		));
		assert!(data.is_some());
	}
}

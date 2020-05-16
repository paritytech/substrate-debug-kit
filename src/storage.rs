//! Some helpers to read storage.

use crate::primitives::{twox_128, Hash};
use codec::Decode;
use frame_support::StorageHasher;
use jsonrpsee::{
	common::{to_value as to_json_value, Params},
	Client,
};
use sp_core::storage::{StorageData, StorageKey};
use std::fmt::Debug;

type StorageKeyPair = Vec<(StorageKey, StorageData)>;

/// create key for a simple value.
pub fn value_key(module: &[u8], storage: &[u8]) -> StorageKey {
	let mut final_key = [0u8; 32];
	final_key[0..16].copy_from_slice(&twox_128(module));
	final_key[16..32].copy_from_slice(&twox_128(storage));
	StorageKey(final_key.to_vec())
}

/// create key for a map.
pub fn map_key<H: StorageHasher>(module: &[u8], storage: &[u8], encoded_key: &[u8]) -> StorageKey {
	let prefix = map_prefix_raw(module, storage);
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
	let prefix = map_prefix_raw(module, storage);
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
	StorageKey(map_prefix_raw(module, storage))
}

/// create key prefix for a map as a raw byte vec.
pub fn map_prefix_raw(module: &[u8], storage: &[u8]) -> Vec<u8> {
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
	let key = map_prefix_key(module.clone(), storage.clone());
	let serialized_key = to_json_value(key).expect("StorageKey serialization infallible");
	let at = to_json_value(at).expect("Block hash serialization infallible");
	let raw: StorageKeyPair = client
		.request("state_getPairs", Params::Array(vec![serialized_key, at]))
		.await
		.expect("Storage state_getPairs failed");

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

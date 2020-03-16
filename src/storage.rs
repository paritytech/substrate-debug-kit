//! Some helpers to read storage.

use std::fmt::Debug;
use jsonrpsee::{common::{to_value as to_json_value, Params}, Client};
use frame_support::storage::generator::Linkage;
use sp_core::storage::{StorageData, StorageKey};
use codec::Decode;
use crate::primitives::{Hash, blake2_256, twox_128};

/// create key for a simple value.
pub fn value(module: String, storage: String) -> StorageKey {
	let mut final_key = [0u8; 32];
	final_key[0..16].copy_from_slice(&twox_128(module.as_bytes()));
	final_key[16..32].copy_from_slice(&twox_128(storage.as_bytes()));
	StorageKey(final_key.to_vec())
}

/// create key for a map.
pub fn map(module: String, storage: String, encoded_key: &[u8]) -> StorageKey {
	let module_key = twox_128(module.as_bytes());
	let storage_key = twox_128(storage.as_bytes());
	let key = blake2_256(encoded_key);
	let mut final_key = Vec::with_capacity(module_key.len() + storage_key.len() + key.len());
	final_key.extend_from_slice(&module_key);
	final_key.extend_from_slice(&storage_key);
	final_key.extend_from_slice(&key);
	StorageKey(final_key)
}

/// create key for a linked_map head.
pub fn linked_map_head(module: String, storage: String) -> StorageKey {
	let head_prefix = "HeadOf".to_string() + &storage;
	let mut final_key = [0u8; 32];
	final_key[0..16].copy_from_slice(&twox_128(module.as_bytes()));
	final_key[16..32].copy_from_slice(&twox_128(head_prefix.as_bytes()));
	StorageKey(final_key.to_vec())
}

/// Read from a raw key regardless of the type.
pub async fn read<T: Decode>(key: StorageKey, client: &Client, at: Hash) -> Option<T> {
	let serialized_key = to_json_value(key).expect("StorageKey serialization infallible");
	let at = to_json_value(at).expect("Block hash serialization infallible");
	let raw: Option<StorageData> =
		client.request("state_getStorage", Params::Array(vec![serialized_key, at]))
			.await
			.expect("Storage request failed");
	let encoded = raw.map(|d| d.0)?;
	<T as Decode>::decode(&mut encoded.as_slice()).ok()
}

/// enumerate and return all pairings of a linked map. Hopefully substrate will provide easier
/// ways of doing this in the future.
pub async fn enumerate_linked_map<K, T>(
	module: String,
	storage: String,
	client: &Client,
	at: Hash,
) -> Vec<(K, T)>
	where K: Decode + Debug + Clone + AsRef<[u8]>, T: Decode + Clone + Debug,
{
	let maybe_head_key = read::<K>(
		linked_map_head(
			module.clone(),
			storage.clone(),
		),
		&client,
		at,
	).await;

	if let Some(head_key) = maybe_head_key {
		let mut ptr = head_key;
		let mut enumerations = Vec::<(K, T)>::new();
		loop {
			let (next_value, next_key) = read::<(T, Linkage<K>)>(
				map(
					module.clone(),
					storage.clone(),
					ptr.as_ref(),
				),
				&client,
				at,
			).await.unwrap();

			enumerations.push((
				ptr,
				next_value,
			));

			if let Some(next) = next_key.next {
				ptr = next;
			} else {
				break;
			}
		}
		enumerations
	} else {
		vec![]
	}
}

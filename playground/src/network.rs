use ansi_term::{Colour, Style};
use codec::Decode;
use frame_support::{Blake2_128Concat, Twox64Concat};
use frame_system::AccountInfo;
use jsonrpsee::{
	common::{to_value as to_json_value, Params},
	Client,
};
use pallet_balances::AccountData;
use pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo;
use sp_core::storage::{StorageData, StorageKey};
use sp_runtime::traits::Convert;
use sub_storage::primitives::*;

#[cfg(feature = "kusama")]
pub use kusama_runtime as runtime;
#[cfg(feature = "polkadot")]
pub use polkadot_runtime as runtime;

/// Get the block at a particular hash
pub async fn get_block(client: &Client, at: Hash) -> runtime::SignedBlock {
	let at = to_json_value(at).expect("Block hash serialization infallible");
	let data: Option<runtime::SignedBlock> = client
		.request("chain_getBlock", Params::Array(vec![at]))
		.await
		.expect("Failed to decode block");

	data.unwrap()
}

/// Get the extrinsic info
pub async fn query_info(
	extrinsic: sp_core::Bytes,
	client: &Client,
	at: Hash,
) -> RuntimeDispatchInfo<Balance> {
	let at = to_json_value(at).expect("Block hash serialization infallible");
	let extrinsic = to_json_value(extrinsic).expect("extrinsic serialization infallible");
	let data: Option<RuntimeDispatchInfo<Balance>> = client
		.request("payment_queryInfo", Params::Array(vec![extrinsic, at]))
		.await
		.unwrap();

	data.unwrap()
}

/// Get the events at the given block.
pub async fn get_events_at(
	client: &Client,
	at: Hash,
) -> Option<Vec<frame_system::EventRecord<runtime::Event, Hash>>> {
	let key = sub_storage::value_key(b"System", b"Events");
	sub_storage::read(key, client, at).await
}

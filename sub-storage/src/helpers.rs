//! Some helper functions for common substrate chains.

use ansi_term::Colour;
use codec::Decode;
use frame_support::{Blake2_128Concat, Twox64Concat};
use frame_system::AccountInfo;
use jsonrpsee::common::{to_value as to_json_value, Params};
use jsonrpsee::Client;
use node_primitives::{AccountId, Balance, Hash, Nonce};
use pallet_balances::AccountData;
use sp_core::storage::{StorageData, StorageKey};

/// Get the nick of a given account id.
pub async fn get_nick(who: &AccountId, client: &Client, at: Hash) -> String {
	let nick = crate::read::<(Vec<u8>, Balance)>(
		crate::map_key::<Twox64Concat>(b"Nicks", b"NameOf", who.as_ref()),
		client,
		at,
	)
	.await;

	if nick.is_some() {
		String::from_utf8(nick.unwrap().0).unwrap()
	} else {
		String::from("[NO_NICK]")
	}
}

/// Get the identity of an account.
pub async fn get_identity(who: &AccountId, client: &Client, at: Hash) -> String {
	use pallet_identity::{Data, Registration};

	let maybe_subidentity = crate::read::<(AccountId, Data)>(
		crate::map_key::<Blake2_128Concat>(b"Identity", b"SuperOf", who.as_ref()),
		client,
		at,
	)
	.await;

	let maybe_identity = crate::read::<Registration<Balance>>(
		crate::map_key::<Twox64Concat>(
			b"Identity",
			b"IdentityOf",
			maybe_subidentity
				.as_ref()
				.map_or(who.as_ref(), |x| x.0.as_ref()),
		),
		client,
		at,
	)
	.await;

	if let Some(identity) = maybe_identity {
		let info = identity.info;
		let display = info.display;

		let result = match display {
			Data::Raw(bytes) => format!(
				"{}",
				Colour::Yellow
					.bold()
					.paint(String::from_utf8(bytes).expect("Identity not utf-8"))
			),
			_ => format!("{}", Colour::Red.bold().paint("???")),
		};
		if let Some(sub_identity) = maybe_subidentity {
			match sub_identity.1 {
				Data::Raw(bytes) => format!(
					"{} ({})",
					result,
					Colour::Yellow.paint(String::from_utf8(bytes).expect("Identity not utf-8"))
				),
				_ => format!("{}", Colour::Red.paint("???")),
			}
		} else {
			result
		}
	} else {
		"NO_IDENT".to_string()
	}
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

/// Get the account data at the given block.
pub async fn get_account_data_at(
	account: &AccountId,
	client: &Client,
	at: Hash,
) -> AccountInfo<Nonce, AccountData<Balance>> {
	crate::read::<AccountInfo<Nonce, AccountData<Balance>>>(
		crate::map_key::<Blake2_128Concat>(b"System", b"Account", account.as_ref()),
		client,
		at,
	)
	.await
	.unwrap()
}

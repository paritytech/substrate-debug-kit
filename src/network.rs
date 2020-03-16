use jsonrpsee::common::Params;
use codec::Decode;
use sp_runtime::traits::Convert;
use sp_core::storage::StorageData;
use crate::primitives::{Balance, AccountId, Hash};
use crate::{storage, Client};

// Total issuance.
pub static mut ISSUANCE: *mut u128 = 0 as *mut u128;

/// a way to attach the total issuance to `CurrencyToVoteHandler`.
pub trait GetTotalIssuance {
	fn get_total_issuance() -> Balance;
}

/// Something that holds the total issuance.
pub struct TotalIssuance;

impl GetTotalIssuance for TotalIssuance {
	fn get_total_issuance() -> Balance {
		unsafe {
			*ISSUANCE
		}
	}
}

pub struct CurrencyToVoteHandler<T>(std::marker::PhantomData<T>);
impl<T: GetTotalIssuance> CurrencyToVoteHandler<T> {
	fn factor() -> u128 {
		(T::get_total_issuance() / u64::max_value() as u128).max(1)
	}
}

impl<T: GetTotalIssuance> Convert<u128, u64> for CurrencyToVoteHandler<T> {
	fn convert(x: Balance) -> u64 { (x / Self::factor()) as u64 }
}

impl<T: GetTotalIssuance> Convert<u128, u128> for CurrencyToVoteHandler<T> {
	fn convert(x: u128) -> Balance { x * Self::factor() }
}

pub async fn get_nick(who: &AccountId, client: &Client, at: Hash) -> String {
	let nick = storage::read::<(Vec<u8>, Balance)>(
		storage::map("Sudo".to_string(), "NameOf".to_string(), who.as_ref()),
		client,
		at,
	).await;

	if nick.is_some() {
		String::from_utf8(nick.unwrap().0).unwrap()
	} else {
		String::from("NO_NICK")
	}
}

pub async fn get_head(client: &Client) -> Hash {
	let data: Option<StorageData> = client.request("chain_getFinalizedHead", Params::None)
		.await
		.expect("Storage request failed");
	let now_raw = data.expect("Should always get the head hash").0;
	<Hash as Decode>::decode(&mut &*now_raw).expect("Block hash should decode")
}

pub async fn get_total_issuance(client: &Client, at: Hash) -> Balance {
	let maybe_total_issuance = storage::read::<Balance>(
		storage::value(
			"Balances".to_string(),
			"TotalIssuance".to_string()
		),
		&client,
		at,
	).await;

	maybe_total_issuance.unwrap_or(0)
}

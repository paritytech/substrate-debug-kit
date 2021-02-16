use crate::{
	primitives::{AccountId, Balance, Hash},
	storage, Client,
};
use atomic_refcell::AtomicRefCell as RefCell;
use codec::Encode;
use sp_runtime::traits::Convert;
static ISSUANCE: RefCell<Balance> = RefCell::new(0);

/// Deals with total issuance
pub mod issuance {
	use super::{get_total_issuance, ISSUANCE};
	use crate::{Balance, Client, Hash};

	/// Get the previously set total issuance.
	pub fn get() -> Balance {
		ISSUANCE.borrow().clone()
	}

	/// Set the total issuance. Any code wanting to use `CurrencyToVoteHandler` must call this first
	/// to set correct value in the global pointer.
	pub async fn set(client: &Client, at: Hash) {
		let total_issuance = get_total_issuance(client, at).await;
		*ISSUANCE.borrow_mut() = total_issuance;
	}
}

pub struct CurrencyToVoteHandler;
impl CurrencyToVoteHandler {
	fn factor() -> u128 {
		(issuance::get() / u64::max_value() as u128).max(1)
	}
}

impl Convert<u128, u64> for CurrencyToVoteHandler {
	fn convert(x: Balance) -> u64 {
		(x / Self::factor()) as u64
	}
}

impl Convert<u128, u128> for CurrencyToVoteHandler {
	fn convert(x: u128) -> Balance {
		x * Self::factor()
	}
}

/// Get total issuance of the chain.
async fn get_total_issuance(client: &Client, at: Hash) -> Balance {
	let maybe_total_issuance =
		storage::read::<Balance>(storage::value_key(b"Balances", b"TotalIssuance"), &client, at)
			.await;

	maybe_total_issuance.unwrap_or(0)
}

pub async fn get_validators_and_expo_at(
	client: &Client,
	at: Hash,
) -> (pallet_staking::EraIndex, Vec<(AccountId, pallet_staking::Exposure<AccountId, Balance>)>) {
	use frame_support::Twox64Concat;
	let validators = sub_storage::read::<Vec<crate::primitives::AccountId>>(
		sub_storage::value_key(b"Session", b"Validators"),
		&client,
		at,
	)
	.await
	.expect("Validators must exist at each block.");

	let era = sub_storage::read::<pallet_staking::ActiveEraInfo>(
		sub_storage::value_key(b"Staking", b"ActiveEra"),
		client,
		at,
	)
	.await
	.expect("Current era must exist at the given block.");

	let era = era.index;

	let mut validators_and_expo = vec![];

	for v in validators.into_iter() {
		let expo = sub_storage::read::<pallet_staking::Exposure<AccountId, Balance>>(
			sub_storage::double_map_key::<Twox64Concat, Twox64Concat>(
				b"Staking",
				b"ErasStakers",
				era.encode().as_ref(),
				v.as_ref(),
			),
			client,
			at,
		)
		.await
		.expect("Staker at era must have exposure");

		validators_and_expo.push((v, expo))
	}

	(era, validators_and_expo)
}

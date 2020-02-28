//! Helpers to read staking module.

use pallet_staking::{ValidatorPrefs, Nominations, StakingLedger, Exposure};
use pallet_staking::slashing::{SlashingSpans};
use crate::{
	storage, Client, Staker,
	primitives::{AccountId, Balance, Hash},
};

const MODULE: &'static str = "Staking";

pub async fn get_candidates(client: &Client, at: Hash) -> Vec<AccountId> {
	storage::enumerate_linked_map::<
		AccountId,
		ValidatorPrefs,
	>(
		MODULE.to_string(),
		"Validators".to_string(),
		client,
		at,
	).await.into_iter().map(|(v, _p)| v).collect::<Vec<AccountId>>()
}

pub async fn get_voters(client: &Client, at: Hash) -> Vec<(AccountId, Vec<AccountId>)> {
	let nominators: Vec<(AccountId, Nominations<AccountId>)> = storage::enumerate_linked_map::<
		AccountId,
		Nominations<AccountId>,
	>(
		MODULE.to_string(),
		"Nominators".to_string(),
		client,
		at,
	).await;

	nominators
		.into_iter()
		.enumerate()
		.map(|(idx, (who, n))| {
			let submitted_in = n.submitted_in;
			let initial_len = n.targets.len();
			let mut targets = n.targets;
			targets.retain(|target| {
				let maybe_slashing_spans = async_std::task::block_on(
					slashing_span_of(&target, client, at)
				);
				maybe_slashing_spans.map_or(
					true,
					|spans| submitted_in >= spans.last_nonzero_slash(),
				)
			});
			log::trace!(
				target: "staking",
				"[{}] retaining {}/{} nominations for {:?}",
				idx,
				targets.len(),
				initial_len,
				who,
			);
			(who, targets)
		})
		.collect::<Vec<(AccountId, Vec<AccountId>)>>()
}

pub async fn get_staker_info_entry(stash: &AccountId, client: &Client, at: Hash) -> Staker {
	let ctrl = storage::read::<AccountId>(
		storage::map(MODULE.to_string(), "Bonded".to_string(), stash.as_ref()),
		&client,
		at,
	).await.expect("All stakers must have a ctrl.");

	let ledger = storage::read::<StakingLedger<AccountId, Balance>>(
		storage::map(MODULE.to_string(), "Ledger".to_string(), ctrl.as_ref()),
		&client,
		at,
	).await.expect("All stakers must have a ledger.");

	Staker { ctrl: Some(ctrl), stake: ledger.active }
}

pub async fn slashing_span_of(stash: &AccountId, client: &Client, at: Hash)
	-> Option<SlashingSpans>
{
	storage::read::<SlashingSpans>(
		storage::map(MODULE.to_string(), "SlashingSpans".to_string(), stash.as_ref()),
		&client,
		at,
	).await
}

pub async fn exposure_of(stash: &AccountId, client: &Client, at: Hash)
	-> Exposure<AccountId, Balance>
{
	storage::read::<Exposure<AccountId, Balance>>(
		storage::map(MODULE.to_string(), "Stakers".to_string(), stash.as_ref()),
		&client,
		at,
	).await.unwrap_or_default()
}

pub async fn create_snapshot_nominators(client: &Client, at: Hash) -> Vec<AccountId> {
	storage::enumerate_linked_map::<
		AccountId,
		Nominations<AccountId>,
	>(
		MODULE.to_string(),
		"Nominators".to_string(),
		client,
		at,
	).await.iter().map(|(who, _)| who.clone()).collect()
}

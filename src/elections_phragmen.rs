
use crate::primitives::{Balance, Hash, AccountId};
use crate::{storage, Staker, Client};
const MODULE: &'static str = "PhragmenElection";

pub async fn get_candidates(client: &Client, at: Hash) -> Vec<AccountId> {
	let mut members = storage::read::<Vec<(AccountId, Balance)>>(
		storage::value(MODULE.to_string(), "Members".to_string()),
		client,
		at,
	).await.unwrap_or_default().into_iter().map(|(m, _)| m).collect::<Vec<AccountId>>();

	let runners = storage::read::<Vec<(AccountId, Balance)>>(
		storage::value(MODULE.to_string(), "RunnersUp".to_string()),
		client,
		at,
	).await.unwrap_or_default().into_iter().map(|(m, _)| m).collect::<Vec<AccountId>>();

	let candidates = storage::read::<Vec<AccountId>>(
		storage::value(MODULE.to_string(), "Candidates".to_string()),
		client,
		at,
	).await.unwrap_or_default();

	members.extend(candidates);
	members.extend(runners);

	members
}

pub async fn get_voters(client: &Client, at: Hash) -> Vec<(AccountId, Vec<AccountId>)> {
	storage::enumerate_linked_map::<
		AccountId,
		Vec<AccountId>,
	>(
		MODULE.to_string(),
		"VotesOf".to_string(),
		client,
		at,
	)
		.await
		.into_iter()
		.collect::<Vec<(AccountId, Vec<AccountId>)>>()
}

pub async fn get_staker_info_entry(voter: &AccountId, client: &Client, at: Hash) -> Staker {
	let stake = storage::read::<Balance>(
		storage::map(MODULE.to_string(), "StakeOf".to_string(), voter.as_ref()),
		&client,
		at,
	).await.unwrap_or_default();

	Staker { ctrl: None, stake }
}

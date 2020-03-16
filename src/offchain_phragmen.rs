use crate::{
	primitives::{AccountId, Hash, Balance},
	network, staking, Client, t_start, t_stop, Staker,
};
use std::convert::TryInto;
use std::collections::BTreeMap;
use pallet_staking::CompactAssignments as Compact;
use sp_phragmen::{PhragmenResult, build_support_map, evaluate_support, elect, reduce};

pub async fn prepare_offchain_submission(
	validator_count: usize,
	minimum_validator_count: usize,
	candidates: Vec<AccountId>,
	voters: Vec<(AccountId, Vec<AccountId>)>,
	staker_infos: BTreeMap<AccountId, Staker>,
	client: &Client,
	at: Hash,
) -> Compact {
	let slashable_balance = |who: &AccountId| -> Balance {
		staker_infos.get(who).unwrap().stake
	};

	t_start!(prepare_solution);
	let PhragmenResult { winners, assignments } = elect::<
		AccountId,
		Balance,
		_,
		network::CurrencyToVoteHandler<network::TotalIssuance>,
		pallet_staking::OffchainAccuracy,
	>(
		validator_count,
		minimum_validator_count,
		candidates,
		voters,
		slashable_balance,
	).ok_or("Phragmen failed to elect.").unwrap();

	let mut snapshot_nominators = staking::create_snapshot_nominators(&client, at).await;
	let snapshot_validators = staking::get_candidates(&client, at).await;
	snapshot_nominators.extend(snapshot_validators.clone());

	// all helper closures
	let nominator_index = |a: &AccountId| -> Option<pallet_staking::NominatorIndex> {
		snapshot_nominators.iter().position(|x| x == a).and_then(|i|
			<usize as TryInto<pallet_staking::NominatorIndex>>::try_into(i).ok()
		)
	};
	let validator_index = |a: &AccountId| -> Option<pallet_staking::ValidatorIndex> {
		snapshot_validators.iter().position(|x| x == a).and_then(|i|
			<usize as TryInto<pallet_staking::ValidatorIndex>>::try_into(i).ok()
		)
	};
	let slashable_balance_of_extended = |who: &AccountId| -> u128 {
		slashable_balance(who) as u128
	};

	// Clean winners.
	let winners = winners.into_iter().map(|(w, _)| w).collect::<Vec<AccountId>>();

	// convert into absolute value and to obtain the reduced version.
	let mut staked = sp_phragmen::assignment_ratio_to_staked(
		assignments,
		slashable_balance_of_extended,
	);

	reduce(&mut staked);

	// Convert back to ratio assignment. This takes less space.
	let low_accuracy_assignment = sp_phragmen::assignment_staked_to_ratio(staked);

	let _score = {
		let staked = sp_phragmen::assignment_ratio_to_staked(
			low_accuracy_assignment.clone(),
			slashable_balance_of_extended,
		);

		let (support_map, _) = build_support_map::<AccountId>(
			winners.as_slice(),
			staked.as_slice(),
		);
		evaluate_support::<AccountId>(&support_map)
	};

	// compact encode the assignment.
	let compact = pallet_staking::CompactAssignments::from_assignment(
		low_accuracy_assignment,
		nominator_index,
		validator_index,
	).unwrap();

	// winners to index.
	let _winners = winners.into_iter().map(|w|
		snapshot_validators.iter().position(|v| *v == w).unwrap().try_into().unwrap()
	).collect::<Vec<pallet_staking::ValidatorIndex>>();
	t_stop!(prepare_solution);

	compact
}

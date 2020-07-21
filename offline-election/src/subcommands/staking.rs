//! Helpers to read staking module.

use crate::{
	network,
	primitives::{AccountId, Balance, Hash},
	storage, Client, Currency, Opt, StakingConfig, LOG_TARGET,
};
use codec::Encode;
use pallet_staking::slashing::SlashingSpans;
use pallet_staking::{EraIndex, Exposure, Nominations, StakingLedger, ValidatorPrefs};
use sp_npos_elections::*;
use sp_runtime::traits::Convert;
use std::collections::BTreeMap;
use std::convert::TryInto;

const MODULE: &[u8] = b"Staking";

/// A staker
#[derive(Debug, Clone, Default)]
struct Staker {
	ctrl: Option<AccountId>,
	stake: Balance,
}

fn assert_supports_total_equal(s1: &SupportMap<AccountId>, s2: &SupportMap<AccountId>) {
	assert!(s1.iter().all(|(v, s)| s2.get(v).unwrap().total == s.total))
}

async fn get_current_era(client: &Client, at: Hash) -> EraIndex {
	storage::read::<EraIndex>(storage::value_key(MODULE, b"CurrentEra"), client, at)
		.await
		.expect("CurrentEra must exist")
}

async fn get_candidates(client: &Client, at: Hash) -> Vec<AccountId> {
	storage::enumerate_map::<AccountId, ValidatorPrefs>(MODULE, b"Validators", client, at)
		.await
		.expect("Staking::validators should be enumerable.")
		.into_iter()
		.map(|(v, _p)| v)
		.collect::<Vec<AccountId>>()
}

async fn get_voters(client: &Client, at: Hash) -> Vec<(AccountId, Vec<AccountId>)> {
	let nominators: Vec<(AccountId, Nominations<AccountId>)> = storage::enumerate_map::<
		AccountId,
		Nominations<AccountId>,
	>(MODULE, b"Nominators", client, at)
	.await
	.expect("Staking::nominators should be enumerable");

	let mut result = vec![];
	for (idx, (who, n)) in nominators.into_iter().enumerate() {
		// retain only targets who have not been yet slashed recently. This is highly dependent
		// on the staking implementation.
		let submitted_in = n.submitted_in;
		let targets = n.targets;
		let mut filtered_targets = vec![];
		// TODO: move back to closures and retain, but async-std::block_on can't work well here for
		// whatever reason. Or move to streams?
		for target in targets.iter() {
			let maybe_slashing_spans = slashing_span_of(&target, client, at).await;
			if maybe_slashing_spans.map_or(true, |spans| submitted_in >= spans.last_nonzero_slash())
			{
				filtered_targets.push(target.clone());
			}
		}

		log::trace!(
			target: LOG_TARGET,
			"[{}] retaining {}/{} nominations for {:?}",
			idx,
			filtered_targets.len(),
			targets.len(),
			who,
		);

		result.push((who, targets));
	}

	result
}

async fn get_staker_info_entry(stash: &AccountId, client: &Client, at: Hash) -> Staker {
	let ctrl = storage::read::<AccountId>(
		storage::map_key::<frame_support::Twox64Concat>(MODULE, b"Bonded", stash.as_ref()),
		&client,
		at,
	)
	.await
	.expect("All stashes must have 'Bonded' storage.");

	let ledger = storage::read::<StakingLedger<AccountId, Balance>>(
		storage::map_key::<frame_support::Blake2_128Concat>(MODULE, b"Ledger", ctrl.as_ref()),
		&client,
		at,
	)
	.await
	.expect("All controllers must have a 'Ledger' storage");

	Staker {
		ctrl: Some(ctrl),
		stake: ledger.active,
	}
}

/// Get the slashing span of a voter stash.
pub async fn slashing_span_of(
	stash: &AccountId,
	client: &Client,
	at: Hash,
) -> Option<SlashingSpans> {
	storage::read::<SlashingSpans>(
		storage::map_key::<frame_support::Twox64Concat>(MODULE, b"SlashingSpans", stash.as_ref()),
		&client,
		at,
	)
	.await
}

async fn exposure_of(
	stash: &AccountId,
	era: EraIndex,
	client: &Client,
	at: Hash,
) -> Exposure<AccountId, Balance> {
	storage::read::<Exposure<AccountId, Balance>>(
		storage::double_map_key::<frame_support::Twox64Concat, frame_support::Twox64Concat>(
			MODULE,
			b"ErasStakers",
			era.encode().as_ref(),
			stash.as_ref(),
		),
		&client,
		at,
	)
	.await
	.unwrap_or_default()
}

async fn get_validator_count(client: &Client, at: Hash) -> u32 {
	storage::read::<u32>(storage::value_key(MODULE, b"ValidatorCount"), client, at)
		.await
		.unwrap_or(50)
}

async fn create_snapshot_nominators(client: &Client, at: Hash) -> Vec<AccountId> {
	storage::enumerate_map::<AccountId, Nominations<AccountId>>(MODULE, b"Nominators", client, at)
		.await
		.unwrap()
		.iter()
		.map(|(who, _)| who.clone())
		.collect()
}

async fn prepare_offchain_submission(
	count: usize,
	min_count: usize,
	candidates: Vec<AccountId>,
	voters: Vec<(AccountId, Vec<AccountId>)>,
	staker_infos: BTreeMap<AccountId, Staker>,
	client: &Client,
	at: Hash,
) -> pallet_staking::CompactAssignments {
	let slashable_balance = |who: &AccountId| -> Balance { staker_infos.get(who).unwrap().stake };
	let slashable_balance_votes = |who: &AccountId| -> VoteWeight {
		<network::CurrencyToVoteHandler as Convert<Balance, VoteWeight>>::convert(
			slashable_balance(who),
		)
	};

	t_start!(prepare_solution);
	let ElectionResult {
		winners,
		assignments,
	} = seq_phragmen::<AccountId, pallet_staking::OffchainAccuracy>(
		count,
		min_count,
		candidates,
		voters
			.iter()
			.cloned()
			.map(|(v, t)| (v.clone(), slashable_balance_votes(&v), t))
			.collect::<Vec<_>>(),
	)
	.expect("Phragmen failed to elect.");

	let mut snapshot_nominators = create_snapshot_nominators(&client, at).await;
	let snapshot_validators = get_candidates(&client, at).await;
	snapshot_nominators.extend(snapshot_validators.clone());

	// all helper closures
	let nominator_index = |a: &AccountId| -> Option<pallet_staking::NominatorIndex> {
		snapshot_nominators
			.iter()
			.position(|x| x == a)
			.and_then(|i| <usize as TryInto<pallet_staking::NominatorIndex>>::try_into(i).ok())
	};
	let validator_index = |a: &AccountId| -> Option<pallet_staking::ValidatorIndex> {
		snapshot_validators
			.iter()
			.position(|x| x == a)
			.and_then(|i| <usize as TryInto<pallet_staking::ValidatorIndex>>::try_into(i).ok())
	};

	// Clean winners.
	let winners = winners
		.into_iter()
		.map(|(w, _)| w)
		.collect::<Vec<AccountId>>();

	// convert into absolute value and to obtain the reduced version.
	let mut staked = assignment_ratio_to_staked(assignments, slashable_balance_votes);

	reduce(&mut staked);

	// Convert back to ratio assignment. This takes less space.
	let low_accuracy_assignment = assignment_staked_to_ratio(staked);

	let _score = {
		let staked =
			assignment_ratio_to_staked(low_accuracy_assignment.clone(), slashable_balance_votes);

		let (support_map, _) =
			build_support_map::<AccountId>(winners.as_slice(), staked.as_slice());
		evaluate_support::<AccountId>(&support_map)
	};

	// compact encode the assignment.
	let compact = pallet_staking::CompactAssignments::from_assignment(
		low_accuracy_assignment,
		nominator_index,
		validator_index,
	)
	.unwrap();

	// winners to index.
	let _winners = winners
		.into_iter()
		.map(|w| {
			snapshot_validators
				.iter()
				.position(|v| *v == w)
				.unwrap()
				.try_into()
				.unwrap()
		})
		.collect::<Vec<pallet_staking::ValidatorIndex>>();
	t_stop!(prepare_solution);

	compact
}

/// Main run function of the sub-command.
pub async fn run(client: &Client, opt: Opt, conf: StakingConfig) {
	let at = opt.at.unwrap();
	let val_count = get_validator_count(&client, at).await as usize;
	let verbosity = opt.verbosity;
	let iterations = conf.iterations;
	let count = conf.count.unwrap_or(val_count);
	let reduce = conf.reduce;

	if count != val_count {
		log::warn!(
			target: LOG_TARGET,
			"`count` provided ({:?}) differs from validator count on-chain ({}).",
			count,
			val_count,
		);
	}

	t_start!(data_scrape);
	// stash key of all wannabe candidates.
	let candidates = get_candidates(client, at).await;

	// stash key of current voters, including maybe self vote.
	let mut all_voters = get_voters(&client, at).await;

	// add self-vote
	candidates.iter().for_each(|v| {
		let self_vote = (v.clone(), vec![v.clone()]);
		all_voters.push(self_vote);
	});

	// get the slashable balance of every entity
	let mut staker_infos: BTreeMap<AccountId, Staker> = BTreeMap::new();

	for stash in candidates.iter().chain(all_voters.iter().map(|(s, _)| s)) {
		let staker_info = get_staker_info_entry(&stash, &client, at).await;
		staker_infos.insert(stash.clone(), staker_info);
	}
	t_stop!(data_scrape);

	let slashable_balance = |who: &AccountId| -> Balance { staker_infos.get(who).unwrap().stake };
	let slashable_balance_votes = |who: &AccountId| -> VoteWeight {
		<network::CurrencyToVoteHandler as Convert<Balance, VoteWeight>>::convert(
			slashable_balance(who),
		)
	};

	// run phragmen
	t_start!(phragmen_run);
	let ElectionResult {
		winners,
		assignments,
	} = seq_phragmen::<AccountId, pallet_staking::ChainAccuracy>(
		count,
		0,
		candidates.clone(),
		all_voters
			.iter()
			.cloned()
			.map(|(v, t)| (v.clone(), slashable_balance_votes(&v), t))
			.collect::<Vec<_>>(),
	)
	.expect("Phragmen failed to elect.");
	t_stop!(phragmen_run);

	let elected_stashes = winners
		.iter()
		.map(|(s, _)| s.clone())
		.collect::<Vec<AccountId>>();

	t_start!(ratio_into_staked_run);
	let mut staked_assignments =
		assignment_ratio_to_staked(assignments.clone(), slashable_balance_votes);
	t_stop!(ratio_into_staked_run);

	t_start!(build_support_map_run);
	let (mut supports, _) =
		build_support_map::<AccountId>(&elected_stashes, staked_assignments.as_slice());
	t_stop!(build_support_map_run);

	let mut initial_score = evaluate_support(&supports);

	if iterations > 0 {
		// prepare and run post-processing.
		t_start!(equalize_post_processing);
		let done = balance_solution(&mut staked_assignments, &mut supports, 0, iterations);
		t_stop!(equalize_post_processing);
		let improved_score = evaluate_support(&supports);
		log::info!(
			target: LOG_TARGET,
			"Balanced the results for [{}/{}] iterations, improved slot stake by {:?}",
			done,
			iterations,
			Currency(
				improved_score[0]
					.checked_sub(initial_score[0])
					.unwrap_or_else(|| {
						log::error!(
							target: LOG_TARGET,
							"Balancing has returned a set which has a lower slot stake. This is most likely a serious bug.",
						);
						0
					})
			),
		);
		initial_score = improved_score;
	}

	if reduce {
		t_start!(reducing_solution);
		sp_npos_elections::reduce(&mut staked_assignments);
		t_stop!(reducing_solution);
		// just to check that support has NOT changed
		let (support_after_reduce, _) =
			build_support_map::<AccountId>(&elected_stashes, staked_assignments.as_slice());
		assert_supports_total_equal(&support_after_reduce, &supports);
		supports = support_after_reduce;
	}

	let mut nominator_info: BTreeMap<AccountId, Vec<(AccountId, Balance)>> = BTreeMap::new();

	// only useful if we do check exposure.
	let mut mismatch = 0usize;
	let era = match 0 {
		0 => get_current_era(client, at).await,
		era @ _ => era,
	};
	// TODO: remove this or fix it.
	if false {
		log::debug!(
			target: LOG_TARGET,
			"checking exposures against era index {}",
			era
		);
	}

	log::info!(target: LOG_TARGET, "💸 Winner Validators:");
	for (i, (s, _)) in winners.iter().enumerate() {
		let support = supports.get(&s).unwrap();
		let other_count = support.voters.len();
		let self_stake = support
			.voters
			.iter()
			.filter(|(v, _)| v == s)
			.collect::<Vec<_>>();
		assert!(self_stake.len() == 1);
		println!(
			"#{} --> {} [{:?}] [total backing = {:?} ({} voters)] [own backing = {:?}]",
			i + 1,
			storage::helpers::get_identity::<AccountId, Balance>(s.as_ref(), &client, at).await,
			s,
			Currency(support.total),
			other_count,
			Currency(self_stake[0].1),
		);

		if verbosity >= 1 {
			println!("  Voters:");
			support.voters.iter().enumerate().for_each(|(i, o)| {
				println!(
					"    {}#{} [amount = {:?}] {:?}",
					if *s == o.0 { "*" } else { "" },
					i + 1,
					Currency(o.1),
					o.0
				);
				nominator_info
					.entry(o.0.clone())
					.or_insert(vec![])
					.push((s.clone(), o.1));
			});
			println!("");
		}

		if false {
			let expo = exposure_of(&s, era, &client, at).await;
			if support.total != expo.total {
				mismatch += 1;
				log::warn!(
					target: LOG_TARGET,
					"exposure mismatch with on-chain data, expo.total = {:?} - support.total = {:?} diff = {}",
					expo.total,
					support.total,
					if support.total > expo.total {
						format!("+{}", Currency(support.total - expo.total))
					} else {
						format!("-{}", Currency(expo.total - support.total))
					}
				);
			}
		}
	}
	if mismatch > 0 {
		log::error!("{} exposure mismatches found.", mismatch);
	}

	if verbosity >= 2 {
		log::info!("💰 Nominator Assignments:");
		let mut counter = 1;
		for (nominator, info) in nominator_info.iter() {
			let staker_info = staker_infos.get(&nominator).unwrap();
			let mut sum = 0;
			println!(
				"#{} {:?} // active_stake = {:?}",
				counter,
				nominator,
				Currency(staker_info.stake),
			);
			println!("  Distributions:");
			info.iter().enumerate().for_each(|(i, (c, s))| {
				sum += *s;
				println!("    #{} {:?} => {:?}", i, c, Currency(*s));
			});
			counter += 1;
			let diff = sum.max(staker_info.stake) - sum.min(staker_info.stake);
			// acceptable diff is one millionth of a Currency
			assert!(
				diff < 1_000,
				"diff( sum_nominations,  staker_info.ledger.active) = {}",
				diff
			);
			println!("");
		}
	}

	let compact = prepare_offchain_submission(
		count,
		0,
		candidates.clone(),
		all_voters.clone(),
		staker_infos.clone(),
		&client,
		at,
	)
	.await;

	log::info!(
		target: LOG_TARGET,
		"validator intentions count {:?}",
		candidates.len(),
	);
	log::info!(
		target: LOG_TARGET,
		"nominator intentions count {:?}",
		all_voters.len() - candidates.len(),
	);
	log::info!(
		target: LOG_TARGET,
		"solution score {:?}",
		initial_score
			.iter()
			.map(|n| format!("{:?}", Currency(*n)))
			.collect::<Vec<_>>(),
	);
	log::info!(
		target: LOG_TARGET,
		"Staking rate: {}%",
		initial_score[1] as f64 * 100f64 / network::issuance::get() as f64,
	);
	log::info!(
		target: LOG_TARGET,
		"Phragmen Assignment size {} bytes.",
		codec::Encode::encode(&assignments).len(),
	);
	log::info!(
		target: LOG_TARGET,
		"Phragmen compact Assignment size {} bytes.",
		codec::Encode::encode(&compact).len(),
	);

	// potentially write to json file
	if let Some(output_file) = conf.output {
		use std::fs::File;

		let output = serde_json::json!({
			"supports": supports,
			"winners": elected_stashes,
		});

		serde_json::to_writer_pretty(&File::create(output_file).unwrap(), &output).unwrap();
	}
}

//! Helpers to read staking module.

use crate::{
	network,
	primitives::{AccountId, Balance, Hash},
	storage, Client, Currency, Opt, StakingConfig, LOG_TARGET,
};
use codec::Encode;
use pallet_staking::{
	slashing::SlashingSpans, EraIndex, Exposure, Nominations, StakingLedger, ValidatorPrefs,
};
use sp_npos_elections::*;
use sp_runtime::traits::Convert;
use std::{collections::BTreeMap, convert::TryInto};

const MODULE: &[u8] = b"Staking";

// TODO: remove and use the new one once runtime 0.29 is there.
#[derive(codec::Decode, Clone, Debug)]
struct OldValidatorPrefs {
	#[codec(compact)]
	pub commission: sp_runtime::Perbill,
}

fn assert_supports_total_equal(s1: &SupportMap<AccountId>, s2: &SupportMap<AccountId>) {
	assert!(s1.iter().all(|(v, s)| s2.get(v).unwrap().total == s.total))
}

/// Get the current era.
pub(crate) async fn get_current_era(client: &Client, at: Hash) -> EraIndex {
	storage::read::<EraIndex>(storage::value_key(MODULE, b"CurrentEra"), client, at)
		.await
		.expect("CurrentEra must exist")
}

async fn get_candidates(client: &Client, at: Hash) -> Vec<AccountId> {
	storage::enumerate_map::<AccountId, OldValidatorPrefs>(MODULE, b"Validators", client, at)
		.await
		.expect("Staking::validators should be enumerable.")
		.into_iter()
		.map(|(v, _p)| v)
		.collect::<Vec<AccountId>>()
}

async fn stake_of(stash: &AccountId, client: &Client, at: Hash) -> Balance {
	let ctrl = storage::read::<AccountId>(
		storage::map_key::<frame_support::Twox64Concat>(MODULE, b"Bonded", stash.as_ref()),
		&client,
		at,
	)
	.await
	.expect("All stashes must have 'Bonded' storage.");

	storage::read::<StakingLedger<AccountId, Balance>>(
		storage::map_key::<frame_support::Blake2_128Concat>(MODULE, b"Ledger", ctrl.as_ref()),
		&client,
		at,
	)
	.await
	.expect("All controllers must have a 'Ledger' storage")
	.active
}

async fn get_voters(client: &Client, at: Hash) -> Vec<(AccountId, VoteWeight, Vec<AccountId>)> {
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

		let stake = stake_of(&who, client, at).await;
		result.push((who, to_vote_weight(stake), targets));
	}

	result
}

/// Get the slashing span of a voter stash.
pub(crate) async fn slashing_span_of(
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

/// Get the exposure of `stash` at `era`.
pub async fn exposure_of(
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

fn to_vote_weight(balance: Balance) -> VoteWeight {
	<network::CurrencyToVoteHandler as Convert<Balance, VoteWeight>>::convert(balance)
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

	// stash key of all wannabe candidates.
	let mut candidates = get_candidates(client, at).await;

	// stash key of current voters, including maybe self vote.
	let mut all_voters_and_stake = get_voters(&client, at).await;

	if let Some(path) = conf.manual_override {
		#[derive(serde::Serialize, serde::Deserialize)]
		struct Override {
			voters: Vec<(AccountId, u64, Vec<AccountId>)>,
			voters_remove: Vec<AccountId>,
			candidates: Vec<AccountId>,
			candidates_remove: Vec<AccountId>,
		}

		let file = std::fs::read(path).unwrap();
		let json_str = std::str::from_utf8(file.as_ref()).unwrap();
		let manual: Override = serde_json::from_str(json_str).unwrap();

		// add any additional candidates
		manual.candidates.iter().for_each(|c| {
			if candidates.contains(c) {
				println!("manual override: {:?} is already a candidate.", c);
			} else {
				println!("manual override: {:?} is added as candidate.", c);
				candidates.push(c.clone())
			}
		});
		// remove any that are in removal list.
		candidates.retain(|c| !manual.candidates_remove.contains(c));

		// add any new votes
		manual.voters.iter().for_each(|v| {
			if let Some(mut already_existing_voter) =
				all_voters_and_stake.iter_mut().find(|vv| vv.0 == v.0)
			{
				println!("manual override: {:?} is already a voter. Overriding votes.", v.0);
				already_existing_voter.1 = v.1.into();
				already_existing_voter.2 = v.2.clone();
			} else {
				println!("manual override: {:?} is added as voters.", v.0);
				all_voters_and_stake.push(v.clone())
			}
		});

		// remove any of them
		all_voters_and_stake.retain(|v| !manual.voters_remove.contains(&v.0));
	}

	// add self-vote
	for c in candidates.iter() {
		let self_vote =
			(c.clone(), to_vote_weight(stake_of(&c, &client, at).await), vec![c.clone()]);
		all_voters_and_stake.push(self_vote);
	}

	let slashable_balance_votes = |who: &AccountId| -> VoteWeight {
		all_voters_and_stake.iter().find(|v| &v.0 == who).map(|v| v.1).unwrap_or_default()
	};

	// run phragmen
	t_start!(phragmen_run);
	let ElectionResult { winners, assignments } =
		seq_phragmen::<AccountId, pallet_staking::ChainAccuracy>(
			count,
			candidates.clone(),
			all_voters_and_stake.clone(),
			Some((iterations, 0)),
		)
		.expect("Phragmen failed to elect.");
	t_stop!(phragmen_run);

	let elected_stashes = winners.iter().map(|(s, _)| s.clone()).collect::<Vec<AccountId>>();

	t_start!(ratio_into_staked_run);
	let mut staked_assignments =
		assignment_ratio_to_staked(assignments.clone(), slashable_balance_votes);
	t_stop!(ratio_into_staked_run);

	t_start!(build_support_map_run);
	let mut supports =
		to_support_map::<AccountId>(&elected_stashes, staked_assignments.as_slice()).unwrap();
	t_stop!(build_support_map_run);

	let initial_score = supports.clone().evaluate();

	if reduce {
		t_start!(reducing_solution);
		sp_npos_elections::reduce(&mut staked_assignments);
		t_stop!(reducing_solution);
		// just to check that support has NOT changed
		let support_after_reduce =
			to_support_map::<AccountId>(&elected_stashes, staked_assignments.as_slice()).unwrap();
		assert_supports_total_equal(&support_after_reduce, &supports);
		supports = support_after_reduce;
	}

	let mut nominator_info: BTreeMap<AccountId, Vec<(AccountId, Balance)>> = BTreeMap::new();

	log::info!(target: LOG_TARGET, "💸 Winner Validators:");
	let mut oversubscribed: u32 = 0;
	for (i, (s, _)) in winners.iter().enumerate() {
		let support = supports.get(&s).unwrap();
		let other_count = support.voters.len();
		let self_stake = support.voters.iter().filter(|(v, _)| v == s).collect::<Vec<_>>();
		assert!(self_stake.len() <= 1);
		if self_stake.is_empty() {
			println!("⁉️ Self stake for this validator has been removed, seemingly.")
		}

		println!(
			"#{} --> {} [{:?}] [total backing = {:?} ({} voters)] [own backing = {:?}]",
			i + 1,
			storage::helpers::get_identity::<AccountId, Balance>(s.as_ref(), &client, at).await,
			s,
			Currency::from(support.total),
			if other_count > conf.max_payouts {
				oversubscribed += 1;
				ansi_term::Colour::Red.bold().paint(other_count.to_string())
			} else {
				ansi_term::Colour::Green.paint(other_count.to_string())
			},
			self_stake.get(0).map(|s| s.1).map(Currency::from),
		);

		if verbosity >= 1 {
			println!("  Voters:");
			support.voters.iter().enumerate().for_each(|(i, o)| {
				println!(
					"    {}#{} [amount = {:?}] {:?}",
					if *s == o.0 { "*" } else { "" },
					i + 1,
					Currency::from(o.1),
					o.0
				);
				nominator_info.entry(o.0.clone()).or_insert(vec![]).push((s.clone(), o.1));
			});
			println!("");
		}
	}

	log::info!("++ oversubscribed = {}", oversubscribed);

	if verbosity >= 2 {
		log::info!("💰 Nominator Assignments:");
		let mut counter = 1;
		for (nominator, info) in nominator_info.iter() {
			let mut sum = 0;
			let nom_stake = slashable_balance_votes(&nominator);
			println!(
				"#{} {:?} // active_stake = {:?}",
				counter,
				nominator,
				Currency::from(nom_stake.into()),
			);
			println!("  Distributions:");
			info.iter().enumerate().for_each(|(i, (c, s))| {
				sum += *s;
				println!("    #{} {:?} => {:?}", i, c, Currency::from(*s));
			});
			counter += 1;
			let diff = sum.max(nom_stake.into()) - sum.min(nom_stake.into());
			// acceptable diff is one millionth of a Currency
			assert!(diff < 1_000, "diff( sum_nominations,  staker_info.ledger.active) = {}", diff);
		}
	}

	log::info!(target: LOG_TARGET, "validator intentions count {:?}", candidates.len(),);
	log::info!(
		target: LOG_TARGET,
		"nominator intentions count {:?}",
		all_voters_and_stake.len() - candidates.len(),
	);
	log::info!(
		target: LOG_TARGET,
		"solution score {:?}",
		initial_score.iter().map(|n| format!("{:?}", Currency::from(*n))).collect::<Vec<_>>(),
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

	// potentially write to json file
	if let Some(output_file) = conf.output {
		use std::fs::File;

		// We can't really use u128 or arbitrary_precision of serde for now, so sadly all I can do
		// is duplicate the types with u64. Not cool but okay for now.
		#[derive(serde::Serialize, serde::Deserialize)]
		struct Support64 {
			total: u64,
			voters: Vec<(AccountId, u64)>,
		}

		type SupportMap64 = std::collections::BTreeMap<AccountId, Support64>;

		impl From<sp_npos_elections::Support<AccountId>> for Support64 {
			fn from(t: sp_npos_elections::Support<AccountId>) -> Self {
				Self {
					total: t.total.try_into().unwrap(),
					voters: t
						.voters
						.into_iter()
						.map(|(w, v)| (w, (v).try_into().unwrap()))
						.collect::<Vec<_>>(),
				}
			}
		}

		let mut supports_64 = SupportMap64::new();
		for (k, v) in supports.into_iter() {
			supports_64.insert(k, v.into());
		}

		let output = serde_json::json!({
			"supports": supports_64,
			"winners": elected_stashes,
		});

		serde_json::to_writer_pretty(&File::create(output_file).unwrap(), &output).unwrap();
	}
}

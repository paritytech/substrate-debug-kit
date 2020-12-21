//! Helpers to read staking module.

use crate::{
	network,
	primitives::{AccountId, Balance, Hash},
	storage, Client, Currency, Opt, StakingConfig, LOG_TARGET,
};
use codec::Encode;
use pallet_staking::{
	slashing::SlashingSpans, EraIndex, Exposure, Nominations, StakingLedger,
};
use sp_npos_elections::*;
use std::{collections::BTreeMap, convert::TryInto};
use std::fs::File;
use std::path::Path;

const MODULE: &[u8] = b"Staking";

/// A staker
#[derive(Debug, Clone, Default)]
struct Staker {
	ctrl: Option<AccountId>,
	stake: Option<StakingLedger<AccountId, Balance>>,
}

fn assert_supports_total_equal(s1: &SupportMap<AccountId>, s2: &SupportMap<AccountId>) {
	assert!(s1.iter().all(|(v, s)| s2.get(v).unwrap().total == s.total))
}

/// Get the current era.
pub async fn get_current_era(client: &Client, at: Hash) -> EraIndex {
	storage::read::<EraIndex>(storage::value_key(MODULE, b"CurrentEra"), client, at)
		.await
		.expect("CurrentEra must exist")
}

async fn get_candidates(client: &Client, at: Hash, max:usize) -> Vec<AccountId> {
	storage::enumerate_keys_paged::<AccountId>(MODULE, b"Validators", client, at, max)
		.await
		.expect("Staking::validators should be enumerable.")
		.into_iter()
		.collect::<Vec<AccountId>>()
}

async fn get_voters(client: &Client, at: Hash,max:usize) -> Vec<(AccountId, Vec<AccountId>)> {

	let nominators: Vec<(AccountId, Nominations<AccountId>)> = storage::enumerate_map_paged::<
		AccountId,
		Nominations<AccountId>,
	>(MODULE, b"Nominators", client, at, max)
		.await
		.expect("Staking::nominators should be enumerable");


	let mut result = vec![];
	for (_idx, (who, n)) in nominators.into_iter().enumerate() {
		let targets = n.targets;

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
	.await;

	if !ctrl.is_some() {
		return Staker{
			ctrl: None,
			stake: None,
		}
	}

	let ledger = storage::read::<StakingLedger<AccountId, Balance>>(
		storage::map_key::<frame_support::Blake2_128Concat>(MODULE, b"Ledger", ctrl.clone().unwrap().as_ref()),
		&client,
		at,
	).await;

	if !ledger.is_some() {
		return Staker{
			ctrl: None,
			stake: None,
		}
	}

	return Staker {
		ctrl: ctrl,
		stake: ledger,
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

/// Get validator count for next era
async fn get_validator_count(client: &Client, at: Hash) -> u32 {
	storage::read::<u32>(storage::value_key(MODULE, b"ValidatorCount"), client, at)
		.await
		.unwrap_or(50)
}

/// run phragmen directly from data scraped
pub async fn run_phragmen(data:ScrapeData,opt: &Opt,conf: &StakingConfig) {
	let verbosity = opt.verbosity;
	let candidates = data.candidates;
	let all_voters = data.voters;
	let mut staker_infos:BTreeMap<AccountId, u128> = BTreeMap::new();
	for (k, v) in data.stakers.clone().into_iter() {
		let balance = v as u128;
		staker_infos.insert(k, balance);
	}
	let iterations = conf.iterations;
	let count = conf.count.unwrap_or(candidates.len());
	let reduce = conf.reduce;
	let slashable_balance = |who: &AccountId| -> Balance{
		return staker_infos.get(who).unwrap().clone()
	};
	let slashable_balance_votes = |who: &AccountId| -> VoteWeight {
		if sub_tokens::dynamic::get_network() == "darwinia"{
			return slashable_balance(who) as u64
		}else{
			return <network::CurrencyToVoteHandler as Convert<Balance, VoteWeight>>::convert(
				slashable_balance(who),
			)
		}
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
		// Some((iterations, 0)),
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
	let mut supports =
		build_support_map::<AccountId>(&elected_stashes, staked_assignments.as_slice()).0;
	t_stop!(build_support_map_run);

	// TODO: remove once balancing is merged into set-phragmen
	t_start!(balancing);
	sp_npos_elections::balance_solution(&mut staked_assignments, &mut supports, 0, iterations);
	t_stop!(balancing);

	let initial_score = evaluate_support(&supports);

	if reduce {
		t_start!(reducing_solution);
		sp_npos_elections::reduce(&mut staked_assignments);
		t_stop!(reducing_solution);
		// just to check that support has NOT changed
		let support_after_reduce =
			build_support_map::<AccountId>(&elected_stashes, staked_assignments.as_slice()).0;
		assert_supports_total_equal(&support_after_reduce, &supports);
		supports = support_after_reduce;
	}

	let mut nominator_info: BTreeMap<AccountId, Vec<(AccountId, Balance)>> = BTreeMap::new();

	log::info!(target: LOG_TARGET, "💸 Winner Validators:");
	for (i, (s, _)) in winners.iter().enumerate() {
		let support = supports.get(&s).unwrap();
		let other_count = support.voters.len();
		let self_stake = support
			.voters
			.iter()
			.filter(|(v, _)| v == s)
			.collect::<Vec<_>>();
		if self_stake.len() >0 {
			assert!(self_stake.len() == 1);
			println!(
				"#{} --> [{:?}] [total backing = {:?} ({} voters)] [own backing = {:?}]",
				i + 1,
				s,
				Currency::from(support.total),
				other_count,
				Currency::from(self_stake[0].1),
			);
		}

		if verbosity >= 1 {
			println!("  Voters for {:?}:",*s );
			support.voters.iter().enumerate().for_each(|(i, o)| {
				println!(
					"    {}#{} [amount = {:?}] {:?}",
					if *s == o.0 { "*" } else { "" },
					i + 1,
					Currency::from(o.1),
					o.0
				);
				nominator_info
					.entry(o.0.clone())
					.or_insert(vec![])
					.push((s.clone(), o.1));
			});
			println!("");
		}
	}

	if verbosity >= 2 {
		log::info!("💰 Nominator Assignments:");
		let mut counter = 1;
		for (nominator, info) in nominator_info.iter() {
			let staker_info = staker_infos.get(&nominator).unwrap().clone();
			let mut sum = 0;
			println!(
				"#{} {:?} // active_stake = {:?}",
				counter,
				nominator,
				Currency::from(staker_info),
			);
			println!("  Distributions:");
			info.iter().enumerate().for_each(|(i, (c, s))| {
				sum += *s;
				println!("    #{} {:?} => {:?}", i, c, Currency::from(*s));
			});
			counter += 1;
			let diff = sum.max(staker_info) - sum.min(staker_info);
			// acceptable diff is one millionth of a Currency
			assert!(
				diff < 1_000,
				"diff( sum_nominations,  staker_info.ledger.active) = {}",
				diff
			);
			println!("");
		}
	}

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
			.map(|n| format!("{:?}", Currency::from(*n)))
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

	// potentially write to json file
	if let Some(output_file) = &conf.output {

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

		serde_json::to_writer_pretty(&File::create(Path::new(&output_file)).unwrap(), &output).unwrap();
	}
}

use serde::{Serialize, Deserialize};
use std::io::BufReader;
use sp_runtime::traits::Convert;

/// ScrapeData
#[derive(Serialize, Deserialize)]
pub struct ScrapeData {
	candidates: Vec<AccountId>,
	voters: Vec<(AccountId, Vec<AccountId>)>,
	stakers: BTreeMap<AccountId, u64>,
}

/// scrape data from rpc
pub async fn scrape(client: &Client,opt: &Opt, conf: &StakingConfig) -> ScrapeData{
	let at = opt.at.unwrap();
	let val_count = get_validator_count(&client, at).await as usize;
	let count = conf.count.unwrap_or(val_count);
	if count != val_count {
		log::warn!(
			target: LOG_TARGET,
			"`count` provided ({:?}) differs from validator count on-chain ({}).",
			count,
			val_count,
		);
	}
	let max = conf.max.unwrap_or(100000);

	t_start!(data_scrape);

	let candidates = get_candidates(&client, at, max).await;

	// stash key of current voters, including maybe self vote.
	let mut all_voters = get_voters(&client, at,max).await;

	// add self-vote
	candidates.iter().for_each(|v| {
		let self_vote = (v.clone(), vec![v.clone()]);
		all_voters.push(self_vote);
	});

	// get the slashable balance of every entity
	let mut staker_infos: BTreeMap<AccountId, u128> = BTreeMap::new();
	let mut staker_infos_u64: BTreeMap<AccountId, u64> = BTreeMap::new();
	for stash in all_voters.iter().map(|(s, _)| s) {
		let staker_info = get_staker_info_entry(&stash, &client, at).await;
		if staker_info.ctrl.is_some()&&staker_info.stake.is_some(){
			let balance = staker_info.stake.unwrap().active;
			println!(
				"#{:?} // active_stake = {:?}",
				stash,
				Currency::from(balance),
			);
			staker_infos.insert(stash.clone(), balance);
			let balance_u64 = balance as u64;
			staker_infos_u64.insert(stash.clone(),balance_u64);
		}
	}
	t_stop!(data_scrape);

	let data = ScrapeData {
		candidates: candidates.clone(),
		voters: all_voters.clone(),
		stakers:staker_infos_u64.clone(),
	};
	let output_file = Path::new("./data_scraped.json");
	let output = serde_json::json!(data);
	serde_json::to_writer_pretty(&File::create(output_file).unwrap(), &output).unwrap();
	return data;
}

/// Main run function of the sub-command.
pub async fn run(client: &Client, opt: Opt, conf: StakingConfig) {
	let data:ScrapeData;
	if conf.input.is_none() {
		data = scrape(client,&opt,&conf).await
	}else{
		let file = File::open(conf.input.clone().unwrap()).unwrap();
		let reader = BufReader::new(file);
		data = serde_json::from_reader(reader).unwrap();
	}
	run_phragmen(data,&opt,&conf).await;
}

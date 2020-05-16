use crate::primitives::{AccountId, Balance, Hash};
use crate::{network, storage, Client, CommonConfig, KSM, LOG_TARGET};
use sp_phragmen::*;
use sp_runtime::traits::Convert;
use std::collections::BTreeMap;

const MODULE: &[u8] = b"PhragmenElection";

async fn get_candidates(client: &Client, at: Hash) -> Vec<AccountId> {
	let mut members = storage::read::<Vec<(AccountId, Balance)>>(
		storage::value_key(MODULE, b"Members"),
		client,
		at,
	)
	.await
	.expect("Members must exist")
	.into_iter()
	.map(|(m, _)| m)
	.collect::<Vec<AccountId>>();

	let runners = storage::read::<Vec<(AccountId, Balance)>>(
		storage::value_key(MODULE, b"RunnersUp"),
		client,
		at,
	)
	.await
	.expect("Runners-up must exists")
	.into_iter()
	.map(|(m, _)| m)
	.collect::<Vec<AccountId>>();

	let candidates =
		storage::read::<Vec<AccountId>>(storage::value_key(MODULE, b"Candidates"), client, at)
			.await
			.unwrap_or_default();

	log::trace!(
		target: LOG_TARGET,
		"candidates composed of: {} members, {} runners-up, {} new candidates.",
		members.len(),
		runners.len(),
		candidates.len(),
	);

	members.extend(candidates);
	members.extend(runners);

	members
}

async fn get_voters_and_budget(
	client: &Client,
	at: Hash,
) -> Vec<(AccountId, Balance, Vec<AccountId>)> {
	storage::enumerate_map::<AccountId, (Balance, Vec<AccountId>)>(MODULE, b"Voting", client, at)
		.await
		.unwrap()
		.into_iter()
		.map(|(n, (b, t))| (n, b, t))
		.collect::<Vec<_>>()
}

struct CommandConfig {
	pub iterations: usize,
	pub count: usize,
	pub min_count: usize,
	pub output: Option<String>,
}

impl From<&clap::ArgMatches<'_>> for CommandConfig {
	fn from(matches: &clap::ArgMatches<'_>) -> Self {
		let iterations: usize = matches
			.value_of("iterations")
			.unwrap_or("0")
			.parse()
			.unwrap();

		let output = matches.value_of("output").map(|o| o.to_string());

		let count = matches.value_of("count").unwrap_or("50").parse().unwrap();

		let min_count = matches
			.value_of("min-count")
			.unwrap_or("0")
			.parse()
			.unwrap();

		Self {
			iterations,
			count,
			min_count,
			output,
		}
	}
}

pub async fn run(client: &Client, common_config: CommonConfig, matches: &clap::ArgMatches<'_>) {
	let command_config = CommandConfig::from(matches);
	let at = common_config.at;
	let verbosity = common_config.verbosity;
	let iterations = command_config.iterations;

	let to_votes = |b: Balance| -> VoteWeight {
		<network::CurrencyToVoteHandler as Convert<Balance, VoteWeight>>::convert(b)
	};

	t_start!(data_scrape);
	// all candidates
	let candidates = get_candidates(client, at).await;

	// all voters.
	let all_voters = get_voters_and_budget(&client, at)
		.await
		.into_iter()
		.map(|(n, b, t)| (n, to_votes(b), t))
		.collect::<Vec<_>>();

	// budget of each voter
	let mut voter_weight: BTreeMap<AccountId, VoteWeight> = BTreeMap::new();

	// This is needed to create closures and such.
	for (voter, budget, _) in all_voters.iter() {
		voter_weight.insert(voter.clone(), *budget);
	}
	t_stop!(data_scrape);

	let weight_of = |who: &AccountId| -> VoteWeight { *voter_weight.get(who).unwrap() };

	// run phragmen
	t_start!(phragmen_run);
	let PhragmenResult {
		winners,
		assignments,
	} = elect::<AccountId, pallet_staking::ChainAccuracy>(
		command_config.count,
		command_config.min_count,
		candidates,
		all_voters,
	)
	.expect("Phragmen failed to elect.");
	t_stop!(phragmen_run);

	let elected_stashes = winners
		.iter()
		.map(|(s, _)| s.clone())
		.collect::<Vec<AccountId>>();

	t_start!(ratio_into_staked_run);
	let staked_assignments = assignment_ratio_to_staked(assignments.clone(), weight_of);
	t_stop!(ratio_into_staked_run);

	t_start!(build_support_map_run);
	let (supports, _) =
		build_support_map::<AccountId>(&elected_stashes, staked_assignments.as_slice());
	t_stop!(build_support_map_run);

	if iterations > 0 {
		// prepare and run post-processing.
		unimplemented!();
	}

	log::info!(target: LOG_TARGET, "👨🏻‍⚖️ Members:");
	for (i, s) in winners.iter().enumerate() {
		println!(
			"#{} --> {} [{:?}][total backing = {:?}]",
			i + 1,
			network::get_identity(&s.0, &client, at).await,
			s.0,
			KSM(supports.get(&s.0).unwrap().total),
		);

		if verbosity >= 1 {
			let support = supports.get(&s.0).expect("members must have support");
			println!("  Voters:");
			support.voters.iter().enumerate().for_each(|(i, o)| {
				println!(
					"	{}#{} [amount = {:?}] {:?}",
					if s.0 == o.0 { "*" } else { "" },
					i,
					KSM(o.1),
					o.0
				);
			});
			println!("");
		}
	}
}

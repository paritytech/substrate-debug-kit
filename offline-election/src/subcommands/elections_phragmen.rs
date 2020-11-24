use crate::{
	network,
	primitives::{AccountId, Balance, Hash},
	storage, Client, CouncilConfig, Currency, Opt, LOG_TARGET,
};
use sp_npos_elections::*;
use sp_runtime::traits::{Convert, Zero};
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

/// Main run function of the sub-command.
pub async fn run(client: &Client, opt: Opt, conf: CouncilConfig) {
	let at = opt.at.unwrap();
	let verbosity = opt.verbosity;
	let iterations = conf.iterations;
	let desired_members =
		sub_storage::get_const::<u32>(client, "ElectionsPhragmen", "DesiredMembers", at)
			.await
			.expect("DesiredMembers const must exist.");

	let desired_runners_up =
		sub_storage::get_const::<u32>(client, "ElectionsPhragmen", "DesiredRunnersUp", at)
			.await
			.expect("DesiredRunnersUp const must exist.");
	let count = conf
		.count
		.unwrap_or_else(|| (desired_members + desired_runners_up) as usize);

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
	let ElectionResult {
		winners,
		assignments,
	} = seq_phragmen::<AccountId, pallet_staking::ChainAccuracy>(
		count,
		0,
		candidates,
		all_voters.clone(),
		// None,
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
	let supports =
		build_support_map::<AccountId>(&elected_stashes, staked_assignments.as_slice()).0;
	t_stop!(build_support_map_run);

	if iterations > 0 {
		// prepare and run post-processing.
		unimplemented!()
	}

	log::info!(target: LOG_TARGET, "👨🏻‍⚖️ Members:");
	for (i, s) in winners.iter().enumerate() {
		println!(
			"#{} --> {} [{:?}][total backing = {:?}]",
			i + 1,
			storage::helpers::get_identity::<AccountId, Balance>(s.0.as_ref(), &client, at).await,
			s.0,
			Currency::from(supports.get(&s.0).unwrap().total),
		);

		if verbosity >= 1 {
			let support = supports.get(&s.0).expect("members must have support");
			println!("  Voters:");
			support.voters.iter().enumerate().for_each(|(i, o)| {
				println!(
					"	{}#{} [amount = {:?}] {:?}",
					if s.0 == o.0 { "*" } else { "" },
					i,
					Currency::from(o.1),
					o.0
				);
			});
			println!("");
		}
	}

	let new_members = winners
		.into_iter()
		.take(desired_members as usize)
		.collect::<Vec<_>>();
	let mut prime_votes: Vec<_> = new_members
		.iter()
		.map(|(c, _)| (c, VoteWeight::zero()))
		.collect();
	for (_, stake, targets) in all_voters.into_iter() {
		for (votes, who) in targets
			.iter()
			.enumerate()
			.map(|(votes, who)| ((16 - votes) as u32, who))
		{
			if let Ok(i) = prime_votes.binary_search_by_key(&who, |k| k.0) {
				prime_votes[i].1 += stake * votes as VoteWeight;
			}
		}
	}
	let prime = prime_votes
		.into_iter()
		.max_by_key(|x| x.1)
		.map(|x| x.0.clone());

	log::info!(target: LOG_TARGET, "👑 Prime: {:?}", prime);
}

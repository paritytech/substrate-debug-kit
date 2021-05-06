use crate::{
	network,
	primitives::{AccountId, Balance, Hash},
	storage, Client, CouncilConfig, Currency, Opt, LOG_TARGET,
};
use sp_npos_elections::*;
use sp_runtime::traits::{Convert, Zero};
use std::{collections::BTreeMap};

const MODULE: &[u8] = b"PhragmenElection";

async fn get_candidates(client: &Client, at: Hash) -> Vec<AccountId> {
	let mut members = storage::read::<Vec<(AccountId, Balance, Balance)>>(
		storage::value_key(MODULE, b"Members"),
		client,
		at,
	)
	.await
	.expect("Members must exist")
	.into_iter()
	.map(|(m, _, _)| m)
	.collect::<Vec<AccountId>>();

	let runners = storage::read::<Vec<(AccountId, Balance, Balance)>>(
		storage::value_key(MODULE, b"RunnersUp"),
		client,
		at,
	)
	.await
	.expect("Runners-up must exists")
	.into_iter()
	.map(|(m, _, _)| m)
	.collect::<Vec<AccountId>>();

	let candidates = storage::read::<Vec<(AccountId, Balance)>>(
		storage::value_key(MODULE, b"Candidates"),
		client,
		at,
	)
	.await
	.unwrap_or_default()
	.into_iter()
	.map(|(c, _)| c)
	.collect::<Vec<_>>();

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
	storage::enumerate_map::<AccountId, (Vec<AccountId>, Balance, Balance)>(
		MODULE, b"Voting", client, at,
	)
	.await
	.unwrap()
	.into_iter()
	.map(|(n, (t, b, _))| (n, b, t))
	.collect::<Vec<_>>()
}

/// Main run function of the sub-command.
pub async fn run(client: &Client, opt: Opt, conf: CouncilConfig) {
	let at = opt.at.unwrap();
	let verbosity = opt.verbosity;
	let desired_members =
		sub_storage::get_const::<u32>(client, "ElectionsPhragmen", "DesiredMembers", at)
			.await
			.expect("DesiredMembers const must exist.");

	let desired_runners_up =
		sub_storage::get_const::<u32>(client, "ElectionsPhragmen", "DesiredRunnersUp", at)
			.await
			.expect("DesiredRunnersUp const must exist.");
	let count = conf.count.unwrap_or_else(|| (desired_members + desired_runners_up) as usize);

	let to_votes = |b: Balance| -> VoteWeight {
		<network::CurrencyToVoteHandler as Convert<Balance, VoteWeight>>::convert(b)
	};

	// all candidates
	let mut candidates = get_candidates(client, at).await;

	// all voters.
	let mut all_voters = get_voters_and_budget(&client, at)
		.await
		.into_iter()
		.map(|(n, b, t)| (n, to_votes(b), t))
		.collect::<Vec<_>>();

	if let Some(path) = conf.manual_override {
		#[derive(serde::Serialize, serde::Deserialize)]
		struct VotersMutate {
			who: AccountId,
			votes: Vec<AccountId>,
		}

		#[derive(serde::Serialize, serde::Deserialize)]
		struct Override {
			pub voters: Vec<(AccountId, u64, Vec<AccountId>)>,
			pub voters_remove: Vec<AccountId>,
			pub voters_mutate: Vec<VotersMutate>,
			pub candidates: Vec<AccountId>,
			pub candidates_remove: Vec<AccountId>,
		}

		let file = std::fs::read(path).unwrap();
		let json_str = std::str::from_utf8(file.as_ref()).unwrap();
		let manual: Override = serde_json::from_str(json_str).unwrap();

		// add any additional candidates
		manual.candidates.iter().for_each(|c| {
			if candidates.contains(c) {
				log::warn!(target: LOG_TARGET, "manual override: {:?} is already a candidate.", c);
			} else {
				log::warn!(target: LOG_TARGET, "manual override: {:?} is added as candidate.", c);
				candidates.push(c.clone())
			}
		});
		// remove any that are in removal list.
		candidates.retain(|c| !manual.candidates_remove.contains(c));

		// add any new votes
		manual.voters.iter().for_each(|v| {
			if let Some(mut already_existing_voter) = all_voters.iter_mut().find(|vv| vv.0 == v.0) {
				log::warn!(
					target: LOG_TARGET,
					"manual override: {:?} is already a voter. Overriding votes and stake.",
					v.0,
				);
				already_existing_voter.1 = v.1;
				already_existing_voter.2 = v.2.clone();
			} else {
				log::warn!(target: LOG_TARGET, "manual override: {:?} is added as voters.", v.0);
				all_voters.push(v.clone())
			}
		});

		manual.voters_mutate.iter().for_each(|VotersMutate { who, votes }| {
			if let Some(mut already_existing_voter) = all_voters.iter_mut().find(|vv| &vv.0 == who)
			{
				log::warn!(target: LOG_TARGET, "manual override: {:?}. Overriding votes.", who);
				already_existing_voter.2 = votes.clone();
			}
		});

		// remove any of them
		all_voters.retain(|v| !manual.voters_remove.contains(&v.0));
	}

	// budget of each voter
	let mut voter_weight: BTreeMap<AccountId, VoteWeight> = BTreeMap::new();

	// This is needed to create closures and such.
	for (voter, budget, _) in all_voters.iter() {
		voter_weight.insert(voter.clone(), *budget);
	}

	let weight_of = |who: &AccountId| -> VoteWeight { *voter_weight.get(who).unwrap() };

	// run phragmen
	t_start!(phragmen_run);
	let ElectionResult { winners, assignments } = seq_phragmen::<
		AccountId,
		pallet_staking::ChainAccuracy,
	>(count, candidates, all_voters.clone(), None)
	.expect("Phragmen failed to elect.");
	t_stop!(phragmen_run);

	let elected_stashes = winners.iter().map(|(s, _)| s.clone()).collect::<Vec<AccountId>>();

	let staked_assignments = assignment_ratio_to_staked(assignments.clone(), weight_of);

	let supports =
		to_support_map::<AccountId>(&elected_stashes, staked_assignments.as_slice()).unwrap();

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

	let mut new_members = winners.into_iter().take(desired_members as usize).collect::<Vec<_>>();
	new_members.sort_by_key(|(m, _)| m.clone());
	let mut prime_votes: Vec<_> = new_members.iter().map(|(c, _)| (c, Balance::zero())).collect();
	for (_, stake, targets) in all_voters.into_iter() {
		for (vote_multiplier, who) in
			targets.iter().enumerate().map(|(vote_position, who)| ((16 - vote_position) as u32, who))
		{
			if let Ok(i) = prime_votes.binary_search_by_key(&who, |k| k.0) {
				prime_votes[i].1 += (stake as Balance) * (vote_multiplier as Balance);
			}
		}
	}
	let prime = prime_votes.into_iter().max_by_key(|x| x.1).map(|x| x.0.clone());

	if let Some(prime) = prime {
		log::info!(
			target: LOG_TARGET,
			"👑 Prime: {}",
			storage::helpers::get_identity::<AccountId, Balance>(prime.as_ref(), &client, at).await
		);
	}
}

// Copyright 2019 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! A Rust RPC client for a substrate node with utility snippets to scrape the node's data and run
//! function on top of them.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

// whatever node you are connecting to. Polkadot, substrate etc.
pub use primitives::{Hash, Balance, AccountId, BlockNumber};

use std::{fmt, fmt::Debug, collections::BTreeMap, convert::TryInto};
use separator::Separatable;
use clap::{App, load_yaml};
use jsonrpsee::Client;
use sp_core::crypto::{set_default_ss58_version, Ss58AddressFormat};
pub use sc_rpc_api::state::StateClient;
use sp_phragmen::{
	elect, PhragmenResult, build_support_map,
};
use node_runtime::{Runtime, Staking, Balances};

mod network;
mod staking;
mod elections_phragmen;
mod storage;
mod primitives;
mod offchain_phragmen;
mod mock;
#[macro_use]
mod timing;

/// A staker
#[derive(Debug, Clone)]
pub struct Staker {
	ctrl: Option<AccountId>,
	stake: Balance,
}

/// Wrapper to pretty-print ksm (or any other 12 decimal) token.
struct KSM(Balance);

const DECIMAL_POINTS: Balance = 1_000_000_000_000;

impl fmt::Debug for KSM {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let num: u128 = self.0.try_into().unwrap();
		write!(f, "{}_KSM ({})", self.0 / DECIMAL_POINTS, num.separated_string())
	}
}

impl fmt::Display for KSM {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let num: u128 = self.0.try_into().unwrap();
		write!(f, "{}", num.separated_string())
	}
}

fn main() {
	env_logger::try_init().ok();

	let yaml = load_yaml!("../cli.yml");
	let app = App::from(yaml);
	let matches = app.get_matches();

	let uri = matches.value_of("uri")
		.unwrap_or("ws://localhost:9944")
		.to_string();

	let validator_count = matches
		.subcommand_matches("phragmen")
		.unwrap()
		.value_of("count")
		.unwrap_or("50")
		.parse()
		.unwrap();
	let minimum_validator_count = matches
		.subcommand_matches("phragmen")
		.unwrap()
		.value_of("min-count")
		.unwrap_or("0")
		.parse()
		.unwrap();
	let iterations: usize = matches
		.subcommand_matches("phragmen")
		.unwrap()
		.value_of("iterations")
		.unwrap_or("0")
		.parse()
		.unwrap();

	// optionally at certain block hash
	let maybe_at: Option<String> = matches.value_of("at").map(|s| s.to_string());

	// Verbosity degree.
	let verbosity = matches.occurrences_of("verbose");

	// chose json output file.
	let maybe_output_file = matches.value_of("output");

	// self-vote?
	let do_self_vote = !matches.is_present("no-self-vote");

	// staking or elections?
	let do_elections = matches.is_present("elections");

	// setup address format
	let addr_format = match matches.value_of("network").unwrap_or("kusama") {
		"kusama" => Ss58AddressFormat::KusamaAccountDirect,
		"polkadot" => Ss58AddressFormat::PolkadotAccountDirect,
		"substrate" => Ss58AddressFormat::SubstrateAccountDirect,
		_ => panic!("invalid address format"),
	};

	async_std::task::block_on(async {
		// connect to a node.
		let transport = jsonrpsee::transport::ws::WsTransportClient::new(&uri)
			.await
			.expect("Failed to connect to client");
		let client: Client = jsonrpsee::raw::RawClient::new(transport).into();

		// get the latest block hash
		let head = network::get_head(&client).await;

		// potentially replace with the given hash
		let at: Hash = if let Some(at) = maybe_at {
			Hash::from_slice(&hex::decode(at).expect("invalid hash format given"))
		} else {
			head
		};

		// Get the total issuance and update the global pointer to it.
		let mut total_issuance = network::get_total_issuance(&client, at).await;
		unsafe { network::ISSUANCE = &mut total_issuance; }

		set_default_ss58_version(addr_format);

		t_start!(data_scrape);
		// stash key of all wannabe candidates.
		let candidates = if do_elections {
			elections_phragmen::get_candidates(&client, at).await
		} else {
			staking::get_candidates(&client, at).await
		};

		// stash key of current voters, including maybe self vote.
		let mut all_voters = if do_elections {
			elections_phragmen::get_voters(&client, at).await
		} else {
			staking::get_voters(&client, at).await
		};

		// add self-vote
		if do_self_vote {
			candidates.iter().for_each(|v| {
				let self_vote = (v.clone(), vec![v.clone()]);
				all_voters.push(self_vote);
			});
		}

		// get the slashable balance of every entity
		let mut staker_infos: BTreeMap<AccountId, Staker> = BTreeMap::new();

		let mut targets = candidates.clone();
		targets.extend(all_voters.iter().map(|(n, _)| n.clone()).collect::<Vec<AccountId>>());
		for stash in targets.iter() {
			let staker_info =
				if do_elections {
					elections_phragmen::get_staker_info_entry(&stash, &client, at).await
				} else {
					staking::get_staker_info_entry(&stash, &client, at).await
				};
			staker_infos.insert(stash.clone(), staker_info);
		};

		let slashable_balance = |who: &AccountId| -> Balance {
			staker_infos.get(who).unwrap().stake
		};
		t_stop!(data_scrape);

		// run phragmen
		t_start!(phragmen_run);
		let PhragmenResult { winners, assignments } = elect::<
			AccountId,
			Balance,
			_,
			network::CurrencyToVoteHandler<network::TotalIssuance>,
			pallet_staking::ChainAccuracy,
		>(
			validator_count,
			minimum_validator_count,
			candidates.clone(),
			all_voters.clone(),
			slashable_balance,
		).ok_or("Phragmen failed to elect.").unwrap();
		t_stop!(phragmen_run);

		let elected_stashes = winners.iter().map(|(s, _)| s.clone()).collect::<Vec<AccountId>>();

		t_start!(ratio_into_staked_run);
		let staked_assignments = sp_phragmen::assignment_ratio_to_staked(assignments.clone(), slashable_balance);
		t_stop!(ratio_into_staked_run);

		t_start!(build_support_map_run);
		let (supports, _) = build_support_map::<AccountId>(&elected_stashes, staked_assignments.as_slice());
		t_stop!(build_support_map_run);

		if iterations > 0 {
			// prepare and run post-processing.
			unimplemented!();
		}

		let mut slot_stake = u128::max_value();
		let mut nominator_info: BTreeMap<AccountId, Vec<(AccountId, Balance)>> = BTreeMap::new();

		println!("+++ Winner Validators:");
		for (i, s) in winners.iter().enumerate() {
			println!("#{} == {} [{:?}]", i + 1, network::get_nick(&s.0, &client, at).await, s.0);
			let support = supports.get(&s.0).unwrap();
			let others_sum: Balance = support.voters.iter().map(|(_n, s)| s).sum();
			let other_count = support.voters.len();

			assert_eq!(support.total, others_sum, "a support total has been wrong");

			let expo = staking::exposure_of(&s.0, &client, at).await;
			if support.total != expo.total {
				log::warn!(
					target: "offline_phragmen",
					"exposure mismatch with on-chain data, expo.total = {:?} - support.total = {:?} diff = {}",
					expo.total,
					support.total,
					if support.total > expo.total {
						format!("+{}", KSM(support.total - expo.total))
					} else {
						format!("-{}", KSM(expo.total - support.total))
					}
				);
			} else {
				log::debug!(
					target: "offline_phragmen",
					"exposure matches with on-chain data.",
				)
			}

			println!(
				"[stake_total: {:?}] [vote_count: {}] [ctrl: {:?}]",
				KSM(support.total),
				other_count,
				staker_infos.get(&s.0).unwrap().ctrl,
			);

			if support.total < slot_stake { slot_stake = support.total; }

			if verbosity >= 1 {
				println!("  Voters:");
				support.voters.iter().enumerate().for_each(|(i, o)| {
					println!(
						"	{}#{} [amount = {:?}] {:?}",
						if s.0 == o.0 { "*" } else { "" },
						i,
						KSM(o.1),
						o.0
					);
					nominator_info.entry(o.0.clone()).or_insert(vec![]).push((s.0.clone(), o.1));
				});
			}

			println!("");
		};

		if verbosity >= 2 {
			println!("+++ Assignments:");
			let mut counter = 1;
			for (nominator, info) in nominator_info.iter() {
				let staker_info = staker_infos.get(&nominator).unwrap();
				let mut sum = 0;
				println!(
					"#{} {:?} // active_stake = {:?}",
					counter,
					nominator, KSM(staker_info.stake),
				);
				println!("  Distributions:");
				info.iter().enumerate().for_each(|(i, (c, s))| {
					sum += *s;
					println!("    #{} {:?} => {:?}", i, c, KSM(*s));
				});
				counter += 1;
				let diff = sum.max(staker_info.stake) - sum.min(staker_info.stake);
				// acceptable diff is one millionth of a KSM
				assert!(diff < 1_000, "diff( sum_nominations,  staker_info.ledger.active) = {}", diff);
				println!("");
			}
		}

		let compact = offchain_phragmen::prepare_offchain_submission(
			validator_count,
			minimum_validator_count,
			candidates.clone(),
			all_voters.clone(),
			staker_infos.clone(),
			&client,
			at,
		).await;


		mock::empty_ext_with_runtime::<Runtime>().execute_with(|| {
			use frame_support::traits::{Currency, StoredMap};
			use frame_support::assert_ok;
			use frame_support::storage::StorageValue;
			use sp_runtime::traits::Dispatchable;

			for c in candidates.clone() {
				let e = staker_infos.get(&c).unwrap();
				let ctrl = e.ctrl.as_ref().unwrap();
				let stake = e.stake;
				Balances::make_free_balance_be(&c, stake);

				let call = node_runtime::Call::Staking(pallet_staking::Call::bond(
					pallet_indices::Address::<Runtime>::Id(ctrl.clone()),
					stake,
					Default::default(),
				));
				println!("Bonding {:?}/{:?} with {:?}", &c, &ctrl, &stake);
				let o = frame_system::Origin::<Runtime>::Signed(c);
				assert_ok!(Dispatchable::dispatch(call, o.into()));

				<pallet_staking::EraElectionStatus<Runtime>>::put(pallet_staking::ElectionStatus::Open(1));
			}

			Staking::check_and_replace_solution(
				Default::default(),
				compact.clone(),
				pallet_staking::ElectionCompute::OnChain,
				Default::default(),
			);
		});

		eprintln!("++ connected to [{}]", uri);
		eprintln!("++ at [{}]", at);
		eprintln!("++ total_issuance = {:?}", KSM(total_issuance));
		eprintln!("++ validator intentions count {:?}", candidates.len());
		eprintln!("++ nominator intentions count {:?}", all_voters.len() - candidates.len());
		eprintln!(
			"++ args: [count to elect = {}] [min-count = {}] [output = {:?}] [iterations = {}] [do_self_vote {}] [do_elections {}]",
			validator_count,
			minimum_validator_count,
			maybe_output_file,
			iterations,
			do_self_vote,
			do_elections,
		);
		eprintln!("++ final slot_stake {:?}", KSM(slot_stake));
		eprintln!("++ Phragmen Assignment size {} bytes.", codec::Encode::encode(&assignments).len());
		eprintln!("++ Phragmen compact Assignment size {} bytes.", codec::Encode::encode(&compact).len());

		// potentially write to json file
		if let Some(output_file) = maybe_output_file {
			use std::fs::File;

			let output = serde_json::json!({
				"supports": supports,
				"winners": elected_stashes,
			});

			serde_json::to_writer_pretty(
				&File::create(format!("{}", output_file)).unwrap(),
				&output
			).unwrap();
		}
	})
}

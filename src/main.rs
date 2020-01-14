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

//! An extended version of the code in `substrate/node/rpc-client/` which reads the staking info
//! of a chain and runs the phragmen election with the given parameters offline.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

use futures::Future;
use hyper::rt;
use std::{fmt, fmt::Debug, collections::BTreeMap, convert::TryInto};
use codec::Decode;
use separator::Separatable;
use clap::{Arg, App};
use jsonrpc_core_client::transports::{http, ws};
use sp_core::crypto::{set_default_ss58_version, Ss58AddressFormat};
pub use sc_rpc_api::state::StateClient;

pub use polkadot_primitives::{Hash, Balance, AccountId};
use sp_core::storage::StorageKey;
use sp_core::hashing::{blake2_256, twox_128};
use sp_phragmen::{
	elect, equalize, PhragmenResult, PhragmenStakedAssignment, build_support_map,
};
use sp_runtime::traits::Convert;
use support::storage::generator::Linkage;

// TODO: clean function interfaces: probably no more passing string.
// TODO: allow it to read data from remote node (there's an issue with JSON-PRC client).

type Client = StateClient<Hash>;

/// A staker
#[derive(Debug)]
pub struct Staker {
	ctrl: Option<AccountId>,
	stake: Balance,
}

/// Wrapper to pretty-print ksm (or any other 12 decimal) token.
struct KSM(Balance);

const DECIMAL_POINTS: Balance = 1_000_000_000_000;

impl fmt::Debug for KSM {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let num: u64 = self.0.try_into().unwrap();
		write!(f, "{}_KSM ({})", self.0 / DECIMAL_POINTS, num.separated_string())
	}
}

impl fmt::Display for KSM {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let num: u64 = self.0.try_into().unwrap();
		write!(f, "{}", num.separated_string())
	}
}

// Total issuance.
static mut ISSUANCE: *mut u128 = 0 as *mut u128;

/// some helpers to create some storage keys.
mod keys {
	use super::{StorageKey, blake2_256, twox_128};

	/// create key for a simple value.
	pub fn value(module: String, storage: String) -> StorageKey {
		let mut final_key = [0u8; 32];
		final_key[0..16].copy_from_slice(&twox_128(module.as_bytes()));
		final_key[16..32].copy_from_slice(&twox_128(storage.as_bytes()));
		StorageKey(final_key.to_vec())
	}

	/// create key for a map.
	pub fn map(module: String, storage: String, encoded_key: &[u8]) -> StorageKey {
		let module_key = twox_128(module.as_bytes());
		let storage_key = twox_128(storage.as_bytes());
		let key = blake2_256(encoded_key);
		let mut final_key = Vec::with_capacity(module_key.len() + storage_key.len() + key.len());
		final_key.extend_from_slice(&module_key);
		final_key.extend_from_slice(&storage_key);
		final_key.extend_from_slice(&key);
		StorageKey(final_key)
	}

	/// create key for a linked_map head.
	pub fn linked_map_head(module: String, storage: String) -> StorageKey {
		let head_prefix = "HeadOf".to_string() + &storage;
		let mut final_key = [0u8; 32];
		final_key[0..16].copy_from_slice(&twox_128(module.as_bytes()));
		final_key[16..32].copy_from_slice(&twox_128(head_prefix.as_bytes()));
		StorageKey(final_key.to_vec())
	}
}

/// Some helpers to read storage.
mod storage {
	use super::{StorageKey, Future, Decode, Debug, Linkage, Client};
	use super::keys;

	/// Read from a raw key regardless of the type.
	pub fn read<T: Decode>(key: StorageKey, client: &Client) -> Option<T> {
		let raw = client.storage(key, None).wait().unwrap();
		let encoded = raw.map(|d| d.0)?;
		<T as Decode>::decode(&mut encoded.as_slice()).ok()
	}

	/// enumerate and return all pairings of a linked map. Hopefully substrate will provide easier
	/// ways of doing this in the future.
	pub fn enumerate_linked_map<K, T>(
		module: String,
		storage: String,
		client: &Client
	) -> Vec<(K, T)>
		where K: Decode + Debug + Clone + AsRef<[u8]>, T: Decode + Clone + Debug,
	{
		let maybe_head_key = read::<K>(
			keys::linked_map_head(
				module.clone(),
				storage.clone(),
			),
			&client,
		);
		if let Some(head_key) = maybe_head_key {
			let mut ptr = head_key;
			let mut enumerations = Vec::<(K, T)>::new();
			loop {
				let (next_value, next_key) = read::<(T, Linkage<K>)>(
					keys::map(
						module.clone(),
						storage.clone(),
						ptr.as_ref(),
					),
					&client,
				).unwrap();

				enumerations.push((
					ptr,
					next_value,
				));

				if let Some(next) = next_key.next {
					ptr = next;
				} else {
					break;
				}
			}
			enumerations
		} else {
			vec![]
		}
	}
}

/// Some implementations that need to be in sync with how the network is working. See the runtime
/// of the node to which you are connecting for details.
mod network {
	use super::{Balance, Convert, Client, AccountId};
	use super::{storage, keys};

	pub const TOLERANCE: u128 = 0_u128;
	/// a way to attach the total issuance to `CurrencyToVoteHandler`.
	pub trait GetTotalIssuance {
		fn get_total_issuance() -> Balance;
	}

	pub struct CurrencyToVoteHandler<T>(std::marker::PhantomData<T>);
	impl<T: GetTotalIssuance> CurrencyToVoteHandler<T> {
		fn factor() -> u128 {
			(T::get_total_issuance() / u64::max_value() as u128).max(1)
		}
	}

	impl<T: GetTotalIssuance> Convert<u128, u64> for CurrencyToVoteHandler<T> {
		fn convert(x: Balance) -> u64 { (x / Self::factor()) as u64 }
	}

	impl<T: GetTotalIssuance> Convert<u128, u128> for CurrencyToVoteHandler<T> {
		fn convert(x: u128) -> Balance { x * Self::factor() }
	}

	pub fn get_nick(client: &Client, who: &AccountId) -> String {
		let nick = storage::read::<(Vec<u8>, Balance)>(
			keys::map("Sudo".to_string(), "NameOf".to_string(), who.as_ref()),
			client,
		);

		if nick.is_some() {
			String::from_utf8(nick.unwrap().0).unwrap()
		} else {
			String::from("NO_NICK")
		}
	}
}

mod staking_utils {
	use super::{AccountId, storage, keys, Staker, Balance, Client};
	use staking::{ValidatorPrefs, Nominations, StakingLedger};

	pub fn get_candidates(client: &Client) -> Vec<AccountId> {
		storage::enumerate_linked_map::<
			AccountId,
			ValidatorPrefs,
		>(
			"Staking".to_string(),
			"Validators".to_string(),
			client,
		).into_iter().map(|(v, _p)| v).collect::<Vec<AccountId>>()
	}

	pub fn get_voters(client: &Client) -> Vec<(AccountId, Vec<AccountId>)> {
		storage::enumerate_linked_map::<
			AccountId,
			Nominations<AccountId>,
		>(
			"Staking".to_string(),
			"Nominators".to_string(),
			client,
		)
			.into_iter()
			.map(|(who, n)| (who, n.targets))
			.collect::<Vec<(AccountId, Vec<AccountId>)>>()
	}

	pub fn get_staker_info_entry(stash: &AccountId, client: &Client) -> Staker {
		let ctrl = storage::read::<AccountId>(
			keys::map("Staking".to_string(), "Bonded".to_string(), stash.as_ref()),
			&client,
		).expect("All stakers must have a ledger.");

		let ledger = storage::read::<StakingLedger<AccountId, Balance>>(
			keys::map("Staking".to_string(), "Ledger".to_string(), ctrl.as_ref()),
			&client,
		).expect("All stakers must have a ledger.");

		Staker { ctrl: Some(ctrl), stake: ledger.active }
	}
}

mod election_utils {
	use super::{AccountId, storage, keys, Staker, Balance, Client};
	const MODULE: &'static str = "PhragmenElection";

	pub fn get_candidates(client: &Client) -> Vec<AccountId> {
		let mut members = storage::read::<Vec<(AccountId, Balance)>>(
			keys::value(MODULE.to_string(), "Members".to_string()),
			client,
		).unwrap_or_default().into_iter().map(|(m, _)| m).collect::<Vec<AccountId>>();

		let runners = storage::read::<Vec<(AccountId, Balance)>>(
			keys::value(MODULE.to_string(), "RunnersUp".to_string()),
			client,
		).unwrap_or_default().into_iter().map(|(m, _)| m).collect::<Vec<AccountId>>();

		let candidates = storage::read::<Vec<AccountId>>(
			keys::value(MODULE.to_string(), "Candidates".to_string()),
			client,
		).unwrap_or_default();

		members.extend(candidates);
		members.extend(runners);

		members
	}

	pub fn get_voters(client: &Client) -> Vec<(AccountId, Vec<AccountId>)> {
		storage::enumerate_linked_map::<
			AccountId,
			Vec<AccountId>,
		>(
			MODULE.to_string(),
			"VotesOf".to_string(),
			client,
		)
			.into_iter()
			.collect::<Vec<(AccountId, Vec<AccountId>)>>()
	}

	pub fn get_staker_info_entry(voter: &AccountId, client: &Client) -> Staker {
		let stake = storage::read::<Balance>(
			keys::map(MODULE.to_string(), "StakeOf".to_string(), voter.as_ref()),
			&client,
		).unwrap_or_default();

		Staker { ctrl: None, stake }
	}
}

fn main() {
	rt::run(rt::lazy(|| {
		// WILL NOT WORK. to connect to a remote node. Yet, the ws client is not being properly
		// created and there is no way to pass SSL cert. stuff.
		// let uri = "wss://kusama-rpc.polkadot.io/";
		// let target_url = url::Url::parse(uri).unwrap();
		// let client: Client = ws::connect(&target_url).wait().unwrap();

		// connect to a local node.
		let uri = "http://localhost:9933";
		let client: Client = http::connect::<Client>(uri).wait().unwrap();

		let matches = App::new("offline-phragmen")
			.version("0.1")
			.author("Kian Paimani <kian@parity.io>")
			.about("Runs the phragmen election algorithm of any substrate chain with staking module offline (aka. off the chain) and predicts the results.")
			.arg(Arg::with_name("count")
				.short("c")
				.long("count")
				.help("count of member/validators to elect. Default is 50.")
				.takes_value(true)
			).arg(Arg::with_name("network")
				.short("n")
				.long("network")
				.help("network address format. Can be kusama|polkadot|substrate. Default is kusama.")
				.takes_value(true)
			).arg(Arg::with_name("output")
				.short("o")
				.long("output")
				.help("json output file name. dumps the results into if given.")
				.takes_value(true)
			).arg(Arg::with_name("min-count")
				.short("m")
				.long("min-count")
				.help("minimum number of members/validators to elect. If less candidates are available, phragmen will go south. Default is 0.")
				.takes_value(true)
			).arg(Arg::with_name("iterations")
				.short("i")
				.long("iters")
				.help("number of post-processing iterations to run. Default is 2")
				.takes_value(true)
			).arg(Arg::with_name("no-self-vote")
				.short("s")
				.long("no-self-vote")
				.help("disable self voting for candidates")
			).arg(Arg::with_name("elections")
				.short("e")
				.long("elections")
				.help("execute the council election.")
			).arg(Arg::with_name("verbose")
				.short("v")
				.multiple(true)
				.long("verbose")
				.help("Print more output")
			).get_matches();

		let validator_count = matches.value_of("count")
			.unwrap_or("50")
			.parse()
			.unwrap();
		let minimum_validator_count = matches.value_of("min-count")
			.unwrap_or("0")
			.parse()
			.unwrap();
		let iterations: usize = matches.value_of("iterations")
			.unwrap_or("2")
			.parse()
			.unwrap();
		let verbosity = matches.occurrences_of("v");

		// chose json output file.
		let maybe_output_file = matches.value_of("output");

		// self-vote?
		let do_self_vote = !matches.is_present("no-self-vote");

		// staking or elections?
		let do_elections = matches.is_present("elections");

		// Get the total issuance and update the global pointer to it.
		let maybe_total_issuance = storage::read::<Balance>(
			keys::value(
				"Balances".to_string(),
				"TotalIssuance".to_string()
			),
			&client,
		);
		struct TotalIssuance;
		impl network::GetTotalIssuance for TotalIssuance {
			fn get_total_issuance() -> Balance {
				unsafe {
					*ISSUANCE
				}
			}
		}
		let mut total_issuance = maybe_total_issuance.unwrap_or(0);
		unsafe { ISSUANCE = &mut total_issuance; }

		// setup address format
		let addr_format = match matches.value_of("network").unwrap_or("kusama") {
			"kusama" => Ss58AddressFormat::KusamaAccountDirect,
			"polkadot" => Ss58AddressFormat::PolkadotAccountDirect,
			"substrate" => Ss58AddressFormat::SubstrateAccountDirect,
			_ => panic!("invalid address format"),
		};
		set_default_ss58_version(addr_format);

		// start file scraping timer.
		let start_data = std::time::Instant::now();

		// stash key of all wannabe candidates.
		let candidates = if do_elections { election_utils::get_candidates(&client) } else { staking_utils::get_candidates(&client) };

		// stash key of current nominators
		let mut voters = if do_elections { election_utils::get_voters(&client) } else { staking_utils::get_voters(&client) };

		// add self-vote
		if do_self_vote {
			candidates.iter().for_each(|v| {
				let self_vote = (v.clone(), vec![v.clone()]);
				voters.push(self_vote);
			});
		}

		// get the slashable balance of every entity
		let mut staker_infos: BTreeMap<AccountId, Staker> = BTreeMap::new();

		let mut all_stakers= candidates.clone();
		all_stakers.extend(voters.iter().map(|(n, _)| n.clone()).collect::<Vec<AccountId>>());
		all_stakers.iter().for_each(|stash| {
			let staker_info = if do_elections { election_utils::get_staker_info_entry(&stash, &client) } else { staking_utils::get_staker_info_entry(&stash, &client) };
			staker_infos.insert(stash.clone(), staker_info);
		});

		let slashable_balance = |who: &AccountId| -> Balance {
			// NOTE: if we panic here then someone has voted for a non-candidate afaik.
			staker_infos.get(who).unwrap().stake
		};

		// run phragmen
		let data_elapsed = start_data.elapsed().as_millis();

		let start_phragmen = std::time::Instant::now();
		let PhragmenResult { winners, assignments } = elect::<
			AccountId,
			Balance,
			_,
			network::CurrencyToVoteHandler<TotalIssuance>
		>(
			validator_count,
			minimum_validator_count,
			candidates.clone(),
			voters.clone(),
			slashable_balance,
		).ok_or("Phragmen failed to elect.").unwrap();

		let to_votes = |b: Balance|
			<network::CurrencyToVoteHandler<TotalIssuance> as Convert<Balance, u64>>::convert(b) as u128;

		let elected_stashes = winners.iter().map(|(s, _)| s.clone()).collect::<Vec<AccountId>>();
		let mut supports = build_support_map::<Balance, AccountId, _, network::CurrencyToVoteHandler<TotalIssuance>>(
			&elected_stashes,
			&assignments,
			slashable_balance,
		);

		if iterations > 0 {
			// prepare and run post-processing.
			let mut staked_assignments
			: Vec<(AccountId, Vec<PhragmenStakedAssignment<AccountId>>)>
			= Vec::with_capacity(assignments.len());
			for (n, assignment) in assignments.iter() {
				let mut staked_assignment
				: Vec<PhragmenStakedAssignment<AccountId>>
				= Vec::with_capacity(assignment.len());
				for (c, per_thing) in assignment.iter() {
					let nominator_stake = to_votes(slashable_balance(n));
					let other_stake = *per_thing * nominator_stake;
					staked_assignment.push((c.clone(), other_stake));
				}
				staked_assignments.push((n.clone(), staked_assignment));
			}

			equalize::<
				_,
				_,
				network::CurrencyToVoteHandler<TotalIssuance>,
				_,
			>(
				staked_assignments,
				&mut supports,
				network::TOLERANCE,
				iterations,
				slashable_balance,
			);
		}

		let phragmen_elapsed = start_phragmen.elapsed().as_millis();

		let mut slot_stake = u128::max_value();
		let mut nominator_info: BTreeMap<AccountId, Vec<(AccountId, Balance)>> = BTreeMap::new();

		println!("\n######################################\n+++ Winner Validators:");
		winners.iter().enumerate().for_each(|(i, s)| {
			println!("#{} == {} [{:?}]", i + 1, network::get_nick(&client, &s.0), s.0);
			let support = supports.get(&s.0).unwrap();
			let others_sum: Balance = support.voters.iter().map(|(_n, s)| s).sum();
			let other_count = support.voters.len();

			println!(
				"[stake_total: {:?}] [vote_count: {}] [ctrl: {:?}]",
				KSM(others_sum),
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

			assert_eq!(others_sum, support.total);

		});

		if verbosity >= 2 {
			println!("\n######################################\n+++ Updated Assignments:");
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

		println!("============================");
		println!("++ connected to [{}]", uri);
		println!("++ total_issuance = {:?}", KSM(total_issuance));
		println!("++ candidates intentions count {:?}", candidates.len());
		println!("++ voters intentions count {:?}", voters.len());
		println!(
			"++ args: [count to elect = {}] [min-count = {}] [output = {:?}] [iterations = {}] [do_self_vote {}] [do_elections {}]",
			validator_count,
			minimum_validator_count,
			maybe_output_file,
			iterations,
			do_self_vote,
			do_elections,
		);
		println!("++ final slot_stake {:?}", KSM(slot_stake));
		println!("++ Data fetch Completed in {} ms.", data_elapsed);
		println!("++ Phragmen Completed in {} ms.", phragmen_elapsed);
		println!("++ Phragmen Assignment size {} bytes.", codec::Encode::encode(&assignments).len());

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

		futures::future::ok::<(), ()>(())

	}))
}

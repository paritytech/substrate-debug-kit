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
use jsonrpc_core_client::transports::{http};
use substrate_rpc_api::state::StateClient;


use polkadot_primitives::{Hash, Balance, AccountId};
use substrate_primitives::storage::StorageKey;
use substrate_primitives::hashing::{blake2_256, twox_128};
use substrate_phragmen::{elect, equalize, PhragmenResult, PhragmenStakedAssignment, Support, SupportMap};
use sr_primitives::traits::Convert;
use support::storage::generator::Linkage;
use staking::{StakingLedger, ValidatorPrefs};

// TODO: clean function interfaces: probably no more passing string.
// TODO: show address in valid kusama format.
// TODO: allow it to read data from remote node (there's an issue with JSON-PRC client).
// TODO: read number of candidates and minimum from the chain.

type Client = StateClient<Hash>;

/// A staker
#[derive(Debug)]
struct Staker {
	ctrl: AccountId,
	ledger: StakingLedger<AccountId, Balance>,
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
		let storage_key = module + " " + &storage;
		StorageKey(twox_128(storage_key.as_bytes()).to_vec())
	}

	/// create key for a map.
	pub fn map(module: String, storage: String, encoded_key: &[u8]) -> StorageKey {
		let mut storage_key = Vec::from((module + " " + &storage).as_bytes());
		storage_key.extend_from_slice(&encoded_key);
		StorageKey(blake2_256(storage_key.as_slice()).to_vec())
	}

	/// create key for a linked_map head.
	pub fn linked_map_head(module: String, storage: String, encoded_key: &[u8]) -> StorageKey {
		let key_string = "head of ".to_string() + &module + " " + &storage;
		let mut key = key_string.as_bytes().to_vec();
		key.extend_from_slice(&encoded_key);
		// StorageKey(key)
		StorageKey(blake2_256(key.as_slice()).to_vec())
	}
}

/// Some helpers to read storage.
mod storage {
	use super::{StorageKey, Future, Decode, Debug, Linkage, Client};
	use super::storage;
	use super::keys;

	/// Read from a raw key regardless of the type.
	pub fn read<T: Decode>(key: StorageKey, client: &Client) -> Option<T> {
		let encoded = client.storage(key, None).wait().unwrap().map(|d| d.0)?;
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
		let maybe_head_key = storage::read::<K>(
			keys::linked_map_head(
				module.clone(),
				storage.clone(),
				"".as_bytes(),
			),
			&client,
		);

		if let Some(head_key) = maybe_head_key {
			let mut ptr = head_key;
			let mut enumerations = Vec::<(K, T)>::new();
			loop {
				let (next_value, next_key) = storage::read::<(T, Linkage<K>)>(
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
	use super::{Balance, Convert};

	pub const TOLERANCE: u128 = 0_u128;
	pub const ITERATIONS: usize = 2_usize;

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
}

fn main() {
	rt::run(rt::lazy(|| {
		// WILL NOT WORK. to connect to a remote node. Yet, the ws client is not being properly
		// created and there is no way to pass SSL cert. stuff.
		// let uri = "wss://canary-5.kusama.network/";
		// let URL = url::Url::parse(uri).unwrap();
		// let client: Client<Hash> = ws::connect(&URL).wait().unwrap();

		// connect to a local node.
		let uri = "http://localhost:9933";
		let client: Client = http::connect(uri).wait().unwrap();


		let matches = App::new("offline-phragmen")
			.version("0.1")
			.author("Kian Paimani <kian@parity.io>")
			.about("Runs the phragmen election algorithm of any substrate chain with staking module offline (aka. off the chain) and predicts the results.")
			.arg(Arg::with_name("count")
				.short("c")
				.long("count")
				.value_name("VALIDATOR_COUNT")
				.help("count of validators to elect. Should be equal to chain.staking.validatorCount. Default is 50.")
				.takes_value(true)
			).arg(Arg::with_name("network")
				.short("n")
				.long("network")
				.value_name("NETWORK")
				.help("network address format. Can be kusama|polkadot|substrate. Default is kusama.")
				.takes_value(true)
			).arg(Arg::with_name("output")
				.short("o")
				.long("output")
				.value_name("FILE")
				.help("Json output file name. dumps the results into if given.")
				.takes_value(true)
			).arg(Arg::with_name("min-count")
				.short("m")
				.long("min-count")
				.value_name("MIN_VALIDATOR_COUNT")
				.help("minimum number of validators to elect. If less candidates are available, phragmen will go south. Should be equal to chain.staking.minimumValidatorCount. Default is 10.")
				.takes_value(true)
			).get_matches();

		let validator_count = matches.value_of("count")
			.unwrap_or("50")
			.parse()
			.unwrap_or(50);
		let minimum_validator_count = matches.value_of("min-count")
			.unwrap_or("10")
			.parse()
			.unwrap_or(10);

		// setup address format
		let addr_format = match matches.value_of("network").unwrap_or("kusama") {
			"kusama" => Ss58AddressFormat::KusamaAccountDirect,
			"polkadot" => Ss58AddressFormat::PolkadotAccountDirect,
			"substrate" => Ss58AddressFormat::SubstrateAccountDirect,
			_ => panic!("invalid address format"),
		};
		use substrate_primitives::crypto::{set_default_ss58_version, Ss58AddressFormat};
		set_default_ss58_version(addr_format);

		// chose json output file
		let output_file = matches.value_of("output");
		if output_file.is_some() {
			panic!("output to json not implemented.");
		}

		// stash key of all wannabe candidates.
		let validators = storage::enumerate_linked_map::<
			AccountId,
			ValidatorPrefs<Balance>,
		>(
			"Staking".to_string(),
			"Validators".to_string(),
			&client
		).into_iter().map(|(v, _p)| v).collect::<Vec<AccountId>>();

		// stash key of current nominators
		let nominators = storage::enumerate_linked_map::<
			AccountId,
			Vec<AccountId>,
		>(
			"Staking".to_string(),
			"Nominators".to_string(),
			&client,
		);

		// get the slashable balance of every entity
		let mut staker_infos: BTreeMap<AccountId, Staker> = BTreeMap::new();

		let mut all_stakers = validators.clone();
		all_stakers.extend(nominators.iter().map(|(n, _)| n.clone()).collect::<Vec<AccountId>>());
		all_stakers.iter().for_each(|stash| {
			let ctrl = storage::read::<AccountId>(
				keys::map("Staking".to_string(), "Bonded".to_string(), stash.as_ref()),
				&client,
			).expect("All stakers must have a ledger.");

			let ledger = storage::read::<StakingLedger<AccountId, Balance>>(
				keys::map("Staking".to_string(), "Ledger".to_string(), ctrl.as_ref()),
				&client,
			).expect("All stakers must have a ledger.");

			staker_infos.insert(stash.clone(), Staker { ctrl, ledger});
		});

		let slashable_balance = |who: &AccountId| -> Balance {
			// NOTE: if we panic here then someone has voted for a non-candidate afaik.
			staker_infos.get(who).unwrap().ledger.active
		};

		// Get the total issuance and update the global pointer to it.
		let mut total_issuance = storage::read::<Balance>(
			keys::value(
				"Balances".to_string(),
				"TotalIssuance".to_string()
			),
			&client,
		).unwrap();
		unsafe { ISSUANCE = &mut total_issuance; }

		struct TotalIssuance;
		impl network::GetTotalIssuance for TotalIssuance {
			fn get_total_issuance() -> Balance {
				unsafe {
					*ISSUANCE
				}
			}
		}

		// run phragmen
		let PhragmenResult { winners, assignments } = elect::<
			AccountId,
			Balance,
			_,
			network::CurrencyToVoteHandler<TotalIssuance>
		>(
			validator_count,
			minimum_validator_count,
			validators.clone(),
			nominators.clone(),
			slashable_balance,
			true,
		).ok_or("Phragmen failed to elect.").unwrap();
		let elected_stashes = winners.iter().map(|(s, _)| s.clone()).collect::<Vec<AccountId>>();

		let to_votes = |b: Balance|
			<network::CurrencyToVoteHandler<TotalIssuance> as Convert<Balance, u64>>::convert(b) as u128;

		// Initialize the support of each candidate.
		let mut supports = <SupportMap<AccountId>>::new();
		elected_stashes
			.iter()
			.map(|e| (e, to_votes(slashable_balance(e))))
			.for_each(|(e, s)| {
				let item = Support { own: s, total: s, ..Default::default() };
				supports.insert(e.clone(), item);
			});

		// build support struct.
		for (n, assignment) in assignments.iter() {
			for (c, per_thing) in assignment.iter() {
				let nominator_stake = to_votes(slashable_balance(n));
				let other_stake = *per_thing * nominator_stake;
				let support= supports.get_mut(c).unwrap();
				support.total = support.total.saturating_add(other_stake);
				support.others.push((n.clone(), other_stake));
			}
		}

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
			network::ITERATIONS,
			slashable_balance,
		);

		let mut slot_stake = u128::max_value();
		let mut nominator_info: BTreeMap<AccountId, Vec<(AccountId, Balance)>> = BTreeMap::new();

		println!("\n######################################\n+++ Winner Validators:");
		winners.iter().enumerate().for_each(|(i, s)| {
			println!("#{} == {:?}", i + 1, s.0);
			let support = supports.get(&s.0).unwrap();
			let others_sum: Balance = support.others.iter().map(|(_n, s)| s).sum();
			let other_count = support.others.len();
			println!(
				"  [stake_total: {:?}] [stake_own: {:?} ({}%)] [other_stake_sum: {:?} ({}%)] [other_stake_count: {}] [ctrl: {:?}]",
				KSM(support.total),
				KSM(support.own),
				support.own * 100 / support.total,
				KSM(others_sum),
				others_sum * 100 / support.total,
				other_count,
				staker_infos.get(&s.0).unwrap().ctrl,
			);
			assert_eq!(support.total, support.own + others_sum);
			if support.total < slot_stake { slot_stake = support.total; }
			println!("  Voters:");
			support.others.iter().enumerate().for_each(|(i, o)| {
				println!("	#{} [amount = {:?}] {:?}", i, KSM(o.1), o);

				nominator_info.entry(o.0.clone()).or_insert(vec![]).push((s.0.clone(), o.1));
			});
		});

		println!("\n######################################\n+++ Updated Assignments:");
		let mut counter = 1;
		for (nominator, info) in nominator_info.iter() {
			let staker_info = staker_infos.get(&nominator).unwrap();
			let mut sum = 0;
			println!("#{} {:?} // active_stake = {:?}", counter, nominator, KSM(staker_info.ledger.active));
			println!("  Distributions:");
			info.iter().enumerate().for_each(|(i, (c, s))| {
				sum += *s;
				println!("    #{} {:?} => {:?}", i, c, KSM(*s));
			});
			counter += 1;
			let diff = sum.max(staker_info.ledger.active) - sum.min(staker_info.ledger.active);
			// acceptable diff is one millionth of a KSM
			assert!(diff < 1_000, "diff{ sum_nominations,  staker_info.ledger.active} = ");
			println!("");
		}

		println!("============================");
		println!("++ connected to [{}]", uri);
		println!("++ total_issuance = {:?}", KSM(total_issuance));
		println!("++ args: [count to elect = {}] [min-count = {}] [output = {:?}]", validator_count, minimum_validator_count, output_file);
		println!("++ validator intentions count {:?}", validators.len());
		println!("++ nominator intentions count {:?}", nominators.len());
		println!("++ final slot_stake {:?}", KSM(slot_stake));
		futures::future::ok::<(), ()>(())
	}))
}

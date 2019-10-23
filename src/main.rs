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

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

//! An extended version of the code in `substrate/node/rpc-client/` which reads the staking info
//! of a chain and runs the phragmen election with the given parameters offline.

use futures::Future;
use hyper::rt;
use std::{fmt::Debug, collections::BTreeMap};
use codec::Decode;

use substrate_rpc::state::StateClient;
use jsonrpc_core_client::transports::{http};

use node_primitives::{Hash, Balance, AccountId};
use support::storage::generator::Linkage;
use sr_primitives::traits::Convert;
use substrate_primitives::storage::StorageKey;
use substrate_primitives::hashing::{blake2_256, twox_128};
use substrate_phragmen::{elect, equalize, PhragmenResult, PhragmenStakedAssignment, Support, SupportMap};
use staking::{StakingLedger, ValidatorPrefs};

/// A staker
#[derive(Debug)]
struct Staker {
	ctrl: AccountId,
	ledger: StakingLedger<AccountId, Balance>,
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
	use super::{Hash, StateClient, StorageKey, Future, Decode, Debug, Linkage};
	use super::storage;
	use super::keys;

	/// Read from a raw key regardless of the type.
	pub fn read<T: Decode>(key: StorageKey, client: &StateClient<Hash>) -> Option<T> {
		let encoded = client.storage(key, None).wait().unwrap().map(|d| d.0)?;
		<T as Decode>::decode(&mut encoded.as_slice()).ok()
	}

	/// enumerate and return all pairings of a linked map. Hopefully substrate will provide easier
	/// ways of doing this in the future.
	pub fn enumerate_linked_map<K, T>(
		module: String,
		storage: String,
		client: &StateClient<Hash>
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

	pub const LIMIT: usize = 999;
	pub const VALIDATOR_COUNT: usize = 50;
	pub const MIN_VALIDATOR_COUNT: usize = 10;

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
		// connect to a local node.
		let uri = "http://localhost:9933";
		let client: StateClient<Hash> = http::connect(uri).wait().unwrap();

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

		println!("{{");
		println!("  \"validator_count\": {:?},", validators.len());
		println!("  \"nominator_count\": {:?},", nominators.len());

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

		println!("  \"total_issuance\": {},", total_issuance);
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
			network::LIMIT,
			network::MIN_VALIDATOR_COUNT,
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
		let nominator_info: BTreeMap<AccountId, Vec<(AccountId, Balance)>> = BTreeMap::new();

		println!("  \"candidates\":");
		println!("  [");
		winners.iter().enumerate().for_each(|(i, s)| {
			if i > 0 { println!("    ,"); }
			println!("    {{");
			println!("      \"rank\": {},", i + 1);
			println!("      \"pub_key_stash\": \"{}\",", staker_infos.get(&s.0).unwrap().ledger.stash);
			let support = supports.get(&s.0).unwrap();
			let others_sum: Balance = support.others.iter().map(|(_n, s)| s).sum();
			let other_count = support.others.len();

			println!("      \"stake_total\": {},", support.total);
			println!("      \"stake_validator\": {},", support.own);
			println!("      \"other_stake_sum\": {},", others_sum);
			println!("      \"other_stake_count\": {},", other_count);
			println!("      \"pub_key_controller\": \"{}\",", staker_infos.get(&s.0).unwrap().ctrl);
			println!("      \"voters\":");
			println!("      [");

			assert_eq!(support.total, support.own + others_sum);
			if support.total < slot_stake && i < network::VALIDATOR_COUNT { slot_stake = support.total; }
			support.others.iter().enumerate().for_each(|(j, o)| {
				if j > 0 { println!("        ,"); }
				println!("        {{");
				println!("          \"stake_nominator\": {},", o.1);
				println!("          \"pub_key_nominator\": \"{}\"", o.0);
				println!("        }}");
			});
			println!("      ]");
			println!("    }}");
		});
		println!("  ],");

		for (nominator, info) in nominator_info.iter() {
			let staker_info = staker_infos.get(&nominator).unwrap();
			let mut sum = 0;
			info.iter().enumerate().for_each(|(_i, (_c, s))| {
				sum += *s;
			});
			assert!(sum.max(staker_info.ledger.active) - sum.min(staker_info.ledger.active) < 10);
		}
		println!("  \"final_slot_stake\": \"{}\"", slot_stake);
		println!("}}");
		futures::future::ok::<(), ()>(())
	}))
}

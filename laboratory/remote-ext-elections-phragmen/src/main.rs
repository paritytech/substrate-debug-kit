//! A remote ext template for elections-phragmen pallet in a live chain.
//!
//! Note: You need to have this repo cloned next to a local substrate repo, and have all the
//! dependencies pointed to a local one. To do so, use `node update_cargo.js local`.

#![allow(unused_imports)]
use frame_support::{
	impl_outer_origin, migration::*, parameter_types, traits::Get,
	weights::constants::RocksDbWeight, IterableStorageMap, StorageMap, StoragePrefixedMap,
	Twox64Concat,
};
use pallet_elections_phragmen::*;
use sp_core::H256;
use sp_runtime::traits::IdentityLookup;
use std::{cell::RefCell, collections::BTreeMap};
use sub_storage::Hash;

#[cfg(feature = "kusama")]
type Token = tokens::KSM;
#[cfg(feature = "kusama")]
const SPEC_REFCOUNT_U32: u32 = 2025;
#[cfg(feature = "kusama")]
const CHAIN: &'static str = "kusama";

#[cfg(not(feature = "kusama"))]
type Token = tokens::DOT;
#[cfg(not(feature = "kusama"))]
const SPEC_REFCOUNT_U32: u32 = 25;
#[cfg(not(feature = "kusama"))]
const CHAIN: &'static str = "polkadot";

use paste::paste;
macro_rules! parameter_types_thread_local {
	(
		$(
			static $name:ident : $type:ty = $default:expr;
		)*
	) => {
		parameter_types_thread_local! {
			@THREAD_LOCAL($(
				$name, $type, $default,
			)*)
		}

		parameter_types_thread_local! {
			@GETTER_STRUCT($(
				$name, $type,
			)*)
		}
	};
	(@THREAD_LOCAL($($name:ident, $type:ty, $default:expr,)*)) => {
		thread_local! {
			$(
				static $name: RefCell<$type> = RefCell::new($default);
			)*
		}
	};
	(@GETTER_STRUCT($($name:ident, $type:ty,)*)) => {
		$(
			paste! {
				pub struct [<$name:camel>];
				impl Get<$type> for [<$name:camel>] {
					fn get() -> $type { $name.with(|v| v.borrow().clone() )}
				}
				impl [<$name:camel>] {
					#[allow(dead_code)]
					fn set(t: $type) {
						$name.with(|v| *v.borrow_mut() = t);
					}
				}
			}
		)*
	}
}

pub mod time {
	use super::*;
	pub const MILLISECS_PER_BLOCK: Moment = 6000;
	pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;
	pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 1 * HOURS;

	pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;

	pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);
}

pub struct CurrencyToVoteHandler;
impl CurrencyToVoteHandler {
	fn factor() -> Balance {
		(Balances::total_issuance() / u64::max_value() as Balance).max(1)
	}
}
impl sp_runtime::traits::Convert<Balance, u64> for CurrencyToVoteHandler {
	fn convert(x: Balance) -> u64 {
		(x / Self::factor()) as u64
	}
}
impl sp_runtime::traits::Convert<u128, Balance> for CurrencyToVoteHandler {
	fn convert(x: u128) -> Balance {
		x * Self::factor()
	}
}

pub(crate) type AccountId = sp_core::crypto::AccountId32;
pub(crate) type BlockNumber = u32;
pub(crate) type Balance = u128;
pub(crate) type Moment = u64;
pub(crate) type Header = sp_runtime::generic::Header<BlockNumber, sp_runtime::traits::BlakeTwo256>;

pub(crate) type Balances = pallet_balances::Module<Runtime>;
pub(crate) type Elections = pallet_elections_phragmen::Module<Runtime>;
pub(crate) type WrongElections = pallet_elections_phragmen_faulty::Module<Runtime>;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Runtime;

impl_outer_origin! {
	pub enum Origin for Runtime where system = frame_system {}
}

impl frame_system::Trait for Runtime {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Index = u32;
	type BlockNumber = BlockNumber;
	type Call = ();
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = ();
	type MaximumBlockWeight = ();
	type DbWeight = RocksDbWeight;
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = ();
	type AvailableBlockRatio = ();
	type MaximumBlockLength = ();
	type Version = ();
	// type ModuleToIndex = ();
	type PalletInfo = ();
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
}

impl pallet_balances::Trait for Runtime {
	type Balance = Balance;
	type MaxLocks = ();
	type Event = ();
	type DustRemoval = ();
	type ExistentialDeposit = ();
	type AccountStore = frame_system::Module<Runtime>;
	type WeightInfo = ();
}

parameter_types_thread_local! {
	static DESIRED_MEMBERS: u32 = 13;
	static DESIRED_RUNNERS_UP: u32 = 20;
	static VOTING_BOND: Balance = 5 * DOLLARS;
	static CANDIDACY_BOND: Balance = 100 * DOLLARS;
}

parameter_types! {
	pub const ElectionsPhragmenModuleId: frame_support::traits::LockIdentifier = *b"phrelect";
	// well I am assuming that these never changed in polkadot...
	pub const TermDuration: BlockNumber = 7 * time::DAYS;
}

impl pallet_elections_phragmen::Trait for Runtime {
	type ModuleId = ElectionsPhragmenModuleId;
	type Event = ();
	type Currency = Balances;
	type CurrencyToVote = frame_support::traits::U128CurrencyToVote;
	// type CurrencyToVote = CurrencyToVoteHandler;
	type ChangeMembers = ();
	type InitializeMembers = ();
	type CandidacyBond = CandidacyBond;
	type VotingBond = VotingBond;
	type TermDuration = TermDuration;
	type DesiredMembers = DesiredMembers;
	type DesiredRunnersUp = DesiredRunnersUp;
	type LoserCandidate = ();
	type KickedMember = ();
	type BadReport = ();
	type WeightInfo = ();
}
impl pallet_elections_phragmen_faulty::Trait for Runtime {
	type ModuleId = ElectionsPhragmenModuleId;
	type Event = ();
	type Currency = Balances;
	type CurrencyToVote = frame_support::traits::U128CurrencyToVote;
	// type CurrencyToVote = CurrencyToVoteHandler;
	type ChangeMembers = ();
	type InitializeMembers = ();
	type CandidacyBond = CandidacyBond;
	type VotingBond = VotingBond;
	type TermDuration = TermDuration;
	type DesiredMembers = DesiredMembers;
	type DesiredRunnersUp = DesiredRunnersUp;
	type LoserCandidate = ();
	type KickedMember = ();
	type BadReport = ();
	type WeightInfo = ();
}

const URI: &'static str = "ws://localhost:9944";

pub const DOTS: Balance = 1_000_000_000_000;
pub const DOLLARS: Balance = DOTS / 100;
pub const CENTS: Balance = DOLLARS / 100;
pub const MILLICENTS: Balance = CENTS / 1_000;

fn is_subset_of<T: Eq + PartialEq + Clone>(x: &[T], y: &[T]) -> Result<(), ()> {
	let mut y = y.clone().to_vec();
	x.iter()
		.map(|e| {
			let idx = y.iter().position(|z| z == e).ok_or(())?;
			y.remove(idx);
			Ok(())
		})
		.collect::<Result<_, _>>()
}

fn migrate_back_to_u8_ref_count() {
	log::warn!("Migrating accounts back to u8 ref count");
	frame_system::Account::<Runtime>::translate::<
		(u32, u32, <Runtime as frame_system::Trait>::AccountData),
		_,
	>(|_key, (nonce, rc, data)| {
		Some(frame_system::AccountInfo {
			nonce,
			refcount: rc as u8,
			data,
		})
	});
}

macro_rules! tokenify {
	($list:ident) => {
		$list.iter().map(|x| Token::from(*x)).collect::<Vec<_>>()
	};
}

#[async_std::main]
async fn main() -> () {
	env_logger::init();
	sp_core::crypto::set_default_ss58_version(sp_core::crypto::Ss58AddressFormat::PolkadotAccount);
	let client = sub_storage::create_ws_client(URI).await;

	// let elections = if cfg!(feature = "kusama") {
	// 	include!("elections.kusama")
	// } else {
	// 	include!("elections.polkadot")
	// }
	let elections = vec![(
		"683d498f5778918b269bc2b0a9622267e61700f369c6957b1e4a265be3ce1e24",
		vec![1000000000000],
		"2020-08-19T00:07:18+00:00",
	)]
	.iter()
	.rev()
	.map(|(h, x, y)| (hex::decode(h).unwrap(), x, y))
	.map(|(h, x, y)| {
		let mut hash = [0u8; 32];
		assert_eq!(h.len(), 32);
		hash.copy_from_slice(h.as_ref());
		(Hash::from(hash), x.clone(), *y)
	})
	.collect::<Vec<(Hash, Vec<Balance>, &str)>>();

	let mut slash_record: BTreeMap<AccountId, Vec<(Hash, Balance, Balance)>> = BTreeMap::new();
	for (at, deposits, timestamp) in elections {
		let header = sub_storage::get_header(&client, at).await;
		let parent = header.parent_hash;

		let spec = sub_storage::get_runtime_version(&client, at)
			.await
			.spec_version;

		let response = reqwest::blocking::get(
			&format!(
				"https://explorer-31.polkascan.io/{}/api/v1/runtime-module/{}-electionsphragmen?include=calls,events,storage,constants,errors",
				CHAIN,
				spec,
			)
		).unwrap().text().unwrap();
		let parsed = json::parse(&response).unwrap();
		&parsed["included"].members().for_each(|x| {
			if x["type"] == "runtimeconstant"
				&& x["id"] == format!("{}-electionsphragmen-DesiredRunnersUp", spec)
			{
				let d: u32 = x["attributes"]["value"].to_string().parse().unwrap();
				DesiredRunnersUp::set(d);
			}
			if x["type"] == "runtimeconstant"
				&& x["id"] == format!("{}-electionsphragmen-DesiredMembers", spec)
			{
				let d: u32 = x["attributes"]["value"].to_string().parse().unwrap();
				DesiredMembers::set(d);
			}
			if x["type"] == "runtimeconstant"
				&& x["id"] == format!("{}-electionsphragmen-VotingBond", spec)
			{
				let d: Balance = x["attributes"]["value"].to_string().parse().unwrap();
				VotingBond::set(d);
			}
			if x["type"] == "runtimeconstant"
				&& x["id"] == format!("{}-electionsphragmen-CandidacyBond", spec)
			{
				let d: Balance = x["attributes"]["value"].to_string().parse().unwrap();
				CandidacyBond::set(d);
			}
		});

		if deposits.len() == 0 || deposits.iter().all(|d| *d > CandidacyBond::get()) {
			log::warn!(
				"No deposits smaller than candidacy bond at {}, thus no slashes. Skipping.",
				at
			);
			continue;
		}

		println!("📅 Timestamp {:?}", timestamp);
		println!(
			"🧮 Spec = {}, Members = {}, RunnersUp = {}, Voting = {}, Candidacy = {}",
			spec,
			DesiredMembers::get(),
			DesiredRunnersUp::get(),
			Token::from(VotingBond::get()),
			Token::from(CandidacyBond::get()),
		);

		// for sanity-check, check the chain state. note that now we do `at`, not `parent`.
		let (post_election_members, post_chain_runners_up) = {
			(
				sub_storage::read::<Vec<(AccountId, Balance)>>(
					sub_storage::value_key(b"PhragmenElection", b"Members"),
					&client,
					at,
				)
				.await
				.unwrap()
				.into_iter()
				.map(|(x, _)| x)
				.collect::<Vec<_>>(),
				sub_storage::read::<Vec<(AccountId, Balance)>>(
					sub_storage::value_key(b"PhragmenElection", b"RunnersUp"),
					&client,
					at,
				)
				.await
				.unwrap()
				.into_iter()
				.map(|(x, _)| x)
				.collect::<Vec<_>>(),
			)
		};

		// get correct slashes.
		let correct_slashed = remote_externalities::Builder::new()
			.module("PhragmenElection")
			.module("Balances")
			.module("System")
			.at(parent)
			.build_async()
			.await
			.execute_with(|| {
				if spec >= SPEC_REFCOUNT_U32 {
					migrate_back_to_u8_ref_count()
				}
				let result = Elections::do_phragmen();
				assert_eq!(Elections::members_ids(), post_election_members);
				assert_eq!(Elections::runners_up_ids(), post_chain_runners_up);
				result
			});

		remote_externalities::Builder::new()
			.module("PhragmenElection")
			.module("Balances")
			.module("System")
			.at(parent)
			.build_async()
			.await
			.execute_with(|| {
				if spec >= SPEC_REFCOUNT_U32 {
					migrate_back_to_u8_ref_count()
				}
				let wrong_slashed = WrongElections::do_phragmen();
				assert_eq!(Elections::members_ids(), post_election_members);
				assert_eq!(Elections::runners_up_ids(), post_chain_runners_up);

				let should_deposit = wrong_slashed
					.iter()
					.filter(|(_, effective, _)| *effective > 0)
					.map(|(_, x, _)| x)
					.cloned()
					.collect::<Vec<_>>();

				wrong_slashed.iter().for_each(|w| {
					if correct_slashed.iter().find(|x| x.0 == w.0).is_some() {
						// any correct slash must be not a member or runner-up anymore.
						assert!(
							!post_election_members.contains(&w.0)
								&& !post_chain_runners_up.contains(&w.0)
						);
						println!("👀 ✅  {:?} was correctly slashed", w);
					} else {
						slash_record
							.entry(w.0.clone())
							.or_default()
							.push((at, w.1, w.2));
						println!(
							"👎🏻 wrongly slashed = {} -> actual: {} / leftover: {}",
							w.0, w.1, w.2
						)
					}
				});

				println!("💰 Deposits = {:?}", tokenify!(deposits));
				println!("💰 Effective slashes = {:?}", tokenify!(should_deposit));

				assert!(is_subset_of(&should_deposit, &deposits).is_ok());
			});
	}

	#[derive(Debug)]
	pub enum Stat {
		Out,
		RunnerUp,
		Member,
	}

	impl std::fmt::Display for Stat {
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			match self {
				Stat::Out => write!(f, "⬇️ Out"),
				Stat::Member => write!(f, "✅ Member"),
				Stat::RunnerUp => write!(f, "✅ RunnerUp"),
			}
		}
	}

	let (current_members, current_runners_up) = remote_externalities::Builder::new()
		.module("PhragmenElection")
		.build_async()
		.await
		.execute_with(|| (Elections::members_ids(), Elections::runners_up_ids()));

	let stat_of = |v: &AccountId| -> Stat {
		let is_member = current_members.contains(v);
		let is_runner_up = current_runners_up.contains(v);
		match (is_member, is_runner_up) {
			(true, false) => Stat::Member,
			(false, true) => Stat::RunnerUp,
			(false, false) => Stat::Out,
			_ => panic!(),
		}
	};

	for (v, record) in slash_record.iter() {
		let sum_effective_slash = record.iter().map(|(_, x, _)| x).sum::<Balance>();
		let stat = stat_of(v);

		println!(
			"{} => Sum effective slash = {:?} ==> Current Stat {}",
			v,
			Token::from(sum_effective_slash),
			stat,
		);
	}

	println!("account,effective_slash");
	for (v, record) in slash_record.iter() {
		let sum_effective_slash = record.iter().map(|(_, x, _)| x).sum::<Balance>();
		println!("{},{}", v, sum_effective_slash,);
	}

	remote_externalities::Builder::new()
		.module("PhragmenElection")
		.module("Balances")
		.module("System")
		.build_async()
		.await
		.execute_with(|| {
			migrate_back_to_u8_ref_count();
			let mut corrupt = 0;
			<Voting<Runtime>>::iter().for_each(|(v, _)| {
				let reserved = Balances::reserved_balance(&v);
				let stat = stat_of(&v);
				if reserved < VotingBond::get() {
					corrupt += 1;
					println!("❌ corrupt account = {} // Stat = {}", v, stat);
					println!("📕 Slash records");
					slash_record
						.entry(v)
						.or_default()
						.iter()
						.for_each(|r| println!("\t{:?}", r));
				}
			});
		});
}

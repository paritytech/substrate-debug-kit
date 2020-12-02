//! A remote ext template for elections-phragmen pallet in a live chain.
//!
//! Note: You need to have this repo cloned next to a local substrate repo, and have all the
//! dependencies pointed to a local one. To do so, use `node update_cargo.js local`.

#![allow(unused_imports)]
use async_std::task::block_on;
use frame_support::{
	impl_outer_origin, migration::*, parameter_types, traits::Get,
	weights::constants::RocksDbWeight, IterableStorageMap, StorageMap, StoragePrefixedMap,
	StorageValue, Twox64Concat,
};
use pallet_elections_phragmen::*;
use sp_core::H256;
use sp_runtime::traits::{Block as BlockT, IdentityLookup};
use std::{cell::RefCell, collections::BTreeMap};
use sub_storage::Hash;

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

pub(crate) type AccountId = sp_core::crypto::AccountId32;
pub(crate) type BlockNumber = u32;
pub(crate) type Balance = u128;
pub(crate) type Moment = u64;
pub(crate) type Header = sp_runtime::generic::Header<BlockNumber, sp_runtime::traits::BlakeTwo256>;
pub(crate) type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub(crate) type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<AccountId, (), (), ()>;

frame_support::construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Event<T>},
		Balances: pallet_balances::{Module, Call, Event<T>, Config<T>},
		ElectionsPhragmen: pallet_elections_phragmen::{Module, Call, Event<T>, Config<T>},
	}
);

impl frame_system::Config for Runtime {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Index = u32;
	type BlockNumber = BlockNumber;
	type Call = Call;
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
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
}

impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type MaxLocks = ();
	type Event = ();
	type DustRemoval = ();
	type ExistentialDeposit = ();
	type AccountStore = frame_system::Module<Runtime>;
	type WeightInfo = ();
}

parameter_types! {
	pub static DesiredMembers: u32 = 13;
	pub static DesiredRunnersUp: u32 = 20;
	pub static VotingBond: Balance = 5 * DOLLARS;
	pub static CandidacyBond: Balance = 100 * DOLLARS;
}

parameter_types! {
	pub const ElectionsPhragmenModuleId: frame_support::traits::LockIdentifier = *b"phrelect";
	// well I am assuming that these never changed in polkadot...
	pub const TermDuration: BlockNumber = 7 * time::DAYS;
}

impl pallet_elections_phragmen::Config for Runtime {
	type ModuleId = ElectionsPhragmenModuleId;
	type Event = ();
	type Currency = Balances;
	type CurrencyToVote = frame_support::traits::U128CurrencyToVote;
	type ChangeMembers = ();
	type InitializeMembers = ();
	type CandidacyBond = CandidacyBond;
	type VotingBondBase = VotingBond;
	type VotingBondFactor = ();
	type TermDuration = TermDuration;
	type DesiredMembers = DesiredMembers;
	type DesiredRunnersUp = DesiredRunnersUp;
	type LoserCandidate = ();
	type KickedMember = ();
	type WeightInfo = ();
}

pub const DOTS: Balance = 1_000_000_000_000;
pub const DOLLARS: Balance = DOTS / 100;
pub const CENTS: Balance = DOLLARS / 100;
pub const MILLICENTS: Balance = CENTS / 1_000;

const URI: &'static str = "ws://localhost:9944";

pub struct PhragmenElectionDepositRuntimeUpgrade;
impl pallet_elections_phragmen::migrations_3_0_0::V2ToV3 for PhragmenElectionDepositRuntimeUpgrade {
	type AccountId = AccountId;
	type Balance = Balance;
	type Module = ElectionsPhragmen;
}
impl frame_support::traits::OnRuntimeUpgrade for PhragmenElectionDepositRuntimeUpgrade {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		pallet_elections_phragmen::migrations_3_0_0::apply::<Self>(5 * CENTS, DOLLARS)
	}
}

#[async_std::main]
async fn main() -> () {
	env_logger::init();

	sp_core::crypto::set_default_ss58_version(sp_core::crypto::Ss58AddressFormat::KusamaAccount);
	let client = sub_storage::create_ws_client(URI).await;
	let head = sub_storage::get_head(&client).await;

	remote_externalities::Builder::default()
		.module("System")
		.module("PhragmenElection") // actual module data..
		.module("ElectionsPhragmen") // the pallet version.. fuck
		.build()
		.execute_with(|| {
			// to ensure that all voters and members encode the same.

			let pre_all_voters = block_on(sub_storage::enumerate_map::<
				AccountId,
				(Balance, Vec<AccountId>),
			>(b"PhragmenElection", b"Voting", &client, head))
			.unwrap();
			let pre_members = block_on(sub_storage::read::<Vec<(AccountId, Balance)>>(
				sub_storage::value_key(b"PhragmenElection", b"Members"),
				&client,
				head,
			))
			.unwrap();
			let pre_runners_up = block_on(sub_storage::read::<Vec<(AccountId, Balance)>>(
				sub_storage::value_key(b"PhragmenElection", b"RunnersUp"),
				&client,
				head,
			))
			.unwrap();

			let weight = pallet_elections_phragmen::migrations_3_0_0::apply::<
				PhragmenElectionDepositRuntimeUpgrade,
			>(5 * CENTS, 1 * DOLLARS);

			assert!(weight > 0);

			<Voting<Runtime>>::iter().for_each(|(_, voting)| {
				assert_eq!(voting.deposit, 5 * CENTS);
			});

			<Members<Runtime>>::get().iter().for_each(|h| {
				assert_eq!(h.deposit, 1 * DOLLARS);
			});

			<RunnersUp<Runtime>>::get().iter().for_each(|h| {
				assert_eq!(h.deposit, 1 * DOLLARS);
			});

			<Candidates<Runtime>>::get().iter().for_each(|(_, d)| {
				assert_eq!(*d, 1 * DOLLARS);
			});

			assert_eq!(pre_all_voters.len(), <Voting<Runtime>>::iter().count());
			assert_eq!(pre_members.len(), <Members<Runtime>>::get().len());
			assert_eq!(pre_runners_up.len(), <RunnersUp<Runtime>>::get().len());

			pre_all_voters
				.into_iter()
				.map(|(who, (_, votes))| (who, votes))
				.all(|(who, votes)| <Voting<Runtime>>::get(who).votes == votes);

			pre_members.into_iter().all(|(m, b)| {
				<Members<Runtime>>::get()
					.iter()
					.find(|x| x.who == m)
					.unwrap()
					.stake == b
			});

			pre_runners_up.into_iter().all(|(r, b)| {
				<RunnersUp<Runtime>>::get()
					.iter()
					.find(|x| x.who == r)
					.unwrap()
					.stake == b
			});
		})
}

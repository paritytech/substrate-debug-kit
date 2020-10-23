//! A remote ext template for elections-phragmen pallet in a live chain.
//!
//! Note: You need to have this repo cloned next to a local substrate repo, and have all the
//! dependencies pointed to a local one. To do so, use `node update_cargo.js local`.

use frame_support::{
	impl_outer_origin, migration::*, parameter_types, weights::constants::RocksDbWeight,
	IterableStorageMap, StorageMap, StoragePrefixedMap, Twox64Concat,
};
use pallet_elections_phragmen::*;
use sp_core::H256;
use sp_runtime::traits::IdentityLookup;
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
	type ModuleToIndex = ();
	// type PalletInfo = ();
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
}

impl pallet_balances::Trait for Runtime {
	type Balance = Balance;
	// type MaxLocks = ();
	type Event = ();
	type DustRemoval = ();
	type ExistentialDeposit = ();
	type AccountStore = frame_system::Module<Runtime>;
	type WeightInfo = ();
}

parameter_types! {
	pub const ElectionsPhragmenModuleId: frame_support::traits::LockIdentifier = *b"phrelect";
	pub const DesiredMembers: u32 = 13;
	pub const DesiredRunnersUp: u32 = 20;
	pub const TermDuration: BlockNumber = 4 * time::HOURS;
}

impl pallet_elections_phragmen::Trait for Runtime {
	type ModuleId = ElectionsPhragmenModuleId;
	type Event = ();
	type Currency = Balances;
	// type CurrencyToVote = frame_support::traits::U128CurrencyToVote;
	type CurrencyToVote = CurrencyToVoteHandler;
	type ChangeMembers = ();
	type InitializeMembers = ();
	type CandidacyBond = ();
	type VotingBond = ();
	// type VotingBondBase = ();
	// type VotingBondFactor = ();
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
pub const DOLLARS: Balance = DOTS / 6;
pub const CENTS: Balance = DOLLARS / 100;

#[derive(codec::Encode, codec::Decode, Debug)]
pub struct OldAccountInfo<Index, AccountData> {
	/// The number of transactions this account has sent.
	pub nonce: Index,
	/// The number of other modules that currently depend on this account's existence. The account
	/// cannot be reaped until this is zero.
	pub refcount: u8,
	/// The additional data that belongs to this account. Used to store the balance(s) in a lot of
	/// chains.
	pub data: AccountData,
}

#[async_std::main]
async fn main() -> () {
	env_logger::init();
	sp_core::crypto::set_default_ss58_version(sp_core::crypto::Ss58AddressFormat::PolkadotAccount);
	let client = sub_storage::create_ws_client(URI).await;

	// let at = sub_storage::get_head(&client).await;
	let mut at: Hash =
		hex_literal::hex!("a5158a831a17d88f5f20ef25fb097098658998fc69717ee5416529629d269717")
			.into();

	let hex = hex_literal::hex!("1ebd2c29909eb603331b960308a070b839ee78e80fe12ef05e4639a176ab743e");
	let target = AccountId::from(hex);

	// loop {
	// 	println!("at = {:?}", at);
	// 	println!(
	// 		"{:?} =? {:?}",
	// 		target,
	// 		sub_storage::read::<OldAccountInfo<u32, pallet_balances::AccountData<Balance>>>(
	// 			sub_storage::map_key::<frame_support::Blake2_128Concat>(
	// 				b"System",
	// 				b"Account",
	// 				target.as_ref(),
	// 			),
	// 			&client,
	// 			at,
	// 		)
	// 		.await
	// 		.unwrap()
	// 	);

	// 	println!(
	// 		"Voting => {:?}",
	// 		sub_storage::read::<(Balance, Vec<AccountId>)>(
	// 			sub_storage::map_key::<frame_support::Twox64Concat>(
	// 				b"PhragmenElection",
	// 				b"Voting",
	// 				target.as_ref(),
	// 			),
	// 			&client,
	// 			at,
	// 		)
	// 		.await
	// 		.unwrap()
	// 	);

	// 	at = sub_storage::get_header(&client, at).await.parent_hash;
	// }

	remote_externalities::Builder::new()
		.module("PhragmenElection")
		.module("Balances")
		.module("System")
		.build_async()
		.await
		.execute_with(|| {
			use frame_support::traits::OnInitialize;
			Elections::on_initialize(806400);
		})
}

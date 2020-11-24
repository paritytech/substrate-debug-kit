use frame_support::impl_outer_origin;
use frame_support::{parameter_types, weights::constants::RocksDbWeight};
use sp_core::H256;
use sp_runtime::traits::IdentityLookup;
use pallet_society::*;

pub const DOTS: Balance = 1_000_000_000_000;
pub const DOLLARS: Balance = DOTS / 6;
pub const CENTS: Balance = DOLLARS / 100;
pub const MILLICENTS: Balance = CENTS / 1_000;

pub const MILLISECS_PER_BLOCK: Moment = 6000;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

pub(crate) type AccountId = sp_core::crypto::AccountId32;
pub(crate) type BlockNumber = u32;
pub(crate) type Moment = u64;
pub(crate) type Balance = u128;
pub(crate) type Header = sp_runtime::generic::Header<BlockNumber, sp_runtime::traits::BlakeTwo256>;

pub(crate) type Balances = pallet_balances::Module<Runtime>;
pub(crate) type Society = Module<Runtime>;
pub(crate) type System = frame_system::Module<Runtime>;

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
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type ModuleToIndex = ();
	// type PalletInfo =  = ();
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
	pub const CandidateDeposit: Balance = 10 * DOLLARS;
	pub const WrongSideDeduction: Balance = 2 * DOLLARS;
	pub const MaxStrikes: u32 = 10;
	pub const RotationPeriod: BlockNumber = 80 * HOURS;
	pub const PeriodSpend: Balance = 500 * DOLLARS;
	pub const MaxLockDuration: BlockNumber = 36 * 30 * DAYS;
	pub const ChallengePeriod: BlockNumber = 7 * DAYS;
	pub const SocietyModuleId: sp_runtime::ModuleId = sp_runtime::ModuleId(*b"py/socie");
}

impl Trait for Runtime {
	type Event = ();
	type Currency = pallet_balances::Module<Self>;
	type Randomness = frame_support::traits::TestRandomness;
	type CandidateDeposit = CandidateDeposit;
	type WrongSideDeduction = WrongSideDeduction;
	type MaxStrikes = MaxStrikes;
	type PeriodSpend = PeriodSpend;
	type MembershipChanged = ();
	type RotationPeriod = RotationPeriod;
	type MaxLockDuration = MaxLockDuration;
	type FounderSetOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type SuspensionJudgementOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type ChallengePeriod = ChallengePeriod;
	type ModuleId = SocietyModuleId;
}

// const URI: &'static str = "wss://kusama-rpc.polkadot.io";
const URI: &'static str = "ws://localhost:9944";

#[async_std::main]
async fn main() -> () {
	let _ = env_logger::Builder::from_default_env()
		.format_module_path(true)
		.format_level(true)
		.try_init();
	let client = sub_storage::create_ws_client(URI).await;
	let now = sub_storage::get_head(&client).await;

	// let soc = hex_literal::hex!["6d6f646c70792f736f6369650000000000000000000000000000000000000000"];

	remote_externalities::Builder::new()
		// .module("System")
		// .module("Society")
		// .module("Balances")
		.uri(URI.to_owned())
		// .at(hex_literal::hex!["6625469dad4f3a4a26481c852b64ee04f90c4278ae97532c7076aa6b0db4142a"].into())
		.at(now)
		.build_async()
		.await
		.execute_with(|| {
			dbg!(Society::candidates());
			dbg!(Society::bids());
			dbg!(Society::pot());
			let mut members = Society::members();
			Society::rotate_period(&mut members);
			dbg!(Society::candidates());
			dbg!(Society::bids());
			dbg!(Society::pot());
		})
}

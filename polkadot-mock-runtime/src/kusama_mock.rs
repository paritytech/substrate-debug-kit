use frame_support::construct_runtime;
use frame_support::{parameter_types, weights::constants::RocksDbWeight};
use sp_runtime::traits::{IdentityLookup, Block as BlockT};
use crate::common::*;

pub const DOTS: Balance = 1_000_000_000_000;
pub const DOLLARS: Balance = DOTS / 6;
pub const CENTS: Balance = DOLLARS / 100;
pub const MILLICENTS: Balance = CENTS / 1_000;

pub const MILLISECS_PER_BLOCK: Moment = 6000;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

parameter_types! {
	pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights::simple_max(1024);
}

impl frame_system::Config for Runtime {
	type BaseCallFilter = ();
	type BlockWeights = BlockWeights;
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Index = Index;
	type BlockNumber = BlockNumber;
	type Call = Call;
	type Hash = Hash;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = ();
	type Version = ();
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type PalletInfo = ();
	type SS58Prefix = ();
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
	pub const CandidateDeposit: Balance = 10 * DOLLARS;
	pub const WrongSideDeduction: Balance = 2 * DOLLARS;
	pub const MaxStrikes: u32 = 10;
	pub const RotationPeriod: BlockNumber = 80 * HOURS;
	pub const PeriodSpend: Balance = 500 * DOLLARS;
	pub const MaxLockDuration: BlockNumber = 36 * 30 * DAYS;
	pub const ChallengePeriod: BlockNumber = 7 * DAYS;
	pub const SocietyModuleId: sp_runtime::ModuleId = sp_runtime::ModuleId(*b"py/socie");
}

impl pallet_society::Config for Runtime {
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

frame_support::construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Event<T>},
		Balances: pallet_balances::{Module, Call, Event<T>, Config<T>},
		Society: pallet_society::{Module, Call, Event<T>, Config<T>},
	}
);

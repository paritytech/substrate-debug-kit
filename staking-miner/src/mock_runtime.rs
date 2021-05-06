use frame_support::{
	impl_outer_dispatch, impl_outer_origin, parameter_types,
	weights::{
		constants::{RocksDbWeight, WEIGHT_PER_SECOND},
		Weight,
	},
};
use sp_core::H256;
use sp_runtime::{
	curve::PiecewiseLinear,
	testing::TestXt,
	traits::{IdentityLookup, Saturating},
	Perbill,
};
use std::cell::RefCell;

pub(crate) type AccountId = sp_core::crypto::AccountId32;
pub(crate) type BlockNumber = u32;
pub(crate) type Balance = u128;
pub(crate) type Header = sp_runtime::generic::Header<BlockNumber, sp_runtime::traits::BlakeTwo256>;

pub(crate) type Session = pallet_session::Module<Runtime>;
pub(crate) type Timestamp = pallet_timestamp::Module<Runtime>;
pub(crate) type Staking = pallet_staking::Module<Runtime>;
pub(crate) type Balances = pallet_balances::Module<Runtime>;

use frame_support::weights::constants::{BlockExecutionWeight, ExtrinsicBaseWeight};
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Runtime;

pub const AVERAGE_ON_INITIALIZE_WEIGHT: Perbill = Perbill::from_perthousand(25);
parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub const MaximumBlockWeight: Weight = 2 * WEIGHT_PER_SECOND;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	// / Assume 10% of weight for average on_initialize calls.
	pub MaximumExtrinsicWeight: Weight =
		AvailableBlockRatio::get().saturating_sub(AVERAGE_ON_INITIALIZE_WEIGHT)
		* MaximumBlockWeight::get();
	pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
}

impl_outer_origin! {
	pub enum Origin for Runtime where system = frame_system {}
}

impl_outer_dispatch! {
	pub enum Call for Runtime where origin: Origin {
		staking::Staking,
	}
}

impl frame_system::Config for Runtime {
	type AccountData = pallet_balances::AccountData<Balance>;
	type AccountId = AccountId;
	type AvailableBlockRatio = AvailableBlockRatio;
	type BaseCallFilter = ();
	type BlockExecutionWeight = frame_support::weights::constants::BlockExecutionWeight;
	type BlockHashCount = BlockHashCount;
	type BlockNumber = BlockNumber;
	type Call = Call;
	type DbWeight = RocksDbWeight;
	type Event = ();
	type ExtrinsicBaseWeight = frame_support::weights::constants::ExtrinsicBaseWeight;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type Header = Header;
	type Index = u32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type MaximumBlockLength = MaximumBlockLength;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumExtrinsicWeight = MaximumExtrinsicWeight;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type Origin = Origin;
	type PalletInfo = ();
	type SystemWeightInfo = ();
	type Version = ();
}

impl pallet_balances::Config for Runtime {
	type AccountStore = frame_system::Module<Runtime>;
	type Balance = Balance;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ();
	type MaxLocks = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Config for Runtime {
	type MinimumPeriod = MinimumPeriod;
	type Moment = u64;
	type OnTimestampSet = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const UncleGenerations: u64 = 0;
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(25);
}

sp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {}
}

pub struct TestSessionHandler;
impl pallet_session::SessionHandler<AccountId> for TestSessionHandler {
	const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] = &[];

	fn on_genesis_session<Ks: sp_runtime::traits::OpaqueKeys>(_validators: &[(AccountId, Ks)]) {}

	fn on_new_session<Ks: sp_runtime::traits::OpaqueKeys>(
		_: bool,
		_: &[(AccountId, Ks)],
		_: &[(AccountId, Ks)],
	) {
	}

	fn on_disabled(_: usize) {}
}

impl pallet_session::Config for Runtime {
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type Event = ();
	type Keys = SessionKeys;
	type NextSessionRotation = pallet_session::PeriodicSessions<(), ()>;
	type SessionHandler = TestSessionHandler;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type ShouldEndSession = pallet_session::PeriodicSessions<(), ()>;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type WeightInfo = ();
}

impl pallet_session::historical::Config for Runtime {
	type FullIdentification =
		pallet_staking::Exposure<<Self as frame_system::Config>::AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Self>;
}

pallet_staking_reward_curve::build! {
	const I_NPOS: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_000,
		max_inflation: 0_100_000,
		ideal_stake: 0_500_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

parameter_types! {
	pub const BondingDuration: u32 = 3;
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &I_NPOS;
	pub const MaxNominatorRewardedPerValidator: u32 = 64;
	pub const UnsignedPriority: u64 = 1 << 20;
	pub const MinSolutionScoreBump: Perbill = Perbill::zero();
	pub const MaxIterations: u32 = 10;
	pub OffchainSolutionWeightLimit: Weight = MaximumExtrinsicWeight::get()
		.saturating_sub(BlockExecutionWeight::get())
		.saturating_sub(ExtrinsicBaseWeight::get());
}

thread_local! {
	pub static REWARD_REMAINDER_UNBALANCED: RefCell<u128> = RefCell::new(0);
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Runtime
where
	Call: From<LocalCall>,
{
	type Extrinsic = Extrinsic;
	type OverarchingCall = Call;
}

pub type Extrinsic = TestXt<Call, ()>;

use parking_lot::RwLock;
use sp_core::offchain::{
	testing::{PoolState, TestOffchainExt, TestTransactionPoolExt},
	OffchainExt, TransactionPoolExt,
};
/// Just in case, we should not really need this often.
use sp_io::TestExternalities;
use std::sync::Arc;
pub fn offchainify(ext: &mut TestExternalities, iterations: u32) -> Arc<RwLock<PoolState>> {
	let (offchain, offchain_state) = TestOffchainExt::new();
	let (pool, pool_state) = TestTransactionPoolExt::new();

	let mut seed = [0_u8; 32];
	seed[0..4].copy_from_slice(&iterations.to_le_bytes());
	offchain_state.write().seed = seed;

	ext.register_extension(OffchainExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));

	pool_state
}

impl pallet_staking::Config for Runtime {
	type BondingDuration = BondingDuration;
	type Currency = pallet_balances::Module<Runtime>;
	type CurrencyToVote = frame_support::traits::U128CurrencyToVote;
	type ElectionLookahead = ();
	type Event = ();
	type Call = Call;
	type MaxIterations = MaxIterations;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type MinSolutionScoreBump = MinSolutionScoreBump;
	type NextNewSession = Session;
	type Reward = ();
	type RewardCurve = RewardCurve;
	type RewardRemainder = ();
	type SessionInterface = Self;
	type SessionsPerEra = ();
	type Slash = ();
	type SlashCancelOrigin = frame_system::EnsureRoot<<Self as frame_system::Trait>::AccountId>;
	type SlashDeferDuration = ();
	type UnixTime = Timestamp;
	type UnsignedPriority = UnsignedPriority;
	type OffchainSolutionWeightLimit = OffchainSolutionWeightLimit;
	type WeightInfo = crate::polkadot_weight::WeightInfo<Runtime>;
}

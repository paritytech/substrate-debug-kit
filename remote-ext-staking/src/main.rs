use frame_support::{
	impl_outer_dispatch, impl_outer_origin, parameter_types,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_core::H256;
use sp_runtime::{
	curve::PiecewiseLinear,
	testing::TestXt,
	traits::{Convert, IdentityLookup},
	Perbill,
};
use std::cell::RefCell;
use sub_storage::Hash;

pub struct CurrencyToVoteHandler;

impl CurrencyToVoteHandler {
	fn factor() -> Balance {
		(Balances::total_issuance() / u64::max_value() as Balance).max(1)
	}
}

impl Convert<Balance, u64> for CurrencyToVoteHandler {
	fn convert(x: Balance) -> u64 {
		(x / Self::factor()) as u64
	}
}

impl Convert<u128, Balance> for CurrencyToVoteHandler {
	fn convert(x: u128) -> Balance {
		x * Self::factor()
	}
}

macro_rules! init_log {
	() => {
		let _ = env_logger::try_init();
	};
}

pub(crate) type AccountId = sp_core::crypto::AccountId32;
pub(crate) type BlockNumber = u32;
pub(crate) type Balance = u128;
pub(crate) type Header = sp_runtime::generic::Header<BlockNumber, sp_runtime::traits::BlakeTwo256>;

pub(crate) type Session = pallet_session::Module<Runtime>;
pub(crate) type Timestamp = pallet_timestamp::Module<Runtime>;
pub(crate) type Staking = pallet_staking::Module<Runtime>;
pub(crate) type Balances = pallet_balances::Module<Runtime>;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Runtime;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl_outer_origin! {
	pub enum Origin for Runtime where system = frame_system {}
}

impl_outer_dispatch! {
	pub enum Call for Runtime where origin: Origin {
		staking::Staking,
	}
}

impl frame_system::Trait for Runtime {
	type AccountData = pallet_balances::AccountData<Balance>;
	type AccountId = AccountId;
	type AvailableBlockRatio = AvailableBlockRatio;
	type BaseCallFilter = ();
	type BlockExecutionWeight = ();
	type BlockHashCount = BlockHashCount;
	type BlockNumber = BlockNumber;
	type Call = Call;
	type DbWeight = RocksDbWeight;
	type Event = ();
	type ExtrinsicBaseWeight = ();
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type Header = Header;
	type Index = u32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type MaximumBlockLength = MaximumBlockLength;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type PalletInfo = ();
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type Origin = Origin;
	type SystemWeightInfo = ();
	type Version = ();
}

impl pallet_balances::Trait for Runtime {
	type AccountStore = frame_system::Module<Runtime>;
	type Balance = Balance;
	type DustRemoval = ();
	type MaxLocks = ();
	type Event = ();
	type ExistentialDeposit = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Trait for Runtime {
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

impl pallet_session::Trait for Runtime {
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type Event = ();
	type Keys = SessionKeys;
	type NextSessionRotation = pallet_session::PeriodicSessions<(), ()>;
	type SessionHandler = TestSessionHandler;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type ShouldEndSession = pallet_session::PeriodicSessions<(), ()>;
	type ValidatorId = <Self as frame_system::Trait>::AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type WeightInfo = ();
}

impl pallet_session::historical::Trait for Runtime {
	type FullIdentification =
		pallet_staking::Exposure<<Self as frame_system::Trait>::AccountId, Balance>;
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

impl pallet_staking::Trait for Runtime {
	type BondingDuration = BondingDuration;
	type Call = Call;
	type Currency = pallet_balances::Module<Runtime>;
	type CurrencyToVote = CurrencyToVoteHandler;
	type ElectionLookahead = ();
	type Event = ();
	type MaxIterations = ();
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
	type OffchainSolutionWeightLimit = ();
	type WeightInfo = ();
}

#[async_std::main]
async fn main() -> () {
	use sp_core::crypto::Ss58Codec;
	init_log!();

	let client = sub_storage::create_ws_client("ws://localhost:9944").await;
	let now: Hash = hex_literal::hex!["75047e12a4755516d1a4703f5f594c76ba3ced8b9ab8d937961969569910f9d2"].into();
	let validator: AccountId = <AccountId as Ss58Codec>::from_ss58check(&"139ANth4tdC4cHRWvesuxkuYNWfq9wgFmxV5C98fy8Xmyg8E").unwrap();

	remote_externalities::Builder::new()
		.module("Staking")
		.at(now)
		.build_async()
		.await
		.execute_with(|| {
			dbg!(Staking::do_payout_stakers(validator.clone(), 118));
			dbg!(Staking::do_payout_stakers(validator.clone(), 119));
			dbg!(Staking::do_payout_stakers(validator.clone(), 120));
			dbg!(Staking::do_payout_stakers(validator.clone(), 121));
			dbg!(Staking::do_payout_stakers(validator.clone(), 122));
		});
}

//! A mock polkadot runtime, to be used for testing.
//!
//! ## Warning
//!
//! Maintaining this runtime is difficult and it might become out of sync. Don't use for anything
//! important, and take it all with a (big) grain of salt.
//!
//! ## Specification
//!
//! A `Runtime` is exposed that implements `System`, `Balances`, `TransactionPayment`,
//! `RandomnessCollectiveFlip`, `Scheduler`, `Timestamp` and `Indices`. Other pallets can be
//! implemented optionally via specific features.
//!
//! More details:
//! - All weights are `()`, meaning that the default substrate weights get used. TODO: it should
//!   not.
//! - The `SignedExtra` type is up to date (albeit it is unlikely to be used), except for the
//!   claims.
//! - Pallet indices are the same. Transactions should be decodable.

use crate::common::{*, self};
use frame_support::{
	parameter_types,
	weights::{Weight, DispatchClass, constants::RocksDbWeight},
};
use sp_runtime::traits::IdentityLookup;
use frame_system::limits;
use frame_support::weights::constants::{BlockExecutionWeight, ExtrinsicBaseWeight, WEIGHT_PER_SECOND};
use sp_runtime::{Perquintill, Perbill, create_runtime_str, FixedPointNumber};
use sp_version::RuntimeVersion;
use pallet_transaction_payment::Multiplier;
use pallet_session::historical as session_historical;

/// The DOTS definition is different between polkadot and kusama.
pub const DOTS: Balance = 1_000_000_000_000;
pub const DOLLARS: Balance = DOTS / 6;
pub const CENTS: Balance = DOLLARS / 100;
pub const MILLICENTS: Balance = CENTS / 1_000;

/// This is the runtime version that we store here. It does not mean anything other than saying that
/// this repo has last been synced with polkadot at this version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("polkadot-mock"),
	impl_name: create_runtime_str!("parity-polkadot-mock"),
	authoring_version: 0,
	spec_version: 9000,
	impl_version: 0,
	// we don't implement any of the runtime APIs here.
	apis: sp_version::create_apis_vec![[]],
	transaction_version: 7,
};

pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(1);
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
pub const MAXIMUM_BLOCK_WEIGHT: Weight = 2 * WEIGHT_PER_SECOND;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub BlockLength: limits::BlockLength = limits::BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub BlockWeights: limits::BlockWeights = limits::BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have an extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT,
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub const Version: RuntimeVersion = VERSION;
	pub const SS58Prefix: u8 = 0;
}

impl frame_system::Config for Runtime {
	type BaseCallFilter = common::AllowAllCalls;
	type BlockWeights = BlockWeights;
	type BlockLength = BlockLength;
	type DbWeight = RocksDbWeight;
	type Origin = Origin;
	type Index = common::Index;
	type BlockNumber = common::BlockNumber;
	type Call = Call;
	type Hash = common::Hash;
	type Hashing = common::Hashing;
	type AccountId = common::AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = common::Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = Version;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type PalletInfo = PalletInfo;
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
}

impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type MaxLocks = ();
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ();
	type AccountStore = frame_system::Pallet<Runtime>;
	type WeightInfo = ();
}

use frame_support::weights::{WeightToFeePolynomial, WeightToFeeCoefficient, WeightToFeeCoefficients};
use smallvec::smallvec;

pub const TARGET_BLOCK_FULLNESS: Perbill = Perbill::from_percent(25);
pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// in Kusama, extrinsic base weight (smallest non-zero weight) is mapped to 1/10 CENT:
		let p = CENTS;
		let q = 10 * Balance::from(ExtrinsicBaseWeight::get());
		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}

pub type SlowAdjustingFeeUpdate<R> = pallet_transaction_payment::TargetedFeeAdjustment<
	R,
	TargetBlockFullness,
	AdjustmentVariable,
	MinimumMultiplier,
>;

parameter_types! {
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(3, 100_000);
	pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000u128);
	pub const TransactionByteFee: Balance = 10 * MILLICENTS;
}

impl pallet_transaction_payment::Config for Runtime {
	type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, ()>; // Nothing to do, no point in paying the author here.
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = WeightToFee;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
}

parameter_types! {
	pub const MinimumPeriod: u64 = common::SLOT_DURATION / 2;
}
impl pallet_timestamp::Config for Runtime {
	type Moment = u64;
	#[cfg(not(feature = "staking-system"))]
	type OnTimestampSet = ();
	#[cfg(feature = "staking-system")]
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const IndexDeposit: Balance = 10 * DOLLARS;
}

impl pallet_indices::Config for Runtime {
	type AccountIndex = common::Index;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type Event = Event;
	type WeightInfo = ();
}

pub type SignedExtra = (
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckMortality<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
pub type UncheckedExtrinsic = common::UncheckedExtrinsicOf<Call, SignedExtra>;
pub type Block = common::BlockOf<UncheckedExtrinsic>;
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPallets,
	(),
>;
pub type SignedPayload = sp_runtime::generic::SignedPayload<Call, SignedExtra>;

#[cfg(not(feature = "staking-system"))]
frame_support::construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Event<T>} = 0,

		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 3,
		Indices: pallet_indices::{Pallet, Call, Storage, Config<T>, Event<T>} = 4,
		Balances: pallet_balances::{Pallet, Call, Event<T>, Config<T>} = 5,
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 32,
	}
);

#[cfg(feature = "staking-system")]
pub mod consensus_system;

#[cfg(feature = "staking-system")]
frame_support::construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Event<T>} = 0,

		Babe: pallet_babe::{Pallet, Call, Storage, Config, ValidateUnsigned} = 2,

		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 3,
		Indices: pallet_indices::{Pallet, Call, Storage, Config<T>, Event<T>} = 4,
		Balances: pallet_balances::{Pallet, Call, Event<T>, Config<T>} = 5,
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 32,

		Authorship: pallet_authorship::{Pallet, Call, Storage} = 6,
		Staking: pallet_staking::{Pallet, Call, Storage, Config<T>, Event<T>} = 7,
		Offences: pallet_offences::{Pallet, Call, Storage, Event} = 8,
		Historical: session_historical::{Pallet} = 33,
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 9,
		Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config, Event, ValidateUnsigned} = 11,
		ImOnline: pallet_im_online::{Pallet, Call, Storage, Event<T>, ValidateUnsigned, Config<T>} = 12,
		AuthorityDiscovery: pallet_authority_discovery::{Pallet, Call, Config} = 13,
		ElectionProviderMultiPhase: pallet_election_provider_multi_phase::{Pallet, Call, Storage, Event<T>, ValidateUnsigned} = 37,
	}
);

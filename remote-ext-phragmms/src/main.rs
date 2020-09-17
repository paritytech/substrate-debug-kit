use csv::Writer;
use frame_support::{
	impl_outer_dispatch, impl_outer_origin, parameter_types,
	storage::{IterableStorageMap, StorageMap},
	weights::{constants::RocksDbWeight, Weight},
};
use logging_timer::timer;
use pallet_staking::{Nominations, Nominators, SlashingSpans, Validators};
use sp_core::H256;
use sp_npos_elections::{
	assignment_ratio_to_staked_normalized, ElectionResult as PrimitiveElectionResult, VoteWeight,
};
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
	type ModuleToIndex = ();
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
	type WeightInfo = ();
}

#[async_std::main]
async fn main() -> () {
	init_log!();

	let client = sub_storage::create_ws_client("ws://localhost:9944").await;
	let mut now = sub_storage::get_head(&client).await;
	let mut era = sub_storage::read(
		sub_storage::value_key(b"Staking", b"CurrentEra"),
		&client,
		now,
	)
	.await
	.unwrap();

	let mut wtr = Writer::from_path("./out.csv").unwrap();
	wtr.write_record(&["era", "phragmen(5)", "time(ms)", "phragmms(5)", "time(ms)"])
		.unwrap();

	loop {
		let election_status = sub_storage::read::<pallet_staking::ElectionStatus<BlockNumber>>(
			sub_storage::value_key(b"Staking", b"EraElectionStatus"),
			&client,
			now,
		)
		.await
		.unwrap();
		let now_era = sub_storage::read::<u32>(
			sub_storage::value_key(b"Staking", b"CurrentEra"),
			&client,
			now,
		)
		.await
		.unwrap();
		if election_status.is_open() && now_era < era {
			println!(
				"💭 Last block at which election window is open for era {:?} => {:?}",
				era, now,
			);
			era = now_era;
			run(now, era, &mut wtr).await;
		} else {
			let header = sub_storage::get_header(&client, now).await;
			now = header.parent_hash;
		}
	}
}

async fn run(now: Hash, era: u32, wrt: &mut csv::Writer<std::fs::File>) {
	// 1 new DOT
	const THRESHOLD: u128 = 1000_000_000_000;
	const MAX_ITER: usize = 10;
	remote_externalities::Builder::new()
		.module("Staking")
		.at(now)
		.build_async()
		.await
		.execute_with(|| {
			println!("⏰ Scraping done at block {:?}.", now);
			let mut all_nominators: Vec<(AccountId, VoteWeight, Vec<AccountId>)> = Vec::new();
			let mut all_validators = Vec::new();
			for (validator, _) in <Validators<Runtime>>::iter() {
				// append self vote
				let self_vote = (
					validator.clone(),
					Staking::slashable_balance_of_vote_weight(&validator),
					vec![validator.clone()],
				);
				all_nominators.push(self_vote);
				all_validators.push(validator);
			}

			let nominator_votes = <Nominators<Runtime>>::iter().map(|(nominator, nominations)| {
				let Nominations {
					submitted_in,
					mut targets,
					suppressed: _,
				} = nominations;

				// Filter out nomination targets which were nominated before the most recent
				// slashing span.
				targets.retain(|stash| {
					<SlashingSpans<Runtime>>::get(&stash)
						.map_or(true, |spans| submitted_in >= spans.last_nonzero_slash())
				});

				(nominator, targets)
			});
			all_nominators.extend(nominator_votes.map(|(n, ns)| {
				let s = Staking::slashable_balance_of_vote_weight(&n);
				(n, s, ns)
			}));

			println!(
				"👨🏿‍🔬 {} validators {} nominators",
				all_validators.len(),
				all_nominators.len()
			);

			let (phragmms_score, phragmms_time) = {
				let time = timer!("phragmms-core");
				let start = std::time::Instant::now();
				let PrimitiveElectionResult {
					winners,
					assignments,
				} = sp_npos_elections::phragmms::<AccountId, pallet_staking::OffchainAccuracy>(
					Staking::validator_count() as usize,
					all_validators.clone(),
					all_nominators.clone(),
					Some((MAX_ITER, THRESHOLD)),
				)
				.unwrap();
				let elapsed = start.elapsed().as_millis();
				drop(time);

				if Staking::era_election_status().is_open() {
					let (winners, compact, score, size) =
						pallet_staking::offchain_election::prepare_submission::<Runtime>(
							assignments,
							winners,
							true,
						)
						.unwrap();

					let validation_result = Staking::check_and_replace_solution(
						winners,
						compact,
						pallet_staking::ElectionCompute::Unsigned,
						score.clone(),
						era,
						size,
					);
					println!(
						"Validation result of phragmms if submitted would have been: {:?}",
						validation_result,
					);
					(score, elapsed)
				} else {
					panic!("Election window is not open at this given block. Run this test when it is open.");
				}
			};

			let (phragmen_score, phragmen_time) = {
				let time = timer!("phragmen-core");
				let start = std::time::Instant::now();
				let PrimitiveElectionResult {
					winners,
					assignments,
				} = sp_npos_elections::seq_phragmen::<AccountId, Perbill>(
					Staking::validator_count() as usize,
					all_validators.clone(),
					all_nominators.clone(),
					Some((MAX_ITER, THRESHOLD)),
				)
				.unwrap();
				let elapsed = start.elapsed().as_millis();
				drop(time);

				let winners = sp_npos_elections::to_without_backing(winners);

				let staked = assignment_ratio_to_staked_normalized(
					assignments,
					Staking::slashable_balance_of_vote_weight,
				)
				.unwrap();

				let (support, _) = sp_npos_elections::build_support_map::<AccountId>(
					winners.as_ref(),
					staked.as_ref(),
				);
				(sp_npos_elections::evaluate_support(&support), elapsed)
			};

			if sp_npos_elections::is_score_better(phragmms_score, phragmen_score, Perbill::zero()) {
				println!(
					"\t✅ Phragmms score is better: {:?} > {:?}",
					phragmms_score, phragmen_score
				);
			} else {
				println!(
					"\t❌ Phragmms score is worse: {:?} < {:?}",
					phragmms_score, phragmen_score
				);
			}

			wrt.write_record(&[
				format!("{}", era),
				format!("{}", phragmen_score[0]),
				format!("{}", phragmen_time),
				format!("{}", phragmms_score[0]),
				format!("{}", phragmms_time),
			])
			.unwrap();
			wrt.flush().unwrap();
		})
}

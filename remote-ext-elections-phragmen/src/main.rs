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

type Token = tokens::DOT;

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
}

parameter_types! {
	pub const ElectionsPhragmenModuleId: frame_support::traits::LockIdentifier = *b"phrelect";
	// well I am assuming that these never changed in polkadot...
	pub const TermDuration: BlockNumber = 7 * time::DAYS;
	pub const CandidacyBond: Balance = 100 * DOLLARS;
	pub const VotingBond: Balance = 5 * DOLLARS;
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

	let elections = vec![
		(
			"98bb61dda86b967be2a4b73760b9706bb66582a26acb28f1d93382e89b061590",
			vec![],
			"2020-11-04T07:39:48+00:00",
		),
		(
			"0947c4e9079d051e81ffc08078a6315ee62f2260ef27d88c4445c6efca3836dd",
			vec![],
			"2020-10-28T07:18:30+00:00",
		),
		(
			"a419f48967f7f115ae0f2299346d261f46d502a1e0c44fe44eaee6e390e2b81e",
			vec![200410000000, 1000000000000],
			"2020-10-21T06:50:54+00:00",
		),
		(
			"4579cd559dec2772944163a49390273fcfdc0a34e600d9fcad312ec36123aba3",
			vec![1000000000000, 50000000000],
			"2020-10-14T06:16:24+00:00",
		),
		(
			"a2046cdf500d7e4a5c415f1fdeaf29fe03e0b807382cc1c7320228932174a3f4",
			vec![],
			"2020-10-07T05:16:24+00:00",
		),
		(
			"04656a8b9432e614f7f5c09db96b3b1fe41900ae3e11e1cc99aab3836fb968c3",
			vec![1000000000000],
			"2020-09-30T02:59:42+00:00",
		),
		(
			"911c2373a35e928543f20628492f7f88d477a909d33fdd0ca1a73a43079ea5da",
			vec![1000000000000],
			"2020-09-23T01:56:24+00:00",
		),
		(
			"090e75e4462bd822f1a751ddec3ead4c8b4f1a26974b4bd48933ef0840d18e08",
			vec![1000000000000, 1000000000000, 14300000000],
			"2020-09-16T01:36:24+00:00",
		),
		(
			"1329631d9504157a4bf623af1ead296f5e1663e274d72fa4aefba817319f8ba6",
			vec![1000000000000, 201520000000],
			"2020-09-09T01:09:30+00:00",
		),
		(
			"6c39de021db2cfa19fb30b00a68fdec78fd718d10b8db2b2f9158d99e7272c8f",
			vec![1000000000000],
			"2020-09-02T00:49:12+00:00",
		),
		(
			"87f08470435ce197e8245ee0410344a863abde91a591803f5b6ce1899acc9e70",
			vec![1000000000000, 200410000000, 452990000000],
			"2020-08-26T00:27:36+00:00",
		),
		(
			"21d898d43f0b24aaae1058213de7b129f793c21e72b474278f9749bf60736c46",
			vec![1000000000000],
			"2020-08-19T00:07:18+00:00",
		),
		(
			"c346f1108e6703d5b2abcd900ac0b0ad633553fcdd67971e0fae79edfc054fd8",
			vec![],
			"2020-08-18T00:05:18+00:00",
		),
		(
			"441243cc6fa263b45f85c826f1ae8f317458b4de04b402eb4fbe8b7a873ad9d5",
			vec![],
			"2020-08-17T00:03:06+00:00",
		),
		(
			"55a1e9ae293dcb67ea8927891309650572b996921c313a0c6699da7a918c9240",
			vec![],
			"2020-08-15T23:58:12+00:00",
		),
		(
			"6dd0b51f4a77a1f082fa76cefed91d5fb9ad7dbef4676a6fdeb56d74a40aa32e",
			vec![],
			"2020-08-14T23:54:36+00:00",
		),
		(
			"079953c44fa1a613816f2c20fe3422e615d652bc1f6bf90a1c7446a0a80420e4",
			vec![],
			"2020-08-13T23:53:00+00:00",
		),
		(
			"af4ff180b0749e68c366d035f9915afbd6e855097ea917494bddfd030b49ceff",
			vec![],
			"2020-08-12T23:49:30+00:00",
		),
		(
			"4ee2c1e8aef338f642ac08e45f3eb04630fa33d1e8047f547d84fd1fe4c64abf",
			vec![],
			"2020-08-11T23:43:24+00:00",
		),
		(
			"641b459b4eec867a809366fa9b435a0ee911cb35f3732366932da5033c7ab385",
			vec![],
			"2020-08-10T23:35:42+00:00",
		),
		(
			"4b86eff150e94a6af932d77727fc7ae9ecdb6efa9222b815aba6f7c5f9c0d67b",
			vec![],
			"2020-08-09T22:33:18+00:00",
		),
		(
			"0d09d7c56ab7098aecf895cf49c042835704c17900dfafff330f6e7ceb5b230c",
			vec![],
			"2020-08-08T22:29:24+00:00",
		),
		(
			"bd817b662ee231266674e5d4d595f53f18310512f8eb141a60c65d62a9c64f14",
			vec![],
			"2020-08-07T22:14:24+00:00",
		),
		(
			"ef369ae4bddae477e85b9e17d404c80689f82ded056b10d857ba0f67821c7fb8",
			vec![],
			"2020-08-06T22:01:06+00:00",
		),
		(
			"fffefedcad072dcafb07efb43a3fd112b118833d3e1f29caaf1407cf0aac2c8e",
			vec![],
			"2020-08-05T21:46:30+00:00",
		),
		(
			"17429179b2e15908414f86d6908a2b0592d72bf17f7a8951ecf1409dd2815ec1",
			vec![],
			"2020-08-04T21:37:00+00:00",
		),
		(
			"7f5a3beaa9e1aff443dfbc60e0d53301aabcca9b807ecd2ba0778a2240ddf65c",
			vec![],
			"2020-08-03T21:31:24+00:00",
		),
		(
			"8c44ced01b48a4debb7a50f953955e6651600e856b33c424c636357d153c53f4",
			vec![],
			"2020-08-02T21:23:54+00:00",
		),
		(
			"fb864f66996b38c641ef9479e9462c9b552be335fd276f75c46fa407bb64f56f",
			vec![],
			"2020-08-01T21:15:48+00:00",
		),
		(
			"006419712f1f4f04256c94c4cf91daf98d561dad1af00ffadbcd651196a7a0bc",
			vec![],
			"2020-07-31T21:09:18+00:00",
		),
		(
			"fedece5585e58385e0fb7d0431e073c33f266e1f45d412f6ac5ed1cec9069374",
			vec![],
			"2020-07-30T21:04:42+00:00",
		),
		(
			"cb7e4255db0d215803c4730d98f9b22d6c86cc83ab0cd3d0bc07bd56f46d7fd8",
			vec![1000000000000, 1000000000000],
			"2020-07-29T20:58:12+00:00",
		),
		(
			"2f7a65e01e293fb1f2c2359a7444b61acd3c334984c140a0171d94393aa99169",
			vec![1000000000000],
			"2020-07-28T20:52:00+00:00",
		),
		(
			"ec43b3235358dcf3110c112d81a34d65f3c3fd91c2b1cc45c30133a101998ad9",
			vec![1000000000000],
			"2020-07-27T20:44:54+00:00",
		),
		(
			"8572718dad50c45be3f16264772958b5426d7e312a4b29e1e94d51741ad8b9cd",
			vec![1000000000000, 252580000000],
			"2020-07-26T20:38:54+00:00",
		),
		(
			"3b146c67541f6b2f189d508275e5c1f459289e35adc3109655ab583f97514e72",
			vec![1000000000000],
			"2020-07-25T20:33:18+00:00",
		),
		(
			"929608ca578aa32febb5e110741ab2ae535439fe931bc7ca2d61b71c847ffdaa",
			vec![1000000000000],
			"2020-07-24T20:28:06+00:00",
		),
		(
			"7b33e51e7504a02b7d40e60a1a9c5265f39886bc5c7d958f85c9eb9a626a3c2d",
			vec![202580000000, 202580000000, 252580000000],
			"2020-07-23T20:23:36+00:00",
		),
		(
			"6a74381eeb1d71536df650c46b96cab49dc5c534fccc1bd749f874b318a957ca",
			vec![
				1000000000000,
				1000000000000,
				1000000000000,
				804170000000,
				806820000000,
				603640000000,
				1000000000000,
				1000000000000,
			],
			"2020-07-22T20:01:54+00:00",
		),
		(
			"08a73a275d06f7f9d6cd0ed848b7a7ed9a61fb7f5ad75ac20780109f874ec567",
			vec![
				202910000000,
				206300000000,
				1000000000000,
				1000000000000,
				252580000000,
				1000000000000,
				252580000000,
				202580000000,
				1000000000000,
				1000000000000,
				252580000000,
			],
			"2020-07-21T19:55:12+00:00",
		),
		(
			"e1ad6097c2cd2664d8dc4d6f597800a607e5a8df804f8723c117b97845dacfbc",
			vec![
				1000000000000,
				1000000000000,
				1000000000000,
				1000000000000,
				1000000000000,
				1000000000000,
				607240000000,
				1000000000000,
				252580000000,
			],
			"2020-07-20T19:50:06+00:00",
		),
		(
			"5b81b31f41e4d8372f7b7a3517592a561a18de42debad69e15a3dbdf62ee8cad",
			vec![1000000000000, 1000000000000],
			"2020-07-19T19:46:00+00:00",
		),
		(
			"5ec2d9816d5b98719e63edb3fc831388f6b96c1d348948c7adc1d2ce4333c5bc",
			vec![1000000000000],
			"2020-07-18T19:41:18+00:00",
		),
		(
			"d5c1369cec102a336d95f47b3483df810314f279e8a872b868fd2d000aa34c5c",
			vec![1000000000000, 1000000000000],
			"2020-07-17T19:36:18+00:00",
		),
		(
			"d39209ccf8ae3b11a4f78bced0ac6f1ffe10dfe8b3bc3a60d80a2976fb9cc669",
			vec![],
			"2020-07-14T19:27:30+00:00",
		),
		(
			"717ebe9348c0f4edf2e5ac9a010a545d8994b257dc16d3901125234c864bbe94",
			vec![],
			"2020-07-07T18:46:24+00:00",
		),
		(
			"e2bbcdfd9cf9619552dd6f7790728bb1147b2d03be096395594b53c8129c70f0",
			vec![],
			"2020-06-30T17:44:18+00:00",
		),
		(
			"5b986b7b3892a5814f77a63b505472068f54ec81f6674ca1be8aedbb0ad64115",
			vec![],
			"2020-06-23T16:37:48+00:00",
		),
		(
			"31cc28a13e57cb5de24cf7bd72fdcc2a23b08b09cec2196c9ef2ab7566a99f63",
			vec![],
			"2020-06-16T15:40:24+00:00",
		),
		(
			"a88ea7a98261183e87100d0900636081987ba52d41ac0e55ff7ef9d7c676bdd3",
			vec![],
			"2020-06-09T15:38:12+00:00",
		),
		(
			"64d6b684ab55d8104b8e46124a4c23ab927a5d6b72d274f32e05abde01fb17d1",
			vec![],
			"2020-06-02T15:37:12+00:00",
		),
	]
	.into_iter()
	.rev()
	.map(|(h, x, y)| (hex::decode(h).unwrap(), x, y))
	.map(|(h, x, y)| {
		let mut hash = [0u8; 32];
		hash.copy_from_slice(h.as_ref());
		(hash.into(), x, y)
	})
	.collect::<Vec<(Hash, Vec<Balance>, &str)>>();

	let mut slash_record: BTreeMap<AccountId, Vec<(Hash, Balance, Balance)>> = BTreeMap::new();
	for (at, deposits, timestamp) in elections {
		let header = sub_storage::get_header(&client, at).await;
		let parent = header.parent_hash;

		let spec = sub_storage::get_runtime_version(&client, at)
			.await
			.spec_version;

		let response = reqwest::blocking::get(&format!("https://explorer-31.polkascan.io/polkadot/api/v1/runtime-module/{}-electionsphragmen?include=calls,events,storage,constants,errors",spec)).unwrap().text().unwrap();
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
		});

		// for sanity-check, check the chain state. note that now we do `at`, not `parent`.
		let post_election_members = sub_storage::read::<Vec<(AccountId, Balance)>>(
			sub_storage::value_key(b"PhragmenElection", b"Members"),
			&client,
			at,
		)
		.await
		.unwrap()
		.into_iter()
		.map(|(x, _)| x)
		.collect::<Vec<_>>();
		let post_chain_runners_up = sub_storage::read::<Vec<(AccountId, Balance)>>(
			sub_storage::value_key(b"PhragmenElection", b"RunnersUp"),
			&client,
			at,
		)
		.await
		.unwrap()
		.into_iter()
		.map(|(x, _)| x)
		.collect::<Vec<_>>();

		// get correct slashes.
		let correct_slashed = remote_externalities::Builder::new()
			.module("PhragmenElection")
			.module("Balances")
			.module("System")
			.at(parent)
			.build_async()
			.await
			.execute_with(|| {
				if spec == 25 {
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
				if spec == 25 {
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

				println!("📅 Timestamp {:?}", timestamp);
				println!(
					"🧮 Spec = {}, Members = {}, RunnersUp = {}",
					spec,
					DesiredMembers::get(),
					DesiredRunnersUp::get()
				);

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
						// -^^ hash, actual-slashed-amount, leftover, post-slash-reserved,
						println!("👎🏻 wrongly slashed = {:?} -> {} ({})", w.0, w.1, w.2)
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

	// TODO: double check the list of corrupt guys with js api.
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

use crate::primitives::{Balance, Hash};
use crate::KSM;
use crate::{network, storage, Client, CommonConfig};
use codec::Encode;
use frame_metadata::{RuntimeMetadata, RuntimeMetadataPrefixed};
use frame_support;
use hex_literal::hex;
use network::get_account_data_at;
use node_runtime::Call;
use sp_core::{storage::StorageKey, Bytes};
use sp_phragmen::PhragmenScore;

/// Run something.
pub async fn run(client: &Client, _command_config: CommonConfig) {
	last_election_submission(client).await;
	// account_balance_history(&k, crate::KUSAMA_GENESIS.into(), None, client).await;
	// dust(client).await
	// coinbase(client).await
}

/// print the storage layout of chain.
#[allow(unused)]
async fn dust(client: &Client) {
	fn unwrap_decoded<B: Eq + PartialEq + std::fmt::Debug, O: Eq + PartialEq + std::fmt::Debug>(
		input: frame_metadata::DecodeDifferent<B, O>,
	) -> O {
		if let frame_metadata::DecodeDifferent::Decoded(o) = input {
			o
		} else {
			panic!("Data is not decoded: {:?}", input)
		}
	}

	#[derive(Debug, Clone, Default)]
	struct Module {
		pub name: String,
		pub size: usize,
		pub items: Vec<Storage>,
	}

	impl Module {
		fn new(name: String) -> Self {
			Self {
				name,
				..Default::default()
			}
		}
	}

	#[derive(Debug, Clone, Default)]
	struct Storage {
		pub name: String,
		pub size: usize,
	}

	impl Storage {
		fn new(name: String, size: usize) -> Self {
			Self { name, size }
		}
	}

	let mut modules: Vec<Module> = vec![];

	let now = network::get_head(client).await;
	let raw_metadata = network::get_metadata(client, now).await.0;
	let prefixed_metadata = <RuntimeMetadataPrefixed as codec::Decode>::decode(&mut &*raw_metadata)
		.expect("Runtime Metadata failed to decode");
	let metadata = prefixed_metadata.1;

	if let RuntimeMetadata::V11(inner) = metadata {
		let decode_modules = unwrap_decoded(inner.modules);
		for module in decode_modules.into_iter() {
			let name = unwrap_decoded(module.name);
			let storage = unwrap_decoded(module.storage.unwrap());
			let prefix = unwrap_decoded(storage.prefix);
			let entries = unwrap_decoded(storage.entries);
			let mut module_info = Module::new(name.clone());
			for storage_entry in entries.into_iter() {
				let storage_name = unwrap_decoded(storage_entry.name);
				let ty = storage_entry.ty;
				println!("{:?} => {:?}", storage_name, ty);
				let key_prefix =
					storage::module_prefix_raw(prefix.as_bytes(), storage_name.as_bytes());
				let pairs = storage::get_pairs(StorageKey(key_prefix.clone()), client, now).await;
				let pairs = pairs
					.into_iter()
					.map(|(k, v)| (k.0, v.0))
					.collect::<Vec<(Vec<u8>, Vec<u8>)>>();
				let size = pairs.iter().fold(0, |acc, x| acc + x.1.len());
				println!("Pair Count = {:?}", pairs.len());
				println!("Encoded Size Sum = {:?}", size);

				module_info.size += size;
				module_info.items.push(Storage::new(storage_name, size));

				let rpc_sum = network::got_storage_size(StorageKey(key_prefix), client, now).await;
				dbg!(rpc_sum);
			}
		}
	} else {
		log::error!("Invalid Metadata version");
	}
}

#[allow(unused)]
/// Checks the account balance before and after the given block hash. Good for testing slash/reward.
async fn account_balance_around_block(account: &[u8], at: Hash, client: &Client) {
	let block = network::get_block(client, at).await.block;
	let parent_hash = block.header.parent_hash;
	let balance_before = get_account_data_at(account, client, parent_hash)
		.await
		.data
		.free;
	let balance_after = get_account_data_at(account, client, at).await.data.free;

	if balance_after > balance_before {
		println!(
			"✅ Block {}: Increased by: {:?}",
			block.header.number,
			KSM(balance_after - balance_before)
		);
	} else {
		println!(
			"❌ Block {}: Decreased by: {:?}",
			block.header.number,
			KSM(balance_before - balance_after)
		);
	}
}

/// Scrape the account balance of an account from tip of the chain until `from`.
#[allow(unused)]
async fn account_balance_history(account: &[u8], until: Hash, from: Option<Hash>, client: &Client) {
	use textplots::{Chart, Plot, Shape};
	let mut now = from.unwrap_or_else(|| async_std::task::block_on(network::get_head(client)));
	let mut account_data = get_account_data_at(account, client, now).await;
	let mut block = network::get_block(client, now).await.block;
	let mut now_number = block.header.number;

	let mut points = vec![(now_number, account_data.data.free)];
	println!("Current balance {:?} = {:?}", now, account_data.data.free);

	now = block.header.parent_hash;
	loop {
		block = network::get_block(client, now).await.block;
		let parent_hash = block.header.parent_hash;
		let new_account_data = get_account_data_at(&account, client, now).await;
		if new_account_data.data.free != account_data.data.free {
			println!(
				"Account Data at {:?} = {:?}",
				block.header.number, account_data,
			);
			points.push((block.header.number, new_account_data.data.free))
		}

		account_data = new_account_data;
		now = parent_hash;
		if parent_hash == until {
			break;
		}
	}

	let points = points
		.into_iter()
		.map(|(x, y)| {
			(
				x as f32,
				y as f32 / <KSM as frame_support::traits::Get<Balance>>::get() as f32,
			)
		})
		.collect::<Vec<(f32, f32)>>();
	dbg!(&points);
	Chart::new(400, 180, 2411759.0, 2412091.0)
		.lineplot(Shape::Steps(&points))
		.display();
}

/// Get the latest election submissions, and how they change the best score.
#[allow(unused)]
async fn last_election_submission(client: &Client) {
	let submission_weight: u64 = 100000000000;

	#[derive(Default)]
	struct EraScoreDiff(Vec<f64>);

	impl std::fmt::Debug for EraScoreDiff {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			write!(f, "[")?;
			for v in self.0.iter() {
				write!(f, "{:.5}% ,", v * 100f64)?;
			}
			write!(f, "]")
		}
	}

	struct EraInfo {
		message: String,
		score: sp_phragmen::PhragmenScore,
	}

	impl EraInfo {
		fn new(message: String, score: PhragmenScore) -> Self {
			Self { message, score }
		}
	}

	let mut era_dumps: Vec<EraInfo> = vec![];

	let mut now = network::get_head(client).await;
	let mut prev_era: pallet_staking::EraIndex = 0;

	fn compare_scores(this: PhragmenScore, that: PhragmenScore) -> Vec<f64> {
		this.iter()
			.map(|x| *x as i128)
			.zip(that.iter().map(|x| *x as i128))
			.map(|(x, y)| (x - y, y))
			.map(|(diff, that)| diff as f64 / that as f64)
			.take(3)
			.collect()
	}

	loop {
		let block = network::get_block(client, now).await.block;
		let parent_hash = block.header.parent_hash;
		for e in block.extrinsics {
			let call: Call = e.clone().function;

			match call {
				Call::Staking(staking_call) => {
					if let node_runtime::staking::Call::submit_election_solution_unsigned(
						winners,
						compact,
						score,
						era,
					) = staking_call
					{
						if era != prev_era && prev_era != 0 {
							// we have a new era. print dump
							let initial_info = era_dumps.remove(era_dumps.len() - 1);
							println!("[🤑]{}", initial_info.message);
							let mut prev_score = initial_info.score;
							if era_dumps.len() > 0 {
								for EraInfo { message, score } in era_dumps.iter().rev() {
									println!(
										"[{} // {:?}]{}",
										if sp_phragmen::is_score_better(prev_score, score.clone()) {
											"✅"
										} else {
											"❌"
										},
										EraScoreDiff(compare_scores(prev_score, *score)),
										message,
									);
									prev_score = *score;
								}
							}
							era_dumps.clear();
						}

						let info_message = format!(
							"[Era {}, block {}] Submit election solution with score {:?}",
							era, block.header.number, score
						);

						let events = network::get_events_at(client, now)
							.await
							.expect("Must have some events");

						let submission_event = events
							.into_iter()
							.map(|e| e.event)
							.find_map(|event| {
								if let node_runtime::Event::system(system_event) = event {
									match system_event {
										node_runtime::system::Event::<node_runtime::Runtime>::ExtrinsicSuccess(info) => {
											if info.weight == submission_weight { Some("ExtrinsicSuccess".to_string()) } else { None }
										}
										node_runtime::system::Event::<node_runtime::Runtime>::ExtrinsicFailed(err, info) => {
											if info.weight == submission_weight { Some(format!("ExtrinsicFailed({:?})", err)) } else { None }
										}
										_ => None,
									}
								} else {
									// nothing
									None
								}
							})
							.expect("Submission must either succeed or fail");

						let info_event = format!("Event = {:?}", submission_event);
						let message = format!("{} // {}", info_message, info_event);
						era_dumps.push(EraInfo::new(message, score));

						prev_era = era;
					}
				}
				_ => {}
			}
		}

		now = parent_hash;
	}
}

#[allow(unused)]
/// Coinbase fee prediction.
async fn coinbase(client: &Client) {
	let mut now = network::get_head(client).await;
	loop {
		let block = network::get_block(client, now).await.block;
		let parent_hash = block.header.parent_hash;
		let extrinsic_count = block.extrinsics.len();
		for (index, e) in block.extrinsics.into_iter().enumerate() {
			let call: Call = e.clone().function;
			if let node_runtime::Call::Balances(balances_call) = call {
				if let node_runtime::BalancesCall::transfer(dest, amount) = balances_call {
					let info = network::query_info(Bytes(e.encode()), client, now).await;
					let events = network::get_events_at(client, now).await.unwrap();
					// filter the extrinsic events.
					let extrinsic_events = events
						.into_iter()
						.map(|e| e.event)
						.filter_map(|event| {
							if let node_runtime::Event::system(system_event) = event {
								match system_event {
									node_runtime::system::Event::<node_runtime::Runtime>::ExtrinsicSuccess(..) |
									node_runtime::system::Event::<node_runtime::Runtime>::ExtrinsicFailed(..) => {
										Some(system_event)
									}
									_ => None,
								}
							} else {
								// nothing
								None
							}
						})
						.collect::<Vec<node_runtime::system::Event<node_runtime::Runtime>>>();

					assert_eq!(extrinsic_events.len(), extrinsic_count);
					let transfer_event = extrinsic_events.get(index).unwrap();

					let multiplier = storage::read::<sp_runtime::Fixed128>(
						storage::value_key(b"TransactionPayment", b"NextFeeMultiplier"),
						client,
						now,
					)
					.await;
					println!(
						"Found a balances call here {:?} // {:?} // {:?} // {:?}",
						&e, info, multiplier, transfer_event
					);
					break;
				}
			}
		}
		now = parent_hash;
	}
}

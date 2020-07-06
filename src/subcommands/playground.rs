#![allow(unused)]

use crate::primitives::{runtime, Balance, Hash};
use crate::{network, storage, Client, CommonConfig, Currency, LOG_TARGET};
use ansi_term::{Colour::*, Style};
use codec::Encode;
use frame_metadata::{RuntimeMetadata, RuntimeMetadataPrefixed, StorageEntryType};
use frame_support;
use hex_literal::hex;
use network::get_account_data_at;
use sp_core::{storage::StorageKey, Bytes};
use sp_npos_elections::ElectionScore;

/// Main run function of the sub-command.
pub async fn run(client: &Client, common: CommonConfig) {
	// last_election_submission(client).await;
	validators_of_block(client, common.at).await;
	// account_balance_history(&k, crate::KUSAMA_GENESIS.into(), None, client).await;
	// dust(client).await
	// coinbase(client).await
}

async fn validators_of_block(client: &Client, at: Hash) {
	let validators = storage::read::<Vec<node_primitives::AccountId>>(
		storage::value_key(b"Session", b"Validators"),
		&client,
		at,
	)
	.await
	.expect("Validators must exist at each block.");

	for (i, v) in validators.into_iter().enumerate() {
		println!(
			"#{} [{}] - {:?}",
			i + 1,
			network::get_identity(&v, client, at).await,
			v
		);
	}
}

/// print the storage layout of chain.
async fn dust(client: &Client) {
	use separator::Separatable;

	fn unwrap_decoded<B: Eq + PartialEq + std::fmt::Debug, O: Eq + PartialEq + std::fmt::Debug>(
		input: frame_metadata::DecodeDifferent<B, O>,
	) -> O {
		if let frame_metadata::DecodeDifferent::Decoded(o) = input {
			o
		} else {
			panic!("Data is not decoded: {:?}", input)
		}
	}

	fn get_prefix(indent: usize) -> &'static str {
		match indent {
			1 => "├─┬",
			2 => "│ │──",
			_ => panic!("Unexpected indent."),
		}
	}

	const KB: usize = 1024;
	const MB: usize = KB * KB;
	const GB: usize = MB * MB;

	const SPACING: usize = 4;

	struct Size(usize);

	impl std::fmt::Display for Size {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			if self.0 <= KB {
				write!(f, "{: <3}B", self.0)?;
			} else if self.0 <= MB {
				write!(f, "{: <3}K", self.0 / KB)?;
			} else if self.0 <= GB {
				write!(f, "{: <3}M", self.0 / MB)?;
			}

			Ok(())
		}
	}

	#[derive(Debug, Clone, Default)]
	struct Module {
		pub name: String,
		pub size: usize,
		pub items: Vec<Storage>,
	}

	impl std::fmt::Display for Module {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			let mod_style = Style::new().bold().italic().fg(Green);
			write!(
				f,
				"{} {} {}\n",
				mod_style.paint(format!("{}", Size(self.size))),
				get_prefix(1),
				mod_style.paint(self.name.clone())
			)?;
			for s in self.items.iter() {
				write!(f, "{} {} {}\n", Size(s.size), get_prefix(2), s)?;
			}
			Ok(())
		}
	}

	impl Module {
		fn new(name: String) -> Self {
			Self {
				name,
				..Default::default()
			}
		}
	}

	#[derive(Debug, Copy, Clone)]
	pub enum StorageItem {
		Value(usize),
		Map(usize, usize),
	}

	impl std::fmt::Display for StorageItem {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			match self {
				Self::Value(bytes) => write!(f, "Value({} bytes)", bytes.separated_string()),
				Self::Map(bytes, count) => write!(
					f,
					"Value({} bytes, {} keys)",
					bytes.separated_string(),
					count
				),
			}
		}
	}

	impl Default for StorageItem {
		fn default() -> Self {
			Self::Value(0)
		}
	}

	#[derive(Debug, Clone, Default)]
	struct Storage {
		pub name: String,
		pub size: usize,
		pub item: StorageItem,
	}

	impl std::fmt::Display for Storage {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			let item_style = Style::new().italic();
			write!(
				f,
				"{} => {}",
				item_style.paint(self.name.clone()),
				self.item
			)
		}
	}

	impl Storage {
		fn new(name: String, item: StorageItem) -> Self {
			let size = match item {
				StorageItem::Value(s) => s,
				StorageItem::Map(s, _) => s,
			};
			Self { name, item, size }
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

			// skip, if this module has no storage items.
			if module.storage.is_none() {
				log::warn!(
					target: LOG_TARGET,
					"Module with name {:?} seem to have no storage items.",
					name
				);
				continue;
			}

			let storage = unwrap_decoded(module.storage.unwrap());
			let prefix = unwrap_decoded(storage.prefix);
			let entries = unwrap_decoded(storage.entries);
			let mut module_info = Module::new(name.clone());

			for storage_entry in entries.into_iter() {
				let storage_name = unwrap_decoded(storage_entry.name);
				let ty = storage_entry.ty;
				let key_prefix =
					storage::module_prefix_raw(prefix.as_bytes(), storage_name.as_bytes());
				let pairs = storage::get_pairs(StorageKey(key_prefix.clone()), client, now).await;
				let pairs = pairs
					.into_iter()
					.map(|(k, v)| (k.0, v.0))
					.collect::<Vec<(Vec<u8>, Vec<u8>)>>();

				let size = pairs.iter().fold(0, |acc, x| acc + x.1.len());

				log::trace!(
					target: LOG_TARGET,
					"{:?}::{:?} => count: {}, size: {} bytes",
					name,
					storage_name,
					pairs.len(),
					size
				);

				module_info.size += size;
				let item = match ty {
					StorageEntryType::Plain(_) => StorageItem::Value(size),
					StorageEntryType::Map { .. } | StorageEntryType::DoubleMap { .. } => {
						StorageItem::Map(size, pairs.len())
					}
				};
				module_info.items.push(Storage::new(storage_name, item));
			}
			module_info.items.sort_by_key(|x| x.size);
			module_info.items.reverse();
			print!("{}", module_info);
			modules.push(module_info);
		}

		modules.into_iter().for_each(|m| {
			print!("{}", m);
		});
	} else {
		log::error!("Invalid Metadata version");
	}
}

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
			Currency(balance_after - balance_before)
		);
	} else {
		println!(
			"❌ Block {}: Decreased by: {:?}",
			block.header.number,
			Currency(balance_before - balance_after)
		);
	}
}

/// Scrape the account balance of an account from tip of the chain until `from`.
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
		.map(|(x, y)| (x as f32, y as f32 / *crate::DECIMAL_POINTS.borrow() as f32))
		.collect::<Vec<(f32, f32)>>();
	dbg!(&points);
	Chart::new(400, 180, 2411759.0, 2412091.0)
		.lineplot(Shape::Steps(&points))
		.display();
}

/// Get the latest election submissions, and how they change the best score.
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
		score: ElectionScore,
	}

	impl EraInfo {
		fn new(message: String, score: ElectionScore) -> Self {
			Self { message, score }
		}
	}

	let mut era_dumps: Vec<EraInfo> = vec![];

	let mut now = network::get_head(client).await;
	let mut prev_era: pallet_staking::EraIndex = 0;

	fn compare_scores(that: ElectionScore, this: ElectionScore) -> Vec<f64> {
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
			let call: runtime::Call = e.clone().function;

			match call {
				runtime::Call::Staking(staking_call) => {
					if let runtime::staking::Call::submit_election_solution_unsigned(
						winners,
						compact,
						score,
						era,
						size,
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
										if sp_npos_elections::is_score_better(
											score.clone(),
											prev_score,
											<runtime::Runtime as pallet_staking::Trait>::MinSolutionScoreBump::get(),
										) {
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

						let maybe_submission_event =
							events.into_iter().map(|e| e.event).find_map(|event| {
								if let runtime::Event::staking(staking_event) = event {
									match staking_event {
										runtime::staking::Event::<runtime::Runtime>::SolutionStored(compute) => {
											Some("SolutionStored")
										}
										_ => None,
									}
								} else {
									// nothing
									None
								}
							});

						let info_event = format!("Event = {:?}", maybe_submission_event);
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

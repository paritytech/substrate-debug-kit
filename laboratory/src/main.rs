#![allow(unused)]
use ansi_term::{Colour::*, Style};
use codec::Encode;
use frame_support;
use hex_literal::hex;
use jsonrpsee::Client;
use sp_core::{storage::StorageKey, Bytes};
use sp_npos_elections::ElectionScore;
use structopt::StructOpt;
use sub_storage::{helpers, primitives::*};

mod network;
use network::runtime;

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "laboratory", about = "This is where I build new stuff.`")]
pub struct Opt {
	/// The block number at which the scrap should happen. Use only the hex value, no need for a
	/// `0x` prefix.
	#[structopt(long)]
	at: Option<node_primitives::Hash>,

	/// The node to connect to.
	#[structopt(long, default_value = "ws://localhost:9944")]
	uri: String,
}


#[async_std::main]
async fn main() {
	let opt = Opt::from_args();

	// connect to a node.
	let transport = jsonrpsee::transport::ws::WsTransportClient::new(&opt.uri)
		.await
		.expect("Failed to connect to client");
	let client: jsonrpsee::Client = jsonrpsee::raw::RawClient::new(transport).into();

	last_election_submission(&client).await;
	// account_balance_history(&k, crate::KUSAMA_GENESIS.into(), None, client).await;
	// dust(client).await
	// coinbase(client).await
}

/// Checks the account balance before and after the given block hash. Good for testing slash/reward.
async fn account_balance_around_block(account: &AccountId, at: Hash, client: &Client) {
	let block = network::get_block(client, at).await.block;
	let parent_hash = block.header.parent_hash;
	let balance_before = helpers::get_account_data_at(account, client, parent_hash)
		.await
		.data
		.free;
	let balance_after = helpers::get_account_data_at(account, client, at)
		.await
		.data
		.free;

	if balance_after > balance_before {
		println!(
			"✅ Block {}: Increased by: {:?}",
			block.header.number,
			balance_after - balance_before
		);
	} else {
		println!(
			"❌ Block {}: Decreased by: {:?}",
			block.header.number,
			balance_before - balance_after
		);
	}
}

/// Scrape the account balance of an account from tip of the chain until `from`.
async fn account_balance_history(
	account: &AccountId,
	until: Hash,
	from: Option<Hash>,
	client: &Client,
) {
	use helpers::*;
	use sub_storage::get_head;
	use textplots::{Chart, Plot, Shape};

	let mut now = from.unwrap_or_else(|| async_std::task::block_on(get_head(client)));
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
		.map(|(x, y)| (x as f32, y as f32 / 1_000_000_000_000u128 as f32))
		.collect::<Vec<(f32, f32)>>();

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

	let mut now = sub_storage::get_head(client).await;
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

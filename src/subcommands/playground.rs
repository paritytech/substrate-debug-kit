use crate::primitives::{AccountId, Balance, Hash};
use crate::{network, storage, Client, CommonConfig};
use kusama_runtime::{Call, Runtime, Staking, UncheckedExtrinsic};

#[allow(unused)]
pub async fn run(client: &Client, command_config: CommonConfig) {
	last_election_submission(client).await;
}

async fn find_defunct_nominators(client: &Client, common_config: CommonConfig) {}

async fn find_defunct_voter(client: &Client, common_config: CommonConfig) {}

async fn last_election_submission(client: &Client) {
	let mut now = network::get_head(client).await;

	loop {
		let block = network::get_block(client, now).await.block;
		let parent_hash = block.header.parent_hash;
		for e in block.extrinsics {
			let call: Call = e.clone().function;

			match call {
				Call::Staking(staking_call) => {
					println!("Staking transaction: {:?}", &staking_call);
					// this is what I want.
					if let pallet_staking::Call::submit_election_solution_unsigned(
						winners,
						compact,
						score,
						era,
					) = staking_call
					{
						println!("Submit election solution with score {:?}", score);
					}

					// this works
					if let kusama_runtime::staking::Call::submit_election_solution_unsigned(
						winners,
						compact,
						score,
						era,
					) = staking_call
					{
						println!("Submit election solution with score {:?}", score);
					}
				}
				_ => {}
			}
		}

		now = parent_hash;
	}
}

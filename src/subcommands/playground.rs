use crate::primitives::{AccountId, Balance, Hash};
use crate::{network, storage, Client, CommonConfig};
use codec::Encode;
use frame_system::AccountInfo;
use kusama_runtime::{Call, UncheckedExtrinsic};
use pallet_balances::AccountData;
use pallet_staking::Call as StakingCall;

#[allow(unused)]
pub async fn run(client: &Client, command_config: CommonConfig) {
	let account =
		hex_literal::hex!["2c61f078a240b295eb8d19db50e5b27b39225ede1cf718c0872c441cb7ac8d54"];
	let account_data = storage::read::<AccountInfo<u32, AccountData<Balance>>>(
		storage::map_key::<frame_support::Blake2_128Concat>(b"System", b"Account", &account),
		client,
		command_config.at,
	)
	.await;

	dbg!(account_data);
}

async fn find_defunct_nominators(client: &Client, common_config: CommonConfig) {
	//
}

async fn find_defunct_voter(client: &Client, common_config: CommonConfig) {
	// read all current members
	// read all current runners
	// read all current candidates
	// read Voting of target
	// Search if any
}

async fn last_election_submission(client: &Client) {
	let mut now = network::get_head(client).await;

	loop {
		let block = network::get_block(client, now).await.block;
		let parent_hash = block.header.parent_hash;
		println!(
			"Checking {}. {} extrinsics",
			block.header.number,
			block.extrinsics.len(),
		);
		for e in block.extrinsics {
			let call: Call = e.clone().function;
			match call {
				Call::Staking(staking_call) => {
					dbg!(&staking_call);
				}
				_ => {}
			}
		}

		now = parent_hash;
	}
}

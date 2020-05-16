use crate::primitives::{AccountId, Balance, Index};
use crate::{network, storage, Client, CommonConfig};
use codec::Encode;
use frame_system::AccountInfo;
use pallet_balances::AccountData;

#[allow(unused)]
pub async fn run(client: &Client, _: CommonConfig) {
	let account: AccountId =
		hex_literal::hex!["da410f5e2c6e1e29bd53dd81f864380e8bcdf4bd8d8a84f2c5f3897890893452"]
			.into();

	let block_hash_before = network::get_head(client).await;
	let info_at: AccountInfo<Index, AccountData<Balance>> = storage::read(
		storage::map_key::<frame_support::Blake2_128Concat>(
			b"System",
			b"Account",
			account.as_ref(),
		),
		client,
		block_hash_before,
	)
	.await
	.unwrap();

	let account_data = info_at.data;

	let latest = network::get_head(client).await;
	let block = network::get_block(client, latest).await;
	// dbg!(&block);
	for e in block.block.extrinsics {
		let info = network::get_xt_info(client, e.encode().into(), latest).await;
		dbg!(info);
	}
}

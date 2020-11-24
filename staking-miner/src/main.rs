mod mock_runtime;
mod offchain_miner;
mod polkadot_weight;

use sub_storage::Client;

#[async_std::main]
async fn main() {
	env_logger::Builder::from_default_env()
		.format_module_path(true)
		.format_level(true)
		.init();

	let client: Client = sub_storage::create_ws_client("ws://localhost:9944").await;
	let now = sub_storage::get_head(&client).await;

	offchain_miner::run(&client, now).await
}

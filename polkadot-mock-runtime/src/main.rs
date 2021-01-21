mod common;
mod kusama_mock;
mod polkadot_mock;

// const URI: &'static str = "wss://kusama-rpc.polkadot.io";
const URI: &'static str = "ws://localhost:9944";

#[async_std::main]
async fn main() -> () {
	let _ = env_logger::Builder::from_default_env()
		.format_module_path(true)
		.format_level(true)
		.try_init();
	let client = sub_storage::create_ws_client(URI).await;
	let now = sub_storage::get_head(&client).await;

	remote_externalities::Builder::new().uri(URI.to_owned()).build_async().await.execute_with(|| {})
}

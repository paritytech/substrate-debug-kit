use assert_cmd::Command;

#[cfg(feature = "remote-test-kusama")]
const TEST_URI: &'static str = "wss://kusama-rpc.polkadot.io/";
#[cfg(feature = "remote-test-polkadot")]
const TEST_URI: &'static str = "wss://rpc.polkadot.io/";
#[cfg(not(any(feature = "remote-test-kusama", feature = "remote-test-polkadot")))]
const TEST_URI: &'static str = "ws://localhost:9944";

#[test]
fn staking_works() {
	let mut cmd = Command::cargo_bin("offline-election").unwrap();
	cmd.args(&["--uri", TEST_URI, "staking"]).unwrap();
}

#[test]
fn council_works() {
	let mut cmd = Command::cargo_bin("offline-election").unwrap();
	cmd.args(&["--uri", TEST_URI, "council"]).unwrap();
}

#[test]
fn dangling_works() {
	let mut cmd = Command::cargo_bin("offline-election").unwrap();
	cmd.args(&["--uri", TEST_URI, "dangling-nominators"])
		.unwrap();
}

#[test]
fn nominator_check_works() {
	let mut cmd = Command::cargo_bin("offline-election").unwrap();
	let transport =
		async_std::task::block_on(jsonrpsee::transport::ws::WsTransportClient::new(TEST_URI))
			.expect("Failed to connect to client");
	let client: jsonrpsee::Client = jsonrpsee::raw::RawClient::new(transport).into();

	// get the latest block hash
	let head = async_std::task::block_on(sub_storage::get_head(&client));
	let version = async_std::task::block_on(sub_storage::get_runtime_version(&client, head));

	// some totally random account.
	cmd.args(&[
		"--uri",
		TEST_URI,
		"nominator-check",
		"--who",
		if version.spec_name == "kusama".into() {
			"Hph4pHAqDVVdc3vLani7DfQA2TU3FfuuUcBQC8tYbWgBTnC"
		} else if version.spec_name == "polkadot".into() {
			"13Vka4qGSStrNoFZap9qryQCbubfjDVyeradJwU2BG7TxZir"
		} else {
			panic!("unsupported chain.")
		},
	])
	.unwrap();
}

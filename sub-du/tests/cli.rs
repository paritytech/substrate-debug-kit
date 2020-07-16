use assert_cmd::Command;

#[cfg(feature = "remote-test")]
const TEST_URI: &'static str = "wss://kusama-rpc.polkadot.io/";
#[cfg(not(any(feature = "remote-test-kusama", feature = "remote-test-polkadot")))]
const TEST_URI: &'static str = "ws://localhost:9944";

#[test]
fn sub_du_works() {
	let mut cmd = Command::cargo_bin("sub-du").unwrap();
	cmd.args(&["--uri", TEST_URI, "-p"]).unwrap();
}

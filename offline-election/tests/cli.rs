use assert_cmd::Command;

#[cfg(feature = "remote-test")]
const TEST_URI: &'static str = "wss://kusama-rpc.polkadot.io/";
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
#[ignore]
fn dangling_works() {
	unimplemented!()
}

#[test]
#[ignore]
fn nominator_check_works() {
	unimplemented!()
}

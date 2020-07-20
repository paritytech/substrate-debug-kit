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
	// some totally random account.
	cmd.args(&[
		"--uri",
		TEST_URI,
		"nominator-check",
		"--who",
		"Hph4pHAqDVVdc3vLani7DfQA2TU3FfuuUcBQC8tYbWgBTnC",
	])
	.unwrap();
}

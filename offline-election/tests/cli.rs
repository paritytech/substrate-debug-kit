use assert_cmd::Command;

const URI: &'static str = "wss://kusama-rpc.polkadot.io/";

#[test]
fn staking_works() {
	let mut cmd = Command::cargo_bin("offline-election").unwrap();
	cmd.args(&["--uri", URI, "staking"]).unwrap();
}

#[test]
fn council_works() {
	let mut cmd = Command::cargo_bin("offline-election").unwrap();
	cmd.args(&["--uri", URI, "council"]).unwrap();
}

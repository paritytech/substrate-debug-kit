use assert_cmd::Command;

#[test]
fn kusama_works() {
	let mut cmd = Command::cargo_bin("sub-du").unwrap();
	cmd.args(&["--uri", "wss://kusama-rpc.polkadot.io/", "-p"])
		.unwrap();
}

#[test]
fn polkadot_works() {
	let mut cmd = Command::cargo_bin("sub-du").unwrap();
	cmd.args(&["--uri", "wss://rpc.polkadot.io/", "-p"])
		.unwrap();
}

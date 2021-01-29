function test_feature() {
	echo "✅ Testing with feature $1"
	cargo test --features $1 --manifest-path sub-storage/Cargo.toml
	cargo test --features $1 --manifest-path sub-du/Cargo.toml
	cargo test --manifest-path remote-externalities/Cargo.toml
}

function test() {
	cargo test --manifest-path sub-tokens/Cargo.toml
	cargo test --manifest-path offline-election/Cargo.toml
}

test

test_feature remote-test-kusama
test_feature remote-test-polkadot


# remote-externalities

## Remote Externalities

An equivalent of `TestExternalities` that can load its state from a remote substrate based chain.

- For now, the `build()` method is not async and will block. This is so that the test code would be
  freed from dealing with an executor or async tests.
- You typically have two options, either use a mock runtime or a real one. In the case of a mock, you only care about
  the types that you want to query and **they must be the same as the one used in chain**.


#### Example

With a test runtime

```rust
use remote_externalities::Builder;

#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct TestRuntime;

use frame_system as system;
impl_outer_origin! {
	pub enum Origin for TestRuntime {}
}

impl frame_system::Trait for TestRuntime {
	..
	// we only care about these two for now. The rest can be mock. The block number type of
	// kusama is u32.
	type BlockNumber = u32;
	type Header = Header;
	..
}

#[test]
fn test_runtime_works() {
	let hash: Hash =
		hex!["f9a4ce984129569f63edc01b1c13374779f9384f1befd39931ffdcc83acf63a7"].into();
	let parent: Hash =
		hex!["540922e96a8fcaf945ed23c6f09c3e189bd88504ec945cc2171deaebeaf2f37e"].into();
	Builder::new()
		.at(hash)
		.module("System")
		.build()
		.execute_with(|| {
			assert_eq!(
				// note: the hash corresponds to 3098546. We can check only the parent.
				// https://polkascan.io/kusama/block/3098546
				<frame_system::Module<Runtime>>::block_hash(3098545u32),
				parent,
			)
		});
}
```

Or with the real kusama runtime.
```rust
use remote_externalities::Builder;
use kusama_runtime::Runtime;

#[test]
fn test_runtime_works() {
	let hash: Hash =
		hex!["f9a4ce984129569f63edc01b1c13374779f9384f1befd39931ffdcc83acf63a7"].into();
	Builder::new()
		.at(hash)
		.module("Staking")
		.build()
		.execute_with(|| assert_eq!(<pallet_staking::Module<Runtime>>::validator_count(), 400));
}

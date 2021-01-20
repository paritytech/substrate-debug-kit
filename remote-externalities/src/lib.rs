//! # Remote Externalities
//!
//! An equivalent of `TestExternalities` that can load its state from a remote substrate based
//! chain.
//!
//! - For now, the `build()` method is not async and will block. This is so that the test code would
//!   be freed from dealing with an executor or async tests.
//! - You typically have two options, either use a mock runtime or a real one. In the case of a
//!   mock, you only care about the types that you want to query and **they must be the same as the
//!   one used in chain**.
//!
//!
//! ### Example
//!
//! With a test runtime
//!
//! ```ignore
//! use remote_externalities::Builder;
//!
//! #[derive(Clone, Eq, PartialEq, Debug, Default)]
//! pub struct TestRuntime;
//!
//! use frame_system as system;
//! impl_outer_origin! {
//! 	pub enum Origin for TestRuntime {}
//! }
//!
//! impl frame_system::Trait for TestRuntime {
//! 	..
//! 	// we only care about these two for now. The rest can be mock. The block number type of
//! 	// kusama is u32.
//! 	type BlockNumber = u32;
//! 	type Header = Header;
//! 	..
//! }
//!
//! #[test]
//! fn test_runtime_works() {
//! 	let hash: Hash =
//! 		hex!["f9a4ce984129569f63edc01b1c13374779f9384f1befd39931ffdcc83acf63a7"].into();
//! 	let parent: Hash =
//! 		hex!["540922e96a8fcaf945ed23c6f09c3e189bd88504ec945cc2171deaebeaf2f37e"].into();
//! 	Builder::new()
//! 		.at(hash)
//! 		.module("System")
//! 		.build()
//! 		.execute_with(|| {
//! 			assert_eq!(
//! 				// note: the hash corresponds to 3098546. We can check only the parent.
//! 				// https://polkascan.io/kusama/block/3098546
//! 				<frame_system::Module<Runtime>>::block_hash(3098545u32),
//! 				parent,
//! 			)
//! 		});
//! }
//! ```
//!
//! Or with the real kusama runtime.
//!
//! ```ignore
//! use remote_externalities::Builder;
//! use kusama_runtime::Runtime;
//!
//! #[test]
//! fn test_runtime_works() {
//! 	let hash: Hash =
//! 		hex!["f9a4ce984129569f63edc01b1c13374779f9384f1befd39931ffdcc83acf63a7"].into();
//! 	Builder::new()
//! 		.at(hash)
//! 		.module("Staking")
//! 		.build()
//! 		.execute_with(|| assert_eq!(<pallet_staking::Module<Runtime>>::validator_count(), 400));
//! }

use log::*;
use sp_core::hashing::twox_128;
pub use sp_io::TestExternalities;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use sub_storage::StorageKey;

type Hash = sp_core::H256;

macro_rules! wait {
	($e:expr) => {
		async_std::task::block_on($e)
	};
}

const LOG_TARGET: &'static str = "remote-ext";

/// Struct for better hex printing of slice types.
pub struct HexSlice<'a>(&'a [u8]);

impl<'a> HexSlice<'a> {
	pub fn new<T>(data: &'a T) -> HexSlice<'a>
	where
		T: ?Sized + AsRef<[u8]> + 'a,
	{
		HexSlice(data.as_ref())
	}
}

// You can choose to implement multiple traits, like Lower and UpperHex
impl Debug for HexSlice<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		write!(f, "0x")?;
		for byte in self.0 {
			write!(f, "{:x}", byte)?;
		}
		Ok(())
	}
}

/// Extension trait for hex display.
pub trait HexDisplayExt {
	fn hex_display(&self) -> HexSlice<'_>;
}

impl<T: ?Sized + AsRef<[u8]>> HexDisplayExt for T {
	fn hex_display(&self) -> HexSlice<'_> {
		HexSlice::new(self)
	}
}

/// Builder for remote-externalities.
#[derive(Debug, Default)]
pub struct Builder {
	at: Option<Hash>,
	uri: Option<String>,
	inject: Vec<(Vec<u8>, Vec<u8>)>,
	module_filter: Vec<String>,
}

impl Builder {
	/// Create a new builder.
	pub fn new() -> Self {
		Default::default()
	}

	/// Scrape the chain at the given block hash.
	///
	/// If not set, latest finalized will be used.
	pub fn at(mut self, at: Hash) -> Self {
		self.at = Some(at);
		self
	}

	/// Look for a chain at the given URI.
	///
	/// If not set, `ws://localhost:9944` will be used.
	pub fn uri(mut self, uri: String) -> Self {
		self.uri = Some(uri);
		self
	}

	/// Inject a manual list of key and values to the storage.
	pub fn inject(mut self, injections: &[(Vec<u8>, Vec<u8>)]) -> Self {
		for i in injections {
			self.inject.push(i.clone());
		}
		self
	}

	/// Scrape only this module.
	///
	/// If used multiple times, all of the given modules will be used, else the entire chain.
	pub fn module(mut self, module: &str) -> Self {
		self.module_filter.push(module.to_string());
		self
	}

	pub fn cache(mut self) -> Self {
		// TODO
		self
	}

	/// Build the test externalities.
	///
	/// This is an async function, otherwise does the same as `build`.
	pub async fn build_async(self) -> TestExternalities {
		let mut ext = TestExternalities::new_empty();
		let uri = self.uri.unwrap_or(String::from("ws://localhost:9944"));

		let transport = jsonrpsee::transport::ws::WsTransportClient::new(&uri)
			.await
			.expect("Failed to connect to client");
		let client: jsonrpsee::Client = jsonrpsee::raw::RawClient::new(transport).into();

		let head = sub_storage::get_head(&client).await;
		let at = self.at.unwrap_or(head);

		info!(target: LOG_TARGET, "connecting to node {} at {:?}", uri, at);

		let keys_and_values = if self.module_filter.len() > 0 {
			let mut filtered_kv = vec![];
			for f in self.module_filter {
				let hashed_prefix = twox_128(f.as_bytes());
				let module_kv =
					sub_storage::get_pairs(StorageKey(hashed_prefix.to_vec()), &client, at).await;

				info!(
					target: LOG_TARGET,
					"Downloaded data for module {} (count: {} / prefix: {:?}).",
					f,
					module_kv.len(),
					hashed_prefix.hex_display()
				);
				for kv in module_kv.into_iter().map(|(k, v)| (k.0, v.0)) {
					filtered_kv.push(kv);
				}
			}
			filtered_kv
		} else {
			info!(target: LOG_TARGET, "Downloading data for all modules.");
			sub_storage::get_pairs(StorageKey(Default::default()), &client, at)
				.await
				.into_iter()
				.map(|(k, v)| (k.0, v.0))
				.collect::<Vec<_>>()
		};

		// inject all the scraped keys and values.
		info!(target: LOG_TARGET, "injecting a total of {} keys", keys_and_values.len());
		for (k, v) in keys_and_values {
			trace!(target: LOG_TARGET, "injecting {:?} -> {:?}", k.hex_display(), v.hex_display());
			ext.insert(k, v);
		}

		// lastly, insert the injections, if any.
		for (k, v) in self.inject.into_iter() {
			ext.insert(k, v);
		}

		ext
	}

	/// Build the test externalities.
	pub fn build(self) -> TestExternalities {
		let mut ext = TestExternalities::new_empty();
		let uri = self.uri.unwrap_or(String::from("ws://localhost:9944"));

		let transport = wait!(jsonrpsee::transport::ws::WsTransportClient::new(&uri))
			.expect("Failed to connect to client");
		let client: jsonrpsee::Client = jsonrpsee::raw::RawClient::new(transport).into();

		let head = wait!(sub_storage::get_head(&client));
		let at = self.at.unwrap_or(head);

		info!(target: LOG_TARGET, "connecting to node {} at {:?}", uri, at);

		let keys_and_values = if self.module_filter.len() > 0 {
			let mut filtered_kv = vec![];
			for f in self.module_filter {
				let hashed_prefix = twox_128(f.as_bytes());
				debug!(
					target: LOG_TARGET,
					"Downloading data for module {} (prefix: {:?}).",
					f,
					hashed_prefix.hex_display()
				);
				let module_kv =
					wait!(sub_storage::get_pairs(StorageKey(hashed_prefix.to_vec()), &client, at));

				for kv in module_kv.into_iter().map(|(k, v)| (k.0, v.0)) {
					filtered_kv.push(kv);
				}
			}
			filtered_kv
		} else {
			debug!(target: LOG_TARGET, "Downloading data for all modules.");
			wait!(sub_storage::get_pairs(StorageKey(Default::default()), &client, at))
				.into_iter()
				.map(|(k, v)| (k.0, v.0))
				.collect::<Vec<_>>()
		};

		// inject all the scraped keys and values.
		info!(target: LOG_TARGET, "injecting a total of {} keys", keys_and_values.len());
		for (k, v) in keys_and_values {
			trace!(target: LOG_TARGET, "injecting {:?} -> {:?}", k.hex_display(), v.hex_display());
			ext.insert(k, v);
		}

		// lastly, insert the injections, if any.
		for (k, v) in self.inject.into_iter() {
			ext.insert(k, v);
		}

		ext
	}
}

#[cfg(test)]
mod tests_dummy {
	use super::*;
	use frame_support::impl_outer_origin;
	use hex_literal::hex;
	use sp_core::H256;
	use sp_runtime::traits::{BlakeTwo256, IdentityLookup};

	type Header = sp_runtime::generic::Header<u32, BlakeTwo256>;

	const TEST_URI: &'static str = "ws://localhost:9944";

	macro_rules! init_log {
		() => {
			let _ = env_logger::Builder::from_default_env()
				.format_module_path(false)
				.format_level(true)
				.try_init();
		};
	}

	#[derive(Clone, Eq, PartialEq, Debug, Default)]
	pub struct TestRuntime;

	impl_outer_origin! {
		pub enum Origin for TestRuntime {}
	}

	impl frame_system::Trait for TestRuntime {
		type BaseCallFilter = ();
		type Origin = Origin;
		type Call = ();
		type Index = u32;
		type BlockNumber = u32;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = sp_runtime::AccountId32;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type BlockHashCount = ();
		type MaximumBlockWeight = ();
		type DbWeight = ();
		type BlockExecutionWeight = ();
		type ExtrinsicBaseWeight = ();
		type MaximumExtrinsicWeight = ();
		type AvailableBlockRatio = ();
		type MaximumBlockLength = ();
		type Version = ();
		type PalletInfo = ();
		type AccountData = ();
		type OnNewAccount = ();
		type OnKilledAccount = ();
		type SystemWeightInfo = ();
	}

	#[test]
	#[ignore = "can only work if a local node is available."]
	fn test_runtime_works_kusama() {
		init_log!();
		let (hash, parent) = (
			hex!["7813658eb560e0c8620d73356676d1cc160d3f3c4025a178f368dc506bbd3e3c"].into(),
			hex!["989e0785569561ce174507121a9c85d34f72e07cf3a1bfb95a8a2c10ba0e2847"].into(),
		);

		Builder::new().uri(TEST_URI.into()).at(hash).module("System").build().execute_with(|| {
			assert_eq!(<frame_system::Module<TestRuntime>>::block_hash(3098545u32), parent,)
		});
	}
}

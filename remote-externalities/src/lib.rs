use log::*;
use sp_core::hashing::twox_128;
use sp_core::storage::StorageKey;
use sp_io::TestExternalities;
use std::fmt::{Debug, Formatter, Result as FmtResult};

type Hash = sp_core::H256;

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

#[derive(Debug, Default)]
pub struct Builder {
	at: Option<Hash>,
	uri: Option<String>,
	inject: Vec<(Vec<u8>, Vec<u8>)>,
	module_filter: Vec<String>,
}

impl Builder {
	pub fn new() -> Self {
		Default::default()
	}

	pub fn at(mut self, at: Hash) -> Self {
		self.at = Some(at);
		self
	}

	pub fn uri(mut self, uri: String) -> Self {
		self.uri = Some(uri);
		self
	}

	pub fn inject(mut self, injections: &[(Vec<u8>, Vec<u8>)]) -> Self {
		for i in injections {
			self.inject.push(i.clone());
		}
		self
	}

	pub fn module(mut self, module: &str) -> Self {
		self.module_filter.push(module.to_string());
		self
	}

	pub async fn build(self) -> TestExternalities {
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
				debug!(
					target: LOG_TARGET,
					"downloading data for module {} -> {:?}",
					f,
					hashed_prefix.hex_display()
				);
				let module_kv =
					sub_storage::get_pairs(StorageKey(hashed_prefix.to_vec()), &client, at).await;

				for kv in module_kv.into_iter().map(|(k, v)| (k.0, v.0)) {
					filtered_kv.push(kv);
				}
			}
			filtered_kv
		} else {
			debug!(target: LOG_TARGET, "downloading data for all modules");
			sub_storage::get_pairs(StorageKey(Default::default()), &client, at)
				.await
				.into_iter()
				.map(|(k, v)| (k.0, v.0))
				.collect::<Vec<_>>()
		};

		// inject all the scraped keys and values.
		for (k, v) in keys_and_values {
			trace!(
				target: LOG_TARGET,
				"injecting {:?} -> {:?}",
				k.hex_display(),
				v.hex_display()
			);
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
	use kusama_runtime::Runtime;
	use sp_core::H256;
	use sp_runtime::traits::{BlakeTwo256, IdentityLookup};

	type Header = sp_runtime::generic::Header<u32, BlakeTwo256>;

	macro_rules! wait {
		($e:expr) => {
			tokio_test::block_on($e)
		};
	}

	macro_rules! init_log {
		() => {
			let _ = env_logger::Builder::from_default_env()
				.filter_level(log::LevelFilter::Debug)
				.format_module_path(false)
				.format_level(true)
				.try_init();
		};
	}

	#[derive(Clone, Eq, PartialEq, Debug, Default)]
	pub struct TestRuntime;

	use frame_system as system;
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
		type AccountId = u64;
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
		type ModuleToIndex = ();
		type AccountData = ();
		type OnNewAccount = ();
		type OnKilledAccount = ();
	}

	#[test]
	fn test_runtime_works() {
		init_log!();
		let hash: Hash =
			hex!["f9a4ce984129569f63edc01b1c13374779f9384f1befd39931ffdcc83acf63a7"].into();
		let parent: Hash =
			hex!["540922e96a8fcaf945ed23c6f09c3e189bd88504ec945cc2171deaebeaf2f37e"].into();
		wait!(Builder::new().at(hash).module("System").build()).execute_with(|| {
			assert_eq!(
				// note: the hash corresponds to 3098546. We can check only the parent.
				// https://polkascan.io/kusama/block/3098546
				<frame_system::Module<Runtime>>::block_hash(3098545u32),
				parent,
			)
		});
	}

	#[test]
	fn kusama_runtime_works() {
		init_log!();
		let hash: Hash =
			hex!["f9a4ce984129569f63edc01b1c13374779f9384f1befd39931ffdcc83acf63a7"].into();
		wait!(Builder::new().at(hash).module("Staking").build())
			.execute_with(|| assert_eq!(<pallet_staking::Module<Runtime>>::validator_count(), 400));
	}
}

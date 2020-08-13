use ansi_term::{Colour::*, Style};
use frame_metadata::{RuntimeMetadata, RuntimeMetadataPrefixed, StorageEntryType};
use separator::Separatable;
use structopt::StructOpt;
use sub_storage::get_head;
use sub_storage::get_metadata;
use sub_storage::unwrap_decoded;
use sub_storage::Hash;
use sub_storage::StorageKey;

const KB: usize = 1024;
const MB: usize = KB * KB;
const GB: usize = MB * MB;

pub const LOG_TARGET: &'static str = "sub-du";

fn get_prefix(indent: usize) -> &'static str {
	match indent {
		1 => "├─┬",
		2 => "│ │─┬",
		3 => "│ │ │─",
		_ => panic!("Unexpected indent."),
	}
}

struct Size(usize);

impl std::fmt::Display for Size {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if self.0 <= KB {
			write!(f, "{: <4}B", self.0)?;
		} else if self.0 <= MB {
			write!(f, "{: <4}K", self.0 / KB)?;
		} else if self.0 <= GB {
			write!(f, "{: <4}M", self.0 / MB)?;
		}

		Ok(())
	}
}

#[derive(Debug, Clone, Default)]
struct Module {
	pub name: String,
	pub size: usize,
	pub items: Vec<Storage>,
}

impl std::fmt::Display for Module {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mod_style = Style::new().bold().italic().fg(Green);
		write!(
			f,
			"{} {} {}\n",
			mod_style.paint(format!("{}", Size(self.size))),
			get_prefix(2),
			mod_style.paint(self.name.clone())
		)?;
		for s in self.items.iter() {
			write!(f, "{} {} {}\n", Size(s.size), get_prefix(3), s)?;
		}
		Ok(())
	}
}

impl Module {
	fn new(name: String) -> Self {
		Self {
			name,
			..Default::default()
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub enum StorageItem {
	Value(usize),
	Map(usize, usize),
}

impl std::fmt::Display for StorageItem {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Value(bytes) => write!(f, "Value({} bytes)", bytes.separated_string()),
			Self::Map(bytes, count) => {
				write!(f, "Map({} bytes, {} keys)", bytes.separated_string(), count)
			}
		}
	}
}

impl Default for StorageItem {
	fn default() -> Self {
		Self::Value(0)
	}
}

#[derive(Debug, Clone, Default)]
struct Storage {
	pub name: String,
	pub size: usize,
	pub item: StorageItem,
}

impl std::fmt::Display for Storage {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let item_style = Style::new().italic();
		write!(
			f,
			"{} => {}",
			item_style.paint(self.name.clone()),
			self.item
		)
	}
}

impl Storage {
	fn new(name: String, item: StorageItem) -> Self {
		let size = match item {
			StorageItem::Value(s) => s,
			StorageItem::Map(s, _) => s,
		};
		Self { name, item, size }
	}
}

#[derive(Debug, StructOpt)]
#[structopt(
	name = "sub-du",
	about = "a du-like tool that prints the map of storage usage of a substrate chain"
)]
struct Opt {
	/// The block number at which the scrap should happen. Use only the hex value, no need for a
	/// `0x` prefix.
	#[structopt(long)]
	at: Option<Hash>,

	/// The node to connect to.
	#[structopt(long, default_value = "ws://localhost:9944")]
	uri: String,

	/// If true, intermediate values will be printed.
	#[structopt(long, short)]
	progress: bool,
}

#[async_std::main]
async fn main() -> () {
	env_logger::Builder::from_default_env()
		.format_module_path(false)
		.format_level(true)
		.init();

	let opt = Opt::from_args();

	// connect to a node.
	let transport = jsonrpsee::transport::ws::WsTransportClient::new(&opt.uri)
		.await
		.expect("Failed to connect to client");
	let client: jsonrpsee::Client = jsonrpsee::raw::RawClient::new(transport).into();

	let mut modules: Vec<Module> = vec![];

	// potentially replace head with the given hash
	let head = get_head(&client).await;
	let at = opt.at.unwrap_or(head);
	let runtime = sub_storage::get_runtime_version(&client, at).await;

	log::info!(
		target: LOG_TARGET,
		"Scraping at block {:?} of {}({})",
		at,
		runtime.spec_name,
		runtime.spec_version,
	);

	let raw_metadata = get_metadata(&client, at).await.0;
	let prefixed_metadata = <RuntimeMetadataPrefixed as codec::Decode>::decode(&mut &*raw_metadata)
		.expect("Runtime Metadata failed to decode");
	let metadata = prefixed_metadata.1;

	if let RuntimeMetadata::V11(inner) = metadata {
		let decode_modules = unwrap_decoded(inner.modules);
		for module in decode_modules.into_iter() {
			let name = unwrap_decoded(module.name);

			// skip, if this module has no storage items.
			if module.storage.is_none() {
				log::warn!(
					target: LOG_TARGET,
					"Module with name {:?} seem to have no storage items.",
					name
				);
				continue;
			}

			let storage = unwrap_decoded(module.storage.unwrap());
			let prefix = unwrap_decoded(storage.prefix);
			let entries = unwrap_decoded(storage.entries);
			let mut module_info = Module::new(name.clone());

			for storage_entry in entries.into_iter() {
				let storage_name = unwrap_decoded(storage_entry.name);
				let ty = storage_entry.ty;
				let key_prefix =
					sub_storage::module_prefix_raw(prefix.as_bytes(), storage_name.as_bytes());
				let pairs =
					sub_storage::get_pairs(StorageKey(key_prefix.clone()), &client, at).await;
				let pairs = pairs
					.into_iter()
					.map(|(k, v)| (k.0, v.0))
					.collect::<Vec<(Vec<u8>, Vec<u8>)>>();

				let size = pairs.iter().fold(0, |acc, x| acc + x.1.len());

				log::trace!(
					target: LOG_TARGET,
					"{:?}::{:?} => count: {}, size: {} bytes",
					name,
					storage_name,
					pairs.len(),
					size
				);

				module_info.size += size;
				let item = match ty {
					StorageEntryType::Plain(_) => StorageItem::Value(size),
					StorageEntryType::Map { .. } | StorageEntryType::DoubleMap { .. } => {
						StorageItem::Map(size, pairs.len())
					}
				};
				module_info.items.push(Storage::new(storage_name, item));
			}
			module_info.items.sort_by_key(|x| x.size);
			module_info.items.reverse();
			if opt.progress {
				print!("{}", module_info);
			}
			log::debug!(
				target: LOG_TARGET,
				"Scraped module {}. Total size {}.",
				module_info.name,
				module_info.size,
			);
			modules.push(module_info);
		}

		log::info!(
			target: LOG_TARGET,
			"Scraping results done. Final sorted tree:"
		);
		modules.sort_by_key(|m| m.size);
		modules.reverse();

		let total: usize = modules.iter().map(|m| m.size).sum();
		println!("{} {} {}", Size(total), get_prefix(1), runtime.spec_name,);
		modules.into_iter().for_each(|m| {
			print!("{}", m);
		});
	} else {
		log::error!("Unsupported Metadata version");
	}
}

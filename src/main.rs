// Copyright 2019 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! A Rust RPC client for a substrate node with utility snippets to scrape the node's data and run
//! function on top of them.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

// whatever node you are connecting to. Polkadot, substrate etc.
pub use primitives::{AccountId, Balance, BlockNumber, Hash};

use clap::{load_yaml, App};
use frame_support::traits::Get;
use jsonrpsee::Client;
pub use sc_rpc_api::state::StateClient;
use separator::Separatable;
use sp_core::crypto::{set_default_ss58_version, Ss58AddressFormat};
use std::{convert::TryInto, fmt};

mod network;
mod primitives;
mod storage;
#[macro_use]
mod timing;
mod subcommands;

/// Default logging target.
pub const LOG_TARGET: &'static str = "offline-phragmen";

/// Wrapper to pretty-print ksm (or any other 12 decimal) token.
///
/// It stores the decimal points of token type as a `Get<Balance>` implementation.
struct KSM(Balance);

impl Get<Balance> for KSM {
	// return the decimal points
	fn get() -> Balance {
		1_000_000_000_000 as Balance
	}
}

impl fmt::Debug for KSM {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let num: u128 = self.0.try_into().unwrap();
		write!(
			f,
			"{}_KSM ({})",
			self.0 / <Self as Get<Balance>>::get(),
			num.separated_string()
		)
	}
}

impl fmt::Display for KSM {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let num: u128 = self.0.try_into().unwrap();
		write!(f, "{}", num.separated_string())
	}
}

/// Configurations of the top level command itself. This struct behaves like a home-made structopt
/// instance. It implements `From<clap::ArgMatches>` and will be passed to all the sub-commands.
#[derive(Clone, Default)]
pub struct CommonConfig {
	/// Target block, raw as a string.
	pub at_raw: Option<String>,
	/// Target block as hash.
	pub at: Hash,
	/// Address format.
	pub address_format: Ss58AddressFormat,
	/// Verbosity level
	pub verbosity: u64,
	/// The uri of the node to connect to.
	pub uri: String,
}

impl From<&clap::ArgMatches<'_>> for CommonConfig {
	fn from(matches: &clap::ArgMatches) -> Self {
		// uri
		let uri = matches
			.value_of("uri")
			.unwrap_or("ws://localhost:9944")
			.to_string();

		// optionally at certain block hash
		let at_raw = matches.value_of("at").map(|s| s.to_string());

		// Verbosity degree.
		let verbosity = matches.occurrences_of("verbose");

		// address format
		let address_format = match matches.value_of("network").unwrap_or("kusama") {
			"kusama" => Ss58AddressFormat::KusamaAccount,
			"polkadot" => Ss58AddressFormat::PolkadotAccount,
			"substrate" => Ss58AddressFormat::SubstrateAccount,
			_ => panic!("invalid address format"),
		};

		Self {
			at_raw,
			uri,
			address_format,
			verbosity,
			..Default::default()
		}
	}
}

fn main() {
	env_logger::try_init().ok();

	let yaml = load_yaml!("../cli.yml");
	let app = App::from(yaml);
	let matches = app.get_matches();

	let mut common_config = CommonConfig::from(&matches);

	// setup address format
	set_default_ss58_version(common_config.address_format);

	async_std::task::block_on(async {
		// connect to a node.
		let transport = jsonrpsee::transport::ws::WsTransportClient::new(&common_config.uri)
			.await
			.expect("Failed to connect to client");
		let client: Client = jsonrpsee::raw::RawClient::new(transport).into();

		// get the latest block hash
		let head = network::get_head(&client).await;

		// potentially replace with the given hash
		let at: Hash = if let Some(at_str) = common_config.at_raw.clone() {
			Hash::from_slice(&hex::decode(at_str).expect("invalid hash format given"))
		} else {
			head
		};
		common_config.at = at;

		// set total issuance
		network::issuance::set(&client, at).await;

		match matches.subcommand() {
			("staking", Some(sub_m)) => {
				subcommands::staking::run(&client, common_config.clone(), sub_m).await
			}
			("council", Some(sub_m)) => {
				subcommands::elections_phragmen::run(&client, common_config.clone(), sub_m).await
			}
			("playground", Some(_)) => {
				subcommands::playground::run(&client, common_config.clone()).await
			}
			_ => panic!("no sub-command provided"),
		};

		log::info!(
			target: LOG_TARGET,
			"total_issuance = {:?}",
			KSM(network::issuance::get())
		);
		log::info!(target: LOG_TARGET, "connected to [{}]", common_config.uri);
		log::info!(target: LOG_TARGET, "at [{}]", at);
	})
}

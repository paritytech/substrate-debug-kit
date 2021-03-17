// Copyright 2019-2020 Parity Technologies (UK) Ltd.
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

//! # Offline elections
//!
//! Run election algorithms of substrate (all under `sp-npos-elections`) offline.
//!
//! > Substrate seminar about offchain phragmen and how the staking pallet works in substrate.
//! > [youtube.com/watch?v=MjOvVhc1oXw](https://www.youtube.com/watch?v=MjOvVhc1oXw).
//!
//! > Substrate seminar session about this repo prior to the overhaul (`offline-phragmen`):
//! > [youtube.com/watch?v=6omrrY11HEg](youtube.com/watch?v=6omrrY11HEg)
//!
//! > Sub0 Talk about offchain phragmen:
//! > [crowdcast.io/e/sub0-online/7](https://www.crowdcast.io/e/sub0-online/7) /
//! > [youtube.com/watch?v=H9OvpAOebTs](https://www.youtube.com/watch?v=H9OvpAOebTs)
//!
//!
//! ### Builders
//!
//! Several tools have already built on top of this repo, such:
//!
//! - https://polkadot.pro/phragmen.php
//! - https://polkadot.staking4all.org/
//!
//! Note that the npos results generate by this repo or any of the above tools will not be exactly
//! equal to that of polkadot and kusama. This is highly dependent on the arguments passed to the
//! `staking` sub-command. The NPoS solution of both polkadot and kusama is being computed in a
//! non-deterministic way.
//!
//! As of this writing, the validator election of Polkadot/Kusama is as such: seq-phragmen -> random
//! iterations of balancing -> reduce. This translates to:
//!
//! ```
//! cargo run -- staking -i 10 -r
//! ```
//!
//! And **if executed at the correct time** (i.e. while the election window is open), this should
//! *accurately predict the next validator set*, but the nominator stake distribution will be
//! different, because the random number of iterations is not known.
//!
//! ## Usage
//!
//! Simply run `--help`.
//!
//! ```
//! Offline elections app.
//!
//! Provides utilities and debug tools around the election pallets of a substrate chain offline.
//!
//! Can be used to predict next elections, diagnose previous ones, and perform checks on validators and nominators.
//!
//! USAGE:
//!     offline-election [FLAGS] [OPTIONS] <SUBCOMMAND>
//!
//! FLAGS:
//!     -h, --help
//!             Prints help information
//!
//!     -V, --version
//!             Prints version information
//!
//!     -v
//!             Print more output
//!
//!
//! OPTIONS:
//!         --at <at>
//!             The block number at which the scrap should happen. Use only the hex value, no need for a `0x` prefix
//!
//!     -n, --network <network>
//!             Network address format. Can be kusama|polkadot|substrate.
//!
//!             This will also change the token display name. [default: polkadot]
//!         --uri <uri>
//!             The node to connect to [default: ws://localhost:9944]
//!
//!
//! SUBCOMMANDS:
//!     command-center         Display the command center of the staking panel
//!     council                Run the council election
//!     current                Display the current validators
//!     dangling-nominators    Show the nominators who are dangling:
//!     help                   Prints this message or the help of the given subcommand(s)
//!     next                   Display the next queued validators
//!     nominator-check        The general checkup of a nominator
//!     staking                Run the staking election
//!     validator-check        The general checkup of a validators
//! ```
//!
//! ## Overriding data
//!
//! You can override voters and candidates in both staking and council election py passing a `-m` or
//! `--manual-override` flag. This must point to a json file that contains the following keys:
//! 1. `voters`: the new voters to be added.
//! 2. `candidates`: the new candidates to be added.
//! 3. `voters_remove`: voters to be removed.
//! 4. `candidates_remove`: candidates to be removed.
//!
//! Note that first the incomings are added, and then any voter/candidate in the outgoing list is
//! stripped out.
//!
//! Find an example [here](./override_example.json).
//!
//! ## Example usage
//!
//! - Run the council election with 25 members.
//!
//! ```
//! RUST_LOG=offline-phragmen=trace cargo run -- council --count 25
//! ```
//!
//! - Run the staking election with no equalization at a particular block number
//!
//! ```
//! cargo run --at 8b7d6e14221b4fefc4b007660c80af6d4a9ac740c50b6e918f61d521553cd17e staking
//! ```
//!
//! - Run the election with only 50 slots, and print all the nominator distributions
//!
//! ```
//! cargo run -- -vv staking --count 50
//! ```
//!
//! - Run the above again now with `reduce()` and see how most nominator edges are... reduced.
//!
//! ```
//! cargo run -- -vv staking --count 50 --reduce
//! ```
//!
//! - Run the above again now against a remote node.
//!
//! ```
//! cargo run -- --uri wss://kusama-rpc.polkadot.io/ -vv staking --count 50 --reduce
//! ```
//!
//! ## Connecting to a node
//!
//! > Both Polkadot and Kusama are growing fast and scraping the data is becoming harder and harder.
//! I > really recommend you to try this script against a local node, or be prepared to wait for a
//! while.
//!
//! By default it will attempt to connect to a locally running node running at
//! `ws://127.0.0.1:9944`.
//!
//! Connect to a different node using the `--uri` argument e.g. `--uri
//! wss://kusama-rpc.polkadot.io/`.
//!
//! - **`ws://`** prefix: plain (unencrypted) websockets connection.
//! - **`wss://`** prefix: TLS (encrypted) websockets connection.
//!
//! ## Logging
//!
//! Scripts output additional information as logs. You need to enable them by setting `RUST_LOG`
//! environment variable.
//!
//! Also, you can always use `-v`, `-vv`, ... to get more output out of each script.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

// whatever node you are connecting to. Polkadot, substrate etc.
pub use primitives::{AccountId, Balance, BlockNumber, Hash};

use sp_core::crypto::{set_default_ss58_version, Ss58AddressFormat};
use std::path::PathBuf;
use structopt::StructOpt;
use sub_storage as storage;

mod network;
mod primitives;
#[macro_use]
mod timing;
/// Sub commands.
pub mod subcommands;

/// Default logging target.
pub const LOG_TARGET: &'static str = "offline-election";

type Currency = sub_tokens::dynamic::DynamicToken;

pub(crate) type Client = sub_storage::Client;

/// Offline elections scripts.
///
/// Provides utilities and debug tools around the election pallets of a substrate chain offline.
///
/// Can be used to predict next elections, diagnose previous ones, and perform checks on validators
/// and nominators.
#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "offline-elections")]
pub struct Opt {
	/// The block number at which the scrap should happen. Use only the hex value, no need for a
	/// `0x` prefix.
	#[structopt(long)]
	at: Option<primitives::Hash>,

	/// The node to connect to.
	#[structopt(long, default_value = "ws://localhost:9944")]
	uri: String,

	/// Network address format. Can be kusama|polkadot|substrate.
	///
	/// This will also change the token display name.
	///
	/// If not provided, then the spec name of the runtime version at given at will be compared to
	/// be `polkadot`, `kusama` or `substrate`.
	#[structopt(short, long)]
	network: Option<String>,

	/// Print more output.
	#[structopt(short, parse(from_occurrences))]
	verbosity: u64,

	/// The subcommand.
	#[structopt(subcommand)] // Note that we mark a field as a subcommand
	cmd: SubCommands,
}

/// The sub-commands.
#[derive(Debug, StructOpt, Clone)]
pub enum SubCommands {
	/// Run the staking election.
	Staking(StakingConfig),
	/// Run the council election.
	Council(CouncilConfig),
	/// Display the current validators.
	///
	/// Always maps to `session::validators()`.
	Current {},
	/// Display the next queued validators.
	///
	/// Always maps to `session::queued_keys()` and should only have sane values in the first
	/// session of each era.
	Next {},
	/// Display the command center of the staking panel.
	CommandCenter {},
	/// Show the nominators who are dangling:
	///
	/// Those who have voted for a validator who has been slashed since the nomination was
	/// submitted. Such nominations are NOT effective at all and need to be re-submitted.
	DanglingNominators {},
	/// The general checkup of a nominator.
	NominatorCheck {
		/// The nominator's address. Both hex and ss58 encoding are acceptable.
		#[structopt(long)]
		who: AccountId,
	},
	/// The general checkup of a validators.
	ValidatorCheck {
		/// The validator's address. Both hex and ss58 encoding are acceptable.
		#[structopt(long)]
		who: AccountId,
	},
}

/// Arguments that can be passed to the staking sub-command.
#[derive(Debug, StructOpt, Clone)]
pub struct StakingConfig {
	/// Count of member/validators to elect. Default is `Staking.validatorCount`.
	#[structopt(short, long)]
	count: Option<usize>,

	/// Json output file name. dumps the results into if given.
	#[structopt(parse(from_os_str))]
	output: Option<PathBuf>,

	/// Number of balancing rounds.
	#[structopt(short, long, default_value = "0")]
	iterations: usize,

	#[structopt(short, long, default_value = "128")]
	max_payouts: usize,

	/// If reduce is applied to the output.
	#[structopt(short, long, parse(from_flag))]
	reduce: bool,

	/// The override file to interpret
	#[structopt(short, long, parse(from_os_str))]
	manual_override: Option<PathBuf>,
}

/// Arguments that can be passed to the council sub-command.
#[derive(Debug, StructOpt, Clone)]
pub struct CouncilConfig {
	/// Count of member/validators to elect. Default is
	/// `ElectionsPhragmen.desired_members()` + `ElectionsPhragmen.desired_runners_up()`.
	#[structopt(short, long)]
	count: Option<usize>,

	/// Json output file name. dumps the results into if given.
	#[structopt(parse(from_os_str))]
	output: Option<PathBuf>,

	/// The override file to interpret
	#[structopt(short, long, parse(from_os_str))]
	manual_override: Option<PathBuf>,
}

#[async_std::main]
async fn main() -> () {
	env_logger::Builder::from_default_env().format_module_path(false).format_level(true).init();

	let mut opt = Opt::from_args();

	// connect to a node.
	let client = jsonrpsee_ws_client::WsClient::new(
		&opt.uri,
		jsonrpsee_ws_client::WsConfig {
			max_request_body_size: 1024 * 1024 * 1024, // 1GB..
			..Default::default()
		},
	)
	.await
	.unwrap();

	// get the latest block hash
	let head = storage::get_head(&client).await;

	// potentially replace head with the given hash
	let at = opt.at.unwrap_or(head);
	opt.at = Some(at);

	let runtime_version = sub_storage::get_runtime_version(&client, at).await;
	let spec_name = runtime_version.spec_name;
	let network_address = opt.clone().network.unwrap_or_else(|| spec_name.into());
	let address_format = match &network_address[..] {
		"polkadot" => Ss58AddressFormat::PolkadotAccount,
		"kusama" => Ss58AddressFormat::KusamaAccount,
		"substrate" => Ss58AddressFormat::SubstrateAccount,
		_ => panic!("Invalid network/address format."),
	};

	// setup address format and currency based on address format.
	set_default_ss58_version(address_format);
	if address_format.eq(&Ss58AddressFormat::PolkadotAccount) {
		sub_tokens::dynamic::set_name(&"DOT");
		sub_tokens::dynamic::set_decimal_points(10_000_000_000);
	} else if address_format.eq(&Ss58AddressFormat::KusamaAccount) {
		sub_tokens::dynamic::set_name(&"KSM");
		sub_tokens::dynamic::set_decimal_points(1000_000_000_000);
	}

	// set total issuance
	network::issuance::set(&client, at).await;

	log::info!(target: LOG_TARGET, "program args: {:?}", opt);
	log::info!(
		target: LOG_TARGET,
		"total_issuance = {:?}",
		Currency::from(network::issuance::get())
	);

	match opt.clone().cmd {
		SubCommands::Current { .. } => subcommands::current::run(&client, opt.clone()).await,
		SubCommands::Next { .. } => unimplemented!(),
		SubCommands::Staking(conf) => subcommands::staking::run(&client, opt.clone(), conf).await,
		SubCommands::Council(conf) => {
			subcommands::elections_phragmen::run(&client, opt.clone(), conf).await
		}
		SubCommands::DanglingNominators { .. } => {
			subcommands::dangling_nominators::run(&client, opt.clone()).await
		}
		SubCommands::CommandCenter { .. } => unimplemented!(),
		SubCommands::NominatorCheck { who } => {
			subcommands::nominator_check::run(&client, opt.clone(), who).await
		}
		SubCommands::ValidatorCheck { who } => {
			subcommands::validator_check::run(&client, opt.clone(), who).await
		}
	};
}

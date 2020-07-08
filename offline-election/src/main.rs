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

//! TODO

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

// whatever node you are connecting to. Polkadot, substrate etc.
pub use primitives::{AccountId, Balance, BlockNumber, Hash};

use atomic_refcell::AtomicRefCell as RefCell;
use jsonrpsee::Client;
pub use sc_rpc_api::state::StateClient;
use separator::Separatable;
use sp_core::crypto::{set_default_ss58_version, Ss58AddressFormat};
use std::path::PathBuf;
use std::{convert::TryInto, fmt};
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

/// Decimal points of the currency based on the network.
pub static DECIMAL_POINTS: RefCell<Balance> = RefCell::new(1_000_000_000_000);

/// Name of the currency token based on the network.
pub static TOKEN_NAME: RefCell<&'static str> = RefCell::new("KSM");

/// Wrapper to pretty-print currency token.
// TODO: move to another crate.
struct Currency(Balance);

impl fmt::Debug for Currency {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let num: u128 = self.0.try_into().unwrap();
		write!(
			f,
			"{},{:0>3}{} ({})",
			self.0 / *DECIMAL_POINTS.borrow(),
			self.0 % *DECIMAL_POINTS.borrow() / (*DECIMAL_POINTS.borrow() / 1000),
			*TOKEN_NAME.borrow(),
			num.separated_string()
		)
	}
}

impl fmt::Display for Currency {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let num: u128 = self.0.try_into().unwrap();
		write!(f, "{}", num.separated_string())
	}
}

/// A wrapper type for account id that can be parsed from the command line.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParsedAccountId(AccountId);

impl std::str::FromStr for ParsedAccountId {
	type Err = &'static str;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		use sp_core::crypto::Ss58Codec;
		// TODO: finish this: accept also hex string if this fails.
		<AccountId as Ss58Codec>::from_ss58check(s)
			.map_err(|_| "invalid ss58 address")
			.map(|acc| Self(acc))
	}
}

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
	#[structopt(short, long, default_value = "polkadot")]
	network: String,

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
		who: ParsedAccountId,
	},
	/// The general checkup of a validators.
	ValidatorCheck {
		/// The validator's address. Both hex and ss58 encoding are acceptable.
		#[structopt(long)]
		who: ParsedAccountId,
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

	/// If reduce is applied to the output.
	#[structopt(short, long, parse(from_flag))]
	reduce: bool,
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

	/// Number of balancing rounds.
	#[structopt(short, long, default_value = "0")]
	iterations: usize,
}

#[async_std::main]
async fn main() -> () {
	env_logger::builder()
		.filter_level(log::LevelFilter::Debug)
		.format_module_path(false)
		.format_level(true)
		.init();

	let mut opt = Opt::from_args();

	let address_format = match &opt.network[..] {
		"polkadot" => Ss58AddressFormat::PolkadotAccount,
		"kusama" => Ss58AddressFormat::KusamaAccount,
		"substrate" => Ss58AddressFormat::SubstrateAccount,
		_ => panic!("Invalid network/address format."),
	};

	// setup address format and currency based on address format.
	set_default_ss58_version(address_format);
	if address_format.eq(&Ss58AddressFormat::PolkadotAccount) {
		*TOKEN_NAME.borrow_mut() = "DOT";
	}

	// connect to a node.
	let transport = jsonrpsee::transport::ws::WsTransportClient::new(&opt.uri)
		.await
		.expect("Failed to connect to client");
	let client: Client = jsonrpsee::raw::RawClient::new(transport).into();

	// get the latest block hash
	let head = storage::get_head(&client).await;

	// potentially replace head with the given hash
	let at = opt.at.unwrap_or(head);
	opt.at = Some(at);

	// set total issuance
	network::issuance::set(&client, at).await;

	log::info!(target: LOG_TARGET, "program args: {:?}", opt);
	log::info!(
		target: LOG_TARGET,
		"total_issuance = {:?}",
		Currency(network::issuance::get())
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
			subcommands::nominator_check::run(&client, opt.clone(), who.0).await
		}
		SubCommands::ValidatorCheck { .. } => unimplemented!(),
	};
}

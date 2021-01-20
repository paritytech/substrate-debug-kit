use log::*;
use frame_support::traits::OnRuntimeUpgrade;
use node_runtime::AllModules;
use remote_externalities::TestExternalities;
use node_runtime::Runtime;

const LOG_TARGET: &'static str = "migration-dry-run";

type Migrations =
	(node_runtime::UpgradeSessionKeys, node_runtime::PhragmenElectionDepositRuntimeUpgrade);

struct Executive;

impl Executive {
	fn migrate<U: OnRuntimeUpgrade>(mut state: TestExternalities, tests: Box<dyn Fn() -> ()>) {
		state.execute_with(<frame_system::Module<Runtime> as OnRuntimeUpgrade>::on_runtime_upgrade);
		state.execute_with(U::on_runtime_upgrade);
		state.execute_with(<AllModules as OnRuntimeUpgrade>::on_runtime_upgrade);
		state.execute_with(tests);
	}
}

#[async_std::main]
async fn main() -> () {
	let _ = env_logger::Builder::from_default_env()
		.format_module_path(true)
		.format_level(true)
		.try_init();
	let client = sub_storage::create_ws_client(&"ws://localhost:9944").await;
	let now = sub_storage::get_head(&client).await;

	// TODO: nice to have: print all of the pallet versions in storage before and after the
	// migration.

	let state = remote_externalities::Builder::new()
		.at(now)
		.module("PhragmenElection")
		.module("ElectionsPhragmen")
		.module("Session")
		.build_async()
		.await;

	info!("executing on_runtime_upgrade at {}.", now,);
	Executive::migrate::<Migrations>(
		state,
		Box::new(|| {
			// probably verifications need to come from the runtime as well. This can be a
			// placeholder for any additional checks.
		}),
	)
}

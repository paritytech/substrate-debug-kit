use log::*;
use frame_support::traits::OnRuntimeUpgrade;
use remote_externalities::TestExternalities;
use node_runtime::{CustomMigrations, AllModules, System};

const LOG_TARGET: &'static str = "migration-dry-run";

/// Note that the order is important here.
type AllRuntimeMigrations = (System, CustomMigrations, AllModules);

struct Executive;

impl Executive {
	fn migrate(mut state: TestExternalities, tests: Box<dyn Fn() -> ()>) {
		info!(target: LOG_TARGET, "executing pre_migration");
		state.execute_with(<AllRuntimeMigrations as OnRuntimeUpgrade>::pre_migration).unwrap();

		info!(target: LOG_TARGET, "executing on_runtime_upgrade");
		let weight =
			state.execute_with(<AllRuntimeMigrations as OnRuntimeUpgrade>::on_runtime_upgrade);
		info!(target: LOG_TARGET, "migration weight = {}", weight);

		info!(target: LOG_TARGET, "executing post_migration");
		state.execute_with(<AllRuntimeMigrations as OnRuntimeUpgrade>::post_migration).unwrap();

		info!(target: LOG_TARGET, "running custom assertions");
		state.execute_with(tests);
	}
}

#[tokio::main]
async fn main() -> () {
	let _ = env_logger::Builder::from_default_env()
		.format_module_path(true)
		.format_level(true)
		.try_init();

	// TODO: nice to have: print all of the pallet versions in storage before and after the
	// migration.

	let state = remote_externalities::Builder::new()
		.cache_mode(remote_externalities::CacheMode::UseElseCreate)
		.cache_name(remote_externalities::CacheName::Forced(
			"Kusama,0x7f13b9c87b6ba0845ea69a4cde233f2e8979666e86fe62eeba4982da8133023c,.bin".into(),
		))
		.build()
		.await;

	Executive::migrate(
		state,
		Box::new(|| {
			// probably verifications need to come from the runtime as well. This can be a
			// placeholder for any additional checks.
		}),
	)
}

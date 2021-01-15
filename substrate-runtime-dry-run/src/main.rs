use codec::Encode;
use frame_support::{traits::Hooks, weights::GetDispatchInfo};
use jsonrpsee::client::Subscription;
use node_runtime::{CheckedExtrinsic, Runtime, UncheckedExtrinsic};
use sp_runtime::traits::{Applyable, Checkable};
use target_pallet::Pallet;

pub type BlockNumber = <Runtime as frame_system::Config>::BlockNumber;
pub type AcocuntId = <Runtime as frame_system::Config>::AccountId;

#[async_std::main]
async fn main() -> () {
	sp_tracing::try_init_simple();
	let client = sub_storage::create_ws_client(&"ws://localhost:9944").await;
	let mut sub: Subscription<node_runtime::Header> = client
		.subscribe(
			"chain_subscribeNewHeads",
			jsonrpsee::common::Params::None,
			"chain_unsubscribeNewHeads",
		)
		.await
		.unwrap();

	while let ev = sub.next().await {
		let hash = ev.hash();

		// get block
		let block = sub_storage::get_block::<node_runtime::SignedBlock>(&client, hash)
			.await
			.unwrap();
		let n = ev.number;

		// get the entire state
		remote_externalities::Builder::default()
			.at(hash)
			.module("System")
			.module("Session")
			.module("Babe")
			.module("Staking")
			.module("TwoPhaseElectionProvider")
			.build()
			.execute_with(|| {
				let now = BlockNumber::from(n);

				// initialize.
				<Pallet<Runtime> as Hooks<BlockNumber>>::on_initialize(now);

				// read all related transactions and dispatch them
				for extrinsic in block.block.extrinsics {
					match extrinsic.function {
						node_runtime::Call::TwoPhaseElectionProvider(ref inner) => {
							let checked = <UncheckedExtrinsic as Checkable<
								frame_system::ChainContext<Runtime>,
							>>::check(extrinsic.clone(), &Default::default())
							.unwrap();
							let info = checked.get_dispatch_info();
							let len = extrinsic.using_encoded(|e| e.len());
							let result = <CheckedExtrinsic as Applyable>::apply::<Runtime>(
								checked, &info, len,
							);
						}
						_ => {
							println!("Ignoring transaction {:?}", extrinsic);
						}
					}
				}

				// finalize.
				<Pallet<Runtime> as Hooks<BlockNumber>>::on_finalize(now);
			});
	}
}

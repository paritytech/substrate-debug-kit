use crate::mock_runtime::{offchainify, Runtime, Staking, Timestamp};
use codec::Encode;
use frame_support::traits::OffchainWorker;
use frame_support::{
	storage::StorageValue, traits::UnfilteredDispatchable, weights::GetDispatchInfo,
};
use pallet_staking::{
	offchain_election::prepare_submission, ElectionStatus, EraElectionStatus, OffchainAccuracy,
	WeightInfo,
};
use separator::Separatable;
use sp_npos_elections::ElectionResult;
use sub_storage::{Client, Hash};

/// Main function of this command.
pub async fn run(_client: &Client, at: Hash) {
	submit_at(at).await;
}

pub async fn submit_at(at: Hash) {
	remote_externalities::Builder::new()
		.module("Staking")
		.at(at)
		.build_async()
		.await
		.execute_with(|| {
			let queued_score = Staking::queued_score();
			log::info!("queued_score = {:?}", queued_score);
			log::info!("now = {:?}", Timestamp::now());

			// compute raw solution. Note that we use `OffchainAccuracy`.
			let ElectionResult {
				winners,
				assignments,
			} = Staking::do_phragmen::<OffchainAccuracy>(10).unwrap();

			// Create the snapshot at any point.
			let closed = Staking::era_election_status().is_closed();
			if closed {
				log::warn!("Election window is closed. This will not be submitted.");
				Staking::create_stakers_snapshot();
				<EraElectionStatus<Runtime>>::put(ElectionStatus::Open(999));
			}

			// process and prepare it for submission.
			let (winners, compact, score, size) = prepare_submission::<Runtime>(
				assignments,
				winners,
				true,
				<Runtime as pallet_staking::Trait>::OffchainSolutionWeightLimit::get(),
			)
			.unwrap();

			let weight = <Runtime as pallet_staking::Trait>::WeightInfo::submit_solution_better(
				size.validators.into(),
				size.nominators,
				compact.len() as u32,
				winners.len() as u32,
			);
			assert!(
				weight <= <Runtime as pallet_staking::Trait>::OffchainSolutionWeightLimit::get()
			);

			let era = Staking::current_era().unwrap_or_default();

			let inner_call = pallet_staking::Call::<Runtime>::submit_election_solution(
				winners.clone(),
				compact.clone(),
				score,
				era,
				size,
			);

			let len = inner_call.encode().len();
			let info = inner_call.get_dispatch_info();

			log::info!(
				"prepared a seq-phragmen solution with {} balancing iterations and score {:?} and weight = {:?} and len = {}",
				10,
				score.iter().map(|x| x.separated_string()).collect::<Vec<_>>(),
				weight.separated_string(),
				len,
			);

			let pre_dispatch = frame_system::CheckWeight::<Runtime>::do_pre_dispatch(&info, len);
			let validate = frame_system::CheckWeight::<Runtime>::do_validate(&info, len);
			log::info!(
				"Outcome of do_pre_dispatch: {:?} | validate = {:?}",
				pre_dispatch,
				validate
			);
			assert!(pre_dispatch.is_ok() && validate.is_ok());

			let outcome = inner_call
				.dispatch_bypass_filter(crate::mock_runtime::Origin::signed(Default::default()));
			log::info!("Outcome of dispatch: {:?}", outcome);

			let solution_file = format!("era{}_solution", era);
			std::fs::write(
				solution_file.clone(),
				(winners, compact, score, era, size).encode(),
			)
			.unwrap();

			if !closed {
				if clt::confirm("Submit the solution?", false, "n", true) {
					let output = std::process::Command::new("node")
						.args(&["js/build/index.js", &solution_file])
						.output()
						.unwrap();

					log::info!("Exit code of js script = {}", output.status);
					println!("STDOUT\n{}", String::from_utf8_lossy(&output.stdout));
					println!("STDERR\n{}", String::from_utf8_lossy(&output.stderr));
				}
			}
		});
}

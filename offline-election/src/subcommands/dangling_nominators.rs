use crate::{
	primitives::{AccountId, Hash},
	storage,
	subcommands::staking::slashing_span_of,
	Client, Opt, LOG_TARGET,
};
use pallet_staking::Nominations;

/// Check if a vote submitted at the given era for this target is dangling or not.
pub async fn is_dangling(
	target: &AccountId,
	submitted_in: pallet_staking::EraIndex,
	client: &Client,
	at: Hash,
) -> bool {
	let maybe_slashing_spans = slashing_span_of(&target, client, at).await;
	!maybe_slashing_spans.map_or(true, |spans| {
		println!("spans.last_nonzero_slash() = {:?}", spans.last_nonzero_slash());
		submitted_in >= spans.last_nonzero_slash()
	})
}

/// Main run function of the sub-command.
pub async fn run(client: &Client, opt: Opt) {
	let at = opt.at.unwrap();
	let nominators: Vec<(AccountId, Nominations<AccountId>)> =
		storage::enumerate_map::<AccountId, Nominations<AccountId>>(
			b"Staking",
			b"Nominators",
			client,
			at,
		)
		.await
		.expect("Staking::nominators should be enumerable");

	let count = nominators.len();
	let mut ok = 0;
	let mut nok = 0;
	for (idx, (who, n)) in nominators.into_iter().enumerate() {
		// retain only targets who have not been yet slashed recently. This is highly dependent
		// on the staking implementation.
		let submitted_in = n.submitted_in;
		let targets = n.targets;
		let mut filtered_targets = vec![];
		// TODO: move back to closures and retain, but async-std::block_on can't work well here for
		// whatever reason. Or move to streams?
		for target in targets.iter() {
			if !is_dangling(target, submitted_in, client, at).await {
				filtered_targets.push(target.clone());
			}
		}

		if filtered_targets.len() == targets.len() {
			log::debug!(
				target: LOG_TARGET,
				"[{}/{}] Nominator {:?} Ok. Retaining all {} votes.",
				idx,
				count,
				who,
				targets.len()
			);
			ok += 1;
		} else {
			log::warn!(
				target: LOG_TARGET,
				"[{}/{}] Retaining {}/{} of votes for {:?}.",
				idx,
				count,
				filtered_targets.len(),
				targets.len(),
				who
			);
			nok += 1;
		}
	}

	log::info!(target: LOG_TARGET, "✅ {} nominators have effective votes.", ok);
	log::info!(target: LOG_TARGET, "❌ {} nominators have dangling votes.", nok);
}

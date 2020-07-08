use crate::subcommands::staking::slashing_span_of;
use crate::{primitives::AccountId, storage, Client, Opt, LOG_TARGET};
use pallet_staking::Nominations;

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
	nominators
		.into_iter()
		.enumerate()
		.for_each(|(idx, (who, n))| {
			let submitted_in = n.submitted_in;
			let initial_len = n.targets.len();
			let mut targets = n.targets;
			targets.retain(|target| {
				let maybe_slashing_spans =
					async_std::task::block_on(slashing_span_of(&target, client, at));
				maybe_slashing_spans
					.map_or(true, |spans| submitted_in >= spans.last_nonzero_slash())
			});
			if initial_len == targets.len() {
				log::debug!(
					target: LOG_TARGET,
					"[{}/{}] Nominator {:?} Ok. Retaining all {} votes.",
					idx,
					count,
					who,
					initial_len
				);
				ok += 1;
			} else {
				log::warn!(
					target: LOG_TARGET,
					"[{}/{}] Retaining {}/{} of votes for {:?}.",
					idx,
					count,
					targets.len(),
					initial_len,
					who
				);
				nok += 1;
			}
		});
	log::info!(
		target: LOG_TARGET,
		"✅ {} nominators have effective votes.",
		ok
	);
	log::info!(
		target: LOG_TARGET,
		"❌ {} nominators have dangling votes.",
		nok
	);
}

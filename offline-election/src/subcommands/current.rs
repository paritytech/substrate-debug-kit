use crate::{primitives::Balance, Client, Currency, Opt, LOG_TARGET};
use sp_runtime::traits::Bounded;
use sub_storage::{get_head, helpers::*};

/// Main run function of the sub-command.
pub async fn run(client: &Client, config: Opt) {
	let at = config.at.unwrap();
	let (era, validators_and_expo) = crate::network::get_validators_and_expo_at(&client, at).await;
	log::info!(target: LOG_TARGET, "working on era {:?}", era);

	let mut min_stake: Balance = Bounded::max_value();
	for (i, (v, expo)) in validators_and_expo.into_iter().enumerate() {
		println!(
			"#{} [{}] [total: {:?} / others: {:?} / count: {}]- {:?}",
			i + 1,
			get_identity(&v, client, at).await,
			Currency(expo.total),
			Currency(expo.others.iter().map(|indie| indie.value).sum::<Balance>()),
			expo.others.len(),
			v
		);

		if expo.total < min_stake {
			min_stake = expo.total;
		}
	}

	log::info!(
		target: LOG_TARGET,
		"min-staker (score[0]) is {:?}",
		Currency(min_stake)
	);
}

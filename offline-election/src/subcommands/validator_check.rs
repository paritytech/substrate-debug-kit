use crate::{primitives::AccountId, Client, Opt};
use pallet_staking::Nominations;

/// Main run function of the sub-command.
pub async fn run(client: &Client, opt: Opt, who: AccountId) {
	let at = opt.at.unwrap();

	let nominators: Vec<(AccountId, Nominations<AccountId>)> =
		sub_storage::enumerate_map::<AccountId, Nominations<AccountId>>(
			b"Staking",
			b"Nominators",
			client,
			at,
		)
		.await
		.expect("Staking::nominators should be enumerable");

	let my_nominators = nominators
		.into_iter()
		.filter_map(|(w, n)| {
			if n.targets.contains(&who) {
				Some((w, n.submitted_in))
			} else {
				None
			}
		})
		.collect::<Vec<_>>();

	let era = crate::subcommands::staking::get_current_era(client, at).await;
	let exposure = crate::subcommands::staking::exposure_of(&who, era, client, at).await;

	for (n, submitted_in) in my_nominators {
		let is_exposed = exposure
			.others
			.iter()
			.find(|ie| ie.who == n)
			.map(|ie| ie.value);
		let is_dangling =
			crate::subcommands::dangling_nominators::is_dangling(&n, submitted_in, client, at)
				.await;
		println!(
			"Voted from [{:?}] || dangling: {} || exposed: {}",
			n,
			if is_dangling { "❌ Yes" } else { "✅ No" },
			if let Some(val) = is_exposed {
				format!("💰 by {:?}", crate::Currency(val))
			} else {
				"∅".into()
			},
		)
	}
}

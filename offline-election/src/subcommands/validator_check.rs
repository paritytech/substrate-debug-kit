use crate::{primitives::AccountId, subcommands, Client, Currency, Opt};
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
		.filter_map(
			|(w, n)| {
				if n.targets.contains(&who) {
					Some((w, n.submitted_in))
				} else {
					None
				}
			},
		)
		.collect::<Vec<_>>();

	let era = subcommands::staking::get_current_era(client, at).await;
	let exposure = subcommands::staking::exposure_of(&who, era, client, at).await;

	for (n, submitted_in) in my_nominators {
		let is_exposed = exposure.others.iter().find(|ie| ie.who == n).map(|ie| ie.value);
		let is_dangling =
			subcommands::dangling_nominators::is_dangling(&who, submitted_in, client, at).await;
		println!(
			"\t Voted from [{:?}] || dangling: {} || exposed: {}",
			n,
			if is_dangling {
				format!("❌ Yes, submitted in era {}", submitted_in)
			} else {
				"✅ No".into()
			},
			if let Some(val) = is_exposed {
				format!("💰 by {:?}", Currency::from(val))
			} else {
				"∅".into()
			},
		)
	}

	println!("🤑 Total stake = {:?}", Currency::from(exposure.total));
	let maybe_slashing_spans = subcommands::staking::slashing_span_of(&who, client, at).await;
	if let Some(spans) = maybe_slashing_spans {
		println!("⚠️  Last non-zero slash happened at {}", spans.last_nonzero_slash());
		println!("💭g Raw Slashing spans = {:?}", spans);
	} else {
		println!("✅ This validator has no slashing spans.");
	}
	println!("💭 Raw Exposure = {:?}", exposure);
}

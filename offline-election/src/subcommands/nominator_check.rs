use crate::{
	primitives::{AccountId, Balance},
	Client, Opt, LOG_TARGET,
};
use sub_storage::*;

/// Main run function of the sub-command.
pub async fn run(client: &Client, opt: Opt, who: AccountId) {
	let at = opt.at.unwrap();
	let maybe_nomination = read::<pallet_staking::Nominations<AccountId>>(
		map_key::<frame_support::Twox64Concat>(b"Staking", b"Nominators", who.as_ref()),
		&client,
		at,
	)
	.await;

	if maybe_nomination.is_none() {
		log::warn!("{:?} is not a nominator.", who);
		return;
	}

	let nomination = maybe_nomination.expect("Already checked to be some; qed");
	let (era, validators_and_expo) = crate::network::get_validators_and_expo_at(client, at).await;
	log::info!(target: LOG_TARGET, "working on era {:?}", era);
	let mut active_edges = vec![];

	for (v, e) in validators_and_expo.iter() {
		let mut sorted = e.others.clone();
		sorted.sort_by_key(|e| e.value);
		sorted.reverse();

		if let Some(index) = sorted.iter().position(|indie| indie.who == who) {
			active_edges.push((v.clone(), sorted[index].value, index));
			log::debug!(target: LOG_TARGET, "sorted exposure of {:?}", v);
			sorted.iter().for_each(|e| {
				log::debug!(target: LOG_TARGET, "{:?}", e);
			});
		}
	}

	println!("📅 Submitted in era {}", nomination.submitted_in);
	println!("📣 Votes:");
	for t in nomination.targets.iter() {
		let ident = helpers::get_identity::<AccountId, Balance>(t.as_ref(), client, at).await;
		if let Some(active) = active_edges.iter().find(|e| e.0 == *t) {
			let val = crate::Currency(active.1);
			let index = active.2;
			println!(
				"\t✅ Active {:?} ({}) / value: {:?} / index: {:?}",
				t, ident, val, index
			);
			if index > 64 {
				log::warn!("This nomination cannot claim its rewards.");
			}
		} else {
			println!("\t❌ Inactive {:?} ({})", t, ident);
		}
	}
}

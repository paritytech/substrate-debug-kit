import { ApiPromise } from "@polkadot/api";
import { AccountInfo } from "@polkadot/types/interfaces/system/types";
import BN from "bn.js";
import { SSL_OP_CISCO_ANYCONNECT, X_OK } from "constants";
import request, { head } from "request-promise";


async function recordedReserved(whos: string[], api: ApiPromise): Promise<[string, Map<string, BN>, BN, BN][]> {
	let democracyDepositsOf = await api.query.democracy.depositOf.entries();
	let democracyPreImages = await api.query.democracy.preimages.entries();

	let electionsMembers = await api.query.electionsPhragmen.members();
	let electionsRunnersUp = await api.query.electionsPhragmen.runnersUp();
	let electionsCandidates = await api.query.electionsPhragmen.candidates();

	let indicesAccounts = await api.query.indices.accounts.entries();

	let multisigs = await api.query.multisig.multisigs.entries();
	let multisigCalls = await api.query.multisig.calls.entries();

	let treasuryProposals = await api.query.treasury.proposals.entries();
	let treasuryTips = await api.query.treasury.tips.entries();
	let treasuryBounties = await api.query.treasury.bounties.entries();

	let good = 0;
	let bad = 0;

	let result: [string, Map<string, BN>, BN, BN][] = []
	for (let who of whos) {
		let deposits: Map<string, BN> = new Map();

		// democracy: PreImage, DepositsOf,
		democracyDepositsOf.forEach(([_, maybeDepositOf]) => {
			let depositOf = maybeDepositOf.unwrapOrDefault();
			let [backers, deposit] = depositOf;
			if (backers.find((x) => x.toHuman() == who) != undefined) {
				deposits.set("democracy.depositOf", deposit);
			}
		});
		democracyPreImages.forEach(([_, maybePreImage]) => {
			let perImage = maybePreImage.unwrapOrDefault();
			if (perImage.asAvailable.provider.toHuman() == who) {
				deposits.set("democracy.preImages", perImage.asAvailable.deposit);
			}
		});

		// elections-phragmen: Voter, Candidate
		//@ts-ignore
		let voting = (await api.query.electionsPhragmen.voting(who))[1].length > 0 ? api.consts.electionsPhragmen.votingBond : new BN(0)
		//@ts-ignore
		let is_member = electionsMembers.find(([m, _]) => m.toHuman() == who) != undefined
		//@ts-ignore
		let is_runner_up = electionsRunnersUp.find(([m, _]) => m.toHuman() == who) != undefined
		//@ts-ignore
		let is_candidate = electionsCandidates.find((c) => c.toHuman() == who) != undefined
		//@ts-ignore
		let candidacy = is_member || is_runner_up || is_candidate ? api.consts.electionsPhragmen.candidacyBond : new BN(0)
		//@ts-ignore
		deposits.set("elections-phragmen.voter", voting)
		//@ts-ignore
		deposits.set("elections-phragmen.candidacy", candidacy)

		// identity
		let identity = (await api.query.identity.identityOf(who)).unwrapOrDefault();
		deposits.set("identity.deposit", identity.deposit)
		identity.judgements.forEach(([_, j]) => {
			if (j.isFeePaid) {
				deposits.set("identity.judgments", j.asFeePaid)
			}
		});

		// indices
		indicesAccounts.forEach(([k, maybeIndex]) => {
			let [acc, dep, frozen] = maybeIndex.unwrapOrDefault();
			if (acc.toHuman() == who && frozen.isFalse) {
				deposits.set(`indices${k.toHuman()}`, dep);
			}
		});

		// multisig: Multisigs, Calls
		multisigs.forEach(([_, maybeMulti]) => {
			let multi = maybeMulti.unwrapOrDefault()
			if (multi.depositor.toHuman() == who) {
				deposits.set("multisig.multisig", multi.deposit);
			}
		});
		multisigCalls.forEach(([_, maybeCall]) => {
			let [__ , depositor, deposit] = maybeCall.unwrapOrDefault()
			if (depositor.toHuman() == who) {
				deposits.set("multisig.call", deposit);
			}
		});

		// proxy: Proxies, Anonymous(TODO), announcements
		let proxies = (await api.query.proxy.proxies(who))[1];
		deposits.set("proxy.proxies", proxies);
		let announcements = (await api.query.proxy.announcements(who))[1];
		deposits.set("proxy.announcements", announcements);

		// treasury: Proposals, Tips, Curator/Bounties
		treasuryProposals.forEach(([_, maybeProp]) => {
			let prop = maybeProp.unwrapOrDefault();
			if (prop.proposer.toHuman() == who) {
				deposits.set("treasury.proposals", prop.value)
			}
		});
		treasuryTips.forEach(([_, maybeTip]) => {
			let tip = maybeTip.unwrapOrDefault();
			if (tip.who.toHuman() == who) {
				deposits.set("treasury.tip", tip.deposit)
			}
		});
		treasuryBounties.forEach(([_, maybeBounty]) => {
			let bounty = maybeBounty.unwrapOrDefault();
			// Bounty is not funded yet, so there is still a deposit for proposer.
			if (bounty.status.isProposed || bounty.status.isFunded) {
				if (bounty.proposer.toHuman() == who) {
					deposits.set("treasury.bounty.proposer", bounty.bond)
				}
			} else {
				// Curator has a deposit.
				if (!bounty.curatorDeposit.isZero()) {
					//@ts-ignore
					if (bounty.status.value && bounty.status.value.curator) {
						//@ts-ignore
						if (bounty.status.value.curator.toHuman() == who) {
							deposits.set("treasury.bounty.curator", bounty.curatorDeposit)
						}
					}
				}
			}
		})

		let sum = new BN(0);
		for (let [_k, v] of deposits.entries()) {
			sum = sum.add(v)
		}

		let chain = (await api.query.system.account(who)).data.reserved
		let match = chain.eq(sum)
		match ? good++ : bad++;
		console.log(`${match? "✅" : "❌"} - ${who} on-chain reserved = ${chain.toHuman()} module-sum = ${sum}`)
		if (!match) {
			console.log(deposits)
		}
		result.push([who, deposits, sum, chain.toBn()])
	}

	console.log(good, bad)
	return result
}

async function checkAllAccounts(api: ApiPromise) {
	let all_accounts: string[] = (await api.query.system.account.entries()).map(([acc, _]) => {
		//@ts-ignore
		return acc.toHuman()[0]
	});
	console.log(`fetched ${all_accounts.length} accounts`)
	await recordedReserved(all_accounts, api)
}

(async () => {
	const api = await ApiPromise.create();
	// 126GgFcnMtV4upzmk2Qtupxbmrh6yo99W1WxYzMAoF7DGxpz has 2 indices.
	// console.log(await recordedReserved("15kZqsp5RR3wBVbgLPsBXbatf1YJA9cak46znnMbJviwd4En", api))
	await checkAllAccounts(api)
	return;

	let res = await request.get("https://explorer-31.polkascan.io/polkadot/api/v1/event?filter[module_id]=electionsphragmen&filter[event_id]=NewTerm&page[number]=1&page[size]=100")
	res = JSON.parse(res)
	let data = res.data

	let blocks = []
	for (let e of data) {
		let has_new_term = false;
		let deposits = []
		let block_id = e.attributes.block_id
		try {
			let block_data_raw = await request.get(`https://explorer-31.polkascan.io/polkadot/api/v1/block/${block_id}?include=transactions,inherents,events,logs`)
			let block_data = JSON.parse(block_data_raw)
			let hash = block_data.data.attributes.hash
			let events = await api.query.system.events.at(hash)

			for (let ev of events) {
				if (ev.event.meta.name.toHuman() == "NewTerm") {
					has_new_term = true
				}
				if (ev.event.meta.name.toHuman() == "Deposit") {
					deposits.push(ev.event.data[0])
				}
			}
			console.log(`("${hash.slice(2)}", vec![${deposits}], "${block_data.data.attributes.datetime}"),`)
		} catch (e) {
			console.log("Erro for", block_id, e)
		}
	}

	console.log("Done");



})()

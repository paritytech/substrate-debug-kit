import { ApiPromise, WsProvider } from "@polkadot/api";
import BN from "bn.js";
import { StorageKey } from "@polkadot/types/primitive";
import { ITuple } from "@polkadot/types/types";
import { Vec } from "@polkadot/types/codec";
import { BalanceOf, ProxyDefinition, AccountId, BlockHash, } from "@polkadot/types/interfaces/";
import request, { head, post } from "request-promise";
import { assert } from "console";
import { writeFileSync, readFileSync, unlinkSync } from 'fs'
import { blake2AsHex, xxhashAsHex, } from "@polkadot/util-crypto";
import { TextEncoder } from "util";
import { Keyring } from '@polkadot/api';

async function submitPreImage(api: ApiPromise, preImage: string, dryRun: boolean) {
	const keyring = new Keyring({ type: 'sr25519' });
	const SENDER = keyring.addFromUri('//Alice')

	if(dryRun) {
		const tx = await api.tx.democracy.notePreimage(preImage).signAsync(SENDER);
		const info = await api.rpc.payment.queryInfo(tx.toHex());
		const dryRun = await api.rpc.system.dryRun(tx.toHex())
		console.log(info.toHuman());
		console.log(dryRun.toHuman());
	} else {
		const _ = api.tx.democracy.notePreimage(preImage).signAndSend(SENDER, (result) => {
			console.log(`Current status is ${result.status}`);
			if (result.status.isInBlock) {
				console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
			} else if (result.status.isFinalized) {
				console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
			}
		})
	}
}

async function recordedReserved(whos: string[], api: ApiPromise): Promise<[string, Array<[string, BN]>, BN, BN][]> {
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

	// let all_proxy_keys = await api.query.proxy.proxies.keys();
	// let all_proxies: Map<AccountId, ITuple<[Vec<ProxyDefinition>, BalanceOf]>> = new Map();
	// for (let k of all_proxy_keys) {
	// 	try {
	// 		let key = k.toU8a().slice(-32)
	// 		let acc = api.createType('AccountId', k.toU8a().slice(-32))
	// 		let proxy = await api.query.proxy.proxies(key)
	// 		all_proxies.set(acc, proxy)
	// 	} catch (e) {
	// 		console.error("failed to get proxy for", k.toHuman());
	// 	}
	// }

	// let proxyDeposits = []
	// for (let [k, [proxies, dep]] of all_proxies) {
	// 	let delegator_anonymous = k.toHuman()
	// 	let nonce = (await api.query.system.account(delegator_anonymous)).nonce;
	// 	if (nonce.isZero()) {
	// 		if (proxies.length == 1 && !(await api.query.system.account(proxies[0].delegate)).nonce.isZero()) {
	// 			console.log("probably anonymous", delegator_anonymous, proxies.toHuman())
	// 		} else {
	// 			console.log("WTFFF", delegator_anonymous, proxies.toHuman())
	// 		}
	// 	}
	// }

	let good = 0;
	let bad = 0;

	let result: [string, Array<[string, BN]>, BN, BN][] = []
	for (let who of whos) {
		let deposits: Array<[string, BN]> = [];

		// democracy: PreImage, DepositsOf,
		democracyDepositsOf.forEach(([prop, maybeDepositOf]) => {
			let [backers, deposit] = maybeDepositOf.unwrapOrDefault();
			// allow backing multiple times.
			for (let b of backers) {
				if (b.toHuman() == who) {
					deposits.push([`democracy.depositOf-${prop.toHuman()}`, deposit]);
				}
			}
		});
		democracyPreImages.forEach(([_, maybePreImage]) => {
			let perImage = maybePreImage.unwrapOrDefault();
			if (perImage.asAvailable.provider.toHuman() == who) {
				deposits.push(["democracy.preImages", perImage.asAvailable.deposit]);
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
		deposits.push(["elections-phragmen.voter", voting])
		//@ts-ignore
		deposits.push(["elections-phragmen.candidacy", candidacy])

		// identity
		let identity = (await api.query.identity.identityOf(who)).unwrapOrDefault();
		deposits.push(["identity.deposit", identity.deposit]);
		let subs = (await api.query.identity.subsOf(who))[0];
		deposits.push(["identity.subs", subs]);
		identity.judgements.forEach(([_, j]) => {
			if (j.isFeePaid) {
				deposits.push(["identity.judgments", j.asFeePaid])
			}
		});

		// indices
		indicesAccounts.forEach(([k, maybeIndex]) => {
			let [acc, dep, frozen] = maybeIndex.unwrapOrDefault();
			if (acc.toHuman() == who && frozen.isFalse) {
				deposits.push([`indices${k.toHuman()}`, dep]);
			}
		});

		// multisig: Multisigs, Calls
		multisigs.forEach(([_, maybeMulti]) => {
			let multi = maybeMulti.unwrapOrDefault()
			if (multi.depositor.toHuman() == who) {
				deposits.push(["multisig.multisig", multi.deposit]);
			}
		});
		multisigCalls.forEach(([_, maybeCall]) => {
			let [__ , depositor, deposit] = maybeCall.unwrapOrDefault()
			if (depositor.toHuman() == who) {
				deposits.push(["multisig.call", deposit]);
			}
		});

		// proxy: Proxies, Anonymous(TODO), announcements
		try {
			let nonce = (await api.query.system.account(who)).nonce;
			// direct.
			let deposit = (await api.query.proxy.proxies(who))[1];
			deposits.push(["proxy.proxies[direct]", deposit]);
		} catch (e) {
			console.error("ERROR while fetching proxy:", e, who)
		}
		let announcements = (await api.query.proxy.announcements(who))[1];
		deposits.push(["proxy.announcements", announcements]);

		// treasury: Proposals, Tips, Curator/Bounties
		treasuryProposals.forEach(([_, maybeProp]) => {
			let prop = maybeProp.unwrapOrDefault();
			if (prop.proposer.toHuman() == who) {
				deposits.push(["treasury.proposals", prop.bond])
			}
		});
		treasuryTips.forEach(([_, maybeTip]) => {
			let tip = maybeTip.unwrapOrDefault();
			if (tip.finder.toHuman() == who) {
				deposits.push(["treasury.tip", tip.deposit])
			}
		});
		treasuryBounties.forEach(([_, maybeBounty]) => {
			let bounty = maybeBounty.unwrapOrDefault();
			// Bounty is not funded yet, so there is still a deposit for proposer.
			if (bounty.status.isProposed || bounty.status.isFunded) {
				if (bounty.proposer.toHuman() == who) {
					deposits.push(["treasury.bounty.proposer", bounty.bond])
				}
			} else {
				// Curator has a deposit.
				if (!bounty.curatorDeposit.isZero()) {
					//@ts-ignore
					if (bounty.status.value && bounty.status.value.curator) {
						//@ts-ignore
						if (bounty.status.value.curator.toHuman() == who) {
							deposits.push(["treasury.bounty.curator", bounty.curatorDeposit])
						}
					}
				}
			}
		})

		let sum = new BN(0);
		for (let [_k, v] of deposits) {
			sum = sum.add(v)
		}

		let accountData = (await api.query.system.account(who))
		let match = accountData.data.reserved.eq(sum)
		match ? good++ : bad++;
		console.log(`${match? "✅" : "❌"} - ${who} on-chain reserved = ${accountData.data.reserved.toHuman()} (${accountData.data.reserved.toBn()}) // module-sum = ${api.createType('Balance', sum).toHuman()} (${sum})`)
		if (!match) {
			if (accountData.nonce.isZero()) {
				console.log("⚠️  Nonce zero. This is probably a multisig account.")
			}
			console.log(accountData.toHuman())
			deposits.forEach(([m, d]) => console.log(`+ ${m} => ${api.createType('Balance', d).toHuman()}`))
		}
		result.push([who, deposits, sum, accountData.data.reserved.toBn()])
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

async function parseCSV(api: ApiPromise, slashMap: Map<string, BN>) {
	let keys = Array.from(slashMap.keys())
	let stuff = await recordedReserved(keys, api)
	console.log("who,role,should_reserve,has_reserve,missing,effective_slash,trivial,reserved refund,free refund")
	for (let [who, _, should_reserve, has_reserve] of stuff) {
		let effectiveSlash = slashMap.get(who)
		let missing = should_reserve.sub(has_reserve)
		let isTrivial = effectiveSlash?.eq(missing) ? '✅' : '❌'
		let reservedRefund = missing
		let freeRefund = effectiveSlash?.sub(missing)
		let role = await getCurrentRole(who, api)
		console.log(`${who},${role},${should_reserve},${has_reserve},${missing},${effectiveSlash},${isTrivial},${reservedRefund},${freeRefund}`)
	}
}

async function parseCSVSimple(api: ApiPromise, slashMap: Map<string, BN>) {
	console.log("who,role,identity,effective_slash_planck,effective_slash_token")
	for (let [who, effectiveSlash] of slashMap) {
		let role = await getCurrentRole(who, api)
		let identity = (await api.query.identity.identityOf(who)).unwrapOrDefault().info.display.asRaw.toHuman()
		console.log(`${who},${role},${identity?.toString()},${effectiveSlash},${api.createType('Balance', effectiveSlash).toHuman()}`)
	}
}

interface Unreserved {
	amount: BN,
	who: string,
}

interface ElectionBlock {
	time: string,
	at: BlockHash,
	deposits: BN[],
	unreserve: Unreserved[],
}

interface Slash {
	who: string,
	at: BlockHash,
	amount: BN,
}

async function findElections(api: ApiPromise, chain: string): Promise<ElectionBlock[]> {
	let page = 1;
	let data: any[] = [];
	let pre_len = data.length
	while (true) {
		console.log(`fetching page ${page}`)
		let more = JSON.parse(await request.get(`https://explorer-31.polkascan.io/${chain}/api/v1/event?filter[module_id]=electionsphragmen&filter[event_id]=NewTerm&page[number]=${page}&page[size]=100`)).data
		data = data.concat(more)
		if (data.length > pre_len) {
			page++
			pre_len = data.length
		} else {
			break
		}
	}
	console.log(`Collected ${data.length} election events.`)

	let out: ElectionBlock[] = []
	for (let e of data) {
		let has_new_term = false;
		let newTermIndex = 0;
		let deposits = []
		let unreserve = []
		let block_id = e.attributes.block_id
		try {
			let block_data_raw = await request.get(`https://explorer-31.polkascan.io/${chain}/api/v1/block/${block_id}?include=transactions,inherents,events,logs`)
			let block_data = JSON.parse(block_data_raw)
			let at = block_data.data.attributes.hash
			let events = await api.query.system.events.at(at)

			let index = 0
			for (let ev of events) {
				if (ev.event.meta.name.toHuman() == "NewTerm") {
					has_new_term = true
					newTermIndex = index
				}
				index ++
			}

			index = 0
			for (let ev of events) {
				if (
					index <= newTermIndex &&
					// all of the events from this index to newTerm must be deposits
					events.toArray().slice(index, newTermIndex).map(e => e.event.meta.name.toHuman() == "Deposit").indexOf(false) == -1 &&
					ev.event.section == "treasury" &&
					ev.event.meta.name.toHuman() == "Deposit" &&
					(
						ev.phase.isInitialization ||
						(ev.phase.isApplyExtrinsic && ev.phase.asApplyExtrinsic.isZero())
					)
				) {
					deposits.push(new BN(ev.event.data[0].toString()))
				}

				if (ev.event.meta.name.toHuman() == "Unreserved") {
					unreserve.push({ who: ev.event.data[0].toString(), amount: new BN(ev.event.data[1].toString())})
				}
				index ++
			}

			// if we have had no unreserve events, then a bunch of other events should be counted as unreserve
			if (unreserve.length == 0) {
				for (let ev of events) {
					if (ev.event.meta.name.toHuman() == "Tabled") {
						let [_, deposit, depositors] = ev.event.data
						// @ts-ignore
						for (let d of depositors) {
							unreserve.push({ who: d.toHuman(), amount: new BN(deposit.toString()) })
						}
					}
					if (ev.event.meta.name.toHuman() == "PreimageUsed") {
						let [_, depositor, amount] = ev.event.data
						unreserve.push({ who: depositor.toHuman(), amount: new BN(amount.toString()) })
					}
					if (ev.event.meta.name.toHuman() == "Inducted") {
						let [_, new_members] = ev.event.data;
						// @ts-ignore
						for (let m of new_members) {
							unreserve.push({who: m.toHuman(), amount: api.consts.society.candidateDeposit})
						}
					}
				}
			}

			if (!has_new_term) {
				console.log("Something went wrong.")
				process.exit(0)
			}

			out.push( { at, deposits, time: block_data.data.attributes.datetime, unreserve })
			console.log(at, deposits.length, unreserve.length);
		} catch (e) {
			console.log("Error at", block_id, e)
		}
	}

	return out
}

function parseElections(input: ElectionBlock[]) {
	console.log("vec![");
	for (let e of input) {
		console.log(`("${e.at.slice(2)}", vec![${e.deposits}], "${e.time}",),`)
	}
	console.log("]")
}

function findCorrectSlash(preMembers: string[], postMember: string[], preRunnersUp: string[], postRunnersUp: string[]): string[] {
	let outgoing: string[] = [];

	preMembers.forEach((m) => {
		if (postMember.indexOf(m) == -1 && postRunnersUp.indexOf(m) == -1) {
			outgoing.push(m)
		}
	})
	preRunnersUp.forEach((r) => {
		if (postMember.indexOf(r) == -1 && postRunnersUp.indexOf(r) == -1) {
			outgoing.push(r)
		}
	})

	return outgoing
}

async function legacyReservedOf(who: string, when: BlockHash, api: ApiPromise): Promise<BN> {
	let Balances = xxhashAsHex("Balances", 128).slice(2)
	let ReservedBalance = xxhashAsHex("ReservedBalance", 128).slice(2)
	let account = api.createType('AccountId', who).toU8a()
	let accountHash = blake2AsHex(account, 256).slice(2)
	let key = "0x" + Balances + ReservedBalance + accountHash
	let data = await api.rpc.state.getStorage(key, when);
	// @ts-ignore
	return api.createType('Balance', data.unwrapOrDefault())
}

async function detectReservedSlash(who: string, pre: BlockHash, post: BlockHash, api: ApiPromise, unreserve: Unreserved[]): Promise<BN> {
	let pereReserved = BN.max(
		(await api.query.system.account.at(pre, who)).data.reserved,
		await legacyReservedOf(who, pre, api)
	)
	let postReserved = BN.max(
		(await api.query.system.account.at(post, who)).data.reserved,
		await legacyReservedOf(who, post, api)
	)

	// find the sum of unreserve for a balance.
	let sumUnreserve = new BN(0)
	unreserve.forEach(({ amount, who: rwho }) => {
		if (rwho == who) {
			sumUnreserve = sumUnreserve.add(amount)
		}
	})

	// diff is a reduction is reserved balance, that can be caused by a combination of unreserve
	// and slash. Thus, `diff == unreserve + slash`, ergo `slash = diff - unreserve`.
	let diff = pereReserved.sub(postReserved);
	// max is needed -- maybe the unreserve operation was a noop.
	let effectiveSlash = BN.max(diff.sub(sumUnreserve), new BN(0));
	return effectiveSlash
}

function isSubsetOf(x: BN[], y: BN[]): boolean {
	let yClone = Array.from(y);
	for (let e1 of x) {
		let index = yClone.findIndex((e2) => e2.eq(e1))
		if (index == -1) {
			return false
		}
		yClone.splice(index, 1)
	}

	return true
}

function getSubset(slashes: Slash[], deposits: BN[]): Slash[] {
	let out: Slash[] = []
	let depositsClone = Array.from(deposits)
	for (let s of slashes) {
		let index = depositsClone.findIndex((d) => d.eq(s.amount))
		if (index == -1) {
			continue
		} else {
			out.push(s)
			depositsClone.splice(index, 1)
		}
	}

	return out
}

function eqSet(as: Set<any>, bs: Set<any>): boolean {
	if (as.size !== bs.size) return false;
	for (var a of as) if (!bs.has(a)) return false;
	return true;
}

async function calculateRefund(input: ElectionBlock[], api: ApiPromise): Promise<Map<string, BN>> {
	let refunds: Slash[] = [];
	input = input.reverse()
	for (let election of input) {
		// if there are no deposits, then there is nothing that we really care about here.
		if (election.deposits.length == 0) {
			console.log(`📗 [${election.time} / ${election.at}] Skipped.`)
			continue
		}
		let parent = (await api.rpc.chain.getHeader(election.at)).parentHash

		let preCandidates: Vec<AccountId> = await api.query.electionsPhragmen.candidates.at(parent);

		let preMembersRaw: Vec<ITuple<[AccountId, BalanceOf]>> = await api.query.electionsPhragmen.members.at(parent);
		let preMembers = preMembersRaw.map(x => x[0].toHuman());
		let preRunnersUpRaw: Vec<ITuple<[AccountId, BalanceOf]>> = await api.query.electionsPhragmen.runnersUp.at(parent);
		let preRunnersUp = preRunnersUpRaw.map(x => x[0].toHuman());
		let preSet: Set<string> = new Set();
		preMembers.forEach(x => preSet.add(x));
		preRunnersUp.forEach(x => preSet.add(x));

		let postMembersRaw: Vec<ITuple<[AccountId, BalanceOf]>> = await api.query.electionsPhragmen.members.at(election.at);
		let postMembers = postMembersRaw.map(x => x[0].toHuman());
		let postRunnersUpRaw: Vec<ITuple<[AccountId, BalanceOf]>> = await api.query.electionsPhragmen.runnersUp.at(election.at);
		let postRunnersUp = postRunnersUpRaw.map(x => x[0].toHuman());
		let postSet: Set<string> = new Set();
		postMembers.forEach(x => postSet.add(x));
		postRunnersUp.forEach(x => postSet.add(x));


		let all: Set<string> = new Set();
		preMembers.forEach(m => all.add(m))
		preRunnersUp.forEach(m => all.add(m))
		postMembers.forEach(m => all.add(m))
		postRunnersUp.forEach(m => all.add(m))
		assert(Array.from(all.values()).length > 0, "Seemingly we don't have any members here?")

		let correctSlashes = findCorrectSlash(preMembers, postMembers, preRunnersUp, postRunnersUp)
		let allUnreserveReductions = []
		for (let acc of all) {
			let slashRaw = await detectReservedSlash(acc, parent, election.at, api, election.unreserve)
			if (!slashRaw.isZero()) {
				let slash: Slash = { at: election.at, amount: slashRaw, who: acc }
				allUnreserveReductions.push(slash)
			}
		}

		// all final slashes must be subsets of deposits.
		let effectiveSlash = getSubset(allUnreserveReductions, election.deposits);
		for (let s of effectiveSlash) {
			if (correctSlashes.indexOf(s.who) == -1) {
				refunds.push(s)
			}
		}

		if (effectiveSlash.length != allUnreserveReductions.length) {
			console.log("⚠️  A reduction in reserved seem to have been discarded.")
			console.log("Effective" ,effectiveSlash, "All", allUnreserveReductions)
		}

		// defensive only.
		assert(
			isSubsetOf(effectiveSlash.map(x => x.amount), election.deposits),
			`A slash is not deposited. This must be a deduction of reserved for other reasons.`,
			allUnreserveReductions.map(s => `who: ${s.who}, amount: ${s.amount}`),
			election.deposits,
		);

		let candidatesOutcomes = preCandidates.map(c => postSet.has(c.toHuman()))
		let candidateSlashCount = candidatesOutcomes.filter(x => x == false).length

		// sum of candidate slashes and slashes that we record must be the same as deposits (to the
		// best of my knowledge)
		assert(
			candidateSlashCount + effectiveSlash.length == election.deposits.length,
			"sum of candidate slashes and slashes that we record mus the same as deposits",
		)

		// if any candidate made it into the set, the the sets must not be equal
		if (candidatesOutcomes.indexOf(true) > -1 ) {
			assert(!eqSet(preSet, postSet), "if any candidate made it into the set, the the sets must not be equal.")
		}
		// Either all slashes are correct, or the pre-post set must not be equal. We can only have
		// a correct slash when the set changes.
		assert(correctSlashes.length == 0 || !eqSet(preSet, postSet), "Correct slash can only happen when sets are unequal")
		console.log(`📕 [${election.time} / ${election.at}] ${effectiveSlash.length} slashes / ${correctSlashes.length} correct / ${election.deposits.length} deposits / ${election.unreserve.length} unreserve / preSet = ${Array.from(preSet).length} / postSet ${Array.from(postSet).length} / Equal? ${eqSet(preSet, postSet)} / candidates ${preCandidates.length} / outcome ${candidatesOutcomes.toString()}`)
	}

	let perAccountRefund: Map<string, BN> = new Map();
	refunds.forEach(( { amount, who }) => {
		let prev = perAccountRefund.get(who) || new BN(0);
		perAccountRefund.set(who, prev.add(amount))
	})
	return perAccountRefund
}

async function getCurrentRole(who: string, api: ApiPromise): Promise<string> {
	let currentMembers = await api.query.electionsPhragmen.members();
	let currentRunners = await api.query.electionsPhragmen.runnersUp();

	// @ts-ignore
	let isMembers: boolean = currentMembers.findIndex((x) => x[0].toHuman() == who) != -1
	// @ts-ignore
	let isRunner: boolean = currentRunners.findIndex((x) => x[0].toHuman() == who) != -1

	if (isMembers && isRunner) {
		console.log('Cant be member and a runner-up')
		process.exit(1)
	}

	let role = isMembers ? 'Members' : isRunner? 'RunnerUp' : 'None';
	return role
}

interface Refund {
	preImage: string,
	hash: string,
}
async function buildRefundTx(chain: string, slashMap: Map<string, BN>, api: ApiPromise): Promise<Refund> {
	let treasuryAccount = new Uint8Array(32);
	let modulePrefix = new Uint8Array(new TextEncoder().encode("modl"))
	treasuryAccount.set(modulePrefix)
	treasuryAccount.set(api.consts.treasury.moduleId.toU8a(), modulePrefix.length)
	let treasury = api.createType('AccountId', treasuryAccount)

	// verified account kusama: F3opxRbN5ZbjJNU511Kj2TLuzFcDq9BGduA9TgiECafpg29
	// verified account polkadot: 13UVJyLnbVp9RBZYFwFGyDvVd1y27Tt8tkntv6Q7JVPhFsTB
	if (chain == "kusama") {
		assert(treasury.toHuman().toString() === "F3opxRbN5ZbjJNU511Kj2TLuzFcDq9BGduA9TgiECafpg29")
	} else {
		assert(treasury.toHuman().toString() === "13UVJyLnbVp9RBZYFwFGyDvVd1y27Tt8tkntv6Q7JVPhFsTB")
	}
	let sum = new BN(0)
	let transfers = [];
	for (let [who, amount] of slashMap) {
		let tx = api.tx.balances.forceTransfer(treasury, who, amount);
		sum = sum.add(amount)
		console.log(tx.toHuman())
		transfers.push(tx)
	}
	let tx = api.tx.utility.batch(transfers);
	console.log("transaction:", tx.toHuman())
	console.log("preimage: ", tx.method.toHex())
	console.log("hash:", tx.method.hash.toHex())
	console.log("sum: ", api.createType('Balance', sum).toHuman())

	writeFileSync(`${chain}-preimage-${tx.method.hash.toHex()}.bin`, tx.method.toHex());

	return { preImage: tx.method.toHex(), hash: tx.meta.hash.toHex() }
}

(async () => {
	const provider = new WsProvider(process.argv[2])
	const api = await ApiPromise.create( { provider })
	const chain = "polkadot"

	// -- scrape and create a new cache election json file
	// unlinkSync(`elections.${chain}.json`)
	// let elections = await findElections(api, chain);
	// writeFileSync(`elections.${chain}.json`, JSON.stringify(elections))

	// -- use cached file
	let elections: ElectionBlock[] = JSON.parse(readFileSync(`elections.${chain}.json`).toString())
	for (let i = 0; i < elections.length; i++) {
		elections[i].deposits = elections[i].deposits.map(x => new BN(`${x}`, 'hex'))
		elections[i].unreserve = elections[i].unreserve.map( ({ who, amount }) => {
			return { who, amount: new BN(`${amount}`, 'hex') }
		})
	}

	let slashMap = await calculateRefund(elections, api);
	await parseCSVSimple(api, slashMap)
	const { preImage, hash } = await buildRefundTx(chain, slashMap, api);

	// const preImage = readFileSync(`${chain}-preimage-0x683b144f5dc9fe9875261fc75ffb49c7d047669a887ab639d7c322783cf6593d.bin`).toString()
	// console.log('preImage', preImage)

	await submitPreImage(api, preImage, true)
})()

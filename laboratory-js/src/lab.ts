import { ApiPromise, WsProvider } from "@polkadot/api";
import BN from "bn.js";

export async function stakingReport() {
	let endpoint = "ws://localhost::9944"
	const provider = new WsProvider(endpoint);
	const api = await ApiPromise.create({ provider })

	let allNominators = await api.query.staking.nominators.keys();
	let allLedgers = await api.query.staking.ledger.entries();
	let allBonded = await api.query.staking.bonded.entries();

	console.log(`all nominators = ${allNominators.length}`)
	console.log(`all ledgers ${allLedgers.length}`)
	console.log(`all bonded ${allBonded.length}`)
}

export async function dustNominators() {
	let endpoint = "ws://localhost::9944"
	const provider = new WsProvider(endpoint);
	const api = await ApiPromise.create({ provider })

	let entries = await api.query.staking.nominators.keys()
	console.log(entries.length)

	let stakers = []
	for (let x of entries) {
		let k = x.toU8a().slice(-32)
		let ctrl = (await api.query.staking.bonded(k)).unwrap()
		let ledger = (await api.query.staking.ledger(ctrl)).unwrapOrDefault()
		let nomination = await api.query.staking.nominators(k)
		let stake = ledger.active
		stakers.push({ who: x, stake: stake, nomination, ledger })
	}
	stakers.sort((a, b) => {
		if (a.stake.toBn().gt(b.stake.toBn())) { return 1 } else if (a.stake.toBn().lt(b.stake.toBn())) { return -1 } else { return 0 }
	})

	stakers.reverse().forEach(({ who, stake, nomination, ledger }, _) => {
		if (stake.eq(new BN(0)) && ledger.unlocking.length == 0) {
			console.log(who.toHuman(), stake.toHuman(), nomination.toHuman(), ledger.toHuman())
		}
	})
	console.log('done')
}

export async function latestElectionSubmissions() {
	let endpoint = "ws://localhost::9944"
	const provider = new WsProvider(endpoint);
	const api = await ApiPromise.create({ provider })

	const head = await api.rpc.chain.getFinalizedHead();
	let now = head
	console.log(`starting at ${now}`);
	let _electionStatus = await api.query.staking.eraElectionStatus.at(now);
	let _queedScore = await api.query.staking.queuedScore.at(now);
	while (true) {
		let block = await api.rpc.chain.getBlock(now);
		let header = await api.derive.chain.getHeader(now)
		let number = block.block.header.number;
		let extrinsics = block.block.extrinsics;
		let events = await api.query.system.events.at(now)
		// let maximum_weight = api.consts.system.blockWeights.maxBlock;
		// let maximum_length = api.consts.system.blockLength.max;
		let electionStatus = await api.query.staking.eraElectionStatus.at(now);
		let queedScore = await api.query.staking.queuedScore.at(now);

		for (let ext of extrinsics) {
			if (ext.meta.name.toString().includes("submit_election_solution")) {
				let era = await api.query.staking.currentEra.at(now);
				let found = false;
				let weight = await api.query.system.blockWeight.at(now)
				for (let event of events) {
					if (event.event.meta.name.includes("SolutionStored")) {
						console.log(`[${number}] ✅ Found a correct ${ext.meta.name} for era ${era.toHuman()} at block ${number} by ${header?.author} => score ${ext.args[2]}`)
						// console.log(weight, maximum_weight, maximum_length);
						// console.log(`⌚️ Weight = ${weight} (${weight.normal / maximum_weight}). Len = ${ext.encodedLength} (${ext.encodedLength / maximum_length})`)
						found = true
						break;
					}
				}
				if (!found) {
					console.log(`[${number}] ❌ Found a failed ${ext.meta.name} for era ${era.toHuman()} => score ${ext.args[2]}. Weight = ${weight}. Len = ${ext.encodedLength}`)
				}
			}
		}

		for (let event of events) {
			if (event.event.meta.name.includes("StakingElection")) {
				console.log(`[${number}] 🤑 Staking election closed with compute ${event.event.data.toHuman()}`)
				break;
			}
		}

		// if (_electionStatus.isClose !== electionStatus.isClose) {
		// 	console.log(`[${number}] change in election status. previous ${_electionStatus}, now ${electionStatus}`)
		// 	_electionStatus = electionStatus
		// }
		// if (!_queedScore.eq(queedScore)) {
		// 	console.log(`[${number}] change in queued score. previous ${_queedScore}, now ${queedScore}`)
		// 	_queedScore = queedScore
		// }

		now = block.block.header.parentHash
	}
}

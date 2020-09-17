import { ApiPromise, WsProvider } from "@polkadot/api";


export async function perbillTest() {
	let endpoint = "ws://localhost::9944"
	const provider = new WsProvider(endpoint);
	const api = await ApiPromise.create({ provider })

	const { commission } = await api.query.staking.validators("5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty")
	console.log(commission.unwrap().toHex());
	console.log(commission.unwrap().toHuman());
	console.log(commission.unwrap().toU8a());
	console.log(commission.unwrap().toJSON());
	console.log(commission.unwrap().toNumber());
	console.log(commission.unwrap().toBn());
	console.log(commission.unwrap().toString());

}

export async function latestElectionSubmissions() {
	let endpoint = "ws://localhost:9944"
	const provider = new WsProvider(endpoint);
	const api = await ApiPromise.create({ provider })

	const head = await api.rpc.chain.getFinalizedHead();
	let now = head
	while (true) {
		let block = await api.rpc.chain.getBlock(now);
		let extrinsics = block.block.extrinsics;
		let events = await api.query.system.events.at(now)

		for (let ext of extrinsics) {
			if (ext.meta.name.toString().includes("submit_election_solution")) {
				let era = await api.query.staking.currentEra.at(now);
				let found = false;
				let weight = await api.query.system.blockWeight.at(now)
				for (let event of events) {
					if (event.event.meta.name.includes("SolutionStored")) {
						console.log(`✅ Found a correct ${ext.meta.name} for era ${era.toHuman()} => score ${ext.args[2]}. Weight = ${weight}. Len = ${ext.encodedLength}`)
						found = true
						break;
					}
				}
				if (!found) {
					console.log(`❌ Found a failed ${ext.meta.name} for era ${era.toHuman()} => score ${ext.args[2]}. Weight = ${weight}. Len = ${ext.encodedLength}`)
				}
			}
		}

		for (let event of events) {
			if (event.event.meta.name.includes("StakingElection")) {
				console.log(`🤑 Staking election closed at ${now} (${block.block.header.number}) with compute ${event.event.data.toHuman()}`)
				break;
			}
		}

		now = block.block.header.parentHash
	}
}

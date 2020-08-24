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

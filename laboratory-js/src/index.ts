import { ApiPromise } from "@polkadot/api";
import * as lab from "./lab";

(async () => {
	// await lab.stakingReport()
	await lab.dustNominators();
})()

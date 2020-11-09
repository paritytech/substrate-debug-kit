"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const api_1 = require("@polkadot/api");
const bn_js_1 = __importDefault(require("bn.js"));
const request_promise_1 = __importDefault(require("request-promise"));
function recordedReserved(whos, api) {
    return __awaiter(this, void 0, void 0, function* () {
        let democracyDepositsOf = yield api.query.democracy.depositOf.entries();
        let democracyPreImages = yield api.query.democracy.preimages.entries();
        let electionsMembers = yield api.query.electionsPhragmen.members();
        let electionsRunnersUp = yield api.query.electionsPhragmen.runnersUp();
        let electionsCandidates = yield api.query.electionsPhragmen.candidates();
        let indicesAccounts = yield api.query.indices.accounts.entries();
        let multisigs = yield api.query.multisig.multisigs.entries();
        let multisigCalls = yield api.query.multisig.calls.entries();
        let treasuryProposals = yield api.query.treasury.proposals.entries();
        let treasuryTips = yield api.query.treasury.tips.entries();
        let treasuryBounties = yield api.query.treasury.bounties.entries();
        let good = 0;
        let bad = 0;
        let result = [];
        for (let who of whos) {
            let deposits = [];
            // democracy: PreImage, DepositsOf,
            democracyDepositsOf.forEach(([_, maybeDepositOf]) => {
                let depositOf = maybeDepositOf.unwrapOrDefault();
                let [backers, deposit] = depositOf;
                if (backers.find((x) => x.toHuman() == who) != undefined) {
                    deposits.push(["democracy.depositOf", deposit]);
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
            let voting = (yield api.query.electionsPhragmen.voting(who))[1].length > 0 ? api.consts.electionsPhragmen.votingBond : new bn_js_1.default(0);
            //@ts-ignore
            let is_member = electionsMembers.find(([m, _]) => m.toHuman() == who) != undefined;
            //@ts-ignore
            let is_runner_up = electionsRunnersUp.find(([m, _]) => m.toHuman() == who) != undefined;
            //@ts-ignore
            let is_candidate = electionsCandidates.find((c) => c.toHuman() == who) != undefined;
            //@ts-ignore
            let candidacy = is_member || is_runner_up || is_candidate ? api.consts.electionsPhragmen.candidacyBond : new bn_js_1.default(0);
            //@ts-ignore
            deposits.push(["elections-phragmen.voter", voting]);
            //@ts-ignore
            deposits.push(["elections-phragmen.candidacy", candidacy]);
            // identity
            let identity = (yield api.query.identity.identityOf(who)).unwrapOrDefault();
            deposits.push(["identity.deposit", identity.deposit]);
            let subs = (yield api.query.identity.subsOf(who))[0];
            deposits.push(["identity.subs", subs]);
            identity.judgements.forEach(([_, j]) => {
                if (j.isFeePaid) {
                    deposits.push(["identity.judgments", j.asFeePaid]);
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
                let multi = maybeMulti.unwrapOrDefault();
                if (multi.depositor.toHuman() == who) {
                    deposits.push(["multisig.multisig", multi.deposit]);
                }
            });
            multisigCalls.forEach(([_, maybeCall]) => {
                let [__, depositor, deposit] = maybeCall.unwrapOrDefault();
                if (depositor.toHuman() == who) {
                    deposits.push(["multisig.call", deposit]);
                }
            });
            // proxy: Proxies, Anonymous(TODO), announcements
            try {
                let proxies = (yield api.query.proxy.proxies(who))[1];
                deposits.push(["proxy.proxies", proxies]);
                let announcements = (yield api.query.proxy.announcements(who))[1];
                deposits.push(["proxy.announcements", announcements]);
            }
            catch (e) {
                console.error("ERROR while fetching proxy:", e, who);
            }
            // treasury: Proposals, Tips, Curator/Bounties
            treasuryProposals.forEach(([_, maybeProp]) => {
                let prop = maybeProp.unwrapOrDefault();
                if (prop.proposer.toHuman() == who) {
                    deposits.push(["treasury.proposals", prop.bond]);
                }
            });
            treasuryTips.forEach(([_, maybeTip]) => {
                let tip = maybeTip.unwrapOrDefault();
                if (tip.finder.toHuman() == who) {
                    deposits.push(["treasury.tip", tip.deposit]);
                }
            });
            treasuryBounties.forEach(([_, maybeBounty]) => {
                let bounty = maybeBounty.unwrapOrDefault();
                // Bounty is not funded yet, so there is still a deposit for proposer.
                if (bounty.status.isProposed || bounty.status.isFunded) {
                    if (bounty.proposer.toHuman() == who) {
                        deposits.push(["treasury.bounty.proposer", bounty.bond]);
                    }
                }
                else {
                    // Curator has a deposit.
                    if (!bounty.curatorDeposit.isZero()) {
                        //@ts-ignore
                        if (bounty.status.value && bounty.status.value.curator) {
                            //@ts-ignore
                            if (bounty.status.value.curator.toHuman() == who) {
                                deposits.push(["treasury.bounty.curator", bounty.curatorDeposit]);
                            }
                        }
                    }
                }
            });
            let sum = new bn_js_1.default(0);
            for (let [_k, v] of deposits) {
                sum = sum.add(v);
            }
            let accountData = (yield api.query.system.account(who));
            let match = accountData.data.reserved.eq(sum);
            match ? good++ : bad++;
            console.log(`${match ? "✅" : "❌"} - ${who} on-chain reserved = ${accountData.data.reserved.toHuman()} (${accountData.data.reserved.toBn()}) // module-sum = ${api.createType('Balance', sum).toHuman()} (${sum})`);
            if (!match) {
                if (accountData.nonce.isZero()) {
                    console.log("⚠️  Nonce zero. This is probably a multisig account.");
                }
                console.log(accountData.toHuman());
                deposits.forEach(([m, d]) => console.log(`+ ${m} => ${api.createType('Balance', d).toHuman()}`));
            }
            result.push([who, deposits, sum, accountData.data.reserved.toBn()]);
        }
        console.log(good, bad);
        return result;
    });
}
function checkAllAccounts(api) {
    return __awaiter(this, void 0, void 0, function* () {
        let all_accounts = (yield api.query.system.account.entries()).map(([acc, _]) => {
            //@ts-ignore
            return acc.toHuman()[0];
        });
        console.log(`fetched ${all_accounts.length} accounts`);
        // let edge_cases: string[] = [
        // 	"15G4hfDNtNhRc82As8Ep2YfvpM5xVdX7De3P9qSdHerGA6wC",
        // 	"1JCU9za8ZwT51LkDHoVXhLRRvBuCeqTTrdkfZsEW6CVyC2L",
        // 	"12yi4uHFbnSUryffXT7Xq92fbGC3iXvCs3vz9HjVgpb4sBvL",
        // ]
        yield recordedReserved(all_accounts, api);
    });
}
function computeRefund(api) {
    return __awaiter(this, void 0, void 0, function* () {
        let slashed_councilors = new Map([
            ["1RG5T6zGY4XovW75mTgpH6Bx7Y6uwwMmPToMCJSdMwdm4EW", new bn_js_1.default(1603640000000)],
            ["1WG3jyNqniQMRZGQUc7QD2kVLT8hkRPGMSqAb5XYQM1UDxN", new bn_js_1.default(1252580000000)],
            ["1dGsgLgFez7gt5WjX2FYzNCJtaCjGG6W9dA42d9cHngDYGg", new bn_js_1.default(1607240000000)],
            ["1hJdgnAPSjfuHZFHzcorPnFvekSHihK9jdNPWHXgeuL7zaJ", new bn_js_1.default(1252580000000)],
            ["1rwgen2jqJNNg7DpUA4jBvMjyepgiFKLLm3Bwt8pKQYP8Xf", new bn_js_1.default(1252580000000)],
            ["128qRiVjxU3TuT37tg7AX99zwqfPtj2t4nDKUv9Dvi5wzxuF", new bn_js_1.default(1266880000000)],
            ["12Vv2LsLCvPKiXdoVGa3QSs2FMF8zx2c8CPTWwLAwfYSFVS1", new bn_js_1.default(4252580000000)],
            ["12Y8b4C9ar162cBgycxYgxxHG7cLVs8gre9Y5xeMjW3izqer", new bn_js_1.default(1202910000000)],
            ["12mP4sjCfKbDyMRAEyLpkeHeoYtS5USY4x34n9NMwQrcEyoh", new bn_js_1.default(1202580000000)],
            ["12xG1Bn4421hUQAxKwZd9WSxZCJQwJBbwr6aZ4ZxvuR7A1Ao", new bn_js_1.default(1000000000000)],
            ["12xGDBh6zSBc3D98Jhw9jgUVsK8jiwGWHaPTK21Pgb7PJyPn", new bn_js_1.default(1402990000000)],
            ["13Gdmw7xZQVbVoojUCwnW2usEikF2a71y7aocbgZcptUtiX9", new bn_js_1.default(1202580000000)],
            ["13pdp6ALhYkfEBqBM98ztL2Xhv4MTkm9rZ9vyjyXSdirJHx6", new bn_js_1.default(2806820000000)],
            ["14krbTSTJv3aaT1VeBRX7CzoV4crr3adeF3KutdpkCttrxsZ", new bn_js_1.default(1000000000000)],
            ["14mSXQeHpF8NT1tMKu87tAbNDNjm7q9qh8hYa7BY2toNUkTo", new bn_js_1.default(1452990000000)],
            ["15BQUqtqhmqJPyvvEH5GYyWffXWKuAgoSUHuG1UeNdb8oDNT", new bn_js_1.default(1804170000000)],
            ["15MUBwP6dyVw5CXF9PjSSv7SdXQuDSwjX86v1kBodCSWVR7c", new bn_js_1.default(2050000000000)],
            ["15aKvwRqGVAwuBMaogtQXhuz9EQqUWsZJSAzomyb5xYwgBXA", new bn_js_1.default(1452990000000)],
            ["15akrup6APpRegG1TtWkYVuWHYc37tJ8XPN61vCuHQUi65Mx", new bn_js_1.default(1407820000000)],
            ["167rjWHghVwBJ52mz8sNkqr5bKu5vpchbc9CBoieBhVX714h", new bn_js_1.default(1000000000000)],
        ]);
        let keys = Array.from(slashed_councilors.keys());
        let stuff = yield recordedReserved(keys, api);
        console.log("who,should_reserve,has_reserve,effective_slash");
        stuff.forEach(([who, _, should_reserve, has_reserve]) => {
            console.log(`${who},${should_reserve},${has_reserve},${slashed_councilors.get(who)}`);
        });
    });
}
function findElections(api) {
    return __awaiter(this, void 0, void 0, function* () {
        let res = yield request_promise_1.default.get("https://explorer-31.polkascan.io/polkadot/api/v1/event?filter[module_id]=electionsphragmen&filter[event_id]=NewTerm&page[number]=1&page[size]=100");
        res = JSON.parse(res);
        let data = res.data;
        let blocks = [];
        for (let e of data) {
            let has_new_term = false;
            let deposits = [];
            let block_id = e.attributes.block_id;
            try {
                let block_data_raw = yield request_promise_1.default.get(`https://explorer-31.polkascan.io/polkadot/api/v1/block/${block_id}?include=transactions,inherents,events,logs`);
                let block_data = JSON.parse(block_data_raw);
                let hash = block_data.data.attributes.hash;
                let events = yield api.query.system.events.at(hash);
                for (let ev of events) {
                    if (ev.event.meta.name.toHuman() == "NewTerm") {
                        has_new_term = true;
                    }
                    if (ev.event.meta.name.toHuman() == "Deposit") {
                        deposits.push(ev.event.data[0]);
                    }
                }
                console.log(`("${hash.slice(2)}", vec![${deposits}], "${block_data.data.attributes.datetime}"),`);
            }
            catch (e) {
                console.log("Erro for", block_id, e);
            }
        }
        console.log("Done");
    });
}
(() => __awaiter(void 0, void 0, void 0, function* () {
    const api = yield api_1.ApiPromise.create();
    // await findElections(api);
    yield checkAllAccounts(api);
    // await computeRefund(api);
}))();

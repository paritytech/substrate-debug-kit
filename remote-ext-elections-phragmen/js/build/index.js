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
const console_1 = require("console");
const fs_1 = require("fs");
const util_crypto_1 = require("@polkadot/util-crypto");
const util_1 = require("util");
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
        let result = [];
        for (let who of whos) {
            let deposits = [];
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
                let nonce = (yield api.query.system.account(who)).nonce;
                // direct.
                let deposit = (yield api.query.proxy.proxies(who))[1];
                deposits.push(["proxy.proxies[direct]", deposit]);
            }
            catch (e) {
                console.error("ERROR while fetching proxy:", e, who);
            }
            let announcements = (yield api.query.proxy.announcements(who))[1];
            deposits.push(["proxy.announcements", announcements]);
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
        yield recordedReserved(all_accounts, api);
    });
}
function parseCSV(api, slashMap) {
    return __awaiter(this, void 0, void 0, function* () {
        let keys = Array.from(slashMap.keys());
        let stuff = yield recordedReserved(keys, api);
        console.log("who,role,should_reserve,has_reserve,missing,effective_slash,trivial,reserved refund,free refund");
        for (let [who, _, should_reserve, has_reserve] of stuff) {
            let effectiveSlash = slashMap.get(who);
            let missing = should_reserve.sub(has_reserve);
            let isTrivial = (effectiveSlash === null || effectiveSlash === void 0 ? void 0 : effectiveSlash.eq(missing)) ? '✅' : '❌';
            let reservedRefund = missing;
            let freeRefund = effectiveSlash === null || effectiveSlash === void 0 ? void 0 : effectiveSlash.sub(missing);
            let role = yield getCurrentRole(who, api);
            console.log(`${who},${role},${should_reserve},${has_reserve},${missing},${effectiveSlash},${isTrivial},${reservedRefund},${freeRefund}`);
        }
    });
}
function parseCSVSimple(api, slashMap) {
    return __awaiter(this, void 0, void 0, function* () {
        console.log("who,role,identity,effective_slash_planck,effective_slash_token");
        for (let [who, effectiveSlash] of slashMap) {
            let role = yield getCurrentRole(who, api);
            let identity = (yield api.query.identity.identityOf(who)).unwrapOrDefault().info.display.asRaw.toHuman();
            console.log(`${who},${role},${identity === null || identity === void 0 ? void 0 : identity.toString()},${effectiveSlash},${api.createType('Balance', effectiveSlash).toHuman()}`);
        }
    });
}
function findElections(api, chain) {
    return __awaiter(this, void 0, void 0, function* () {
        let page = 1;
        let data = [];
        let pre_len = data.length;
        while (true) {
            console.log(`fetching page ${page}`);
            let more = JSON.parse(yield request_promise_1.default.get(`https://explorer-31.polkascan.io/${chain}/api/v1/event?filter[module_id]=electionsphragmen&filter[event_id]=NewTerm&page[number]=${page}&page[size]=100`)).data;
            data = data.concat(more);
            if (data.length > pre_len) {
                page++;
                pre_len = data.length;
            }
            else {
                break;
            }
        }
        console.log(`Collected ${data.length} election events.`);
        let out = [];
        for (let e of data) {
            let has_new_term = false;
            let newTermIndex = 0;
            let deposits = [];
            let unreserve = [];
            let block_id = e.attributes.block_id;
            try {
                let block_data_raw = yield request_promise_1.default.get(`https://explorer-31.polkascan.io/${chain}/api/v1/block/${block_id}?include=transactions,inherents,events,logs`);
                let block_data = JSON.parse(block_data_raw);
                let at = block_data.data.attributes.hash;
                let events = yield api.query.system.events.at(at);
                let index = 0;
                for (let ev of events) {
                    if (ev.event.meta.name.toHuman() == "NewTerm") {
                        has_new_term = true;
                        newTermIndex = index;
                    }
                    index++;
                }
                index = 0;
                for (let ev of events) {
                    if (index <= newTermIndex &&
                        // all of the events from this index to newTerm must be deposits
                        events.toArray().slice(index, newTermIndex).map(e => e.event.meta.name.toHuman() == "Deposit").indexOf(false) == -1 &&
                        ev.event.section == "treasury" &&
                        ev.event.meta.name.toHuman() == "Deposit" &&
                        (ev.phase.isInitialization ||
                            (ev.phase.isApplyExtrinsic && ev.phase.asApplyExtrinsic.isZero()))) {
                        deposits.push(new bn_js_1.default(ev.event.data[0].toString()));
                    }
                    if (ev.event.meta.name.toHuman() == "Unreserved") {
                        unreserve.push({ who: ev.event.data[0].toString(), amount: new bn_js_1.default(ev.event.data[1].toString()) });
                    }
                    index++;
                }
                // if we have had no unreserve events, then a bunch of other events should be counted as unreserve
                if (unreserve.length == 0) {
                    for (let ev of events) {
                        if (ev.event.meta.name.toHuman() == "Tabled") {
                            let [_, deposit, depositors] = ev.event.data;
                            // @ts-ignore
                            for (let d of depositors) {
                                unreserve.push({ who: d.toHuman(), amount: new bn_js_1.default(deposit.toString()) });
                            }
                        }
                        if (ev.event.meta.name.toHuman() == "PreimageUsed") {
                            let [_, depositor, amount] = ev.event.data;
                            unreserve.push({ who: depositor.toHuman(), amount: new bn_js_1.default(amount.toString()) });
                        }
                        if (ev.event.meta.name.toHuman() == "Inducted") {
                            let [_, new_members] = ev.event.data;
                            // @ts-ignore
                            for (let m of new_members) {
                                unreserve.push({ who: m.toHuman(), amount: api.consts.society.candidateDeposit });
                            }
                        }
                    }
                }
                if (!has_new_term) {
                    console.log("Something went wrong.");
                    process.exit(0);
                }
                out.push({ at, deposits, time: block_data.data.attributes.datetime, unreserve });
                console.log(at, deposits.length, unreserve.length);
            }
            catch (e) {
                console.log("Error at", block_id, e);
            }
        }
        return out;
    });
}
function parseElections(input) {
    console.log("vec![");
    for (let e of input) {
        console.log(`("${e.at.slice(2)}", vec![${e.deposits}], "${e.time}",),`);
    }
    console.log("]");
}
function findCorrectSlash(preMembers, postMember, preRunnersUp, postRunnersUp) {
    let outgoing = [];
    preMembers.forEach((m) => {
        if (postMember.indexOf(m) == -1 && postRunnersUp.indexOf(m) == -1) {
            outgoing.push(m);
        }
    });
    preRunnersUp.forEach((r) => {
        if (postMember.indexOf(r) == -1 && postRunnersUp.indexOf(r) == -1) {
            outgoing.push(r);
        }
    });
    return outgoing;
}
function legacyReservedOf(who, when, api) {
    return __awaiter(this, void 0, void 0, function* () {
        let Balances = util_crypto_1.xxhashAsHex("Balances", 128).slice(2);
        let ReservedBalance = util_crypto_1.xxhashAsHex("ReservedBalance", 128).slice(2);
        let account = api.createType('AccountId', who).toU8a();
        let accountHash = util_crypto_1.blake2AsHex(account, 256).slice(2);
        let key = "0x" + Balances + ReservedBalance + accountHash;
        let data = yield api.rpc.state.getStorage(key, when);
        // @ts-ignore
        return api.createType('Balance', data.unwrapOrDefault());
    });
}
function detectReservedSlash(who, pre, post, api, unreserve) {
    return __awaiter(this, void 0, void 0, function* () {
        let pereReserved = bn_js_1.default.max((yield api.query.system.account.at(pre, who)).data.reserved, yield legacyReservedOf(who, pre, api));
        let postReserved = bn_js_1.default.max((yield api.query.system.account.at(post, who)).data.reserved, yield legacyReservedOf(who, post, api));
        // find the sum of unreserve for a balance.
        let sumUnreserve = new bn_js_1.default(0);
        unreserve.forEach(({ amount, who: rwho }) => {
            if (rwho == who) {
                sumUnreserve = sumUnreserve.add(amount);
            }
        });
        // diff is a reduction is reserved balance, that can be caused by a combination of unreserve
        // and slash. Thus, `diff == unreserve + slash`, ergo `slash = diff - unreserve`.
        let diff = pereReserved.sub(postReserved);
        // max is needed -- maybe the unreserve operation was a noop.
        let effectiveSlash = bn_js_1.default.max(diff.sub(sumUnreserve), new bn_js_1.default(0));
        return effectiveSlash;
    });
}
function isSubsetOf(x, y) {
    let yClone = Array.from(y);
    for (let e1 of x) {
        let index = yClone.findIndex((e2) => e2.eq(e1));
        if (index == -1) {
            return false;
        }
        yClone.splice(index, 1);
    }
    return true;
}
function getSubset(slashes, deposits) {
    let out = [];
    let depositsClone = Array.from(deposits);
    for (let s of slashes) {
        let index = depositsClone.findIndex((d) => d.eq(s.amount));
        if (index == -1) {
            continue;
        }
        else {
            out.push(s);
            depositsClone.splice(index, 1);
        }
    }
    return out;
}
function eqSet(as, bs) {
    if (as.size !== bs.size)
        return false;
    for (var a of as)
        if (!bs.has(a))
            return false;
    return true;
}
function calculateRefund(input, api) {
    return __awaiter(this, void 0, void 0, function* () {
        let refunds = [];
        input = input.reverse();
        for (let election of input) {
            // if there are no deposits, then there is nothing that we really care about here.
            if (election.deposits.length == 0) {
                console.log(`📗 [${election.time} / ${election.at}] Skipped.`);
                continue;
            }
            let parent = (yield api.rpc.chain.getHeader(election.at)).parentHash;
            let preCandidates = yield api.query.electionsPhragmen.candidates.at(parent);
            let preMembersRaw = yield api.query.electionsPhragmen.members.at(parent);
            let preMembers = preMembersRaw.map(x => x[0].toHuman());
            let preRunnersUpRaw = yield api.query.electionsPhragmen.runnersUp.at(parent);
            let preRunnersUp = preRunnersUpRaw.map(x => x[0].toHuman());
            let preSet = new Set();
            preMembers.forEach(x => preSet.add(x));
            preRunnersUp.forEach(x => preSet.add(x));
            let postMembersRaw = yield api.query.electionsPhragmen.members.at(election.at);
            let postMembers = postMembersRaw.map(x => x[0].toHuman());
            let postRunnersUpRaw = yield api.query.electionsPhragmen.runnersUp.at(election.at);
            let postRunnersUp = postRunnersUpRaw.map(x => x[0].toHuman());
            let postSet = new Set();
            postMembers.forEach(x => postSet.add(x));
            postRunnersUp.forEach(x => postSet.add(x));
            let all = new Set();
            preMembers.forEach(m => all.add(m));
            preRunnersUp.forEach(m => all.add(m));
            postMembers.forEach(m => all.add(m));
            postRunnersUp.forEach(m => all.add(m));
            console_1.assert(Array.from(all.values()).length > 0, "Seemingly we don't have any members here?");
            let correctSlashes = findCorrectSlash(preMembers, postMembers, preRunnersUp, postRunnersUp);
            let allUnreserveReductions = [];
            for (let acc of all) {
                let slashRaw = yield detectReservedSlash(acc, parent, election.at, api, election.unreserve);
                if (!slashRaw.isZero()) {
                    let slash = { at: election.at, amount: slashRaw, who: acc };
                    allUnreserveReductions.push(slash);
                }
            }
            // all final slashes must be subsets of deposits.
            let effectiveSlash = getSubset(allUnreserveReductions, election.deposits);
            for (let s of effectiveSlash) {
                if (correctSlashes.indexOf(s.who) == -1) {
                    refunds.push(s);
                }
            }
            if (effectiveSlash.length != allUnreserveReductions.length) {
                console.log("⚠️  A reduction in reserved seem to have been discarded.");
                console.log("Effective", effectiveSlash, "All", allUnreserveReductions);
            }
            // defensive only.
            console_1.assert(isSubsetOf(effectiveSlash.map(x => x.amount), election.deposits), `A slash is not deposited. This must be a deduction of reserved for other reasons.`, allUnreserveReductions.map(s => `who: ${s.who}, amount: ${s.amount}`), election.deposits);
            let candidatesOutcomes = preCandidates.map(c => postSet.has(c.toHuman()));
            let candidateSlashCount = candidatesOutcomes.filter(x => x == false).length;
            // sum of candidate slashes and slashes that we record must be the same as deposits (to the
            // best of my knowledge)
            console_1.assert(candidateSlashCount + effectiveSlash.length == election.deposits.length, "sum of candidate slashes and slashes that we record mus the same as deposits");
            // if any candidate made it into the set, the the sets must not be equal
            if (candidatesOutcomes.indexOf(true) > -1) {
                console_1.assert(!eqSet(preSet, postSet), "if any candidate made it into the set, the the sets must not be equal.");
            }
            // Either all slashes are correct, or the pre-post set must not be equal. We can only have
            // a correct slash when the set changes.
            console_1.assert(correctSlashes.length == 0 || !eqSet(preSet, postSet), "Correct slash can only happen when sets are unequal");
            console.log(`📕 [${election.time} / ${election.at}] ${effectiveSlash.length} slashes / ${correctSlashes.length} correct / ${election.deposits.length} deposits / ${election.unreserve.length} unreserve / preSet = ${Array.from(preSet).length} / postSet ${Array.from(postSet).length} / Equal? ${eqSet(preSet, postSet)} / candidates ${preCandidates.length} / outcome ${candidatesOutcomes.toString()}`);
        }
        let perAccountRefund = new Map();
        refunds.forEach(({ amount, who }) => {
            let prev = perAccountRefund.get(who) || new bn_js_1.default(0);
            perAccountRefund.set(who, prev.add(amount));
        });
        return perAccountRefund;
    });
}
function getCurrentRole(who, api) {
    return __awaiter(this, void 0, void 0, function* () {
        let currentMembers = yield api.query.electionsPhragmen.members();
        let currentRunners = yield api.query.electionsPhragmen.runnersUp();
        // @ts-ignore
        let isMembers = currentMembers.findIndex((x) => x[0].toHuman() == who) != -1;
        // @ts-ignore
        let isRunner = currentRunners.findIndex((x) => x[0].toHuman() == who) != -1;
        if (isMembers && isRunner) {
            console.log('Cant be member and a runner-up');
            process.exit(1);
        }
        let role = isMembers ? 'Members' : isRunner ? 'RunnerUp' : 'None';
        return role;
    });
}
function buildRefundTx(chain, slashMap, api) {
    let treasuryAccount = new Uint8Array(32);
    let modulePrefix = new Uint8Array(new util_1.TextEncoder().encode("modl"));
    treasuryAccount.set(modulePrefix);
    treasuryAccount.set(api.consts.treasury.moduleId.toU8a(), modulePrefix.length);
    let treasury = api.createType('AccountId', treasuryAccount);
    // verified account kusama: F3opxRbN5ZbjJNU511Kj2TLuzFcDq9BGduA9TgiECafpg29
    // verified account polkadot: 13UVJyLnbVp9RBZYFwFGyDvVd1y27Tt8tkntv6Q7JVPhFsTB
    if (chain == "kusama") {
        console_1.assert(treasury.toHuman().toString() === "F3opxRbN5ZbjJNU511Kj2TLuzFcDq9BGduA9TgiECafpg29");
    }
    else {
        console_1.assert(treasury.toHuman().toString() === "13UVJyLnbVp9RBZYFwFGyDvVd1y27Tt8tkntv6Q7JVPhFsTB");
    }
    let sum = new bn_js_1.default(0);
    let transfers = [];
    for (let [who, amount] of slashMap) {
        let tx = api.tx.balances.forceTransfer(treasury, who, amount);
        sum = sum.add(amount);
        console.log(tx.toHuman());
        transfers.push(tx);
    }
    let tx = api.tx.utility.batch(transfers);
    console.log("transaction:", tx.toHuman());
    console.log("hex: ", tx.toHex());
    console.log("sum: ", api.createType('Balance', sum).toHuman());
}
(() => __awaiter(void 0, void 0, void 0, function* () {
    // const provider = new WsProvider('wss://kusama-rpc.polkadot.io/')
    const api = yield api_1.ApiPromise.create();
    const chain = "kusama";
    // -- scrape and create a new cache election json file
    // unlinkSync(`elections.${chain}.json`)
    // let elections = await findElections(api, chain);
    // writeFileSync(`elections.${chain}.json`, JSON.stringify(elections))
    // -- use cached file
    let elections = JSON.parse(fs_1.readFileSync(`elections.${chain}.json`).toString());
    for (let i = 0; i < elections.length; i++) {
        elections[i].deposits = elections[i].deposits.map(x => new bn_js_1.default(`${x}`, 'hex'));
        elections[i].unreserve = elections[i].unreserve.map(({ who, amount }) => {
            return { who, amount: new bn_js_1.default(`${amount}`, 'hex') };
        });
    }
    let slashMap = yield calculateRefund(elections, api);
    yield parseCSVSimple(api, slashMap);
    buildRefundTx(chain, slashMap, api);
}))();

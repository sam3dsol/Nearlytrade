// Checks for the audit fixes TAX-VENUE-01 (tax_add_pair: admin-only, add-only pair list, auto-register for a
// payout from any listed pair) and MAXWALLET-PAYOUT-01 (a listed pair's payout is never refused by the max
// wallet rule), on the gas-metered host (lib/host.mjs):
//   node contracts/token-tax/sim/audit_fixes.mjs tax.wasm
import { readFileSync } from "node:fs";
import { Token } from "./lib/host.mjs";

const [taxPath] = process.argv.slice(2);
const ME = "tok.near", DCL = "dclv2.ref-labs.near", PAIR2 = "pair2.near", ADMIN = "factory.near", REG = 1_250_000_000_000_000_000_000n;
const SUPPLY = 1_000_000_000n, CAP = SUPPLY / 10_000n * 400n; // max_wallet_bps 400: 40,000,000
const dec = (b) => Buffer.from(b).toString();
let fails = 0;
const check = (name, cond, detail = "") => { console.log(`${cond ? "PASS" : "FAIL"}  ${name}${cond ? "" : "  " + detail}`); if (!cond) fails++; };

/// a tax token in its launch window (host block time is 1.8e12 ms, until_ms is past that); pairs as given
function mk({ pairs = [DCL], exempt = ["alice.near", DCL, "locker.near"] } = {}) {
  const t = new Token(readFileSync(taxPath), { me: ME });
  const must = (m, o) => { const r = t.call(m, o); if (!r.ok) throw new Error(`${m}: ${r.err}`); return r; };
  must("new", { pred: ADMIN, args: {
    owner_id: "alice.near", total_supply: String(SUPPLY), metadata: { spec: "ft-1.0.0", name: "T", symbol: "T", decimals: 0 },
    rules: { max_wallet_bps: 400, until_ms: "2000000000000", exempt },
    tax: { buy_bps: 300, sell_bps: 400, pairs, admin: ADMIN, exempt: ["locker.near"] },
  } });
  for (const a of ["bob.near", "carol.near", DCL, PAIR2, "locker.near"]) must("storage_deposit", { pred: a, deposit: REG, args: {} });
  must("ft_transfer", { pred: "alice.near", deposit: 1n, args: { receiver_id: DCL, amount: "500000000" } }); // a sell: taxed
  must("tax_take", { pred: ADMIN }); // clear the vault so every case starts at 0
  t.must = must;
  return t;
}
const bal = (t, a) => BigInt(JSON.parse(dec(t.call("ft_balance_of", { pred: "x.near", args: { account_id: a } }).ret)));
const tax = (t) => JSON.parse(dec(t.call("get_tax", { pred: "x.near" }).ret));
const addPair = (t, pair, pred = ADMIN) => t.call("tax_add_pair", { pred, args: pair === undefined ? {} : { pair } });
const xfer = (t, from, to, amount) => t.call("ft_transfer", { pred: from, deposit: 1n, args: { receiver_id: to, amount: String(amount) } });

// ---------------------------------------------------------------------------------------------
console.log("== 1. tax_add_pair: admin appends, the stored JSON still parses, the new pair is taxed");
{
  const t = mk();
  check("1 before: a transfer into pair2 is wallet to wallet (untaxed)", xfer(t, "alice.near", PAIR2, 10000).ok && bal(t, PAIR2) === 10000n && bal(t, ME) === 0n, `pair2 ${bal(t, PAIR2)} vault ${bal(t, ME)}`);
  const r = addPair(t, PAIR2);
  check("1 admin adds pair2", r.ok && r.logs.some((l) => l === "tax: pair added " + PAIR2), r.err);
  const tx = tax(t).tax;
  check("1 get_tax parses, pairs = [DCL, pair2]", JSON.stringify(tx.pairs) === JSON.stringify([DCL, PAIR2]), JSON.stringify(tx));
  check("1 the rest of the tax JSON is unchanged", tx.buy_bps === 300 && tx.sell_bps === 400 && tx.admin === ADMIN && JSON.stringify(tx.exempt) === '["locker.near"]', JSON.stringify(tx));
  check("1 raw b\"t\" is the init blob with the id spliced in", dec(t.storage.get("t")) === `{"buy_bps":300,"sell_bps":400,"pairs":["${DCL}","${PAIR2}"],"admin":"${ADMIN}","exempt":["locker.near"]}`, dec(t.storage.get("t")));
  const p0 = bal(t, PAIR2);
  check("1 after: a transfer into pair2 is a sell, 4 % to the vault", xfer(t, "alice.near", PAIR2, 10000).ok && bal(t, PAIR2) === p0 + 9600n && bal(t, ME) === 400n, `pair2 ${bal(t, PAIR2)} vault ${bal(t, ME)}`);
  check("1 a transfer out of pair2 is a buy, 3 % to the vault", xfer(t, PAIR2, "bob.near", 1000).ok && bal(t, "bob.near") === 970n && bal(t, ME) === 430n, `bob ${bal(t, "bob.near")} vault ${bal(t, ME)}`);
  check("1 DCL still taxed after the append", xfer(t, DCL, "carol.near", 1000).ok && bal(t, "carol.near") === 970n && bal(t, ME) === 460n, `carol ${bal(t, "carol.near")}`);
  const tk = t.call("tax_take", { pred: ADMIN });
  check("1 tax_take still reads the admin from the new blob", tk.ok && dec(tk.ret) === '"460"', tk.ok ? dec(tk.ret) : tk.err);
  check("1 a third pair appends after the second", addPair(t, "pair3.near").ok && JSON.stringify(tax(t).tax.pairs) === JSON.stringify([DCL, PAIR2, "pair3.near"]), JSON.stringify(tax(t).tax.pairs));
  check("1 a 64-char pair id appends", addPair(t, "p".repeat(55) + ".longname").ok && tax(t).tax.pairs.length === 4);
}

// ---------------------------------------------------------------------------------------------
console.log("== 2. tax_add_pair refusals: non-admin, duplicate, bad id, missing arg; nothing is ever removed");
{
  const t = mk();
  const before = dec(t.storage.get("t"));
  for (const who of ["alice.near", "bob.near", DCL, ME, "locker.near"]) {
    const r = addPair(t, PAIR2, who);
    check(`2 ${who} refused`, !r.ok && r.err === "admin only", r.err);
  }
  check("2 duplicate of the init pair refused", (() => { const r = addPair(t, DCL); return !r.ok && r.err === "pair already listed"; })());
  t.must("tax_add_pair", { pred: ADMIN, args: { pair: PAIR2 } });
  check("2 duplicate of an appended pair refused", (() => { const r = addPair(t, PAIR2); return !r.ok && r.err === "pair already listed"; })());
  for (const bad of ["Bad.near", "a", "..", "x..y", ".near", "near.", "a b", "ab\\u002ecd", "p".repeat(65)]) {
    const r = addPair(t, bad);
    check(`2 invalid id ${JSON.stringify(bad)} refused`, !r.ok && (r.err === "invalid account id" || r.kind === "host"), r.err);
  }
  check("2 missing pair refused", !addPair(t, undefined).ok);
  check("2 null pair refused", !t.call("tax_add_pair", { pred: ADMIN, args: '{"pair":null}' }).ok);
  check("2 non-string pair refused", !t.call("tax_add_pair", { pred: ADMIN, args: '{"pair":5}' }).ok);
  check("2 no method removes a pair; refusals left the blob as it was plus pair2", dec(t.storage.get("t")) === before.replace(`["${DCL}"]`, `["${DCL}","${PAIR2}"]`), dec(t.storage.get("t")));
  check("2 a non-admin refusal rolls back (no write)", (() => { const b = dec(t.storage.get("t")); addPair(t, "pair3.near", "bob.near"); return dec(t.storage.get("t")) === b; })());
}

// ---------------------------------------------------------------------------------------------
console.log("== 3. tax_add_pair on an empty pairs list: no leading comma, and the result parses");
{
  const t = mk({ pairs: [] });
  check("3 empty list: DCL is not a pair yet (untaxed)", xfer(t, DCL, "bob.near", 1000).ok && bal(t, "bob.near") === 1000n && bal(t, ME) === 0n);
  check("3 append to []", addPair(t, DCL).ok && dec(t.storage.get("t")).includes(`"pairs":["${DCL}"]`), dec(t.storage.get("t")));
  check("3 JSON.parse agrees", JSON.stringify(tax(t).tax.pairs) === JSON.stringify([DCL]));
  check("3 now taxed", xfer(t, DCL, "bob.near", 1000).ok && bal(t, "bob.near") === 1970n && bal(t, ME) === 30n, `bob ${bal(t, "bob.near")} vault ${bal(t, ME)}`);
  const t2 = new Token(readFileSync(taxPath), { me: ME });
  const r0 = t2.call("new", { pred: ADMIN, args: `{"owner_id":"alice.near","total_supply":"1000","metadata":{"spec":"ft-1.0.0","name":"T","symbol":"T","decimals":0},"tax":{"buy_bps":300,"sell_bps":400,"pairs":[ \n ],"admin":"${ADMIN}"}}` });
  check("3 whitespace-only array counts as empty", r0.ok && addPair(t2, DCL).ok && JSON.stringify(tax(t2).tax.pairs) === JSON.stringify([DCL]), r0.ok ? dec(t2.storage.get("t")) : r0.err);
}

// ---------------------------------------------------------------------------------------------
console.log("== 4. max wallet: a listed pair's payout is never refused, a plain transfer over the cap still is");
{
  const t = mk();
  t.must("tax_add_pair", { pred: ADMIN, args: { pair: PAIR2 } });
  t.must("ft_transfer", { pred: "alice.near", deposit: 1n, args: { receiver_id: PAIR2, amount: "30000000" } }); // a sell: pair2 gets 28,800,000, under the cap (the appended pair is not rules-exempt)
  t.must("ft_transfer", { pred: "alice.near", deposit: 1n, args: { receiver_id: "bob.near", amount: "20000000" } });
  t.must("ft_transfer", { pred: "alice.near", deposit: 1n, args: { receiver_id: "carol.near", amount: "20000000" } });
  check(`4 cap is ${CAP}`, bal(t, "bob.near") === 20_000_000n && CAP === 40_000_000n);
  const r1 = xfer(t, "alice.near", "bob.near", 25_000_000);
  check("4 wallet to wallet over the cap dies", !r1.ok && /max wallet share/.test(r1.err) && bal(t, "bob.near") === 20_000_000n, r1.err);
  const r2 = xfer(t, PAIR2, "bob.near", 25_000_000); // net 24,250,000 lifts bob to 44,250,000 > cap
  check("4 payout from the appended pair over the cap is NOT refused", r2.ok && bal(t, "bob.near") === 44_250_000n, r2.ok ? String(bal(t, "bob.near")) : r2.err);
  const r3 = xfer(t, DCL, "carol.near", 25_000_000);
  check("4 payout from the init pair over the cap is NOT refused", r3.ok && bal(t, "carol.near") === 44_250_000n, r3.ok ? String(bal(t, "carol.near")) : r3.err);
  const r4 = xfer(t, "bob.near", "carol.near", 1);
  check("4 wallet to wallet onto a wallet already over the cap still dies", !r4.ok && /max wallet share/.test(r4.err), r4.err);
  const r5 = xfer(t, "bob.near", PAIR2, 1_000_000);
  check("4 a sell into the pair is taxed as before and lands (the pair is under the cap)", r5.ok && bal(t, PAIR2) === 28_800_000n - 25_000_000n + 960_000n, r5.ok ? String(bal(t, PAIR2)) : r5.err);
  const r6 = xfer(t, "bob.near", "locker.near", 40_000_001);
  check("4 rules-exempt receiver still exempt", r6.ok, r6.err);
  // the appended pair is not in rules.exempt: a sell into it that lifts the pool over the cap must still land
  const p0 = bal(t, PAIR2);
  const r8 = xfer(t, "alice.near", PAIR2, 50_000_000); // net 48,000,000 lifts pair2 to 52,760,000 > cap
  check("4 sell into the appended pair over the cap is NOT refused", r8.ok && bal(t, PAIR2) === p0 + 48_000_000n && bal(t, PAIR2) > CAP, r8.ok ? String(bal(t, PAIR2)) : r8.err);
  const r9 = t.call("ft_transfer_call", { pred: "alice.near", deposit: 1n, args: { receiver_id: PAIR2, amount: "1000000", msg: "swap" } });
  check("4 ft_transfer_call sell into the appended pair over the cap is NOT refused", r9.ok && bal(t, PAIR2) === p0 + 48_000_000n + 960_000n, r9.err);
  const r10 = xfer(t, "alice.near", "newwallet.near", 1);
  const r11 = xfer(t, "alice.near", "carol.near", 1);
  check("4 wallet to wallet over the cap still dies after that", !r10.ok && !r11.ok && /max wallet share/.test(r11.err), r11.err);
  check("4 a payout that lands under the cap works as before", (() => { const r = xfer(t, PAIR2, "locker.near", 10); return r.ok; })());
  // ft_transfer_call from a pair (a router paying out with a call) takes the same path
  const r7 = t.call("ft_transfer_call", { pred: DCL, deposit: 1n, args: { receiver_id: "carol.near", amount: "1000000", msg: "" } });
  check("4 ft_transfer_call payout from a pair over the cap is NOT refused", r7.ok && bal(t, "carol.near") === 44_250_000n + 970_000n, r7.ok ? String(bal(t, "carol.near")) : r7.err);
}

// ---------------------------------------------------------------------------------------------
console.log("== 5. max wallet still enforced on a token whose pairs list is the init one (no regression)");
{
  const t = mk();
  t.must("ft_transfer", { pred: "alice.near", deposit: 1n, args: { receiver_id: "bob.near", amount: "40000000" } });
  const r = xfer(t, "alice.near", "bob.near", 1);
  check("5 at the cap: one more dies", !r.ok && /max wallet share/.test(r.err), r.err);
  const r2 = xfer(t, PAIR2, "bob.near", 1); // pair2 NOT listed on this token: a plain wallet
  check("5 an unlisted account is a plain wallet, its transfer over the cap dies", !r2.ok && /not registered|enough balance|max wallet share/.test(r2.err), r2.err);
  t.must("ft_transfer", { pred: "alice.near", deposit: 1n, args: { receiver_id: PAIR2, amount: "100" } });
  const r3 = xfer(t, PAIR2, "bob.near", 1);
  check("5 funded unlisted account over the cap dies", !r3.ok && /max wallet share/.test(r3.err), r3.err);
}

// ---------------------------------------------------------------------------------------------
console.log("== 6. auto-register: a payout from any listed pair, not only the hardcoded exchange");
{
  const t = mk();
  t.must("ft_transfer", { pred: "alice.near", deposit: 1n, args: { receiver_id: PAIR2, amount: "100000" } });
  const r0 = xfer(t, PAIR2, "newbie.near", 1000);
  check("6 before listing: pair2 to an unregistered wallet dies", !r0.ok && r0.err === "The receiver is not registered", r0.err);
  t.must("tax_add_pair", { pred: ADMIN, args: { pair: PAIR2 } });
  const r1 = xfer(t, PAIR2, "newbie.near", 1000);
  check("6 after listing: pair2 payout auto-registers, net of the buy tax", r1.ok && bal(t, "newbie.near") === 970n && r1.logs.some((l) => l.startsWith("auto-registered newbie.near")), r1.ok ? dec(r1.logs.join("|")) : r1.err);
  const rec = t.storage.get("bnewbie.near");
  check("6 the record carries the auto flag (17 bytes, flag 1)", rec && rec.length === 17 && rec[16] === 1);
  const sb = t.call("storage_balance_of", { pred: "x.near", args: { account_id: "newbie.near" } });
  check("6 registered per NEP-145", sb.ok && JSON.parse(dec(sb.ret)) !== null);
  const r2 = xfer(t, DCL, "newbie2.near", 1000);
  check("6 the init pair still auto-registers", r2.ok && bal(t, "newbie2.near") === 970n, r2.err);
  const r3 = xfer(t, "alice.near", "newbie3.near", 1000);
  check("6 a plain wallet still cannot pay an unregistered wallet", !r3.ok && r3.err === "The receiver is not registered", r3.err);
  const r4 = xfer(t, PAIR2, "bad..id", 7);
  check("6 pair payout to an invalid id refused", !r4.ok && r4.err === "invalid account id", r4.err);
  const r5 = t.call("ft_transfer_call", { pred: PAIR2, deposit: 1n, args: { receiver_id: "newbie4.near", amount: "1000", msg: "" } });
  check("6 ft_transfer_call from the new pair auto-registers too", r5.ok && bal(t, "newbie4.near") === 970n, r5.err);
  // the token account cannot stake the storage: the payout dies as before
  t.accountBalance = 0n;
  const r6 = xfer(t, PAIR2, "newbie5.near", 1000);
  check("6 no NEAR to stake: dies with the usual message", !r6.ok && r6.err === "The receiver is not registered", r6.err);
}

// ---------------------------------------------------------------------------------------------
console.log("== 7. gas: ft_transfer paths with the extra pair lookup, sim lower bound");
{
  const t = mk();
  t.must("tax_add_pair", { pred: ADMIN, args: { pair: PAIR2 } });
  t.must("ft_transfer", { pred: "alice.near", deposit: 1n, args: { receiver_id: PAIR2, amount: "30000000" } });
  t.must("ft_transfer", { pred: "alice.near", deposit: 1n, args: { receiver_id: "bob.near", amount: "20000000" } });
  const tg = (g) => (Number(g) / 1e12).toFixed(3);
  const plain = xfer(t, "alice.near", "carol.near", 1000);
  const payout = xfer(t, PAIR2, "bob.near", 25_000_000);
  const auto = xfer(t, PAIR2, "newbie.near", 1000);
  const add = t.call("tax_add_pair", { pred: ADMIN, args: { pair: "pair3.near" } });
  console.log(`  plain ${tg(plain.gas)} TGas | pair payout over cap ${tg(payout.gas)} | auto-register ${tg(auto.gas)} | tax_add_pair ${tg(add.gas)}`);
  check("7 all four paths ran", plain.ok && payout.ok && auto.ok && add.ok);
}

console.log(fails ? `\n${fails} FAILED` : "\nall passed");
process.exit(fails ? 1 : 0);

// Checks for the final token versions (raw + tax), on the gas-metered host (lib/host.mjs):
//   node contracts/token-tax/sim/final_checks.mjs raw.wasm tax.wasm [base_raw.wasm base_tax.wasm]
// With the base builds given, the gas lines also print the baseline figures.
import { readFileSync } from "node:fs";
import { Token, TGAS, validAccount } from "./lib/host.mjs";

const [rawPath, taxPath, baseRaw, baseTax] = process.argv.slice(2);
const ME = "tok.near", DCL = "dclv2.ref-labs.near", ADMIN = "factory.near", REG = 1_250_000_000_000_000_000_000n;
const L64A = "a".repeat(55) + ".longname", L64B = "b".repeat(55) + ".longname"; // 64-char ids
const dec = (b) => Buffer.from(b).toString();
let fails = 0;
const check = (name, cond, detail = "") => { console.log(`${cond ? "PASS" : "FAIL"}  ${name}${cond ? "" : "  " + detail}`); if (!cond) fails++; };
const tg = (g) => (Number(g) / 1e12).toFixed(3);

function mk(path, tax, { decimals = 0, supply = "1000000000" } = {}) {
  const t = new Token(readFileSync(path), { me: ME });
  const args = { owner_id: "alice.near", total_supply: supply, metadata: { spec: "ft-1.0.0", name: "T", symbol: "T", decimals } };
  if (tax) args.tax = { buy_bps: 300, sell_bps: 400, pairs: [DCL], admin: ADMIN, exempt: ["locker.near"] };
  const must = (m, o) => { const r = t.call(m, o); if (!r.ok) throw new Error(`${m}: ${r.err}`); return r; };
  must("new", { pred: ADMIN, args });
  for (const a of ["bob.near", DCL, L64A, L64B, "router.near"]) must("storage_deposit", { pred: a, deposit: REG, args: {} });
  must("ft_transfer", { pred: "alice.near", deposit: 1n, args: { receiver_id: L64A, amount: "100000000" } });
  must("ft_transfer", { pred: "alice.near", deposit: 1n, args: { receiver_id: DCL, amount: "500000000" } });
  t.must = must;
  return t;
}
const bal = (t, a) => BigInt(JSON.parse(dec(t.call("ft_balance_of", { pred: "x.near", args: { account_id: a } }).ret)));

/// ft_transfer_call from `from` to `to`, then its resolve with `result`; returns {ftc, res}
function roundtrip(t, from, to, amount, result, msg = "swap") {
  const ftc = t.call("ft_transfer_call", { pred: from, deposit: 1n, args: { receiver_id: to, amount: String(amount), msg } });
  if (!ftc.ok) return { ftc };
  const cb = ftc.promises.find((p) => p.after !== undefined);
  const res = t.call("ft_resolve_transfer", { pred: ME, args: cb.actions[0].args, result });
  return { ftc, res, cbGas: cb.actions[0].gas };
}

const builds = [["raw", rawPath, false], ["tax", taxPath, true]];

// ---------------------------------------------------------------------------------------------
console.log("== 1. resolve gas: heaviest path (64-char ids, full refund + tax refund), sim lower bound");
for (const [name, path, tax] of [...builds, ...(baseRaw ? [["raw base", baseRaw, false], ["tax base", baseTax, true]] : [])]) {
  const t = mk(path, tax);
  const full = roundtrip(t, L64A, DCL, 10000, { failed: true });
  const part = roundtrip(t, L64A, DCL, 10000, { value: '"4800"' });
  const none = roundtrip(t, L64A, DCL, 10000, { value: '"0"' });
  console.log(`  ${name.padEnd(9)} full refund ${tg(full.res.gas)} TGas | partial ${tg(part.res.gas)} | none ${tg(none.res.gas)} | ft_transfer_call ${tg(full.ftc.gas)} | resolve attached ${tg(full.cbGas)}`);
  if (!name.includes("base")) check(`1 ${name}: resolve paths ok and under 2 TGas in the sim`, full.res.ok && part.res.ok && none.res.ok && full.res.gas < 2n * TGAS && part.res.gas < 2n * TGAS);
}

// ---------------------------------------------------------------------------------------------
console.log("== 2. resolve against a long receiver answer (host charges the register write before the token sees it)");
for (const [name, path, tax] of [...builds, ...(baseRaw ? [["raw base", baseRaw, false], ["tax base", baseTax, true]] : [])]) {
  const row = [];
  for (const n of [64, 65, 10_000, 200_000, 600_000, 1_000_000, 4_194_304]) {
    const t = mk(path, tax);
    const v = '"' + "0".repeat(Math.max(0, n - 3)) + '5"';
    const { res } = roundtrip(t, L64A, DCL, 10000, { value: v.slice(0, n) });
    row.push(`${n} B: ${res.ok ? "" : "FAIL "}${tg(res.gas)}`);
  }
  console.log(`  ${name.padEnd(9)} ${row.join(" | ")}`);
}

// ---------------------------------------------------------------------------------------------
console.log("== 3. receiver answers: never a failed resolve; near-sdk reading");
for (const [name, path, tax] of builds) {
  for (const [v, unused] of [['""', 9600n], ['"340282366920938463463374607431768211456"', 9600n], ['"340282366920938463463374607431768211455"', 9600n], ['"' + "0".repeat(62) + '"', 0n], ['"' + "0".repeat(63) + '"', 9600n], ['"+"', 9600n], ['"+0"', 0n], ['"1e3"', 9600n], ['"４"', 9600n]]) {
    const t = mk(path, tax);
    const a0 = bal(t, L64A);
    const { res } = roundtrip(t, L64A, DCL, 10000, { value: v });
    const net = tax ? 9600n : 10000n, unusedN = unused === 9600n ? net : unused;
    const taxBack = tax && unusedN === net ? 400n : 0n;
    check(`3 ${name} answer ${v.length > 30 ? v.slice(0, 12) + "…(" + v.length + " B)" : v}: unused ${unusedN}`, res.ok && bal(t, L64A) === a0 - (tax ? 10000n : 10000n) + unusedN + taxBack, res.ok ? String(bal(t, L64A) - a0) : res.err);
  }
}

// ---------------------------------------------------------------------------------------------
console.log("== 4. account ids: token check == NEAR rules (storage_deposit with account_id)");
const ids = ["bob.near", "a.b", "ab", "a", "a-b_c.near", "0x" + "ab".repeat(20), "9d7242b3".repeat(8), "..", "a..b", ".bob", "bob.", "-bob", "bob-", "_bob", "bob_", "a__b", "a-_b", "a_-b",
  "a.-b", "a-.b", "a._b", "Bob.near", "bob near", "b".repeat(64), "b".repeat(65), "x.y.z.w", "1.2", "a\\u002eb", "a+b", "a@b", "near", "tok.near"];
for (const [name, path, tax] of builds) {
  let bad = [];
  for (const id of ids) {
    const t = mk(path, tax);
    const r = t.call("storage_deposit", { pred: "payer.near", deposit: REG, args: `{"account_id":"${id}"}` });
    const want = validAccount(id);
    if (r.ok !== want && !(r.ok === false && r.kind === "host")) bad.push(`${id}: token ${r.ok} near ${want}`);
  }
  check(`4 ${name}: ${ids.length} ids agree with NEAR rules`, bad.length === 0, bad.join("; "));
}

// ---------------------------------------------------------------------------------------------
console.log("== 5. memo / msg: accepted only when valid JSON as copied; events and ft_on_transfer args stay JSON");
const MEMOS = [["plain", "hi", true], ["escaped quote", 'a\\"b', true], ["unicode raw", "é🚀", true], ["pair", "\\ud83d\\ude00", true], ["\\u0000", "\\u0000", true],
  ["all escapes", '\\"\\\\\\/\\b\\f\\n\\r\\t', true], ["raw LF", "a\nb", false], ["raw 0x01", "a\x01b", false], ["\\x", "\\x", false], ["\\u12", "\\u12", false], ["\\u12zz", "\\u12zz", false],
  ["lone high", "\\ud800", false], ["lone low", "\\udc00", false], ["high+high", "\\ud800\\ud800", false], ["high+text", "\\ud800abcdef", false], ["trailing \\", "ab\\", false],
  ["1024 B", "m".repeat(1024), true], ["1025 B", "m".repeat(1025), false]];
for (const [name, path, tax] of builds) {
  for (const [label, m, ok] of MEMOS) {
    const t = mk(path, tax);
    const body = Buffer.concat([Buffer.from('{"receiver_id":"bob.near","amount":"5","memo":"'), Buffer.from(m, "utf8"), Buffer.from('"}')]);
    const r = t.call("ft_transfer", { pred: "alice.near", deposit: 1n, args: body });
    const evOk = r.ok && r.logs.filter((l) => l.startsWith("EVENT_JSON:")).every((l) => { try { JSON.parse(l.slice(11)); return true; } catch { return false; } });
    const label2 = `5 ${name} memo ${label}`;
    if (label === "1025 B") { check(label2 + " refused", !r.ok && r.err === "memo too long", r.err); continue; }
    check(label2 + (ok ? " accepted, event is JSON" : " refused"), ok ? evOk : !r.ok, r.ok ? "accepted" : r.err);
    // the same string as msg (no length cap on msg)
    const body2 = Buffer.concat([Buffer.from('{"receiver_id":"bob.near","amount":"5","msg":"'), Buffer.from(m, "utf8"), Buffer.from('"}')]);
    const r2 = t.call("ft_transfer_call", { pred: "alice.near", deposit: 1n, args: body2 });
    const argsOk = r2.ok && r2.promises.flatMap((p) => p.actions).filter((a) => a.method === "ft_on_transfer").every((a) => { try { JSON.parse(dec(a.args)); return true; } catch { return false; } });
    check(`5 ${name} msg ${label}` + (ok ? " accepted, args are JSON" : " refused"), ok ? argsOk : !r2.ok, r2.ok ? "accepted" : r2.err);
  }
}

// ---------------------------------------------------------------------------------------------
console.log("== 6. parser: no input that JSON.parse reads differently, valid JSON still accepted");
const INPUTS = [
  [`{"receiver_id":"bob.near","amount":"5"}`, true],
  [` \t\r\n{ "amount" : "5" , "receiver_id" : "bob.near" , "x" : [1, {"a": "}]"}], "y": null } \n`, true],
  [`{"receiver_id":"bob.near","amount":"5","memo":null}`, true],
  [`{"receiver_id":"evil.near","amount":"5","receiver_id":"bob.near"}`, false],
  [`{"receiver_id":"bob.near","amount":"5","amount":"99"}`, false],
  [`{"receiver_id":"bob.near","receiver\\u005fid":"evil.near","amount":"5"}`, false],
  [`{"receiver_id":"bob.near","amount":"5"} {"receiver_id":"evil.near"}`, false],
  [`{"receiver_id":"bob.near","amount":"5"}x`, false],
  [`{"receiver_id":"bob.near","amount":"5",}`, false],
  [`{"receiver_id":"bob.near" "amount":"5"}`, false],
  [`{"receiver_id":"bob.near",,"amount":"5"}`, false],
  [`{"receiver_id":"bob.near","x":,"amount":"5"}`, false],
  [`{"receiver_id":"bob.near","amount":"5"`, false],
  [`{"receiver_id":"bob.near","amount":"5","x":}`, false],
];
for (const [name, path, tax] of builds) {
  for (const [inp, ok] of INPUTS) {
    const t = mk(path, tax);
    const r = t.call("ft_transfer", { pred: "alice.near", deposit: 1n, args: inp });
    let ref; try { const o = JSON.parse(inp); ref = o.receiver_id; } catch { ref = "(invalid)"; }
    check(`6 ${name} ${ok ? "accepts" : "refuses"} ${inp.slice(0, 70)}`, r.ok === ok && (!r.ok || bal(t, ref) === (ref === "bob.near" ? 5n : 0n)), r.ok ? "accepted" : r.err);
  }
}

// ---------------------------------------------------------------------------------------------
console.log("== 7. transfers to the token account itself are refused; storage_withdraw; DCL auto-register");
for (const [name, path, tax] of builds) {
  const t = mk(path, tax);
  if (!tax) t.must("storage_deposit", { pred: "x.near", deposit: REG, args: { account_id: ME } }); // the raw token account registered by someone
  for (const m of ["ft_transfer", "ft_transfer_call"]) {
    const r = t.call(m, { pred: "alice.near", deposit: 1n, args: { receiver_id: ME, amount: "5", msg: "" } });
    check(`7 ${name} ${m} to the token itself refused`, !r.ok && /does not take transfers/.test(r.err), r.err);
  }
  const w = t.call("storage_withdraw", { pred: "bob.near", deposit: 1n, args: {} });
  check(`7 ${name} storage_withdraw {} ok`, w.ok && dec(w.ret) === `{"total":"${REG}","available":"0"}`, w.err);
  check(`7 ${name} storage_withdraw amount 1 refused`, !t.call("storage_withdraw", { pred: "bob.near", deposit: 1n, args: { amount: "1" } }).ok);
  const a = t.call("ft_transfer", { pred: DCL, deposit: 1n, args: { receiver_id: "newbie.near", amount: "7" } });
  check(`7 ${name} DCL payout auto-registers a valid new account`, a.ok && bal(t, "newbie.near") === (tax ? 7n - 0n : 7n), a.err);
  const b = t.call("ft_transfer", { pred: DCL, deposit: 1n, args: { receiver_id: "bad..id", amount: "7" } });
  check(`7 ${name} DCL payout to an invalid id refused`, !b.ok && b.err === "invalid account id", b.err);
}

// ---------------------------------------------------------------------------------------------
console.log("== 8. gas the token uses before it hands the rest to the receiver (lock3 budgets on ~3.2 TGas), HotZap-sized msg");
{
  const msg = JSON.stringify({ HotZap: { token_id: "x".repeat(40), amount: "1000000000000000000000000000", swap: { pool_ids: ["a|b|10000"], output_token: "wrap.near", min_output_amount: "0" }, add_liquidity_infos: [{ pool_id: "tok.near|wrap.near|10000", left_point: 200, right_point: 500000, amount_x: "1", amount_y: "0", min_amount_x: "0", min_amount_y: "0" }] } });
  for (const [name, path, tax] of [...builds, ...(baseRaw ? [["raw base", baseRaw, false], ["tax base", baseTax, true]] : [])]) {
    const t = mk(path, tax);
    const r = t.call("ft_transfer_call", { pred: "locker.near", deposit: 1n, args: { receiver_id: DCL, amount: "5", msg } });
    t.must("storage_deposit", { pred: "locker.near", deposit: REG, args: {} });
    t.must("ft_transfer", { pred: "alice.near", deposit: 1n, args: { receiver_id: "locker.near", amount: "1000" } });
    const r2 = t.call("ft_transfer_call", { pred: "locker.near", deposit: 1n, args: { receiver_id: DCL, amount: "5", msg } });
    console.log(`  ${name.padEnd(9)} ${r2.ok ? "" : r2.err} ft_transfer_call total ${tg(r2.gas)} TGas (sim; msg ${msg.length} B)`);
  }
}

// ---------------------------------------------------------------------------------------------
console.log("== 9. pro rata tax refund on random 18-decimal partial refunds (mul_div overflow path included)");
{
  let seed = 7; const rnd = () => { seed = (seed * 1103515245 + 12345) % 2147483648; return seed / 2147483648; };
  const rbig = (max) => { let x = 0n; for (let i = 0; i < 6; i++) x = (x << 24n) | BigInt(Math.floor(rnd() * 16777216)); return (x % max) + 1n; };
  const E = 10n ** 18n, SUPPLY = 1_000_000_000n * E;
  let worst = 0, over = 0, n = 0;
  for (let k = 0; k < 300; k++) {
    const t = mk(taxPath, true, { decimals: 18, supply: String(SUPPLY) });
    const holder = "seller.near";
    t.must("storage_deposit", { pred: holder, deposit: REG, args: {} });
    const gross = rbig(SUPPLY / 2n);
    t.must("ft_transfer", { pred: "alice.near", deposit: 1n, args: { receiver_id: holder, amount: String(gross) } });
    const ftc = t.must("ft_transfer_call", { pred: holder, deposit: 1n, args: { receiver_id: DCL, amount: String(gross), msg: "swap" } });
    const a = JSON.parse(dec(ftc.promises.find((p) => p.after !== undefined).actions[0].args));
    const net = BigInt(a.amount), tax = BigInt(a.tax);
    const unused = rbig(net);
    const r = t.call("ft_resolve_transfer", { pred: ME, args: JSON.stringify(a), result: { value: `"${unused}"` } });
    if (!r.ok) { check("9 resolve ok", false, r.err); break; }
    const back = bal(t, holder) - unused;
    const exact = (tax * unused) / net;
    if (back > tax) over++;
    const err = Number(back > exact ? back - exact : exact - back) / Number(tax || 1n);
    if (err > worst) worst = err;
    const sum = ["alice.near", holder, DCL, L64A, L64B, "bob.near", "router.near", ADMIN, ME].reduce((s, x) => s + bal(t, x), 0n);
    if (sum !== SUPPLY || t.storage.has("p")) { check("9 invariants", false, `sum ${sum}`); break; }
    n++;
  }
  check(`9 ${n} random partial refunds: tax back never above the tax, worst relative error ${worst.toExponential(2)}`, over === 0 && worst < 1e-9 && n === 300);
}

console.log(fails ? `\n${fails} FAILED` : "\nall passed");
process.exit(fails ? 1 : 0);

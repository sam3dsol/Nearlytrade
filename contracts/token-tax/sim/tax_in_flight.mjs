// Runs the built tax-token wasm against a minimal in-memory NEAR host and checks the in-flight tax
// accounting: a tax taken on ft_transfer_call stays in the vault, out of tax_take's reach, until that
// transfer's ft_resolve_transfer runs, so a fully refunded transfer always gets its tax back.
//
//   node contracts/token-tax/sim/tax_in_flight.mjs [path/to/nearpad_token_tax.wasm]   (default /tmp/tax.wasm)
//
// Host semantics kept: every call is a fresh instance (fresh memory), a panic rolls back that call's
// storage writes, promises are recorded and resolved by hand with the promise result the test picks.
import { readFileSync } from "node:fs";

const wasmPath = process.argv[2] || "/tmp/tax.wasm";
const mod = new WebAssembly.Module(readFileSync(wasmPath));
const enc = new TextEncoder();
const dec = new TextDecoder();

const ME = "tok.near";
const DCL = "dclv2.ref-labs.near";
const ADMIN = "factory.near";

let storage = new Map(); // latin1(key) -> Uint8Array
const k = (bytes) => Buffer.from(bytes).toString("latin1");

class Panic extends Error {}

/// one function call; `failAt` = throw on the n-th storage_write/remove (models running out of gas mid-call)
function call(method, { pred, args = {}, deposit = 0n, result = null, failAt = 0 } = {}) {
  const regs = new Map();
  let mem;
  const u8 = () => new Uint8Array(mem.buffer);
  const rd = (len, ptr) => u8().slice(Number(ptr), Number(ptr) + Number(len));
  const wr = (ptr, bytes) => u8().set(bytes, Number(ptr));
  const u128le = (n) => { const b = new Uint8Array(16); for (let i = 0; i < 16; i++) { b[i] = Number(n & 0xffn); n >>= 8n; } return b; };
  const input = enc.encode(typeof args === "string" ? args : JSON.stringify(args));
  const logs = [];
  const promises = [];
  let ret = null;
  let writes = 0;
  const tick = () => { if (failAt && ++writes >= failAt) throw new Panic("simulated out of gas"); };
  const env = {
    register_len: (r) => (regs.has(r) ? BigInt(regs.get(r).length) : 0xffffffffffffffffn),
    read_register: (r, ptr) => wr(ptr, regs.get(r)),
    input: (r) => regs.set(r, input),
    predecessor_account_id: (r) => regs.set(r, enc.encode(pred)),
    current_account_id: (r) => regs.set(r, enc.encode(ME)),
    attached_deposit: (ptr) => wr(ptr, u128le(deposit)),
    account_balance: (ptr) => wr(ptr, u128le(10n ** 25n)),
    storage_usage: () => { let n = 100n; for (const [key, v] of storage) n += 40n + BigInt(key.length + v.length); return n; },
    block_timestamp: () => 1_800_000_000_000_000_000n,
    used_gas: () => 5_000_000_000_000n,
    prepaid_gas: () => 300_000_000_000_000n,
    storage_read: (kl, kp, r) => { const v = storage.get(k(rd(kl, kp))); if (!v) return 0n; regs.set(r, v); return 1n; },
    storage_write: (kl, kp, vl, vp, r) => { tick(); const key = k(rd(kl, kp)); const old = storage.get(key); storage.set(key, rd(vl, vp)); if (old) { regs.set(r, old); return 1n; } return 0n; },
    storage_has_key: (kl, kp) => (storage.has(k(rd(kl, kp))) ? 1n : 0n),
    storage_remove: (kl, kp, r) => { tick(); const key = k(rd(kl, kp)); const old = storage.get(key); if (!old) return 0n; storage.delete(key); regs.set(r, old); return 1n; },
    value_return: (l, p) => { ret = dec.decode(rd(l, p)); },
    log_utf8: (l, p) => logs.push(dec.decode(rd(l, p))),
    panic: () => { throw new Panic("panic"); },
    panic_utf8: (l, p) => { throw new Panic(dec.decode(rd(l, p))); },
    promise_batch_create: (l, p) => BigInt(promises.push({ to: dec.decode(rd(l, p)), actions: [] }) - 1),
    promise_batch_action_transfer: (i, ptr) => promises[Number(i)].actions.push({ transfer: rd(16n, ptr) }),
    promise_batch_action_function_call: (i, ml, mp, al, ap, _amt, gas) => promises[Number(i)].actions.push({ method: dec.decode(rd(ml, mp)), args: dec.decode(rd(al, ap)), gas }),
    promise_then: (i, l, p, ml, mp, al, ap, _amt, gas) => BigInt(promises.push({ after: Number(i), to: dec.decode(rd(l, p)), actions: [{ method: dec.decode(rd(ml, mp)), args: dec.decode(rd(al, ap)), gas }] }) - 1),
    promise_return: () => {},
    promise_result: (_i, r) => {
      if (!result) throw new Panic("no promise result in this call");
      if (result.failed) return 2n;
      regs.set(r, enc.encode(result.value));
      return 1n;
    },
  };
  const snapshot = new Map(storage);
  const inst = new WebAssembly.Instance(mod, { env });
  mem = inst.exports.memory;
  try {
    inst.exports[method]();
  } catch (e) {
    storage = snapshot; // the receipt fails: none of its writes land
    if (e instanceof Panic) return { ok: false, err: e.message, logs };
    throw e;
  }
  return { ok: true, ret, logs, promises };
}

// ------------------------------------------------------------------------------------------------
// helpers over the token
// ------------------------------------------------------------------------------------------------
const bal = (a) => BigInt(JSON.parse(call("ft_balance_of", { pred: "x.near", args: { account_id: a } }).ret));
const supply = () => BigInt(JSON.parse(call("ft_total_supply", { pred: "x.near" }).ret));
const inFlight = () => { const v = storage.get("p"); if (!v) return 0n; let n = 0n; for (let i = 15; i >= 0; i--) n = (n << 8n) | BigInt(v[i]); return n; };
const pending = () => BigInt(JSON.parse(call("get_tax", { pred: "x.near" }).ret).pending);
const take = () => { const r = call("tax_take", { pred: ADMIN }); if (!r.ok) throw new Error("tax_take: " + r.err); return BigInt(JSON.parse(r.ret)); };
function must(r, what) { if (!r.ok) throw new Error(what + ": " + r.err); return r; }
/// ft_transfer_call; returns the resolve args the token scheduled for itself
function sell(from, amount, to = DCL) {
  const r = must(call("ft_transfer_call", { pred: from, deposit: 1n, args: { receiver_id: to, amount: String(amount), msg: "swap" } }), "ft_transfer_call " + from);
  const cb = r.promises.find((p) => p.after !== undefined);
  return cb.actions[0].args;
}
const resolve = (args, result, failAt = 0) => call("ft_resolve_transfer", { pred: ME, args, result, failAt });
const accounts = ["alice.near", "bob.near", "carol.near", DCL, ADMIN, ME, "locker.near"];

let fails = 0;
function check(name, cond, detail = "") {
  console.log(`${cond ? "PASS" : "FAIL"}  ${name}${cond ? "" : "  " + detail}`);
  if (!cond) fails++;
}
/// every token is on some balance, and the vault always covers what is in flight
function invariants(step) {
  const sum = accounts.reduce((s, a) => s + bal(a), 0n);
  check(`${step}: balances sum to supply`, sum === supply(), `sum ${sum} supply ${supply()}`);
  check(`${step}: vault >= in flight`, bal(ME) >= inFlight(), `vault ${bal(ME)} in flight ${inFlight()}`);
}

function fresh() {
  storage = new Map();
  must(call("new", {
    pred: ADMIN,
    args: {
      owner_id: "alice.near",
      total_supply: "1000000",
      metadata: { spec: "ft-1.0.0", name: "T", symbol: "T", decimals: 0 },
      tax: { buy_bps: 300, sell_bps: 400, pairs: [DCL], admin: ADMIN, exempt: ["locker.near"] },
    },
  }), "new");
  for (const a of ["bob.near", "carol.near", DCL, "locker.near"]) must(call("storage_deposit", { pred: a, deposit: 1_250_000_000_000_000_000_000n, args: {} }), "register " + a);
  must(call("ft_transfer", { pred: "alice.near", deposit: 1n, args: { receiver_id: "bob.near", amount: "100000" } }), "fund bob");
  must(call("ft_transfer", { pred: "alice.near", deposit: 1n, args: { receiver_id: DCL, amount: "500000" } }), "fund pool"); // a sell: taxed
  take(); // clear the vault so each case starts at 0
}

// ------------------------------------------------------------------------------------------------
console.log("wasm:", wasmPath);

// 1. the bug: collect between the sell and its failed swap must not keep the seller's tax
fresh();
{
  const a0 = bal("alice.near");
  const cb = sell("alice.near", 10000); // 4 % tax = 400, net 9600 to DCL
  check("1 sell: tax held in flight", inFlight() === 400n && bal(ME) === 400n, `in flight ${inFlight()} vault ${bal(ME)}`);
  check("1 get_tax pending excludes in-flight tax", pending() === 0n, `pending ${pending()}`);
  check("1 tax_take between transfer and resolve takes 0", take() === 0n);
  const r = must(resolve(cb, { failed: true }), "resolve");
  check("1 failed swap: seller whole (amount + tax back)", bal("alice.near") === a0, `alice ${a0} -> ${bal("alice.near")}`);
  check("1 failed swap: resolve reports 0 used", r.ret === '"0"', r.ret);
  check("1 in-flight key removed at 0", !storage.has("p"));
  check("1 nothing left to take", take() === 0n);
  invariants("1");
}

// 2. swap succeeds: the tax is released to the admin
fresh();
{
  const cb = sell("alice.near", 10000);
  const r = must(resolve(cb, { value: '"0"' }), "resolve");
  check("2 success: used = gross (amount + tax)", r.ret === '"10000"', r.ret);
  check("2 success: in flight released", inFlight() === 0n && !storage.has("p"));
  check("2 success: pending = tax", pending() === 400n, `pending ${pending()}`);
  check("2 success: admin takes the tax", take() === 400n);
  invariants("2");
}

// 3. partial refund: part of the trade happened; the tax on the part that came back goes back pro rata
fresh();
{
  const a0 = bal("alice.near");
  const cb = sell("alice.near", 10000); // net 9600, tax 400
  const r = must(resolve(cb, { value: '"4800"' }), "resolve");
  // 4800 of 9600 came back: half the tax (200) goes back with it
  check("3 partial: seller gets the unused part and its tax back", bal("alice.near") === a0 - 10000n + 4800n + 200n, `alice ${bal("alice.near")}`);
  check("3 partial: used = 9600 + 400 - 4800 - 200", r.ret === '"5000"', r.ret);
  check("3 partial: in flight released", inFlight() === 0n);
  check("3 partial: admin takes the tax on the used part", take() === 200n);
  invariants("3");
}

// 3b. partial refund on 18-decimal amounts, where tax * refund overflows u128 (the scaled mul_div)
{
  storage = new Map();
  const E = 10n ** 18n;
  must(call("new", { pred: ADMIN, args: { owner_id: "alice.near", total_supply: String(1_000_000_000n * E), metadata: { spec: "ft-1.0.0", name: "T", symbol: "T", decimals: 18 },
    tax: { buy_bps: 300, sell_bps: 400, pairs: [DCL], admin: ADMIN, exempt: ["locker.near"] } } }), "new 18");
  for (const a of ["bob.near", "carol.near", DCL, "locker.near"]) must(call("storage_deposit", { pred: a, deposit: 1_250_000_000_000_000_000_000n, args: {} }), "register " + a);
  const a0 = bal("alice.near");
  const gross = 300_000_000n * E + 123_456_789n; // 30 % of supply
  const cb = sell("alice.near", gross);
  const { amount, tax } = JSON.parse(cb);
  const net = BigInt(amount), t = BigInt(tax);
  check("3b tax * refund overflows u128", t * (net - 7n) >= 1n << 128n);
  const unused = net / 3n + 7n;
  const r = must(resolve(cb, { value: `"${unused}"` }), "resolve 18");
  const exact = (t * unused) / net;
  const back = bal("alice.near") - (a0 - gross) - unused;
  check("3b tax back within 1e-9 of exact pro rata, never above the tax", back <= t && (back > exact ? back - exact : exact - back) * 1_000_000_000n <= t, `back ${back} exact ${exact}`);
  check("3b used = gross - unused - tax back", BigInt(JSON.parse(r.ret)) === gross - unused - back, r.ret);
  check("3b admin takes the rest of the tax", take() === t - back);
  check("3b in flight released", !storage.has("p"));
  invariants("3b");
}

// 4. several in flight, plain taxed transfers in between, resolves out of order
fresh();
{
  const a0 = bal("alice.near"), b0 = bal("bob.near");
  const cbA = sell("alice.near", 10000); // tax 400
  const cbB = sell("bob.near", 5000); // tax 200
  must(call("ft_transfer", { pred: DCL, deposit: 1n, args: { receiver_id: "carol.near", amount: "1000" } }), "buy"); // buy tax 30, not in flight
  check("4 in flight = 400 + 200", inFlight() === 600n, `${inFlight()}`);
  check("4 vault = 630", bal(ME) === 630n, `${bal(ME)}`);
  check("4 tax_take takes only the plain buy tax", take() === 30n);
  invariants("4a");
  must(resolve(cbB, { value: '"0"' }), "resolve B"); // bob's swap went through
  check("4 after B resolves: in flight = 400", inFlight() === 400n, `${inFlight()}`);
  check("4 tax_take takes B's tax only", take() === 200n);
  invariants("4b");
  must(resolve(cbA, { failed: true }), "resolve A"); // alice's swap failed
  check("4 A refunded in full after B's tax was taken", bal("alice.near") === a0, `alice ${bal("alice.near")}`);
  check("4 B paid amount + tax", bal("bob.near") === b0 - 5000n);
  check("4 in flight back to 0", inFlight() === 0n && !storage.has("p"));
  check("4 vault empty, nothing to take", bal(ME) === 0n && take() === 0n);
  invariants("4c");
}

// 5. a resolve that fails (out of gas part way): its writes roll back, the tax stays held
fresh();
{
  const a0 = bal("alice.near");
  const cbA = sell("alice.near", 10000);
  const cbB = sell("bob.near", 5000);
  const r = resolve(cbA, { failed: true }, 3); // dies on its 3rd storage write
  check("5 resolve A failed", !r.ok, r.err);
  check("5 A's tax still held, vault untouched", inFlight() === 600n && bal(ME) === 600n, `in flight ${inFlight()} vault ${bal(ME)}`);
  check("5 tax_take cannot take a failed resolve's tax", take() === 0n);
  must(resolve(cbB, { failed: true }), "resolve B");
  check("5 B still refunded in full", bal("bob.near") === 100000n, `bob ${bal("bob.near")}`);
  check("5 A's 400 stays reserved in the vault", inFlight() === 400n && bal(ME) === 400n && take() === 0n);
  check("5 A lost the swap amount to the receiver (NEP-141 on a failed resolve), not more", bal("alice.near") === a0 - 10000n);
  invariants("5");
}

// 6. the token account (the vault) takes no transfers, so no transfer_call into it can be refunded out of
//    other transfers' in-flight tax
fresh();
{
  const v0 = bal(ME);
  for (const m of ["ft_transfer", "ft_transfer_call"]) {
    const r = call(m, { pred: "bob.near", deposit: 1n, args: { receiver_id: ME, amount: "300", msg: "" } });
    check(`6 ${m} into the vault refused`, !r.ok && /does not take transfers/.test(r.err) && bal(ME) === v0, r.err);
  }
  invariants("6");
}

// 7. seller unregisters before the failed swap resolves: refund burned, tax released to the admin
fresh();
{
  must(call("ft_transfer", { pred: "alice.near", deposit: 1n, args: { receiver_id: "carol.near", amount: "10000" } }), "fund carol");
  const cb = sell("carol.near", 10000);
  must(call("storage_unregister", { pred: "carol.near", deposit: 1n, args: { force: true } }), "unregister");
  const s0 = supply();
  must(resolve(cb, { failed: true }), "resolve");
  check("7 refund burned", supply() === s0 - 9600n);
  check("7 tax released and takeable", inFlight() === 0n && take() === 400n);
  invariants("7");
}

// 8. untaxed transfer_call (wallet to wallet) never touches the in-flight key
fresh();
{
  const cb = sell("alice.near", 1000, "bob.near");
  check("8 untaxed transfer_call: no in-flight key", !storage.has("p") && JSON.parse(cb).tax === "0");
  must(resolve(cb, { failed: true }), "resolve");
  check("8 untaxed refund", !storage.has("p") && bal("bob.near") === 100000n);
}

// 9. a receiver answer the token cannot read never fails the resolve: near-sdk's reading (refund all)
for (const [v, unused] of [['""', 9600n], ['"340282366920938463463374607431768211456"', 9600n], ['"' + "0".repeat(70) + '"', 9600n],
  ['"-1"', 9600n], ['"abc"', 9600n], ["5", 9600n], ["", 9600n], ["null", 9600n], ['"+4800"', 4800n], [' "4800"\n', 4800n], ['"0004800"', 4800n], ['"99999999"', 9600n]]) {
  fresh();
  const a0 = bal("alice.near");
  const cb = sell("alice.near", 10000);
  const r = resolve(cb, { value: v });
  const taxBack = unused === 9600n ? 400n : 200n;
  check(`9 answer ${JSON.stringify(v).slice(0, 30)}: resolve ok, unused ${unused} + its tax back`, r.ok && bal("alice.near") === a0 - 10000n + unused + taxBack && !storage.has("p"), r.ok ? `alice ${bal("alice.near")}` : r.err);
  invariants("9");
}

// 10. storage_withdraw (NEP-145): nothing is ever available
fresh();
{
  const ok0 = call("storage_withdraw", { pred: "bob.near", deposit: 1n, args: {} });
  check("10 storage_withdraw() returns the balance", ok0.ok && JSON.parse(ok0.ret).available === "0", ok0.err);
  const ok1 = call("storage_withdraw", { pred: "bob.near", deposit: 1n, args: { amount: "0" } });
  check("10 storage_withdraw(0) ok", ok1.ok, ok1.err);
  check("10 storage_withdraw(1) refused", !call("storage_withdraw", { pred: "bob.near", deposit: 1n, args: { amount: "1" } }).ok);
  check("10 storage_withdraw needs 1 yocto", !call("storage_withdraw", { pred: "bob.near", deposit: 0n, args: {} }).ok);
  check("10 storage_withdraw unregistered refused", !call("storage_withdraw", { pred: "zed.near", deposit: 1n, args: {} }).ok);
}

console.log(fails ? `\n${fails} FAILED` : "\nall passed");
process.exit(fails ? 1 : 0);

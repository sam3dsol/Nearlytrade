// Fuzzes the hand-written JSON argument parsing of the no-SDK token builds (raw and tax), side by side.
//
//   node contracts/token-tax/sim/fuzz_json.mjs [--iters N] [--seed S] [--scale] a.wasm b.wasm ...
//
// Every input runs on every build from the same starting state and is checked three ways:
//   1. safety: a bad input may only end in a guest panic; a wasm trap, a runaway loop (meter.mjs guard)
//      or a host error other than the expected ones is reported with the input.
//   2. reference: the args are also decoded the way a near-sdk token would (strict RFC 8259 JSON,
//      serde rules: duplicate fields refused, unknown fields ignored, AccountId / U128 checks). The token
//      accepting what the reference refuses is "lenient"; both accepting with a different reading
//      (receiver, amount, memo, msg) is a "DIFFERENTIAL".
//   3. invariants after each accepted call: balances sum to supply, every EVENT_JSON log and every
//      ft_on_transfer args blob is valid JSON, every registered account is a valid NEAR account id.
// The builds are also compared with each other (same outcome for the same input); --scale measures
// gas against input size for the worst shapes found.
import { readFileSync } from "node:fs";
import { basename } from "node:path";
import { Token, validAccount, TGAS } from "./lib/host.mjs";

const argv = process.argv.slice(2);
const opt = (name, def) => { const i = argv.indexOf(name); if (i < 0) return def; const v = argv[i + 1]; argv.splice(i, 2); return v; };
const ITERS = Number(opt("--iters", 3000));
let seed = Number(opt("--seed", 1));
const SCALE = argv.includes("--scale") ? (argv.splice(argv.indexOf("--scale"), 1), true) : false;
if (!argv.length) { console.error("usage: fuzz_json.mjs [--iters N] [--seed S] [--scale] token.wasm ..."); process.exit(2); }

const enc = new TextEncoder();
const utf8 = new TextDecoder("utf-8", { fatal: true });
const cat = (...parts) => Buffer.concat(parts.map((p) => (typeof p === "string" ? Buffer.from(p, "latin1") : Buffer.from(p))));
const show = (b, max = 160) => { const s = JSON.stringify(Buffer.from(b).toString("latin1")); return s.length > max ? s.slice(0, max) + `…(${b.length} B)` : s; };

// ---------------------------------------------------------------------------------------------
// seeded PRNG
// ---------------------------------------------------------------------------------------------
function rnd() { seed |= 0; seed = (seed + 0x6d2b79f5) | 0; let t = Math.imul(seed ^ (seed >>> 15), 1 | seed); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }
const ri = (n) => Math.floor(rnd() * n);
const pick = (a) => a[ri(a.length)];
const chance = (p) => rnd() < p;

// ---------------------------------------------------------------------------------------------
// strict reference JSON (bytes in; objects keep every pair, duplicates included)
// ---------------------------------------------------------------------------------------------
class JErr extends Error {}
function strictParse(b) {
  let i = 0;
  const ws = () => { while (i < b.length && (b[i] === 0x20 || b[i] === 0x09 || b[i] === 0x0a || b[i] === 0x0d)) i++; };
  const fail = (m) => { throw new JErr(m + " at " + i); };
  function str() {
    i++;
    const out = [];
    const flush = [];
    for (;;) {
      if (i >= b.length) fail("unterminated string");
      const c = b[i];
      if (c === 0x22) { i++; break; }
      if (c < 0x20) fail("control character in string");
      if (c === 0x5c) {
        const e = b[i + 1];
        const simple = { 0x22: '"', 0x5c: "\\", 0x2f: "/", 0x62: "\b", 0x66: "\f", 0x6e: "\n", 0x72: "\r", 0x74: "\t" }[e];
        if (simple !== undefined) { out.push(...enc.encode(simple)); i += 2; continue; }
        if (e !== 0x75) fail("invalid escape");
        const hex = Buffer.from(b.subarray(i + 2, i + 6)).toString("latin1");
        if (!/^[0-9a-fA-F]{4}$/.test(hex)) fail("invalid \\u escape");
        let cp = parseInt(hex, 16);
        i += 6;
        if (cp >= 0xd800 && cp <= 0xdbff) {
          const h2 = Buffer.from(b.subarray(i, i + 6)).toString("latin1");
          if (!/^\\u[dD][c-fC-F][0-9a-fA-F]{2}$/.test(h2)) fail("lone surrogate");
          cp = 0x10000 + ((cp - 0xd800) << 10) + (parseInt(h2.slice(2), 16) - 0xdc00);
          i += 6;
        } else if (cp >= 0xdc00 && cp <= 0xdfff) fail("lone surrogate");
        out.push(...enc.encode(String.fromCodePoint(cp)));
        continue;
      }
      out.push(c); i++;
    }
    try { return { t: "str", v: utf8.decode(Uint8Array.from(out)) }; } catch { fail("invalid utf-8"); }
    return flush;
  }
  function val() {
    ws();
    if (i >= b.length) fail("eof");
    const c = b[i];
    if (c === 0x7b) {
      i++; ws();
      const pairs = [];
      if (b[i] === 0x7d) { i++; return { t: "obj", pairs }; }
      for (;;) {
        ws();
        if (b[i] !== 0x22) fail("expected key");
        const k = str().v;
        ws();
        if (b[i] !== 0x3a) fail("expected :");
        i++;
        pairs.push([k, val()]);
        ws();
        if (b[i] === 0x2c) { i++; continue; }
        if (b[i] === 0x7d) { i++; return { t: "obj", pairs }; }
        fail("expected , or }");
      }
    }
    if (c === 0x5b) {
      i++; ws();
      const items = [];
      if (b[i] === 0x5d) { i++; return { t: "arr", items }; }
      for (;;) {
        items.push(val()); ws();
        if (b[i] === 0x2c) { i++; continue; }
        if (b[i] === 0x5d) { i++; return { t: "arr", items }; }
        fail("expected , or ]");
      }
    }
    if (c === 0x22) return str();
    const rest = Buffer.from(b.subarray(i, i + 400)).toString("latin1");
    let m;
    if ((m = /^(true|false|null)/.exec(rest))) { i += m[0].length; return { t: m[0] === "null" ? "null" : "bool", v: m[0] === "true" }; }
    if ((m = /^-?(0|[1-9]\d*)(\.\d+)?([eE][+-]?\d+)?/.exec(rest))) { i += m[0].length; return { t: "num", v: m[0] }; }
    fail("unexpected byte 0x" + c.toString(16));
  }
  try {
    const v = val(); ws();
    if (i !== b.length) fail("trailing bytes");
    return { ok: true, v };
  } catch (e) {
    if (e instanceof JErr) return { ok: false, err: e.message.replace(/ at \d+$/, "") };
    throw e;
  }
}
const U128_MAX = (1n << 128n) - 1n;
/// Rust u128::from_str, which near-sdk's U128 uses: optional '+', ASCII digits, no overflow
const rustU128 = (s) => (/^\+?\d+$/.test(s) && BigInt(s) <= U128_MAX ? BigInt(s) : null);

/// decode args as a near-sdk method with these fields would. fields: {name: [type, required]}
function refDecode(fields, bytes) {
  const p = strictParse(bytes);
  if (!p.ok) return { ok: false, why: "invalid json: " + p.err };
  if (p.v.t !== "obj") return { ok: false, why: "not an object" };
  const out = {}, seen = new Set(), last = {};
  for (const [k, v] of p.v.pairs) {
    if (!(k in fields)) continue;
    last[k] = v;
    if (seen.has(k)) return { ok: false, why: "duplicate field", dupKey: k, pairs: p.v.pairs };
    seen.add(k);
    const [type] = fields[k];
    if (v.t === "null" && !fields[k][1]) { out[k] = null; continue; }
    if (type === "bool") { if (v.t !== "bool") return { ok: false, why: `${k}: not a bool` }; out[k] = v.v; continue; }
    if (type === "any") { out[k] = v; continue; }
    if (v.t !== "str") return { ok: false, why: `${k}: not a string` };
    if (type === "account" && !validAccount(v.v)) return { ok: false, why: `${k}: invalid AccountId` };
    if (type === "u128") { const n = rustU128(v.v); if (n === null) return { ok: false, why: `${k}: invalid u128` }; out[k] = n; continue; }
    out[k] = v.v;
  }
  for (const [k, [, req]] of Object.entries(fields)) if (req && !(k in out)) return { ok: false, why: "missing field " + k };
  return { ok: true, args: out };
}

// ---------------------------------------------------------------------------------------------
// builds and their starting state
// ---------------------------------------------------------------------------------------------
const ME = "tok.near", DCL = "dclv2.ref-labs.near", ADMIN = "factory.near", REG = 1_250_000_000_000_000_000_000n;
const builds = argv.map((p) => {
  const t = new Token(readFileSync(p), { me: ME });
  const tax = t.exports.has("tax_take");
  return { label: basename(p).replace(/\.wasm$/, ""), path: p, t, tax };
});
const NEW_ARGS = (tax) => ({
  owner_id: "alice.near", total_supply: "1000000",
  metadata: { spec: "ft-1.0.0", name: "T", symbol: "T", decimals: 0 },
  ...(tax ? { tax: { buy_bps: 300, sell_bps: 400, pairs: [DCL], admin: ADMIN, exempt: ["locker.near"] } } : {}),
});
for (const B of builds) {
  const must = (m, o) => { const r = B.t.call(m, o); if (!r.ok) throw new Error(`${B.label} setup ${m}: ${r.err}`); return r; };
  must("new", { pred: ADMIN, args: NEW_ARGS(B.tax) });
  for (const a of ["bob.near", "carol.near", DCL, ...(B.tax ? [] : [ME, ADMIN])]) must("storage_deposit", { pred: a, deposit: REG, args: {} });
  must("ft_transfer", { pred: "alice.near", deposit: 1n, args: { receiver_id: "bob.near", amount: "100000" } });
  must("ft_transfer", { pred: "alice.near", deposit: 1n, args: { receiver_id: DCL, amount: "500000" } });
  B.base = B.t.snapshot();
}
const le = (v) => { let n = 0n; for (let i = 15; i >= 0; i--) n = (n << 8n) | BigInt(v[i]); return n; };
const balances = (B) => { const m = {}; for (const [k, v] of B.t.storage) if (k[0] === "b") m[k.slice(1)] = le(v); return m; };

// ---------------------------------------------------------------------------------------------
// generators
// ---------------------------------------------------------------------------------------------
const L = (s) => Buffer.from(s, "latin1");
const HEX64 = "9d7242b3".repeat(8);
const ACCOUNTS = [
  ["bob.near", 6], ["carol.near", 3], ["dave.near", 3], [DCL, 3], [HEX64, 1], ["0x" + "ab".repeat(20), 1], ["a-b_c.near", 1], [ME, 1], [ADMIN, 1],
  ["..", 1], ["a..b", 1], [".bob", 1], ["bob.", 1], ["-bob", 1], ["bob-", 1], ["a__b", 1], ["a-_b", 1], ["", 1], ["a", 1], ["Bob.near", 1],
  ["bob near", 1], ["b".repeat(65), 1], ["bob\\u002enear", 1], ["bob\\u0000", 1], ["alice.near", 1],
].flatMap(([a, w]) => Array(w).fill(a));
const GOOD_ACCOUNTS = ["bob.near", "carol.near", "dave.near", DCL, HEX64, "a-b_c.near"];
const acct = () => (chance(0.6) ? `"${pick(GOOD_ACCOUNTS)}"` : chance(0.9) ? `"${pick(ACCOUNTS)}"` : pick(["null", "5", "true", "{}", `["bob.near"]`, `"bob.near`, `bob.near`]));
const AMOUNTS = ["1", "5", "100", "99999", "100000", "100001", "0", "00", "007", "+1", "-1", " 1", "1 ", "1e3", "0x10", "1.0", "", "１",
  String(U128_MAX), String(U128_MAX + 1n), "9".repeat(39), "1" + "0".repeat(39), "9".repeat(100), "1\\u0030"];
const amt = () => (chance(0.6) ? `"${pick(["1", "5", "100", "99999", "007"])}"` : chance(0.9) ? `"${pick(AMOUNTS)}"` : pick(["5", "null", "true", "[]", `{"x":"5"}`, `"5`, "-0", "5e0"]));
const STRINGS = ["", "hi", "a\\\"b", "\\\\", "\\/", "\\u0000", "\\ud800", "\\udc00x", "\\ud83d\\ude00", "\\x", "\\u12", "\\u12zz", "\xc3\xa9", "a\x01b",
  "a\nb", "a\tb", "\x7f", "\xff", "\xc3", "\xed\xa0\x80", "}]}", "EVENT_JSON:{}", "\\n\\t", "\\\"},{\\\"a\\\":\\\"", "\\\\\\\"", "x".repeat(300)];
const text = () => {
  if (chance(0.03)) return `"${"m".repeat(pick([5000, 17000]))}"`;
  return chance(0.5) ? `"${pick(["", "hi", "swap", "a\\\"b"])}"` : chance(0.9) ? `"${pick(STRINGS)}"` : pick(["null", "5", "true", "{}", `{"a":"}"}`, `["x"]`]);
};
const junk = () => pick([`"x"`, "1", "-2.5e3", "true", "null", "[]", "{}", `{"receiver_id":"carol.near","amount":"7"}`, `[1,{"a":[2,"]"]}]`, `"}\\"]"`, `{"a":{"b":{"c":[[[["}"]]]]}}}`]);

/// method table: fields as near-sdk would declare them + how to call it
const METHODS = {
  ft_transfer: { fields: { receiver_id: ["account", 1], amount: ["u128", 1], memo: ["string", 0] }, pred: "alice.near", deposit: 1n },
  ft_transfer_call: { fields: { receiver_id: ["account", 1], amount: ["u128", 1], memo: ["string", 0], msg: ["string", 1] }, pred: "alice.near", deposit: 1n },
  dcl_payout: { method: "ft_transfer", fields: { receiver_id: ["account", 1], amount: ["u128", 1], memo: ["string", 0] }, pred: DCL, deposit: 1n },
  storage_deposit: { fields: { account_id: ["account", 0], registration_only: ["bool", 0] }, pred: "carol.near", deposit: REG },
  storage_unregister: { fields: { force: ["bool", 0] }, pred: "bob.near", deposit: 1n },
  burn: { fields: { amount: ["u128", 1] }, pred: "bob.near", deposit: 1n },
  ft_balance_of: { fields: { account_id: ["account", 1] }, pred: "x.near", deposit: 0n },
  storage_balance_of: { fields: { account_id: ["account", 1] }, pred: "x.near", deposit: 0n },
};
const VALUE = { account: acct, u128: amt, string: text, bool: () => pick(["true", "false", "null", `"true"`, "1", "tru"]) };
const KEYVAR = (k) => (chance(0.97) ? `"${k}"` : pick([`"${k.replace("_", "\\u005f")}"`, `"${k} "`, `"${k.toUpperCase()}"`, `"\\u0000${k}"`]));

function genStructured(M) {
  const pairs = [];
  for (const [k, [type]] of Object.entries(M.fields)) if (chance(0.92)) pairs.push([KEYVAR(k), VALUE[type]()]);
  if (chance(0.3)) for (let n = 1 + ri(3); n; n--) pairs.push([`"${pick(["x", "extra", "data", "memo2", "a b"])}"`, junk()]);
  if (chance(0.2) && pairs.length) { const [k] = pick(pairs); const v = VALUE[M.fields[JSON.parse(k.replace(/\\u005f/, "_").replace(/\\u0000/, "")) ?? ""]?.[0] ?? "string"]?.() ?? junk(); pairs.splice(ri(pairs.length + 1), 0, [k, v]); }
  if (chance(0.5)) for (let i = pairs.length - 1; i > 0; i--) { const j = ri(i + 1); [pairs[i], pairs[j]] = [pairs[j], pairs[i]]; }
  const dirty = chance(0.35); // otherwise the frame is valid JSON and only the values vary
  const W = () => (dirty && chance(0.2) ? pick(["\x0b", "\xa0", "\x0c"]) : pick(["", "", "", " ", "\n", "\t", "\r\n  "]));
  const sep = () => (dirty && chance(0.2) ? pick(["", ",,", " ,,"]) : pick([",", ", ", " ,"]));
  const colon = () => (dirty && chance(0.1) ? pick(["", "::", "="]) : pick([":", " : "]));
  const close = () => (dirty && chance(0.4) ? pick(["", "]", "}}", "},", "} x"]) : "}");
  let s = W() + "{" + W() + pairs.map(([k, v]) => k + W() + colon() + W() + v).map((x, i) => (i ? sep() + W() : "") + x).join("") + W() + close() + W();
  if (chance(0.05)) s = pick(["", "null", "[]", `["bob.near","5"]`, "{", "}", `"x"`]);
  if (chance(0.05)) s += pick(["garbage", `{"amount":"1"}`, "\x00"]);
  return L(s);
}
const INTERESTING = L(`"{}[]:,\\ \n\t0123456789aeu+-.\x00\x01\x7f\xff\xc3`);
function mutate(b) {
  let x = Buffer.from(b);
  for (let n = 1 + ri(4); n; n--) {
    const p = ri(x.length + 1);
    switch (ri(6)) {
      case 0: x = cat(x.subarray(0, p), x.subarray(Math.min(x.length, p + 1 + ri(4)))); break;
      case 1: x = cat(x.subarray(0, p), Uint8Array.of(INTERESTING[ri(INTERESTING.length)]), x.subarray(p)); break;
      case 2: if (x.length) x[Math.min(p, x.length - 1)] = INTERESTING[ri(INTERESTING.length)]; break;
      case 3: x = x.subarray(0, p); break;
      case 4: { const q = ri(x.length + 1); const [a, z] = p < q ? [p, q] : [q, p]; x = cat(x.subarray(0, z), x.subarray(a, z), x.subarray(z)); break; }
      case 5: x = cat(x.subarray(0, p), L(pick([`\\"`, `\\`, `"`, `\\u`, "}", "]", `,"amount":"9"`, `,"receiver_id":"carol.near"`])), x.subarray(p)); break;
    }
  }
  return x;
}

// ---------------------------------------------------------------------------------------------
// reading what the token did
// ---------------------------------------------------------------------------------------------
const EV = /^EVENT_JSON:\{"standard":"nep141","version":"1\.0\.0","event":"ft_transfer","data":\[\{"old_owner_id":"([^"]*)","new_owner_id":"([^"]*)","amount":"(\d+)"(?:,"memo":"([\s\S]*)")?\}\]\}$/;
/// a string body as the token copied it (still escaped) → its decoded value, or {invalid} when it is not valid JSON
const rawStr = (b) => { const p = strictParse(cat('"', b, '"')); return p.ok ? p.v.v : { invalid: Buffer.from(b).toString("latin1") }; };
function tokenReading(M, B, before, r) {
  const after = balances(B);
  const ev = r.logs.map((l) => EV.exec(l)).find((m) => m && m[1] === M.pred);
  const o = {};
  if ("receiver_id" in M.fields && ev) {
    o.receiver_id = ev[2];
    o.amount = (before[M.pred] ?? 0n) - (after[M.pred] ?? 0n);
    o.memo = ev[4] === undefined ? null : rawStr(Buffer.from(ev[4], "utf8"));
  }
  if (M.fields.msg) {
    const call = r.promises.flatMap((p) => p.actions).find((a) => a.method === "ft_on_transfer");
    const m = call && /"msg":"([\s\S]*)"\}$/.exec(Buffer.from(call.args).toString("latin1"));
    o.msg = m ? rawStr(Buffer.from(m[1], "latin1")) : undefined;
  }
  if (M.fields.account_id && M === METHODS.storage_deposit) o.account_id = Object.keys(after).find((a) => !(a in before)) ?? "(already registered)";
  if (M.fields.amount && !M.fields.receiver_id) o.amount = (before[M.pred] ?? 0n) - (after[M.pred] ?? 0n);
  return o;
}

// ---------------------------------------------------------------------------------------------
// run
// ---------------------------------------------------------------------------------------------
const stats = {}; // label/method -> counters
const findings = new Map(); // class -> {count, input, detail, builds:Set}
function note(cls, input, detail, label) {
  let f = findings.get(cls);
  if (!f) findings.set(cls, (f = { count: 0, input, detail, builds: new Set() }));
  f.count++; f.builds.add(label);
  if (input.length < f.input.length) { f.input = input; f.detail = detail; }
}
const eqReading = (a, b) => JSON.stringify(a, (_k, v) => (typeof v === "bigint" ? v.toString() : v)) === JSON.stringify(b, (_k, v) => (typeof v === "bigint" ? v.toString() : v));

function runOne(name, M, input) {
  const outcomes = [];
  for (const B of builds) {
    B.t.restore(B.base);
    const before = balances(B);
    const r = B.t.call(M.method ?? name, { pred: M.pred, deposit: M.deposit, args: input });
    const s = (stats[B.label + " " + name] ??= { runs: 0, ok: 0, panic: {}, trap: 0, host: {}, gas: 0, maxOps: 0n, maxGas: 0n, maxIn: 0 });
    s.runs++;
    if (r.ops > s.maxOps) s.maxOps = r.ops;
    if (r.gas > s.maxGas) { s.maxGas = r.gas; s.maxIn = input.length; }
    if (r.ok) s.ok++;
    else if (r.kind === "panic") s.panic[r.err] = (s.panic[r.err] || 0) + 1;
    else if (r.kind === "host") { const k = r.err.split(" ")[0]; s.host[k] = (s.host[k] || 0) + 1; }
    else if (r.kind === "trap") { s.trap++; note(`TRAP ${name}: ${r.err}`, input, r.err, B.label); }
    else { s.gas++; note(`RUNAWAY ${name}`, input, `${r.ops} ops`, B.label); }
    if (r.gas > 300n * TGAS) note(`OVER 300 TGAS ${name}`, input, `${Number(r.gas) / 1e12} TGas`, B.label);
    let reading = null;
    if (r.ok) {
      const after = balances(B);
      const sum = Object.values(after).reduce((a, b) => a + b, 0n);
      const supply = le(B.t.storage.get("s"));
      if (sum !== supply) note(`INVARIANT supply ${name}`, input, `sum ${sum} supply ${supply}`, B.label);
      for (const [k, v] of B.t.storage) if (k[0] === "b" && v.length < 16) note(`INVARIANT short balance record`, input, k, B.label);
      for (const a of Object.keys(after)) if (!(a in before) && !validAccount(a)) note(`registers an account id NEAR rejects (${name})`, input, JSON.stringify(a), B.label);
      for (const l of r.logs) if (l.startsWith("EVENT_JSON:") && !strictParse(Buffer.from(l.slice(11), "utf8")).ok)
        note(`EVENT_JSON log is not valid JSON (${name})`, input, l.slice(0, 200), B.label);
      for (const a of r.promises.flatMap((p) => p.actions)) if (a.args && !strictParse(a.args).ok) note(`${a.method} args are not valid JSON (${name})`, input, show(a.args), B.label);
      reading = tokenReading(M, B, before, r);
    }
    outcomes.push({ B, r, reading });
  }
  // reference comparison (on the first build; the parser is shared, differences between builds are reported below)
  const ref = refDecode(M.fields, input);
  const o0 = outcomes[0];
  if (o0.r.ok && !ref.ok) {
    let cls = `lenient ${name}: ${ref.why}`, continue_dup = false;
    if (ref.why === "duplicate field" && o0.reading) {
      const vals = ref.pairs.filter(([k]) => k === ref.dupKey).map(([, v]) => v.v);
      const got = o0.reading[ref.dupKey];
      if (got === undefined) { note(`lenient ${name}: duplicate "${ref.dupKey}"`, input, ref.why, o0.B.label); continue_dup = true; }
      const which = eqReading(got, vals[0]) || (ref.dupKey === "amount" && got === rustU128(String(vals[0]))) ? "first" : eqReading(got, vals.at(-1)) ? "last" : "other";
      cls = `DIFFERENTIAL ${name}: duplicate "${ref.dupKey}", token uses the ${which}, JSON.parse shows the last`;
      if (continue_dup) return compareBuilds(name, outcomes, input);
      if (vals.length > 1 && eqReading(vals[0], vals.at(-1))) cls = `lenient ${name}: duplicate "${ref.dupKey}" (same value)`;
    }
    note(cls, input, ref.why, o0.B.label);
  } else if (!o0.r.ok && ref.ok && o0.r.kind !== "host") {
    note(`stricter ${name}: ${o0.r.err}`, input, "reference accepts", o0.B.label);
  } else if (o0.r.ok && ref.ok) {
    for (const [k, v] of Object.entries(o0.reading)) {
      if (k === "account_id" && v === "(already registered)") continue;
      const want = k === "account_id" ? ref.args.account_id ?? M.pred : ref.args[k] ?? (k === "memo" ? null : undefined);
      if (!eqReading(v, want)) note(`DIFFERENTIAL ${name}: ${k} read differently`, input, `token ${JSON.stringify(v, (_k, x) => (typeof x === "bigint" ? String(x) : x))} vs reference ${JSON.stringify(want, (_k, x) => (typeof x === "bigint" ? String(x) : x))}`, o0.B.label);
    }
  }
  compareBuilds(name, outcomes, input);
}
/// builds against each other: same accept/reject + same panic message; same family: same reading too
function compareBuilds(name, outcomes, input) {
  const o0 = outcomes[0];
  const sig = (o) => (o.r.ok ? "ok" : o.r.kind + ":" + o.r.err);
  for (const o of outcomes.slice(1)) {
    if (sig(o) !== sig(o0)) note(`BUILDS DISAGREE ${name}: ${o0.B.label}=${sig(o0)} / ${o.B.label}=${sig(o)}`, input, "", o.B.label);
  }
  for (const o of outcomes) {
    const mate = outcomes.find((x) => x !== o && x.B.tax === o.B.tax);
    if (mate && o.r.ok && mate.r.ok && builds.indexOf(mate.B) < builds.indexOf(o.B) && !eqReading(o.reading, mate.reading))
      note(`BUILDS READ DIFFERENTLY ${name}: ${mate.B.label} vs ${o.B.label}`, input, JSON.stringify([mate.reading, o.reading], (_k, x) => (typeof x === "bigint" ? String(x) : x)), o.B.label);
  }
}

/// ft_resolve_transfer with a fuzzed promise result (what the receiver's ft_on_transfer returned)
function runResolve(value) {
  for (const B of builds) {
    B.t.restore(B.base);
    const args = { sender_id: "alice.near", receiver_id: "bob.near", amount: "1000", ...(B.tax ? { tax: "0" } : {}) };
    const r = B.t.call("ft_resolve_transfer", { pred: ME, args, result: value === null ? { failed: true } : { value } });
    const s = (stats[B.label + " ft_resolve_transfer(result)"] ??= { runs: 0, ok: 0, panic: {}, trap: 0, host: {}, gas: 0, maxOps: 0n, maxGas: 0n, maxIn: 0 });
    s.runs++;
    if (r.ops > s.maxOps) s.maxOps = r.ops;
    if (r.gas > s.maxGas) { s.maxGas = r.gas; s.maxIn = value?.length ?? 0; }
    if (!r.ok) { s.panic[r.err] = (s.panic[r.err] || 0) + 1; note(`resolve fails on a promise result: ${r.err}`, value ?? L(""), "", B.label); continue; }
    s.ok++;
    const used = BigInt(JSON.parse(Buffer.from(r.ret).toString()));
    // near-contract-standards: serde_json::from_slice::<U128>(value) → min(amount, unused), anything else → refund all
    let refUnused = 1000n;
    const p = value && strictParse(value);
    if (p?.ok && p.v.t === "str" && rustU128(p.v.v) !== null) refUnused = rustU128(p.v.v) < 1000n ? rustU128(p.v.v) : 1000n;
    const refUsed = 1000n - refUnused;
    if (used !== refUsed) note(`resolve reads the receiver's return differently from near-sdk`, value, `token used ${used} vs reference ${refUsed}`, B.label);
  }
}

console.log("builds:", builds.map((b) => `${b.label}${b.tax ? " (tax)" : " (raw)"}`).join(", "), `| iters ${ITERS}/method, seed ${seed}`);
const t0 = Date.now();
const SEEDS = {
  ft_transfer: [
    `{"receiver_id":"carol.near","amount":"5","receiver_id":"bob.near"}`,
    `{"receiver_id":"bob.near","amount":"1","amount":"99999"}`,
    `{"receiver_id":"bob.near","receiver\\u005fid":"carol.near","amount":"5"}`,
    `{"receiver_id":"bob.near","amount":"5"} trailing garbage`,
    `{"receiver_id":"bob.near","amount":"5"`,
    `{"receiver_id":"bob.near" "amount":"5" "memo" "x" ]`,
    `{"receiver_id":"bob.near","amount":"5","memo":"\\u12"}`,
    `{"receiver_id":"bob.near","amount":"5","memo":"\\ud800"}`,
    `{"receiver_id":"bob.near","amount":"5","memo":"a\x01b"}`,
    `{"receiver_id":"bob.near","amount":"5","memo":"\\x"}`,
    `{"receiver_id":"bob.near","amount":"+5"}`, `{"receiver_id":"bob.near","amount":"0005"}`, `{"receiver_id":"bob.near","amount":5}`,
  ],
  ft_transfer_call: [
    `{"receiver_id":"bob.near","amount":"5","msg":"a\nb"}`,
    `{"receiver_id":"bob.near","amount":"5","msg":"\\u12"}`,
    `{"receiver_id":"dclv2.ref-labs.near","amount":"5","msg":"","receiver_id":"bob.near"}`,
  ],
  storage_deposit: [`{"account_id":".."}`, `{"account_id":"a..b"}`, `{"account_id":"-bob"}`, `{"account_id":"bob.near."}`, `{"registration_only":"yes"}`, `{`],
  storage_unregister: [`{"force":true,"force":false}`, `{"force":false,"force":true}`, `{"force":"true"}`],
  burn: [`{"amount":"1","amount":"99999"}`],
};
const corpus = {};
for (const [name, M] of Object.entries(METHODS)) {
  corpus[name] = (SEEDS[name] ?? []).map(L);
  for (const input of corpus[name]) runOne(name, M, input);
  for (let n = 0; n < ITERS; n++) {
    let input = genStructured(M);
    if (chance(0.3) && corpus[name].length) input = mutate(pick(corpus[name]));
    else if (chance(0.03)) input = Buffer.from(Array.from({ length: ri(40) }, () => ri(256)));
    if (corpus[name].length < 300) corpus[name].push(input); else corpus[name][ri(300)] = input;
    runOne(name, M, input);
  }
}
const RESULTS = [`""`, `"0"`, `"1000"`, `"5"`, `"99999"`, `"0005"`, `"+5"`, ` "5"`, `"5" `, `5`, `"5\\u0030"`, `""`, `"-1"`, `null`, `"${U128_MAX}"`, `"${U128_MAX + 1n}"`, `"${"9".repeat(40)}"`, `"abc"`, `{}`, ""];
for (const v of RESULTS) runResolve(L(v));
runResolve(null);
for (let n = 0; n < ITERS / 4; n++) runResolve(mutate(L(pick(RESULTS))));
runResolve(L(`"${"0".repeat(200_000)}"`));

// ---------------------------------------------------------------------------------------------
// report
// ---------------------------------------------------------------------------------------------
console.log(`\n${Object.values(stats).reduce((a, s) => a + s.runs, 0)} calls in ${((Date.now() - t0) / 1000).toFixed(1)} s\n`);
console.log("build / method".padEnd(52), "runs    ok  panic  trap runaway host         max ops   max TGas");
for (const [k, s] of Object.entries(stats)) {
  const panics = Object.values(s.panic).reduce((a, b) => a + b, 0), hosts = Object.values(s.host).reduce((a, b) => a + b, 0);
  console.log(k.padEnd(52), String(s.runs).padStart(4), String(s.ok).padStart(5), String(panics).padStart(6), String(s.trap).padStart(5), String(s.gas).padStart(7), String(hosts).padStart(4), String(s.maxOps).padStart(15), (Number(s.maxGas) / 1e12).toFixed(2).padStart(10), `(${s.maxIn} B)`);
}
const order = (c) => (/^(TRAP|RUNAWAY|OVER|INVARIANT)/.test(c) ? 0 : /^DIFFERENTIAL|BUILDS/.test(c) ? 1 : /not valid JSON|registers|resolve/.test(c) ? 2 : /^lenient/.test(c) ? 3 : 4);
console.log(`\nfindings (${findings.size} classes; shortest input shown):`);
for (const [cls, f] of [...findings].sort((a, b) => order(a[0]) - order(b[0]) || a[0].localeCompare(b[0]))) {
  console.log(`\n[${f.count}x ${[...f.builds].join(",")}] ${cls}\n    input ${show(f.input)}${f.detail ? "\n    " + f.detail.slice(0, 300) : ""}`);
}

// ---------------------------------------------------------------------------------------------
// gas vs input size for the heaviest shapes (the caller pays; this checks nothing is super-linear)
// ---------------------------------------------------------------------------------------------
if (SCALE) {
  console.log("\ngas vs input size (ft_transfer_call; key searched last / deep nesting / escape-heavy):");
  const shapes = {
    "many keys first": (n) => `{${`"k":"v",`.repeat(n / 8)}"receiver_id":"bob.near","amount":"1","msg":"m"}`,
    "deep nesting": (n) => `{"x":${"[".repeat(n / 2)}${"]".repeat(n / 2)},"receiver_id":"bob.near","amount":"1","msg":"m"}`,
    "escaped msg": (n) => `{"receiver_id":"bob.near","amount":"1","msg":"${"\\\"".repeat(n / 2)}"}`,
    "long memo": (n) => `{"receiver_id":"bob.near","amount":"1","msg":"m","memo":"${"m".repeat(n)}"}`,
  };
  for (const [shape, f] of Object.entries(shapes)) {
    for (const B of [builds[0], builds.at(-1)]) {
      const row = [];
      for (const n of [1_000, 100_000, 1_500_000, 4_000_000]) {
        B.t.restore(B.base);
        const input = L(f(n));
        const r = B.t.call("ft_transfer_call", { pred: "alice.near", deposit: 1n, args: input });
        row.push(`${(input.length / 1e3).toFixed(0)} KB: ${(Number(r.gas) / 1e12).toFixed(2)} TGas ${(Number(r.ops) / input.length).toFixed(1)} ops/B ${r.ok ? "ok" : r.kind + " " + r.err.slice(0, 30)}`);
      }
      console.log(`  ${shape.padEnd(16)} ${B.label.padEnd(14)} ${row.join(" | ")}`);
    }
  }
}
process.exitCode = [...findings.keys()].some((c) => /^(TRAP|RUNAWAY|OVER|INVARIANT)/.test(c)) ? 1 : 0;

// A minimal in-memory NEAR host for the no-SDK token builds, with gas metering (see meter.mjs).
//
// Per call: a fresh instance (fresh memory, like nearcore), a panic or trap rolls back that call's storage
// writes, promises are recorded (not run). Host checks kept where the tokens can reach them: log_utf8 needs
// valid UTF-8 and at most 100 logs / 16 KiB in total, promise_batch_create needs a valid NEAR account id,
// registers and guest memory are bounds checked. Gas = wasm instructions x wasm_regular_op_cost + the main
// ext costs (register, memory, storage, log, utf8). The ext cost values follow nearcore's parameters.yaml;
// contract loading, trie-node touches and action costs are left out, so treat the TGas as a lower bound.
import { instrument } from "./meter.mjs";

export const REGULAR_OP = 822_756n;
const C = {
  base: 264_768_111n,
  read_memory_base: 2_609_863_200n, read_memory_byte: 3_801_333n,
  write_memory_base: 2_803_794_861n, write_memory_byte: 2_723_772n,
  read_register_base: 2_517_165_186n, read_register_byte: 98_562n,
  write_register_base: 2_865_522_486n, write_register_byte: 3_801_564n,
  utf8_decoding_base: 3_111_779n, utf8_decoding_byte: 291_580_479n,
  log_base: 3_543_313_050n, log_byte: 13_198_791n,
  storage_write_base: 64_196_736_000n, storage_write_key_byte: 70_482_867n, storage_write_value_byte: 31_018_539n, storage_write_evicted_byte: 32_117_595n,
  storage_read_base: 56_356_845_750n, storage_read_key_byte: 30_952_533n, storage_read_value_byte: 5_611_005n,
  storage_remove_base: 53_473_030_500n, storage_remove_key_byte: 38_220_384n, storage_remove_ret_value_byte: 11_531_556n,
  storage_has_key_base: 54_039_896_625n, storage_has_key_byte: 30_790_845n,
};
export const TGAS = 1_000_000_000_000n;
const MAX_LOGS = 100, MAX_LOG_TOTAL = 16384;

/// nearcore AccountId rules: 2..64 chars, parts of [a-z0-9] joined by single '-'/'_', parts separated by '.'
export const validAccount = (s) => s.length >= 2 && s.length <= 64 && /^(([a-z\d]+[-_])*[a-z\d]+\.)*([a-z\d]+[-_])*[a-z\d]+$/.test(s);

export class Panic extends Error {}
export class HostError extends Error {}

const enc = new TextEncoder();
const utf8 = new TextDecoder("utf-8", { fatal: true });
const lat = (b) => Buffer.from(b).toString("latin1");
const bytes = (x) => (x instanceof Uint8Array ? x : enc.encode(typeof x === "string" ? x : JSON.stringify(x)));
const u128le = (n) => { const b = new Uint8Array(16); for (let i = 0; i < 16; i++) { b[i] = Number(n & 0xffn); n >>= 8n; } return b; };

export class Token {
  /// wasm: raw bytes of the token build; me: the token's account id
  constructor(wasm, { me = "tok.near", balance = 10n ** 25n, opLimit = 400_000_000n } = {}) {
    this.mod = new WebAssembly.Module(instrument(wasm));
    this.me = me;
    this.accountBalance = balance;
    this.opLimit = opLimit; // runaway guard: ~330 TGas of plain instructions
    this.storage = new Map(); // latin1(key) -> Uint8Array
    this.exports = new Set(WebAssembly.Module.exports(this.mod).map((e) => e.name));
  }
  /// call `method`. args: object | string | Uint8Array. result: promise result for callbacks ({failed:true} or {value})
  call(method, { pred, args = "", deposit = 0n, result = null } = {}) {
    const input = bytes(args);
    const regs = new Map();
    const logs = [], promises = [];
    let logTotal = 0, ret = null, mem, gas = C.base, inst;
    const charge = (g) => { gas += g; };
    const u8 = () => new Uint8Array(mem.buffer);
    const rd = (len, ptr) => {
      len = Number(len); ptr = Number(ptr);
      if (ptr + len > mem.buffer.byteLength) throw new HostError("MemoryAccessViolation");
      charge(C.read_memory_base + C.read_memory_byte * BigInt(len));
      return u8().slice(ptr, ptr + len);
    };
    const wr = (ptr, b) => {
      ptr = Number(ptr);
      if (ptr + b.length > mem.buffer.byteLength) throw new HostError("MemoryAccessViolation");
      charge(C.write_memory_base + C.write_memory_byte * BigInt(b.length));
      u8().set(b, ptr);
    };
    const setReg = (r, b) => { charge(C.write_register_base + C.write_register_byte * BigInt(b.length)); regs.set(r, b); };
    const str = (len, ptr) => {
      const b = rd(len, ptr);
      charge(C.utf8_decoding_base + C.utf8_decoding_byte * BigInt(b.length));
      try { return utf8.decode(b); } catch { throw new HostError("BadUTF8"); }
    };
    const account = (len, ptr) => {
      const s = str(len, ptr);
      if (!validAccount(s)) throw new HostError("InvalidAccountId " + JSON.stringify(s));
      return s;
    };
    const st = this.storage;
    const env = {
      register_len: (r) => (regs.has(r) ? BigInt(regs.get(r).length) : 0xffffffffffffffffn),
      read_register: (r, ptr) => {
        if (!regs.has(r)) throw new HostError("InvalidRegisterId");
        const b = regs.get(r);
        charge(C.read_register_base + C.read_register_byte * BigInt(b.length));
        wr(ptr, b);
      },
      input: (r) => setReg(r, input),
      predecessor_account_id: (r) => setReg(r, enc.encode(pred)),
      signer_account_id: (r) => setReg(r, enc.encode(pred)),
      current_account_id: (r) => setReg(r, enc.encode(this.me)),
      attached_deposit: (ptr) => wr(ptr, u128le(deposit)),
      account_balance: (ptr) => wr(ptr, u128le(this.accountBalance)),
      storage_usage: () => { let n = 100n; for (const [key, v] of st) n += 40n + BigInt(key.length + v.length); return n; },
      block_timestamp: () => 1_800_000_000_000_000_000n,
      used_gas: () => gas + BigInt(inst.exports.__gas.value) * REGULAR_OP,
      prepaid_gas: () => 300n * TGAS,
      storage_read: (kl, kp, r) => {
        const key = lat(rd(kl, kp));
        charge(C.storage_read_base + C.storage_read_key_byte * BigInt(key.length));
        const v = st.get(key);
        if (!v) return 0n;
        charge(C.storage_read_value_byte * BigInt(v.length));
        setReg(r, v);
        return 1n;
      },
      storage_write: (kl, kp, vl, vp, r) => {
        const key = lat(rd(kl, kp)), v = rd(vl, vp), old = st.get(key);
        charge(C.storage_write_base + C.storage_write_key_byte * BigInt(key.length) + C.storage_write_value_byte * BigInt(v.length));
        st.set(key, v);
        if (old) { charge(C.storage_write_evicted_byte * BigInt(old.length)); setReg(r, old); return 1n; }
        return 0n;
      },
      storage_has_key: (kl, kp) => { const key = lat(rd(kl, kp)); charge(C.storage_has_key_base + C.storage_has_key_byte * BigInt(key.length)); return st.has(key) ? 1n : 0n; },
      storage_remove: (kl, kp, r) => {
        const key = lat(rd(kl, kp)), old = st.get(key);
        charge(C.storage_remove_base + C.storage_remove_key_byte * BigInt(key.length));
        if (!old) return 0n;
        charge(C.storage_remove_ret_value_byte * BigInt(old.length));
        st.delete(key); setReg(r, old); return 1n;
      },
      value_return: (l, p) => { ret = rd(l, p); },
      log_utf8: (l, p) => {
        // nearcore checks both limits before it reads or decodes the message
        if (logs.length >= MAX_LOGS || logTotal + Number(l) > MAX_LOG_TOTAL) throw new HostError("log limit");
        const s = str(l, p);
        logTotal += Number(l);
        charge(C.log_base + C.log_byte * BigInt(l));
        logs.push(s);
      },
      panic: () => { throw new Panic("explicit guest panic"); },
      panic_utf8: (l, p) => { throw new Panic(str(l, p)); },
      promise_batch_create: (l, p) => BigInt(promises.push({ to: account(l, p), actions: [] }) - 1),
      promise_batch_action_transfer: (i, ptr) => promises[Number(i)].actions.push({ transfer: rd(16n, ptr) }),
      promise_batch_action_function_call: (i, ml, mp, al, ap, _amt, g) => promises[Number(i)].actions.push({ method: str(ml, mp), args: rd(al, ap), gas: g }),
      promise_then: (i, l, p, ml, mp, al, ap, _amt, g) => BigInt(promises.push({ after: Number(i), to: account(l, p), actions: [{ method: str(ml, mp), args: rd(al, ap), gas: g }] }) - 1),
      promise_return: () => {},
      promise_results_count: () => (result ? 1n : 0n),
      promise_result: (_i, r) => {
        if (!result) throw new HostError("InvalidPromiseResultIndex");
        if (result.failed) return 2n;
        setReg(r, bytes(result.value));
        return 1n;
      },
    };
    const snapshot = new Map(st);
    inst = new WebAssembly.Instance(this.mod, { env });
    mem = inst.exports.memory;
    inst.exports.__gas_limit.value = this.opLimit;
    const done = (o) => {
      const ops = BigInt(inst.exports.__gas.value);
      return { ...o, logs, promises, ops, gas: gas + ops * REGULAR_OP };
    };
    try {
      inst.exports[method]();
    } catch (e) {
      this.storage.clear();
      for (const [k2, v] of snapshot) this.storage.set(k2, v); // the receipt fails: none of its writes land
      if (e instanceof Panic) return done({ ok: false, kind: "panic", err: e.message });
      if (e instanceof HostError) return done({ ok: false, kind: "host", err: e.message });
      if (e instanceof WebAssembly.RuntimeError) {
        const over = BigInt(inst.exports.__gas.value) > this.opLimit;
        return done({ ok: false, kind: over ? "gas" : "trap", err: e.message });
      }
      throw e;
    }
    return done({ ok: true, ret });
  }
  snapshot() { return new Map(this.storage); }
  restore(s) { this.storage = new Map(s); }
}

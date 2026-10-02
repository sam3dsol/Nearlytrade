// Wasm instruction metering for the sim host, in the spirit of NEAR's own gas instrumentation.
//
// Every function body is cut into straight-line segments (a segment ends after block / loop / if / else /
// end / br / br_if / br_table / return / unreachable). At the start of each segment the instrumenter
// inserts `__gas += <instructions in the segment>`, so the counter is the number of original wasm
// instructions executed (NEAR charges wasm_regular_op_cost per instruction). Loop headers also get
// `if (__gas > __gas_limit) unreachable`, so a runaway loop traps instead of hanging the fuzzer.
//
// Two mutable i64 globals are appended and exported: __gas and __gas_limit. Supports MVP + sign-ext +
// bulk-memory + mutable-globals (what the token builds use); anything else throws.

function readU(b, p) {
  let r = 0n, s = 0n, x;
  do { x = b[p++]; r |= BigInt(x & 0x7f) << s; s += 7n; } while (x & 0x80);
  return [Number(r), p];
}
function skipLeb(b, p) { while (b[p++] & 0x80); return p; }
function encU(n) {
  const out = [];
  let v = BigInt(n);
  do { let x = Number(v & 0x7fn); v >>= 7n; if (v) x |= 0x80; out.push(x); } while (v);
  return out;
}
function encS(n) {
  const out = [];
  let v = BigInt(n);
  for (;;) {
    const x = Number(v & 0x7fn);
    v >>= 7n;
    if ((v === 0n && !(x & 0x40)) || (v === -1n && (x & 0x40))) { out.push(x); return out; }
    out.push(x | 0x80);
  }
}

/// decode one instruction at p; returns end offset
function skipInstr(b, p) {
  const op = b[p++];
  switch (op) {
    case 0x02: case 0x03: case 0x04: { // blocktype
      const t = b[p];
      if (t === 0x40 || (t >= 0x6f && t <= 0x7f)) return p + 1;
      return skipLeb(b, p);
    }
    case 0x0c: case 0x0d: case 0x10: case 0x20: case 0x21: case 0x22: case 0x23: case 0x24: case 0x25: case 0x26: case 0xd2:
      return skipLeb(b, p);
    case 0x0e: { let n; [n, p] = readU(b, p); for (let i = 0; i <= n; i++) p = skipLeb(b, p); return p; }
    case 0x11: p = skipLeb(b, p); return skipLeb(b, p);
    case 0x1c: { let n; [n, p] = readU(b, p); return p + n; }
    case 0x3f: case 0x40: return skipLeb(b, p);
    case 0x41: case 0x42: return skipLeb(b, p);
    case 0x43: return p + 4;
    case 0x44: return p + 8;
    case 0xd0: return p + 1;
    case 0xfc: {
      let sub; [sub, p] = readU(b, p);
      if (sub <= 7) return p;
      if (sub === 8 || sub === 10 || sub === 12 || sub === 14) { p = skipLeb(b, p); return skipLeb(b, p); }
      if (sub === 9 || sub === 11 || sub === 13 || (sub >= 15 && sub <= 17)) return skipLeb(b, p);
      throw new Error("meter: unknown 0xfc sub-opcode " + sub);
    }
    default:
      if (op <= 0x01 || op === 0x05 || op === 0x0b || op === 0x0f || op === 0x1a || op === 0x1b || op === 0xd1) return p;
      if (op >= 0x28 && op <= 0x3e) { p = skipLeb(b, p); return skipLeb(b, p); }
      if (op >= 0x45 && op <= 0xc4) return p;
      throw new Error("meter: unsupported opcode 0x" + op.toString(16) + " at " + (p - 1));
  }
}
const ENDS_SEGMENT = new Set([0x00, 0x02, 0x03, 0x04, 0x05, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f]);

export function instrument(bytes) {
  const b = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  if (b[0] !== 0 || b[1] !== 0x61 || b[2] !== 0x73 || b[3] !== 0x6d) throw new Error("not wasm");
  const sections = [];
  let p = 8;
  while (p < b.length) {
    const id = b[p++];
    let len; [len, p] = readU(b, p);
    sections.push({ id, body: b.subarray(p, p + len) });
    p += len;
  }
  // imported globals shift the index space of defined ones
  let importedGlobals = 0;
  const imp = sections.find((s) => s.id === 2);
  if (imp) {
    const s = imp.body; let q = 0, n; [n, q] = readU(s, q);
    for (let i = 0; i < n; i++) {
      let l; [l, q] = readU(s, q); q += l; [l, q] = readU(s, q); q += l;
      const kind = s[q++];
      if (kind === 0) q = skipLeb(s, q);
      else if (kind === 1) { q++; const f = s[q++]; q = skipLeb(s, q); if (f & 1) q = skipLeb(s, q); }
      else if (kind === 2) { const f = s[q++]; q = skipLeb(s, q); if (f & 1) q = skipLeb(s, q); }
      else if (kind === 3) { q += 2; importedGlobals++; }
    }
  }
  let glob = sections.find((s) => s.id === 6);
  let definedGlobals = 0, globBody = [];
  if (glob) { let q; [definedGlobals, q] = readU(glob.body, 0); globBody = Array.from(glob.body.subarray(q)); }
  const G = importedGlobals + definedGlobals, L = G + 1;
  const newGlob = [...encU(definedGlobals + 2), ...globBody, 0x7e, 0x01, 0x42, 0x00, 0x0b, 0x7e, 0x01, 0x42, 0x00, 0x0b];
  if (glob) glob.body = Uint8Array.from(newGlob);
  else {
    glob = { id: 6, body: Uint8Array.from(newGlob) };
    const at = sections.findIndex((s) => s.id > 6);
    sections.splice(at < 0 ? sections.length : at, 0, glob);
  }
  const exp = sections.find((s) => s.id === 7);
  {
    let q, n; [n, q] = readU(exp.body, 0);
    const rest = Array.from(exp.body.subarray(q));
    const name = (s) => [...encU(s.length), ...Buffer.from(s)];
    exp.body = Uint8Array.from([...encU(n + 2), ...rest, ...name("__gas"), 3, ...encU(G), ...name("__gas_limit"), 3, ...encU(L)]);
  }
  const meter = (n) => [0x23, ...encU(G), 0x42, ...encS(n), 0x7c, 0x24, ...encU(G)];
  const guard = [0x23, ...encU(G), 0x23, ...encU(L), 0x56, 0x04, 0x40, 0x00, 0x0b];
  const code = sections.find((s) => s.id === 10);
  {
    const s = code.body; let q = 0, n; [n, q] = readU(s, q);
    const out = [...encU(n)];
    for (let f = 0; f < n; f++) {
      let size; [size, q] = readU(s, q);
      const end = q + size;
      let r = q, nl; [nl, r] = readU(s, r);
      for (let i = 0; i < nl; i++) { r = skipLeb(s, r); r++; }
      const body = Array.from(s.subarray(q, r));
      // segments: [start, end, instruction count, starts after a loop]
      let segStart = r, count = 0, afterLoop = false;
      const segs = [];
      while (r < end) {
        const op = s[r];
        const nx = skipInstr(s, r);
        count++;
        r = nx;
        if (ENDS_SEGMENT.has(op)) {
          segs.push([segStart, r, count, afterLoop]);
          segStart = r; count = 0; afterLoop = op === 0x03;
        }
      }
      if (r !== end) throw new Error("meter: body overrun in function " + f);
      if (count) segs.push([segStart, r, count, afterLoop]);
      for (const [a, z, c, lp] of segs) {
        if (lp) body.push(...guard);
        body.push(...meter(c));
        for (let i = a; i < z; i++) body.push(s[i]);
      }
      out.push(...encU(body.length), ...body);
      q = end;
    }
    code.body = Uint8Array.from(out);
  }
  const parts = [b.subarray(0, 8)];
  for (const s of sections) parts.push(Uint8Array.from([s.id, ...encU(s.body.length)]), s.body);
  return Buffer.concat(parts);
}

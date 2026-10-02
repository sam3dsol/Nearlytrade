// gas of new() with the factory's exact init shape and a max-size icon (16 KB), per build
//   node contracts/token-tax/sim/init_gas.mjs a.wasm b.wasm ...
import { readFileSync } from "node:fs";
import { Token } from "./lib/host.mjs";
for (const path of process.argv.slice(2)) {
  const row = [];
  for (const iconLen of [0, 8000, 16384]) {
    for (const tax of [false, true]) {
      const t = new Token(readFileSync(path), { me: "pepe-2.nearlytrade.near" });
      if (tax && !t.exports.has("tax_take")) continue;
      const icon = "data:image/svg+xml;utf8," + '<svg xmlns=\\"http://www.w3.org/2000/svg\\">'.padEnd(iconLen, "a");
      const init = `{"owner_id":"lock_4.nearlytrade.near","total_supply":"1000000000000000000000000000","metadata":{"spec":"ft-1.0.0","name":"Pepe \\"the\\" frog","symbol":"PEPE","icon":"${icon}","reference":null,"reference_hash":null,"decimals":18},"rules":{"max_wallet_bps":400,"until_ms":"1790000000000","exempt":["nearlytrade.near","dclv2.ref-labs.near","lock_4.nearlytrade.near","feeswap.near"]}${tax ? ',"tax":{"buy_bps":300,"sell_bps":300,"pairs":["dclv2.ref-labs.near"],"admin":"nearlytrade.near","exempt":["lock_4.nearlytrade.near","feeswap.near"]}' : ',"tax":null'}}`;
      const r = t.call("new", { pred: "nearlytrade.near", args: init });
      row.push(`icon ${iconLen} ${tax ? "tax" : "raw"}: ${r.ok ? "" : "FAIL " + r.err + " "}${(Number(r.gas) / 1e12).toFixed(3)}`);
    }
  }
  console.log(path.split("/").pop().padEnd(28), row.join(" | "));
}

# nearly.trade contracts

Source for the contracts behind [nearly.trade](https://nearly.trade), a token launchpad on NEAR mainnet. Every launch creates a token with a fixed 1,000,000,000 supply and places it as a single-sided position on Rhea's DCL (`dclv2.ref-labs.near`). A locker contract owns that position; it has no way to remove the liquidity, it can only claim fees.

- `contracts/` is the code live on mainnet: the factory (`nearlytrade.near`), the locker that holds every new launch (`lock_6.nearlytrade.near`), and the raw and tax tokens new launches run.
- `legacy/` is every earlier locker and token generation. They are frozen (no access keys, immutable global code) and still hold or run the launches made with them, $NEARLY (`nearly-993927.nearlytrade.near`, raw v1) among them.
- `SPEC.md` describes the deployed code, `FIXES.md` lists the audit fixes and every upgrade since.

Every hash below rebuilds byte for byte from this tree with the commands in **Build**. Code comments are left out of the source here: they are blanked in place, so every line keeps its number and every build is unchanged. The lockers keep their doc comments, because their builds embed them in the contract ABI.

## Contracts

Deployed contracts (final factory upgrade on mainnet on 2026-10-02 19:02 UTC). Hashes are sha256 of the wasm built from this tree with the commands in **Build**; every one rebuilds byte for byte.

| Contract | Path | On chain | Code hash (hex) | Base58 |
|---|---|---|---|---|
| Factory | `contracts/factory` | `nearlytrade.near`, upgraded in place in tx `7y37HWECaHeFmbshWDwES2FpuxBP4aa1Y5nekBVM6gFv` (2026-10-03 06:31 UTC, new launches split 70 creator / 20 protocol / 10 referral pot, no burn pot part, see `FIXES.md`) after `Ejp7RVPL…` (2026-10-02 19:02 UTC, holders mode on every pair, buyback excess no longer booked, pot checks, tickers up to 12 characters), `H1XNfark…` (`tax_return`), `2ouURYue…` (fee split fixed at 70 creator / 20 protocol / 5 burn pot / 5 referral pot for new launches), `7Zj5R8TA…` (icon on the token only) and `E6gkiYsQ…` (2026-10-01, ContractWolf audit fixes = `c427d8f6…`, see `FIXES.md`); no state migration, new state lives in raw storage keys | `a5bb331353f7381889bb6766b8a707316a0915b1c0af99fc2204c5ae61f72aae` (630,931 bytes) | `C9wqAEFGdnCHysNtoFhrpWArxK19chCWbyYTPyMGjmLq` |
| Locker v3 | `contracts/locker3` | `lock_6.nearlytrade.near` (created 2026-10-01, 0 keys, launches #2192 onward; #2064 to #2191 sit in `lock_5` = `1c84bfec…` / `2vKpZcuEkab2hdXomeTN2fj67CdHWXpbDeBZmKJZZpSX`, the same source before the `supply_reserved` fix; #1810 to #2063 in `lock3`/`lock4`/`lock_4`, see `legacy/`) | `0097d9a9244a763c992931682a01f86395f984a16043d06f43730cb923b535e5` (287,321 bytes) | `13KJEWasGi6jrk3X99FDqeCbZ3Uqdpbbkd27fzSkx4Ut` |
| Token (tax) | `contracts/token-tax` | global contract (v5), the factory's tax code hash since tx `BcBUzpJjaQzdq1ToCScXBzi7mJg871UvCgTAxA93pDyk` (published in `DuDAZFmo36G3HJhQSoydcepo8ogRrUPwNtPAs4VMSijp`, 2026-10-01); tax launches created after it run it. Earlier ones keep the code they were created with: v4 `3b01653b…` = `4yLHirATwQvHfn99Vsb98mWMkvVZhjgeJ5ahx4WhC1MZ` (#2061 to the switch), `aca1a7f8…` from #1901, the frozen tax v2 before | `f4265f2230721b2df18a0d78f36b737fe77d955194519a77578b2463602ac3a4` (21,568 bytes) | `HS4R2isPS7hnnY8k2QbxnLHUx9Z9v9mNZnLsrGqwhChh` |
| Token (raw) | `contracts/token-raw` | global contract, the factory's raw code hash since the same tx; earlier launches keep `95ec4ec3…` | `b611c33fef4d070b9cb7c6f6b9e2e7af8782f41e3d02e105ff805736ec01e7ec` (16,150 bytes) | `DFiuocosGQbx3raMt3ACEauZqLXXYzX77MJNf361385R` |

Token accounts are immutable (no keys, code by global hash), so launches made before the switch keep the token code they were created with; those earlier token builds are listed below.

Earlier generations, frozen (they stay live next to the new code). Each folder has a README with the rebuild recipe.

| Contract | Path | Lines | On chain | Code hash | Rebuild check |
|---|---|---|---|---|---|
| Locker v1 | `legacy/locker-v1` | 534 | `lock.nearlytrade.near`, 0 keys, launches #0 to #157 | `J7eJu1WrzbhHe2qehW1WD7cNSvp188bctMxp6HogWgvh` = `fe4a58f7…` | match (docker build in the sourcescan image) |
| Locker v2 | `legacy/locker-v2` | 540 | `lock2.nearlytrade.near`, 0 keys, launches #158 to #1809 | `H2hLLyZ9k5Z5SVRBPrj7Ke9NQMpsENqN5SoSMf52gCHe` = `ee2a2d48…` | match |
| Locker v3 (lock3) | `legacy/locker3-lock3` | 740 | `lock3.nearlytrade.near`, 0 keys, launches #1810 to #1930 | `8buG64GsegXxE4bh6GWC7Xkwx7fQNaCcnA26SfyE5C9X` = `70f2a58e…` | match (2026-10-01) |
| Locker v3 (lock4) | `legacy/locker3-lock4` | 889 | `lock4.nearlytrade.near`, 0 keys, launches #1931 to #1934 (first `add_buy`) | `9uhDnu5xdd6P8HBDhd8ABGpB9atmffARrq5vbHk56eTC` = `845d2ccc…` | match (2026-10-01) |
| Locker v3 (lock_4) | `legacy/locker3-lock_4` | 921 | `lock_4.nearlytrade.near`, 0 keys, launches #1935 to #2063 | `BAXUNRdsDeRMpahkFzSUrzXD7mAQdrsipfbu6VouSfNb` = `97058d1c…` | match (2026-10-01) |
| Token (raw) v1 | `legacy/token-raw-v1` | 572 | global, **$NEARLY** and no-tax launches before 2026-09-28 | `5qScjXG9uRG82Yrh36XZNZgVUYiqPtuC9ep5DvV42eGQ` = `47d7973b…` | match (2026-10-01) |
| Token (raw) v2 | `legacy/token-raw-v2` | 618 | global, no-tax launches 2026-09-28 to #2060 | `B6EjqsNJXfQXYX1KkUaypN8z7KpYcBrf9JNwTVtWUUBY` = `95ec4ec3…` | match (2026-10-01) |
| Token (tax) v1 | `legacy/token-tax-v1` | 666 | global, tax launches before 2026-09-28 | `1uGuBEpx3dFRDrr2wNzm5Vcb5sF3jWY3AKQ3Gopd6we` = `003b1b46…` | match |
| Token (tax) v2 | `legacy/token-tax-v2` | 714 | global, tax launches 2026-09-28 to #1900 | `YXJL2KYynDA52JPBpAP5qDZhJqCTfB6dR5Ee4jdEgDK` = `08134395…` | match |
| Token (tax) v3 | `legacy/token-tax-v3` | 740 | global, tax launches #1901 to #2060 | `Cct6RbLZAEs5WmKkPFr4qqTAWKKzPhE4aTJHVwNLeGBS` = `aca1a7f8…` | match (2026-10-01) |

The factory keeps a per-launch locker schedule, so every locker stays in use for its own launches: `get_lockers` returns lock from 0, lock2 from 158, lock3 from 1810, lock4 from 1931, lock_4 from 1935, lock_5 from 2064 and lock_6 from 2192. Tokens are published once as NEAR global contracts (by code hash) and every launch deploys its token account against one of those hashes. The earlier token generations stay live for older launches (table above).

## Verify the deployed code

All read-only, against `https://rpc.mainnet.fastnear.com`. `<b58>` is the base58 hash from the tables.

```sh
RPC=https://rpc.mainnet.fastnear.com
q() { curl -s $RPC -H 'content-type: application/json' -d "$1"; }

# accounts: code hash and access keys
for a in nearlytrade.near lock_5.nearlytrade.near lock.nearlytrade.near lock2.nearlytrade.near lock3.nearlytrade.near lock4.nearlytrade.near lock_4.nearlytrade.near; do
  q '{"jsonrpc":"2.0","id":1,"method":"query","params":{"request_type":"view_account","finality":"final","account_id":"'$a'"}}' | jq -r '.result.code_hash'
  q '{"jsonrpc":"2.0","id":1,"method":"query","params":{"request_type":"view_access_key_list","finality":"final","account_id":"'$a'"}}' | jq '.result.keys|length'
done
# expected: nearlytrade.near C9wqAEFG…, lock_5.nearlytrade.near 2vKpZcuE… 0 keys, lock.nearlytrade.near J7eJu1Wr… 0 keys, lock2.nearlytrade.near H2hLLyZ9… 0 keys, lock3 8buG64Gs… 0 keys, lock4 9uhDnu5x… 0 keys, lock_4 BAXUNRds… 0 keys

# global contracts: fetch the code by hash and sha256 it
for h in DFiuocosGQbx3raMt3ACEauZqLXXYzX77MJNf361385R 4yLHirATwQvHfn99Vsb98mWMkvVZhjgeJ5ahx4WhC1MZ 5qScjXG9uRG82Yrh36XZNZgVUYiqPtuC9ep5DvV42eGQ B6EjqsNJXfQXYX1KkUaypN8z7KpYcBrf9JNwTVtWUUBY 1uGuBEpx3dFRDrr2wNzm5Vcb5sF3jWY3AKQ3Gopd6we YXJL2KYynDA52JPBpAP5qDZhJqCTfB6dR5Ee4jdEgDK Cct6RbLZAEs5WmKkPFr4qqTAWKKzPhE4aTJHVwNLeGBS; do
  q '{"jsonrpc":"2.0","id":1,"method":"query","params":{"request_type":"view_global_contract_code","finality":"final","code_hash":"'$h'"}}' | jq -r '.result.code_base64' | base64 -d | shasum -a 256
done

# rebuild each frozen contract from this tree and compare (locker-v1 needs docker)
for c in locker-v1 locker-v2 locker3-lock3 locker3-lock4 locker3-lock_4 token-raw-v1 token-raw-v2 token-tax-v1 token-tax-v2 token-tax-v3; do legacy/rebuild.sh $c; done

```

The NEP-330 metadata of the lockers (`contract_source_metadata`) points at the public repo `sam3dsol/Nearlytrade`; lock v1's pinned commit no longer exists there after a history rewrite, which is why the rebuild recipe pins the same values by hand.

## Build

Toolchain: Rust 1.97.1 (`rust-toolchain.toml`), cargo-near 0.22.0, binaryen `wasm-opt` 132.

```sh
(cd contracts/factory && cargo near build non-reproducible-wasm --no-abi)   # -> target/near/nearpad_factory/nearpad_factory.wasm
(cd contracts/locker3 && cargo near build non-reproducible-wasm)            # -> target/near/nearpad_locker3/nearpad_locker3.wasm

for p in token-raw token-tax; do
  RUSTFLAGS="-C target-cpu=mvp" cargo build --release --target wasm32-unknown-unknown -p nearpad-$p
  n=nearpad_${p//-/_}
  wasm-opt -Oz --enable-bulk-memory --enable-bulk-memory-opt --enable-sign-ext --enable-mutable-globals \
    target/wasm32-unknown-unknown/release/$n.wasm -o $n.wasm
done
shasum -a 256 target/near/*/*.wasm nearpad_token_*.wasm
```

Frozen contracts: `legacy/rebuild.sh <name>` (see each `legacy/*/README.md`).

## Tests

- `cargo test -p nearpad-factory`: 141 passed.
- `cargo test -p nearpad-locker3`: 64 passed.
- `cargo test -p nearpad-locker-v1` / `-v2`: 6 passed each.
- Token crates are `near-sys` only and compile for wasm32 alone; they have no host unit tests. Their harnesses:
  - `node contracts/token-tax/sim/tax_in_flight.mjs <token-tax wasm>`: 56 checks of the in-flight tax lock against a simulated host.
  - `node contracts/token-tax/sim/final_checks.mjs <raw wasm> <tax wasm> <previous raw wasm> <previous tax wasm>`: 133 checks of the final fixes (strict argument parsing, account ids, memos, resolve never panicking, pro-rata tax refund incl. the overflow path, storage_withdraw) with gas compared to the previous build; `init_gas.mjs` measures `new` with a 16 KB icon.
  - `node contracts/token-tax/sim/fuzz_json.mjs [--iters N] [--seed S] [--scale] <wasm> ...`: fuzzes the hand-written JSON argument parsing of any token wasm (raw and tax) against a reference parser.
- End-to-end runs were done on a local neard sandbox (neard 2.13.4) with the real mainnet `dclv2.ref-labs.near` (v2.3.14) and `wrap.near` wasm, including upgrading the currently deployed factory in place with existing launches.

Crate names (`nearpad-*`) and the NEP-297 event standard `"nearpad"` are an early internal codename kept for compatibility with the deployed code.

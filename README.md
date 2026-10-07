# nearly.trade contracts

Source for the contracts behind [nearly.trade](https://nearly.trade), a token launchpad on NEAR mainnet. Every launch creates a token with a fixed 1,000,000,000 supply and places it as a single-sided position on Rhea's DCL (`dclv2.ref-labs.near`). A locker contract owns that position; it has no way to remove the liquidity, it can only claim fees.

- `contracts/` is the code live on mainnet: the factory (`nearlytrade.near`), the locker that holds every new launch (`lock_8.nearlytrade.near`), and the raw and tax tokens new launches run.
- `legacy/` is every earlier locker and token generation. They are frozen (no access keys, immutable global code) and still hold or run the launches made with them, $NEARLY (`nearly-993927.nearlytrade.near`, raw v1) among them.
- `SPEC.md` describes the deployed code, `FIXES.md` lists the audit and review fixes and every upgrade since.

Every hash below rebuilds byte for byte from this tree with the commands in **Build**. Code comments are left out of the source here: they are blanked in place, so every line keeps its number and every build is unchanged. The lockers keep their doc comments, because their builds embed them in the contract ABI.

## Contracts

Deployed contracts (factory current since 2026-10-07 21:18 UTC, see `FIXES.md`). Hashes are sha256 of the wasm built from this tree with the commands in **Build**; every one rebuilds byte for byte. The factory and the locker are cargo-near reproducible builds (Docker image pinned in their `Cargo.toml`), so they rebuild to the same bytes on any machine.

| Contract | Path | On chain | Code hash (hex) | Base58 |
|---|---|---|---|---|
| Factory | `contracts/factory` | `nearlytrade.near`, upgraded in place in tx `D1vNiZErqh7tXBCpcjzhKRst2U9afGuSToXmEYBgW8Cc` (2026-10-07, `upgrade_code`: the owner DAO upgrades the factory, see `FIXES.md`) after `5VexucX6…` (2026-10-05, security review fixes, `6ca88c72…` = `8KA6c2mF…`), `7y37HWEC…` (2026-10-03 06:31 UTC, `a5bb3313…` = `C9wqAEFG…`, new launches split 70 creator / 20 protocol / 10 referral pot, no burn pot part), `Ejp7RVPL…` (2026-10-02 19:02 UTC, holders mode on every pair, buyback excess no longer booked, pot checks, tickers up to 12 characters), `H1XNfark…` (`tax_return`), `2ouURYue…` (fee split fixed at 70 creator / 20 protocol / 5 burn pot / 5 referral pot for new launches), `7Zj5R8TA…` (icon on the token only) and `E6gkiYsQ…` (2026-10-01, ContractWolf audit fixes = `c427d8f6…`, see `FIXES.md`); no state migration, new state lives in raw storage keys | `aba3be304a75f9d1ee7b53dab8d1e34675d23f73e8ca0a801e78455cc86ab9f3` (641,045 bytes) | `CZ1XpbUkHpKWJJHKcukmXqreyKvZK56grnXGA3ZqtZVc` |
| Locker v3 | `contracts/locker3` | `lock_8.nearlytrade.near` (created 2026-10-06 in tx `GkwA3qx6R6Nm7ZGAjQdAwuWSYbRudKUU4EFXyedwaGMP`, 0 keys, launches #2596 onward, see `FIXES.md`; #2549 to #2595 sit in `lock_7` = `cc375a25…` / `EkB8CKVfvBfKTFcMjGk9iKTEzjxiUTVqHUBH1HjxTQLd`, 0 keys, built from commit `d3c0e52`; #2192 to #2548 sit in `lock_6` = `0097d9a9…` / `13KJEWasGi6jrk3X99FDqeCbZ3Uqdpbbkd27fzSkx4Ut`, 0 keys, the source of commit `56fea17`; #2064 to #2191 in `lock_5` = `1c84bfec…` / `2vKpZcuEkab2hdXomeTN2fj67CdHWXpbDeBZmKJZZpSX`, the same source before the `supply_reserved` fix; #1810 to #2063 in `lock3`/`lock4`/`lock_4`, see `legacy/`) | `2f7bd173e4aef59e944d36ae0152d22d7d6a08c3dbac5a9a166eb954db898312` (287,404 bytes) | `4CMfhWsUNfXgotni5nnt9vePC1dpb4aQF8x1KQjoyG6u` |
| Token (tax) | `contracts/token-tax` | global contract (v6), the factory's tax code hash since tx `BfYPUuYa9oVFmc3T5Qf4DLFabofCRwuGmFvdSAUGtEcX` (published in `EE6puaHuz1iY9JRZH1diHgURxG4P5tJwWh6f8Q9ALuXY`, 2026-10-05); tax launches created after it run it. Earlier ones keep the code they were created with: v5 `f4265f22…` = `HS4R2isPS7hnnY8k2QbxnLHUx9Z9v9mNZnLsrGqwhChh` (2026-10-01 to #2545, source of commit `56fea17`), v4 `3b01653b…` = `4yLHirATwQvHfn99Vsb98mWMkvVZhjgeJ5ahx4WhC1MZ` (#2061 to the switch), `aca1a7f8…` from #1901, the frozen tax v2 before | `6bfc00b72948064a5c18c8081a1107a7f9e2b3f2068b5060d2f3711ec195a482` (22,258 bytes) | `8GXVnFTxx1d6tYnoqNiQ5YzzQpj1uyxG7u28LeCGmsER` |
| Token (raw) | `contracts/token-raw` | global contract, the factory's raw code hash since the same tx (published in `2QPLWGoQ9JVAqK2BYX28p4MWfogXsoYJgNcNdLk2dqK6`); earlier launches keep `b611c33f…` = `DFiuocosGQbx3raMt3ACEauZqLXXYzX77MJNf361385R` (2026-10-01 to #2545, source of commit `56fea17`) or `95ec4ec3…` | `3c44c33eed3f606d15588c8e09080a0bee8f264b8ded30347171b8306d0f8df9` (16,743 bytes) | `54GH1DmZgArERRH2ed8zJBXT7UxdeumaZe8Du8dtYJrt` |

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

The factory keeps a per-launch locker schedule, so every locker stays in use for its own launches: `get_lockers` returns lock from 0, lock2 from 158, lock3 from 1810, lock4 from 1931, lock_4 from 1935, lock_5 from 2064, lock_6 from 2192, lock_7 from 2549 and lock_8 from 2596. Tokens are published once as NEAR global contracts (by code hash) and every launch deploys its token account against one of those hashes. The earlier token generations stay live for older launches (table above).

## Verify the deployed code

All read-only, against `https://rpc.mainnet.fastnear.com`. `<b58>` is the base58 hash from the tables.

```sh
RPC=https://rpc.mainnet.fastnear.com
q() { curl -s $RPC -H 'content-type: application/json' -d "$1"; }

# accounts: code hash and access keys
for a in nearlytrade.near lock_8.nearlytrade.near lock_7.nearlytrade.near lock_6.nearlytrade.near lock_5.nearlytrade.near lock.nearlytrade.near lock2.nearlytrade.near lock3.nearlytrade.near lock4.nearlytrade.near lock_4.nearlytrade.near; do
  q '{"jsonrpc":"2.0","id":1,"method":"query","params":{"request_type":"view_account","finality":"final","account_id":"'$a'"}}' | jq -r '.result.code_hash'
  q '{"jsonrpc":"2.0","id":1,"method":"query","params":{"request_type":"view_access_key_list","finality":"final","account_id":"'$a'"}}' | jq '.result.keys|length'
done
# expected: nearlytrade.near CZ1XpbUk…, lock_8.nearlytrade.near 4CMfhWsU… 0 keys, lock_7.nearlytrade.near EkB8CKVf… 0 keys, lock_6.nearlytrade.near 13KJEWas… 0 keys, lock_5.nearlytrade.near 2vKpZcuE… 0 keys, lock.nearlytrade.near J7eJu1Wr… 0 keys, lock2.nearlytrade.near H2hLLyZ9… 0 keys, lock3 8buG64Gs… 0 keys, lock4 9uhDnu5x… 0 keys, lock_4 BAXUNRds… 0 keys

# global contracts: fetch the code by hash and sha256 it
for h in 54GH1DmZgArERRH2ed8zJBXT7UxdeumaZe8Du8dtYJrt 8GXVnFTxx1d6tYnoqNiQ5YzzQpj1uyxG7u28LeCGmsER DFiuocosGQbx3raMt3ACEauZqLXXYzX77MJNf361385R HS4R2isPS7hnnY8k2QbxnLHUx9Z9v9mNZnLsrGqwhChh 4yLHirATwQvHfn99Vsb98mWMkvVZhjgeJ5ahx4WhC1MZ 5qScjXG9uRG82Yrh36XZNZgVUYiqPtuC9ep5DvV42eGQ B6EjqsNJXfQXYX1KkUaypN8z7KpYcBrf9JNwTVtWUUBY 1uGuBEpx3dFRDrr2wNzm5Vcb5sF3jWY3AKQ3Gopd6we YXJL2KYynDA52JPBpAP5qDZhJqCTfB6dR5Ee4jdEgDK Cct6RbLZAEs5WmKkPFr4qqTAWKKzPhE4aTJHVwNLeGBS; do
  q '{"jsonrpc":"2.0","id":1,"method":"query","params":{"request_type":"view_global_contract_code","finality":"final","code_hash":"'$h'"}}' | jq -r '.result.code_base64' | base64 -d | shasum -a 256
done

# rebuild each frozen contract from this tree and compare (locker-v1 needs docker)
for c in locker-v1 locker-v2 locker3-lock3 locker3-lock4 locker3-lock_4 token-raw-v1 token-raw-v2 token-tax-v1 token-tax-v2 token-tax-v3; do legacy/rebuild.sh $c; done

```

The NEP-330 metadata of the factory and the lockers (`contract_source_metadata`) points at the public repo `sam3dsol/Nearlytrade`; the factory and `lock_7` pin commit `d3c0e525c15d3239794d8eafda76775833e771b0`, `lock_8` pins `a476bbe47a694e2b24e7060738870065882df011`; lock v1's pinned commit no longer exists there after a history rewrite, which is why the rebuild recipe pins the same values by hand.

## Build

Toolchain: Rust 1.97.1 (`rust-toolchain.toml`), cargo-near 0.22.0, binaryen `wasm-opt` 132.

```sh
# factory + locker: reproducible, in the Docker image pinned in each Cargo.toml (needs docker)
(cd contracts/factory && cargo near build reproducible-wasm)   # -> aba3be30… (commit 38abcd8); at commit d3c0e52 -> 6ca88c72…
(cd contracts/locker3 && cargo near build reproducible-wasm)   # -> 2f7bd173… = lock_8 (commit a476bbe); at commit d3c0e52 -> cc375a25… = lock_7

# tokens (no paths or metadata in these builds, the same bytes on macOS arm64 and Linux x86_64)

for p in token-raw token-tax; do
  RUSTFLAGS="-C target-cpu=mvp" cargo build --release --target wasm32-unknown-unknown -p nearpad-$p
  n=nearpad_${p//-/_}
  wasm-opt -Oz --enable-bulk-memory --enable-bulk-memory-opt --enable-sign-ext --enable-mutable-globals \
    target/wasm32-unknown-unknown/release/$n.wasm -o $n.wasm
done
shasum -a 256 target/near/*/*.wasm nearpad_token_*.wasm   # tokens -> 3c44c33e… (raw), 6bfc00b7… (tax)
```

The superseded builds (factory `a5bb3313…`, `lock_6` `0097d9a9…`, raw `b611c33f…`, tax v5 `f4265f22…`) rebuild from commit `56fea17` with the recipe in its README.

Frozen contracts: `legacy/rebuild.sh <name>` (see each `legacy/*/README.md`).

## Tests

- `cargo test -p nearpad-factory`: 148 passed.
- `cargo test -p nearpad-locker3`: 65 passed.
- `cargo test -p nearpad-locker-v1` / `-v2`: 6 passed each.
- Token crates are `near-sys` only and compile for wasm32 alone; they have no host unit tests. Their harnesses:
  - `node contracts/token-tax/sim/tax_in_flight.mjs <token-tax wasm>`: 56 checks of the in-flight tax lock against a simulated host.
  - `node contracts/token-tax/sim/final_checks.mjs <raw wasm> <tax wasm> <previous raw wasm> <previous tax wasm>`: 133 checks of the final fixes (strict argument parsing, account ids, memos, resolve never panicking, pro-rata tax refund incl. the overflow path, storage_withdraw) with gas compared to the previous build; `init_gas.mjs` measures `new` with a 16 KB icon.
  - `node contracts/token-tax/sim/fuzz_json.mjs [--iters N] [--seed S] [--scale] <wasm> ...`: fuzzes the hand-written JSON argument parsing of any token wasm (raw and tax) against a reference parser.
- End-to-end runs were done on a local neard sandbox (neard 2.13.4) with the real mainnet `dclv2.ref-labs.near` (v2.3.14) and `wrap.near` wasm, including upgrading the currently deployed factory in place with existing launches.

## Known limitations of the earlier token generations

The token generations under `legacy/` are immutable (no access keys, code by global hash), so the review findings that sit in them cannot be patched on chain. The current token code fixes them for every new launch; the site never passes these tokens an account id that cannot exist and builds every call's arguments itself.

- Raw v1/v2 and tax v1/v2/v3: a duplicated JSON field can be read differently by a wallet preview and by the token (N-07); account ids that can never exist are accepted as receivers (N-08); a malformed or oversized receiver result makes `ft_resolve_transfer` abort (N-10).
- Tax v1/v2: tax taken while a transfer's refund is pending can be collected before it resolves; the factory's `collect_tax` is keeper only since 2026-10-05, so only the protocol's keeper triggers it (N-09). Partial-swap overtaxing persists through v3.
- Tax v1: `ft_transfer_call` reports the amount used net of the tax it charged (N-11).
- All generations before this one: a burned refund to an unregistered sender is reported as returned (N-06).
- The oldest lockers (`lock`, `lock2`) lack the methods a whole-system DCL migration would call (N-05). No such migration is planned.

Crate names (`nearpad-*`) and the NEP-297 event standard `"nearpad"` are an early internal codename kept for compatibility with the deployed code.

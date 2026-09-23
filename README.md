# Nearly — factory & locker contracts

Source for the two contracts behind [nearly.trade](https://nearly.trade), a token launchpad on NEAR.

A launch mints 1,000,000,000 tokens once and places **the entire supply as a single-sided position
on a Rhea DCL pool**. There is no bonding curve and no graduation step: the token is a tradable pair
in the block it is created. The position is owned by the locker, which has no way to give it back.

## Contracts

| contract | account | code hash (mainnet) | access keys |
|---|---|---|---|
| factory | `nearlytrade.near` | `9u3Wi48UzuXVY9f5tngGC6q148U3wmprQoiraVbry4dR` | owner key (upgradable) |
| locker | `lock.nearlytrade.near` | `J7eJu1WrzbhHe2qehW1WD7cNSvp188bctMxp6HogWgvh` | **none: deleted, tx `3hh5h8LpjdAzuP4zEikDJwwtXxFUtL8t5eWUsSGpEFMs`** |

**`contracts/factory`** — one contract holds every launch. Tokens are deployed as sub-accounts
(`<slug>-<id>.<factory>`) using a globally published token contract referenced by hash, so the token
code is identical for every launch and cannot be changed after deployment. The factory creates the
Rhea pool, places the range, and accounts for fees. It stays upgradable; it never holds a position.

**`contracts/locker`** — owns every LP position. Callable methods: `add` and `claim` (factory only),
`register` (factory only, a storage deposit on a pair asset), and read-only views `get_factory`,
`get_owed`, `get_owed_near`, `get_owed_token`, `get_owed_quote`. Everything else is a private callback.

## Why the liquidity cannot be pulled

`claim()` is the only method that touches a position, and it calls Rhea's `remove_liquidity` with
the amount **hardcoded to zero**:

```rust
format!(r#"{{"lpt_id":{},"amount":"0","min_amount_x":"0","min_amount_y":"0"}}"#, ...)
```

Removing zero liquidity returns only the fees accrued to the position. There is no method that takes
an amount, no withdraw, no NFT transfer or approval, and no owner field.

**And the code can never change:** `lock.nearlytrade.near` has **zero access keys**. With no key,
nobody (the deployer included) can sign a transaction for it, so no one can deploy new code, add a
key, or move anything. What you read in `contracts/locker/src/lib.rs` is what runs, permanently.

Fees keep flowing without a key: anyone can call `claim_fees(launch_id)` on the factory, which calls
the locker, which forwards the fees back to the factory.

## Verify it yourself

1. **No keys:** `near account list-keys lock.nearlytrade.near network-config mainnet now` returns an
   empty list (or `view_access_key_list` on any RPC).
2. **Code = this source:** the locker embeds its own build info (NEP-330). Call the view
   `contract_source_metadata` on `lock.nearlytrade.near`: it points at this repository at commit
   `97fd423024a2fd0b15978292b45106bc133a34ac` and the pinned build image.
3. **Rebuild it** (Docker required) and compare hashes:

```sh
git clone https://github.com/sam3dsol/Nearlytrade && cd Nearlytrade
git checkout 97fd423024a2fd0b15978292b45106bc133a34ac
cd contracts/locker && cargo near build reproducible-wasm
# SHA-256 hex fe4a58f7bb97dd858d290a01adab8b9afad2f5783c1ac1d12fe603fb394558de
# base58      J7eJu1WrzbhHe2qehW1WD7cNSvp188bctMxp6HogWgvh  = code_hash of lock.nearlytrade.near
```

The factory source here is what `nearlytrade.near` runs today (hash above); being upgradable, it is
the part to re-check against the chain whenever it changes. It controls fee accounting, never the
liquidity.

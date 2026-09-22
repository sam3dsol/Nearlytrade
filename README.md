# Nearly — factory & locker contracts

Source for the two contracts behind [nearly.trade](https://nearly.trade), a token launchpad on NEAR.

A launch mints 1,000,000,000 tokens once and places **the entire supply as a single-sided position
on a Rhea DCL pool**. There is no bonding curve and no graduation step: the token is a tradable pair
in the block it is created. The position is owned by the locker, which has no way to give it back.

## Contracts

| contract | account | code hash (mainnet) |
|---|---|---|
| factory | `nearlytrade.near` | `G88RKRB7rgKMtfzk3qTcBnAzKVKNzMso8cAfGBkx8zy1` |
| locker | `lock.nearlytrade.near` | `Cjsvh37K3r5nrRPuBbpPPCv6u8W6S9Y1sreH9EhB9oLq` |

**`contracts/factory`** — one contract holds every launch. Tokens are deployed as sub-accounts
(`<slug>-<id>.<factory>`) using a globally published token contract referenced by hash, so the token
code is identical for every launch and cannot be changed after deployment. The factory creates the
Rhea pool, places the range, and accounts for fees.

**`contracts/locker`** — owns the LP position. Its entire public surface is `new`, `add`, `claim`
and `get_factory`.

## Why the liquidity cannot be pulled

`claim()` is the only method that touches the position, and it calls Rhea's `remove_liquidity` with
the amount **hardcoded to zero**:

```rust
format!(r#"{{"lpt_id":{},"amount":"0","min_amount_x":"0","min_amount_y":"0"}}"#, ...)
```

Removing zero liquidity returns only the fees accrued to the position. There is no method that takes
an amount, no withdraw, no owner field, and no upgrade path — so no caller, including the deployer,
can move the principal. `claim()` is additionally gated by `assert_factory()`.

Read `contracts/locker/src/lib.rs` — it is about 140 lines. That is the whole guarantee.

## Build

```sh
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
```

## Verification status

The code hashes above are what those accounts serve on mainnet today, read via `view_account`.
A reproducible build matching this source to those hashes has **not** yet been published — until it
is, treat this repository as the source of record for review, and the on-chain hashes as the source
of record for what is running. Contributions that establish a reproducible build are welcome.

# Locker v2: `lock2.nearlytrade.near` (frozen)

- Account: `lock2.nearlytrade.near`, code hash `H2hLLyZ9k5Z5SVRBPrj7Ke9NQMpsENqN5SoSMf52gCHe` (sha256 `ee2a2d48c807f67bf8af8fdbc29aceffc20bc3218d597326cfc034f6a8868c79`).
- Runs launches #158 onward (factory `get_lockers`: `["158","lock2.nearlytrade.near"]`), until the factory's locker switch to lock3.
- Frozen: zero access keys (verified on mainnet 2026-09-30).
- Built 2026-09-26 with `cargo near build non-reproducible-wasm` on the host (ABI embedded, NEP-330 link from `Cargo.toml`'s `repository`).
- Package name on chain: `nearpad-locker 0.2.0` (this crate is `nearpad-locker-v2` only so the workspace resolves).

Rebuild and check:

```sh
legacy/rebuild.sh locker-v2
# same as: copy this crate to <scratch>/contracts/locker as package nearpad-locker, then
# (cd contracts/locker && cargo near build non-reproducible-wasm) with this repo's Cargo.lock
```

Verified 2026-09-30: host build of this source = `ee2a2d48…` = the live code hash.

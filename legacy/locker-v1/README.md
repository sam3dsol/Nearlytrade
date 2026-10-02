# Locker v1: `lock.nearlytrade.near` (frozen)

- Account: `lock.nearlytrade.near`, code hash `J7eJu1WrzbhHe2qehW1WD7cNSvp188bctMxp6HogWgvh` (sha256 `fe4a58f7bb97dd858d290a01adab8b9afad2f5783c1ac1d12fe603fb394558de`).
- Runs launches #0 to #157 (factory `get_lockers`: `["0","lock.nearlytrade.near"]`).
- Frozen: the account has zero access keys (verified on mainnet 2026-09-30), so the code can never change.
- Built 2026-09-23 as a cargo-near reproducible build in `sourcescan/cargo-near:0.22.0-rust-1.97.1`; the NEP-330 metadata pins the source to a commit of the public repo that no longer exists after a history rewrite (same tree as this source).
- Package name on chain: `nearpad-locker 0.2.0` (this crate is `nearpad-locker-v1` only so the workspace resolves).

Rebuild and check (needs docker; the container paths are baked into the wasm):

```sh
legacy/rebuild.sh locker-v1
```

Verified 2026-09-30: the script's docker build of this source produced `fe4a58f7…` = the live code hash.

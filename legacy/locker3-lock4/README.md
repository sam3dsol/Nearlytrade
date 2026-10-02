# Locker v3 (lock4): `lock4.nearlytrade.near` (frozen, 0 keys)

- Code hash `9uhDnu5xdd6P8HBDhd8ABGpB9atmffARrq5vbHk56eTC` (sha256 `845d2ccc539c3c85e46000f03e21994fcd03a18888b93db76d37c7133acfd30b`).
- Holds the positions of launches #1931 to #1934 (the first with `add_buy`). An earlier revision of `contracts/locker3`.
- Frozen: the account has no access keys, so its code cannot change.
- Package name on chain: `nearpad-locker3` (this crate is `nearpad-locker3-lock4` only so the workspace resolves).

Rebuild and check:

```sh
legacy/rebuild.sh locker3-lock4
# = cargo near build non-reproducible-wasm --no-doc   (cargo-near 0.22.0, rustc 1.97.1)
```

Verified 2026-10-01: the rebuild = `845d2ccc…` = the live code, byte for byte.

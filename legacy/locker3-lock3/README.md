# Locker v3 (lock3): `lock3.nearlytrade.near` (frozen, 0 keys)

- Code hash `8buG64GsegXxE4bh6GWC7Xkwx7fQNaCcnA26SfyE5C9X` (sha256 `70f2a58e5a3a68b9011776cc47a13b0cbc8701ce0807fcde1aebe9b6dfba8eea`).
- Holds the positions of launches #1810 to #1930. An earlier revision of `contracts/locker3`.
- Frozen: the account has no access keys, so its code cannot change.
- Package name on chain: `nearpad-locker3` (this crate is `nearpad-locker3-lock3` only so the workspace resolves).

Rebuild and check:

```sh
legacy/rebuild.sh locker3-lock3
# = cargo near build non-reproducible-wasm --no-doc   (cargo-near 0.22.0, rustc 1.97.1)
```

Verified 2026-10-01: the rebuild = `70f2a58e…` = the live code, byte for byte.

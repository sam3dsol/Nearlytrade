# Locker v3 (lock_4): `lock_4.nearlytrade.near` (frozen, 0 keys)

- Code hash `BAXUNRdsDeRMpahkFzSUrzXD7mAQdrsipfbu6VouSfNb` (sha256 `97058d1c88659e60fc06487b6d36364db4b48c234b3ea552723acb6d46e65ba4`).
- Holds the positions of launches #1935 to #2063. An earlier revision of `contracts/locker3`.
- Frozen: the account has no access keys, so its code cannot change.
- Package name on chain: `nearpad-locker3` (this crate is `nearpad-locker3-lock-4` only so the workspace resolves).

Rebuild and check:

```sh
legacy/rebuild.sh locker3-lock_4
# = cargo near build non-reproducible-wasm --no-doc   (cargo-near 0.22.0, rustc 1.97.1)
```

Verified 2026-10-01: the rebuild = `97058d1c…` = the live code, byte for byte.

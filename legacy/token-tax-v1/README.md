# Token (tax) v1: global contract `003b1b46…` (frozen)

- Global contract code hash `1uGuBEpx3dFRDrr2wNzm5Vcb5sF3jWY3AKQ3Gopd6we` (sha256 `003b1b46bfe37276642e126f3095265a10d5d2c81619bd2de1e7523e80dc2625`).
- Runs about 1,160 tax tokens launched before the 2026-09-28 auto-registration change.
- Frozen: global contract code is immutable; token accounts reference it by hash and have no keys.
- Package name on chain: `nearpad-token-tax` (this crate is `nearpad-token-tax-v1` only so the workspace resolves).

Rebuild and check:

```sh
legacy/rebuild.sh token-tax-v1
```

Verified 2026-09-30: `legacy/rebuild.sh token-tax-v1` = `003b1b46…` = the live global code.

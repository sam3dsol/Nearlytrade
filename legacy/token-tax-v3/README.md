# Token (tax) v3: global contract `aca1a7f8…` (frozen)

- Global contract code hash `Cct6RbLZAEs5WmKkPFr4qqTAWKKzPhE4aTJHVwNLeGBS` (sha256 `aca1a7f83e219034ea96c0675bf3dff418b871d5a0f4213738a199aadd5f2f11`).
- Runs the tax tokens launched from #1901 to #2060 (v2 plus the in-flight tax lock); `contracts/token-tax` replaces it from #2061.
- Frozen: global contract code is immutable; token accounts reference it by hash and have no keys.
- Package name on chain: `nearpad-token-tax` (this crate is `nearpad-token-tax-v3` only so the workspace resolves).

Rebuild and check:

```sh
legacy/rebuild.sh token-tax-v3
```

Verified 2026-10-01: `aca1a7f8…` = the live global code.

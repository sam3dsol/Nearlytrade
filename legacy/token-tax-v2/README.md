# Token (tax) v2: global contract `08134395…` (frozen)

- Global contract code hash `YXJL2KYynDA52JPBpAP5qDZhJqCTfB6dR5Ee4jdEgDK` (sha256 `08134395464cd03f468b760e02cea160f9835649221bbb520127e524bc55160e`).
- Runs the tax tokens launched from the 2026-09-28 auto-registration change to launch #1900; tax v3 (`aca1a7f8…`) replaced it from #1901.
- Frozen: global contract code is immutable; token accounts reference it by hash and have no keys.
- Package name on chain: `nearpad-token-tax` (this crate is `nearpad-token-tax-v2` only so the workspace resolves).

Rebuild and check:

```sh
legacy/rebuild.sh token-tax-v2
```

Verified 2026-09-30: `08134395…` = the live global code. (Built in place under `legacy/` the wasm differs
only by the embedded source path string, which is why the script builds it at `contracts/token-tax`.)

# Token (raw) v1: global contract `47d7973b…` (frozen, runs $NEARLY)

- Global contract code hash `5qScjXG9uRG82Yrh36XZNZgVUYiqPtuC9ep5DvV42eGQ` (sha256 `47d7973b0a3bf97a5008f03c7caf15d02fc05ac0c34c1878bdf7ebabf6857fc9`, 15,743 bytes).
- Runs **$NEARLY** (`nearly-993927.nearlytrade.near`, launch #3) and the other no-tax tokens launched before the 2026-09-28 auto-registration change.
- Frozen: global contract code is immutable; token accounts reference it by hash and have no keys.
- Package name on chain: `nearpad-token-raw` (this crate is `nearpad-token-raw-v1` only so the workspace resolves).

Rebuild and check:

```sh
legacy/rebuild.sh token-raw-v1
# = RUSTFLAGS="-C target-cpu=mvp -C target-feature=-bulk-memory,-sign-ext,-reference-types,-multivalue,-nontrapping-fptoint" \
#   cargo +1.96.0 build --release --target wasm32-unknown-unknown -p nearpad-token-raw   (no wasm-opt)
```

Verified 2026-10-01: `legacy/rebuild.sh token-raw-v1` = `47d7973b…` = the live global code, byte for byte.

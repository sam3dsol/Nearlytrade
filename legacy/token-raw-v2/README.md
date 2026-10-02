# Token (raw) v2: global contract `95ec4ec3…` (frozen)

- Global contract code hash `B6EjqsNJXfQXYX1KkUaypN8z7KpYcBrf9JNwTVtWUUBY` (sha256 `95ec4ec33ef96f387164dbb894481f38b6b140ad2aa4059309576208e52cad17`).
- Runs the no-tax tokens launched from the 2026-09-28 auto-registration change to launch #2060; `contracts/token-raw` replaces it from #2061.
- Frozen: global contract code is immutable; token accounts reference it by hash and have no keys.
- Package name on chain: `nearpad-token-raw` (this crate is `nearpad-token-raw-v2` only so the workspace resolves).

Rebuild and check:

```sh
legacy/rebuild.sh token-raw-v2
```

Verified 2026-10-01: `95ec4ec3…` = the live global code.

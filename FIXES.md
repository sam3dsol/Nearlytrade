# ContractWolf audit fixes

Audited tree: commit dbb5ca5. Every change below is in this commit; line numbers are of the files at this commit.

## Factory `contracts/factory/src/lib.rs`

| Finding | Status | Lines | Change |
|---|---|---|---|
| TAX-SELLER-01 | Fixed | 1797-1812, 1839-1841, 1848-1853, 3374-3380, 453, 1968, 4445, 2715-2721 | `process_tax` requires `min_out` and keeps it as the slice's floor (`tf:` counter). `internal_tax_settle` refuses proceeds below `floor * sold / held` (exact 256-bit `mul_div_wide`, since floor and token counts are both ~1e22 on mainnet) and consumes that part of the floor. A slice that never reaches the seller takes its floor back (`on_tax_to_seller`; its new `floor` argument is optional so a slice in flight across the deploy still resolves). `get_tax` exposes `floor`. Owner `clear_tax_floor` for a slice whose market fell under its quote. |
| RCW-015 | Fixed (refund) | 1068-1075 | `on_pool_checked`: a pool at another point refunds the refused `create_pool` deposit (0.1 NEAR) to the creator before the launch is marked failed. Account names stay as they are (the counter naming is a product choice); the squat now costs the attacker more than the creator. |
| BUYBACK-EXCESS-01 | Fixed | 1722-1730 | `on_bb_after`: the part of the balance difference above 104% of `min_out` is booked to the launch's `creator_token_fees` (burn only for a burn launch) instead of staying outside every bucket. |
| WNEAR-SWEEP-01 | Fixed | 1255-1268, 1289-1294, 1702-1707, 2316, 2656-2662, 2669-2713, 229, 4446-4451 | Every failed unwrap is booked to its launch (`wo:` creator, `wb:` buyback bucket) and counted in `uwt`, so `sweep_wnear` stays blocked. `release_dev_buy` moves a refused buy's record to `wo:` instead of dropping it; a late successful unwrap pays it once. New `get_wnear_owed`, `retry_wnear_owed` (anyone), `on_wnear_owed_unwrapped`. |
| RCW-014 | Intended | 1757-1760 (doc) | Holder payouts are computed by the keeper off the holder index; the contract caps every round to what the launch earned for its holders. Kept as designed. |
| ADMIN-KEY-01 | Operational | | No code change. After this build is live: owner methods move to function call keys (`add_fc_key`), the full access key is deleted. |
| LOCKER-SUPPLY-01 (factory side) | Fixed | 2850-2861 | Owner `locker_release_supply(launch_id)` tells a lock_6 locker a failed launch's supply is a remainder. |
| TAX-VENUE-01 (factory side) | Fixed | 2863-2873 | Owner `token_add_pair(launch_id, pair)` calls the token's `tax_add_pair`. |

Tests: 137 pass (`cargo test`), new: `audit_the_sellers_proceeds_must_clear_the_keepers_floor_pro_rata` (5390), `audit_failed_unwraps_stay_owed_to_their_launch_and_a_retry_pays_them` (5289), `releasing_a_dev_buy_keeps_a_retried_unwrap_owed_to_the_creator` (7388), extended `the_buyback_credit_is_capped_at_104_percent_of_min_out` and `a_pool_at_another_price_fails_the_launch_and_frees_the_dev_buy`.

## Locker `contracts/locker3/src/lib.rs` (deploys as a new account, lock_6)

| Finding | Status | Lines | Change |
|---|---|---|---|
| LOCKER-SUPPLY-01 | Fixed | 327-331, 390, 447-452, 997-999, 1051-1061, 1276-1280, 1339-1340 | `supply_reserved[token]` is set on every HotZap send (`add`, `add_buy`, resend) and cleared when `record` books the position. `on_admin_asset` counts it as not free. Factory only `release_supply(token)` for a failed launch. View `get_supply_reserved`. |

Tests: 64 pass (59 + 5 new, lines 1839-1888, 2526-2572 and the add_buy module).

## Tax token `contracts/token-tax/src/lib.rs` (new global code for future launches; live tokens are immutable)

| Finding | Status | Lines | Change |
|---|---|---|---|
| TAX-VENUE-01 | Fixed | 10-14, 19-20, 561-565, 598-601, 638, 792-824 | Admin only, add only `tax_add_pair`; `dcl_id` constant removed; auto registration of swap payouts now keys on any listed pair. |
| MAXWALLET-PAYOUT-01 | Fixed | 520-524, 545-546, 644 | `check_max_wallet` never refuses a transfer from or into a listed pair (a payout follows a swap that already ran). |
| TAX-LP-01 | Intended | | Liquidity flows stay taxed (else range orders trade tax free). The launch page and docs state it. |

Sim: `sim/audit_fixes.mjs` 62 checks, `sim/tax_in_flight.mjs` 102, `sim/final_checks.mjs` 133, all pass.

## Artifacts (recipes as in README.md)

| File | Bytes | sha256 |
|---|---|---|
| artifacts/factory-auditfix.wasm | 624,090 | c427d8f602a472cca33c63f29026e1d43f2f00bee4ed1b48b60b9d87498380a6 |
| artifacts/locker6.wasm | 287,321 | 0097d9a9244a763c992931682a01f86395f984a16043d06f43730cb923b535e5 |
| artifacts/token-tax-v5.wasm | 21,568 | f4265f2230721b2df18a0d78f36b737fe77d955194519a77578b2463602ac3a4 |

Keeper note: `process_tax` now needs a real `min_out` (the taxcrank already sends 97% of a fresh quote); the seller's own sale should use `max(own floor, get_tax.floor * amount / selling)` as its `min_output_amount` so a sale never settles below the floor.

## Live on mainnet (2026-10-01, 21:05 to 21:20 UTC)

| Contract | Account | Code hash (base58) | sha256 | Tx |
|---|---|---|---|---|
| Factory | nearlytrade.near | C9wqAEFGdnCHysNtoFhrpWArxK19chCWbyYTPyMGjmLq (2026-10-03 06:31 UTC, fee split 70 / 20 / 10; before it 8dJzWRwu… = 714f0f88 from 2026-10-02 19:02 UTC, 7DxV8GDC… = 5c77d409 tax_return from ~11:00 UTC, HrNxUUji… = fa6146d5 from ~09:0x UTC tx 2ouURYue…, jxdDdqoj… = 0b0149eb for ~1.5 h, 5Uux4bxF… = 42954cb0 from 05:1x UTC, and the audit-fix code ECi9hvT5… = c427d8f6 from 2026-10-01 21:08 UTC) | a5bb331353f7381889bb6766b8a707316a0915b1c0af99fc2204c5ae61f72aae | 7y37HWECaHeFmbshWDwES2FpuxBP4aa1Y5nekBVM6gFv (deploy, then `set_burn_pot(None)`); before it Ejp7RVPL6z37G12L3daUUTAYVYsofbZG6TDBLjLmYbUG; pots set in EbefPH5N… (referralpot.near) and 9ceXJrEe… (burnpot.near) |
| Locker (launches from #2192) | lock_6.nearlytrade.near | 13KJEWasGi6jrk3X99FDqeCbZ3Uqdpbbkd27fzSkx4Ut | 0097d9a9244a763c992931682a01f86395f984a16043d06f43730cb923b535e5 | created in the same run, 0 access keys |
| Tax token v5 (global code, tax launches from the switch on) | published by nearlyops.near | HS4R2isPS7hnnY8k2QbxnLHUx9Z9v9mNZnLsrGqwhChh | f4265f2230721b2df18a0d78f36b737fe77d955194519a77578b2463602ac3a4 | DuDAZFmo36G3HJhQSoydcepo8ogRrUPwNtPAs4VMSijp (publish), BcBUzpJjaQzdq1ToCScXBzi7mJg871UvCgTAxA93pDyk (factory switch) |

Earlier launches keep their lockers (lock to lock_5) and token codes, as before. The TAX-LP-01 note is live on nearly.trade/docs#tax and on the launch page.

## Withdrawn (2026-10-02): launches paying their own record

Built and tested (factory wasm `2431d3c5…`, repo commit a81f0be) but not deployed: the launch price stays as it was (0.16 NEAR plain, about 0.27 with a logo). The code was removed again in the next commit; the factory's own storage for new launches is instead cut by the change below.

## Follow-up (2026-10-02): the logo lives on the token only

`on_created` drops the icon from the factory's launch record once the token exists (the token's `ft_metadata.icon`, NEP-148, is where wallets, Rhea and explorers read it); the record keeps only the icon's length under the raw key `il:` so a retry still prices the token storage. Existing launches keep their stored icons. Lines: `contracts/factory/src/lib.rs` `icon_len_key` / `icon_len_of` helpers next to `wnear_retry_key`, `on_created` after the token-create error branch, `resume` and the create-error path use `icon_len_of`. Test `the_icon_stays_in_the_factory_record_and_goes_to_the_token_as_live` updated. Factory wasm `42954cb0788507f28eb67f53094f89714407a02b40dafd7905550484a69a331b` (624,369 bytes). The site's indexer takes a missing logo from the token's metadata.

## Follow-up (2026-10-02): the fee split is 70 / 20 / 5 / 5

Not an audit finding. Every launch from this build on splits its pool fee four ways, written into the launch at creation (the existing per-launch `launch_recipients`), never changed after: 70% creator (`MIN_CREATOR_SHARE_BPS` = `MAX_CREATOR_SHARE_BPS` = 7,000, the only value `set_config` accepts; a launch may not carry another), 20% protocol recipients (`PROTOCOL_BPS`), 5% burn pot (`BURN_POT_BPS`, owner `set_burn_pot`), 5% referral pot (`REFERRAL_POT_BPS`, owner `set_referral_pot`); a compile time assert keeps the four at 10,000. A new tax launch's tax splits the same four ways: the creator's side keeps 70% (split creator / burn / holders as the creator chose), 20% protocol, 5% burn pot, 5% referral pot; the non-creator 30% is stored in the field that has always been called `platform_bps` (`PLATFORM_TAX_BPS` = 3,000, which is `PROTOCOL_BPS + BURN_POT_BPS + REFERRAL_POT_BPS`) and routes through the same per-launch recipients. A pot that is not set leaves its part with the protocol. There is no per-launch referrer: the earlier `LaunchArgs.referrer` (deployed for about two hours as `jxdDdqoj…`) is removed; the few launches made with it keep the recipients they were created with. Launches before 2026-10-02 keep their stored 80 / 20 and 20% tax cut. Lines: constants after `TOKEN_BASE_STORAGE_BYTES`, `MIN/MAX_CREATOR_SHARE_BPS`, `PLATFORM_TAX_BPS`; `pot_recipients` in `impl Factory` before `internal_set_launch_recipients`; `launch` after the record insert (`launch_split` event); `internal_tax_distribute` native branch; `set_burn_pot` / `set_referral_pot` before `set_tax_seller`. Test `every_new_launch_splits_its_fee_70_20_5_5_and_the_creator_cannot_pick`. Factory wasm `fa6146d59b8e9d10a5a66c32bfc5c8845bf2b1fa2ff7ea92b62f565cd7cb18f3` (629,747 bytes).

## Follow-up (2026-10-02): an unsold tax slice can come back

Closes the one manual step the TAX-SELLER-01 fix left: when a token's market falls under the floor a slice was handed over with, the seller cannot settle it and the owner had to `clear_tax_floor`. Now the seller hands the slice back instead: `ft_on_transfer` accepts the launch's own token from the tax seller with msg `{"tax_return": <id>}` (at most what the seller holds for that launch; refused in full for any other sender, amount or while the token is swap-locked), moves it back to pending, drops its pro rata part of the floor, emits `tax_returned`; the next `process_tax` quotes it fresh. The keeper does this automatically when its sale quote is under the floor. `clear_tax_floor` stays as a spare. Lines: `ft_on_transfer` (the `tax_return` branch before the `tax_proceeds` parse). Test: the `TXR` block in `audit_the_sellers_proceeds_must_clear_the_keepers_floor_pro_rata`. Factory wasm `5c77d4090e0bcf400adb16a67f9c22e37b5a7d1b09c3768f2ef4b9c6ebd56b8d` (630,460 bytes).

## Follow-up (2026-10-02): final factory build

Deployed 2026-10-02 19:02 UTC in tx `Ejp7RVPL6z37G12L3daUUTAYVYsofbZG6TDBLjLmYbUG`, code `714f0f88…` (base58 `8dJzWRwu…`), no state migration. Rebuild: `(cd contracts/factory && cargo near build non-reproducible-wasm --locked --no-abi)`. Four changes:

1. **Holders mode on every pair.** The launch no longer refuses `fee_mode: "holders"` on a pair asset. On a pair launch the creator's 70% of the quote side is banked in the launch's holders bucket in the pair token (it used to go to `creator_quote_fees`), and `pay_holders` pays it in that token, 10 lines a call, each leg registering the holder first; a leg that bounces is owed to that holder in the same token (`push_owed`). NEAR pair holders launches are unchanged. A creator tax share on a pair still needs fee mode creator.
2. **Buyback excess is reported, not booked.** `on_bb_after` caps what it burns at 104% of `min_out` as before; anything above the cap (a refund or another transfer landing in the window, already counted in its own bucket) is now only reported with `buyback_excess_ignored` and never credited to `creator_token_fees`.
3. **Pot checks.** `set_burn_pot` and `set_referral_pot` refuse the factory's own account. `set_protocol_recipients`, `set_burn_pot` and `set_referral_pot` refuse any combination where protocol recipients plus pots exceed the 4 legs a launch split carries; `pot_recipients` emits `pot_legs_dropped` if it ever falls back.
4. **Tickers up to 12 characters** (`symbol: 1-12 chars`).

Tests: 141 pass (`cargo test -p nearpad-factory`), new: `a_holders_launch_on_a_pair_banks_and_pays_its_holders_in_the_pair_token`, `a_pair_holders_launch_with_a_creator_tax_share_is_still_refused`, `pots_refuse_the_launchpad_and_a_split_that_would_not_fit`, extended `the_buyback_credit_is_capped_at_104_percent_of_min_out`. Mainnet check after the deploy: test launches in every fee mode on NEAR, DIARHEA, NINU, RHEA, USDC, ZEC and a stock pair, each with a buy, a sell and a claim, plus a burn-mode buyback, a NEAR holders payout and a DIARHEA holders payout.

## Follow-up (2026-10-03): the fee split is 70 / 20 / 10

Not an audit finding. Owner's decision: the burn pot part is gone and the referral pot takes 10%. `BURN_POT_BPS` = 0 and `REFERRAL_POT_BPS` = 1,000 (the compile time assert still adds the parts to 10,000); `PLATFORM_TAX_BPS` stays 3,000. A launch made from this build on stores 3 legs: the protocol recipients 2/3 and the referral pot 1/3 of the 30% the creator does not take, so 20% and 10% of the whole fee; a tax launch routes its 30% tax cut through the same legs. A burn pot with a 0 part adds no leg. After the deploy the owner called `set_burn_pot(None)`, so `get_burn_pot` is `null`. Launches keep the split written at their creation: 2026-10-02 to 2026-10-03 06:31 UTC 70 / 20 / 5 / 5, before that 80 / 20.

Deployed 2026-10-03 06:30:56 UTC in tx `7y37HWECaHeFmbshWDwES2FpuxBP4aa1Y5nekBVM6gFv`, code `a5bb3313…` (base58 `C9wqAEFG…`, 630,931 bytes), no state migration. Rebuild: `(cd contracts/factory && cargo near build non-reproducible-wasm --locked --no-abi)`. Tests: 141 pass; renamed `every_new_launch_splits_its_fee_70_20_10_and_the_creator_cannot_pick`, `pots_refuse_the_launchpad_and_a_split_that_would_not_fit` now expects 3 legs. Testnet first: launch recipients [protocol 3,333 + 3,334, referral pot 3,333], a claim paid the referral pot exactly 10% of the fees and the burn pot nothing.

## Security review fixes (2026-10-05)

Independent review of 2026-10-04 (commit `56fea17`, findings N-01 to N-12). Fixed in commit `d3c0e52`, deployed 2026-10-05.

| Finding | Severity | Status | Change |
|---|---|---|---|
| N-01 | Medium | Fixed | Factory: every dev-buy unwrap carries an operation id (`uf:` marker); only the callback holding the current id settles it, `release_dev_buy` never books an unwrap still in flight, a failed unwrap after Done is booked to `wo:`, and the owner's `resolve_unwrap(launch_id, landed)` is allowed only 30 minutes after the unwrap was sent. One settlement per unwrap. |
| N-02 | Medium | Fixed | Factory: `retry_wnear_owed` needs 50 TGas attached and gives its callback a fixed 20 TGas (weight 0), so the bookkeeping always runs; the in-flight record (`wi:`) carries the operation id, amount and bucket. |
| N-03 | Medium | Fixed | Locker: a failed or unreadable `get_tax` of a launch token is not cached; it is read again at the next claim. Factory: `token_add_pair` refuses a token that is an approved quote, so a quote's tax cannot change after a locker read it. |
| N-04 | Medium | Fixed (new launches) | Token: paid registrations record what they are owed back beyond their storage (`u`); automatic registration only spends balance above storage, margin and that amount. |
| N-05 | Medium, conditional | Not changed | A whole-system DCL migration is not planned; the oldest lockers are immutable. Disclosed in README. |
| N-06 | Low | Fixed (new launches) | Token: `ft_resolve_transfer` reports a burned refund as used, not returned. |
| N-07, N-08, N-10, N-11 | Medium/Low | Disclosed | In immutable earlier token generations only; the current token code does not have them. README, Known limitations. |
| N-09 | Medium | Mitigated | Factory: `collect_tax` is keeper only. The token-side behaviour stays in the immutable tax v1/v2 tokens. |
| N-12 | Low | Fixed | Factory: the text storage bound counts the 12-character symbol, so the quote covers the longest valid launch. |

Tests: factory 148, locker 65; token harnesses pass; init and transfer gas unchanged, automatic registration 1.19 to 1.26 TGas.

Builds: factory and locker are now cargo-near reproducible builds (README, Build): factory `6ca88c72…` = `8KA6c2mF…`, locker `cc375a25…` = `EkB8CKVf…`, raw `3c44c33e…` = `54GH1DmZ…`, tax `6bfc00b7…` = `8GXVnFTx…`.

Testnet first, on these exact bytes: the factory put back on the live `a5bb3313…` with existing launches and upgraded to `6ca88c72…` (views unchanged), a new `lock_7` for new launches, old launches still claiming through their locker; dev buy, raw and tax launches, claims, keeper-only `collect_tax`, paid registrations and the longest launch at its exact quote.

Mainnet, 2026-10-05:

| Step | Tx |
|---|---|
| Factory code `6ca88c72…` | `5VexucX6RspiP2ZafjHmgSLXTYGtojtxaj8XuHTRoqXq` |
| Publish raw `3c44c33e…` / tax `6bfc00b7…` | `2QPLWGoQ9JVAqK2BYX28p4MWfogXsoYJgNcNdLk2dqK6` / `EE6puaHuz1iY9JRZH1diHgURxG4P5tJwWh6f8Q9ALuXY` |
| Factory switches new launches to them | `BfYPUuYa9oVFmc3T5Qf4DLFabofCRwuGmFvdSAUGtEcX` |
| `lock_7` created (15 NEAR, code `cc375a25…`, no key) | `98RbJXGHxLNJpWtMooebG3zYRhmRo5vhzm2CkQTga38C` |
| `set_locker_from(lock_7, 2549)` + add gas 111 + add buy | `3beYRK3j59xA9DQ4iNZ6xvKkv7v1M4goKF6TAgBm5Gco` |

Mainnet check after the deploy: test launches #2546 to #2582 (name "test") on the NEAR pair in every option (plain, dev buy, tax with dev buy, burn mode, holders mode, a fee recipient) and on every one of the 30 approved pair assets, plus a tax launch on the $NEARLY pair; a trade filling a tax vault, `collect_tax` refused to anyone but the keeper, a claim through `lock_7`, no wNEAR owed on any launch. All Done.

ADMIN-KEY-01: the factory's full access key stays until the reviewers confirm; owner methods already have function call keys.

## Follow-up (2026-10-06): plain pair tokens forward their quote fees (lock_8)

The N-03 fix made a locker read a pair token's `get_tax` before it withdraws that side of the fees, and it treated any failed or unreadable read of a launchpad token as unknown: nothing withdrawn, read again on the next claim. A plain launchpad token has no `get_tax`, so the read fails on every claim. On `lock_7` the quote side of every launch paired with $NEARLY (raw v1, no `get_tax`) stays at the exchange: it is held in `lock_7`'s DCL balance, booked per launch (`get_owed_quote`), never lost, and `lock_7` has no key and no way to change that.

Every token template this launchpad deployed exports `get_tax` if and only if it carries a tax: checked on mainnet for all 2,593 launches to that date (the six tax generations export it, the four raw ones do not; the one exception, #90, failed with no code and can never be a pair). So in `lock_8` a FAILED view means no tax, for any token, and it is cached as 0. An unreadable answer from a launchpad token is still unknown and read again (N-03 kept for that case).

Tests: locker 66 (a failed read is no tax and the quote is withdrawn on the next claim; an unreadable answer is read again).

Build: locker `2f7bd173…` = `4CMfhWsU…` (commit `a476bbe`, README, Build).

Testnet first, on these exact bytes: a plain token launched on the testnet factory and approved as a pair asset, a launch on that pair on `lock_8`, trades both ways, claim 1 caches no tax, claim 2 books the quote fees to the creator and the protocol and leaves nothing owed or held.

Mainnet, 2026-10-06:

| Step | Tx |
|---|---|
| `lock_8` created (15 NEAR, code `2f7bd173…`, no key), registered on the DCL, wNEAR and all 30 approved pair assets | `GkwA3qx6R6Nm7ZGAjQdAwuWSYbRudKUU4EFXyedwaGMP` |
| `set_locker_from(lock_8, 2596)` + add gas 111 + add buy | `9xB7CoQLXGgJyHtqxeBQJMTZ2YR48Y94P2V9FmZgxmmm` |

Launches #2549 to #2595 stay on `lock_7` with the behaviour above.

## Follow-up (2026-10-07): the owner DAO upgrades the factory

Since 2026-10-06 the factory owner is the Sputnik DAO `nearlytrade.sputnik-dao.near` (4 signers, 3 of 4 approvals; it has no access keys). Every owner method runs only through a passed proposal. Until now an upgrade went through `upgrade(code_hash)`, which needs the code published as a global contract first (about 64 NEAR for the factory).

New owner method `upgrade_code()`: its raw call input is the wasm itself (not JSON). It refuses anything that does not start with the wasm magic bytes, logs an `upgrade` event with the sha256 of the code, then deploys it to the factory and calls the new code's `migrate` in the same receipt, so code without a callable `migrate` reverts and the factory keeps its code. This is the call Sputnik's `UpgradeRemote` proposal makes:

1. anyone stores the wasm in the DAO with `store_blob` (deposit = storage of the code); the DAO returns its sha256, which is the reproducible build hash
2. a proposal `UpgradeRemote { receiver_id: nearlytrade.near, method_name: upgrade_code, hash }` passes with 3 of 4 approvals and calls `upgrade_code` with the stored code
3. the storer takes the deposit back with `remove_blob`

The DAO marks a proposal Approved even if the call it makes fails, so after an upgrade check the factory's code hash on chain. The executing approval needs about 120 TGas (attach 300). `upgrade(code_hash)` stays.

Tests: factory 149 (`upgrade_code`: owner only, the factory's own keys are not the owner, empty and JSON input refused, one receipt = deploy + `migrate`, the event carries the sha256).

Build: factory `aba3be30…` = `CZ1XpbUk…` (commit `38abcd8`, README, Build).

Testnet first, on these exact bytes, a factory owned by a Sputnik DAO: direct deploy of this build, `upgrade_code` called directly by the factory's full key or by an outsider refused, JSON input from the DAO refused, DAO upgrades to the same bytes, to other code and back, and to the previous factory build; code without `migrate` refused with the code unchanged; one approval short leaves every proposal in progress; every deposit refunded.

Mainnet, 2026-10-07:

| Step | Tx |
|---|---|
| Factory code `aba3be30…` deployed, views unchanged | `D1vNiZErqh7tXBCpcjzhKRst2U9afGuSToXmEYBgW8Cc` |
| `store_blob` of the same bytes in the DAO (returns `CZ1XpbUk…`) | `GyMo6SgCeRTADYhjoNdMiCc3YdhBiQJ8Y83xbGqQ46hT` |
| Proposal #5 `UpgradeRemote` to `upgrade_code`, approvals 1 and 2 leave it in progress | `ASWVnKT76rCn9Y1Z3KvtoVJVYuHQe5f9f1fsdWMPeABV`, `BVvuN53dhykhjgK3sbYKmuims8FzrSkCzhfR6pyRyNy3`, `FN1tNRk3Noju2vgU9oErXuiMRNfEtqjaKaZ362AwgopN` |
| Approval 3 executes: `upgrade` event `aba3be30…` raw, code `CZ1XpbUk…`, views unchanged | `GLbDAGenjZ7h65hdhqYM5hUwZncQQi5VwEYGAAZzFvHP` |
| `remove_blob`, deposit refunded | `9HfiUsDBPxuDCeRkiG3c7QYJzidtFMbPzvq33XcDpyky` |

## Follow-up (2026-10-07): the owner DAO creates lockers

A locker is a sub-account `lock_N.nearlytrade.near` with no access key, so only the factory account can create one. Until now that took the factory's own key. New owner method `create_locker()`: like `upgrade_code`, its raw call input is the locker wasm (Sputnik's `UpgradeRemote` sends it from the DAO's blob store). In one batch it creates the next `lock_N` (`get_next_locker`), funds it with the whole locker reserve, deploys the code and calls `new(factory, dcl, wnear)`, adding no key. It refuses input that is not wasm and a reserve below the code's storage plus 1 NEAR. If the batch fails the reserve is booked back and that number is skipped. Switching new launches to the locker stays a separate owner call (`set_locker_from`).

The locker reserve is its own balance (`get_locker_reserve`), never creator, holder or protocol money: anyone adds to it with `fund_locker_reserve()` (payable), the owner takes it back with `withdraw_locker_reserve(to, amount)` (a send that bounces is booked back). A locker needs its code's storage (about 2.9 NEAR) and a little for its records; earlier lockers were given 15 NEAR and use about 3.

`upgrade_code`, `upgrade(code_hash)` and `migrate` are unchanged.

Tests: factory 152 (reserve funded by anyone and withdrawn by the owner only, a bounced withdraw booked back; `create_locker` owner only, empty, JSON and short-reserve input refused, the next number after the highest `lock_N`, one batch = create + whole reserve + deploy + `new`, a failed batch re-credits the reserve and skips the number).

Build: factory `56eb79cb…` = `6rJKScfD…` (commit `034a8e4`, README, Build).

Testnet first, on these exact bytes, with the factory owned by a Sputnik DAO: the DAO upgraded the factory to this build, refused calls (owner only, private callbacks, an empty fund, a short reserve, input that is not wasm), a batch whose `new` fails re-credits the reserve with no account made, a real locker created from the `lock_8` code with no key, initialized and registered on the DCL, new launches switched to it, a launch with a dev buy, a trade and a claim on it, the reserve withdrawn.

Mainnet, 2026-10-07, upgraded by the owner DAO (no locker created yet):

| Step | Tx |
|---|---|
| `store_blob` of the build in the DAO (returns `6rJKScfD…`) | `HR6dspDmXgicMDoMfQhFvDaxqLRfakTKCUPyePhQfoRZ` |
| Proposal #7 `UpgradeRemote` to `upgrade_code`, approvals 1 and 2 leave it in progress | `HjicnqPdws3aGBrSxUAqim2ZYrqmxge3vm4w8b3JZ6f`, `5M9sjNoSoLod5Fgk3G23ruCi9RR4UxPsqLQc2ayyDEzN`, `61dKESv3HFkYhrhkKuExWfGGBABAXk4ijwHwkatvXVmt` |
| Approval 3 executes: `upgrade` event `56eb79cb…` raw, code `6rJKScfD…`, views unchanged | `HMJd2wVyRUNqmtLp9V8G9XZVn2fGGzU4E23Cayiwxiwu` |
| `remove_blob`, deposit refunded | `2NwkdKRp3bku22Qp39Gh5M7i1Vs8KYzFtHzkUYrq1xEn` |

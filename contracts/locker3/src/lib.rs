//! Liquidity locker v3 (lock3): the account that OWNS every launch's DCL position. It has exactly two
//! jobs: place the supply as a single-sided range (`add`) and pull LP fees off a position (`claim`),
//! forwarding them to the factory. There is no method that removes liquidity, and after deployment the
//! account's access keys are deleted, so the code can never change: liquidity is provably locked while
//! the factory next door stays upgradable.
//!
//! The factory-facing interface is lock2's, method for method, so the live factory switches to this
//! account with `set_locker_from` and no redeploy of its own.
//!
//! What changed from lock2, and why:
//! - `add` opens the position with ONE `ft_transfer_call` whose msg is DCL's `HotZap`, instead of a
//!   `"Deposit"` followed by `add_liquidity`. dclv2 v2.3.13's Deposit reads every pool id on every call
//!   (~1,415 storage reads, ~208 TGas on 2026-09-30, +0.08 TGas per new pool), so it stops fitting in
//!   even a 1000 TGas transaction within weeks. HotZap burns ~12 TGas whatever the pool count.
//! - A HotZap does not return the position id, so the locker reads it back with `list_liquidities`,
//!   records it per pool, and returns it to the factory exactly as lock2 returned add_liquidity's.
//! - A repeated `add` for the same pool never sends the supply twice: it returns the recorded position,
//!   finds it on the exchange, or opens it from what already sits in this account's DCL balance.
//! - `claim` leaves the fees in this account's DCL balance, withdraws exactly this launch's share, and
//!   forwards only in the callback that saw it arrive. lock2 forwarded in the same block DCL started
//!   its payout, out of a balance every launch shares, so one launch's fees could fund another's.
use near_sdk::json_types::U128;
use near_sdk::store::LookupMap;
use near_sdk::{env, near, require, AccountId, Gas, GasWeight, NearToken, PanicOnDefault, Promise, PromiseError, PromiseOrValue};

const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);
const NO_DEPOSIT: NearToken = NearToken::from_yoctonear(0);
const FT_STORAGE_REG: NearToken = NearToken::from_yoctonear(1_250_000_000_000_000_000_000);
/// dclv2's storage_balance_bounds().min
const DCL_REGISTER: NearToken = NearToken::from_millinear(500);

const fn tg(g: Gas) -> u64 { g.as_gas() / 1_000_000_000_000 }
/// What CREATING one function-call receipt costs the creating call, on top of the gas it attaches.
///    Callback floors count it per receipt they create; leaving it out is how a callback ends up short
///    with every static number looking right. Mainnet (protocol 86): function_call send 0.2 TGas +
///    receipt creation 0.108 + ~0.05 per KB of args, so ~0.35; 1 is ~3x that. On neard 2.13.4 with the
///    real dclv2 v2.3.13 wasm, on_claimed burned 2.93 TGas in total while creating three receipts.
///    (near-sdk's unit-test mock still charges the old 2.3 TGas per function call, so unit tests that
///    schedule a whole chain attach more than the factory does.)
const T_RECEIPT: u64 = 1;
/// a callback's own execution: read its args and promise results, a few storage reads and writes
/// (measured 1.6 to 2.9 TGas for every callback here, receipts included)
const T_OWN: u64 = 3;
/// what a callback that decides at runtime keeps back for its own remaining execution and the two
/// receipts it may still create (see `can_afford`)
const GAS_TAIL: Gas = Gas::from_tgas(T_OWN + 2 * T_RECEIPT + 1);


/// storage_deposit on the launch token for the exchange (refunded when it is already registered)
const GAS_FT_REG: Gas = Gas::from_tgas(5);
/// The `ft_transfer_call` carrying the HotZap. Our tokens forward `prepaid - used(~3.2) - 15 - 5` to
/// the exchange, and dclv2 v2.3.13 burned 11.9 TGas on exactly this message for a 1B supply
/// (mainnet tx 73GLYtcHVHkHww8H3r6mo9tnm3cYTZjF7igSSm2w7n1J, ~40 storage reads, independent of the
/// pool count). 50 forwards ~27: a 2.3x margin. Unlike lock2's Deposit this does not grow with the
/// exchange, so a fixed number is the right shape and every spare TGas goes to the lookup instead.
const GAS_ZAP: Gas = Gas::from_tgas(50);
/// `list_liquidities` for one page of LIST_PAGE rows. Each row makes DCL compute that position's
/// unclaimed fees: measured 3.27 TGas for 1 row and 10.7 for 8 (+1.06 a row) on the real dclv2
/// wasm, and `from_index` costs nothing (reading from index 7 cost the same as from 0). The page stays
/// small; the position is almost always the first row read. 22 is 2x a full page.
const GAS_LIST: Gas = Gas::from_tgas(22);
const LIST_PAGE: u64 = 8;
/// `get_user_asset` on the exchange: one map read
const GAS_ASSET: Gas = Gas::from_tgas(10);
/// add_liquidity from the exchange balance. Measured for lock2: burns ~9.5 plus a 3.8 refund subtree;
/// 28 and 34 failed on mainnet, 52 passed.
const GAS_ADD_LIQ: Gas = Gas::from_tgas(62);
/// Static floors of the add callbacks. Each also takes every spare TGas by weight, and checks at
/// runtime that it can afford its next step before scheduling it (see `can_afford`).
/// on_listed: parse one page (8 rows, ~3 KB of JSON) and record a find; continuing needs more, by weight
const GAS_CB_LISTED: Gas = Gas::from_tgas(10);
/// on_zapped: its own work before the check, then what `can_afford` asks for a lookup
/// (list_liquidities + on_listed, plus the tail). Measured: the check runs after ~1.7 TGas.
const GAS_CB_ZAPPED: Gas = Gas::from_tgas(2 + 2 * T_RECEIPT + tg(GAS_LIST) + tg(GAS_CB_LISTED) + tg(GAS_TAIL));
/// on_asset: read one number and decide; continuing needs more, taken by weight
const GAS_CB_ASSET: Gas = Gas::from_tgas(8);
/// on_added: record the position id
const GAS_CB_ADDED: Gas = Gas::from_tgas(6);

/// what contracts/factory attaches to `add` at the least (GAS_LOCKER_ADD); inside the launch
/// transaction it passes everything it can spare, ~207 TGas, and ~267 from a resume
const T_FACTORY_BUDGET: u64 = 180;
/// add() = one batch receipt with two function calls (storage_deposit + the HotZap) and on_zapped
const T_ADD_NEEDS: u64 = T_OWN + 3 * T_RECEIPT + tg(GAS_FT_REG) + tg(GAS_ZAP) + tg(GAS_CB_ZAPPED);
const _: () = assert!(T_ADD_NEEDS <= T_FACTORY_BUDGET, "locker.add no longer fits the factory's GAS_LOCKER_ADD");
/// the HotZap has to reach the exchange with a margin over what it burns (11.9 measured)
const _: () = assert!(tg(GAS_ZAP) >= 4 + 15 + 5 + 24, "GAS_ZAP no longer forwards 2x a HotZap's burn to the exchange");


/// wrap.near `near_deposit`
const GAS_WRAP_DEPOSIT: Gas = Gas::from_tgas(5);
/// The `ft_transfer_call` on wrap.near that carries the first buy. wrap.near forwards `prepaid - 30`
/// to the exchange, and dclv2 burned 12.3 and 13.6 TGas on the first two buys into PLAID's pool
/// (mainnet, 2026-09-30). A swap with `swap_out_recipient` also pays the output transfer and its
/// callback, and the exchange refuses to start one with under ~50: measured on the real dclv2
/// wasm, 40 fails with "Exceeded the prepaid gas" after the swap, 50 passes (transfer 27,
/// callback 10). 85 leaves it 55, and the hop passes every spare TGas on as well.
const GAS_BUY: Gas = Gas::from_tgas(85);
/// on_refunded / on_unwrapped: read one result, book, maybe one transfer
const GAS_CB_REFUNDED: Gas = Gas::from_tgas(T_OWN + 2);
const GAS_CB_UNWRAPPED: Gas = Gas::from_tgas(T_OWN + 2 * T_RECEIPT + tg(GAS_CB_REFUNDED));
/// on_bought: read the swap's result, then an unwrap or a transfer and their callbacks
const GAS_CB_BOUGHT: Gas = Gas::from_tgas(T_OWN + 2 * T_RECEIPT + tg(GAS_UNWRAP) + tg(GAS_CB_UNWRAPPED));
/// buy_hop: the wrap.near batch (near_deposit + the swap) and on_bought
const GAS_BUY_HOP: Gas = Gas::from_tgas(T_OWN + 2 * T_RECEIPT + tg(GAS_WRAP_DEPOSIT) + tg(GAS_BUY) + tg(GAS_CB_BOUGHT));
/// on_add_bought: read two results, return
const GAS_CB_ADD_BOUGHT: Gas = Gas::from_tgas(T_OWN + 2);
/// add_buy() = the token batch (storage_deposit + HotZap) with on_zapped, the hop, and on_add_bought
const T_ADD_BUY_NEEDS: u64 = T_OWN + 4 * T_RECEIPT + tg(GAS_FT_REG) + tg(GAS_ZAP) + tg(GAS_CB_ZAPPED) + tg(GAS_BUY_HOP) + tg(GAS_CB_ADD_BOUGHT);
/// what the factory has to attach to `add_buy`; inside a 300 TGas launch transaction it has ~230
/// left for this step once its own callback is paid for
const T_FACTORY_ADD_BUY: u64 = 230;
const _: () = assert!(T_ADD_BUY_NEEDS <= T_FACTORY_ADD_BUY, "add_buy no longer fits what the launch transaction can attach");
/// the swap has to reach the exchange with what it refuses to run under (50 measured), plus a margin
const _: () = assert!(tg(GAS_BUY) >= 30 + 50 + 5, "GAS_BUY no longer gets a first buy past the exchange's own minimum");


/// what contracts/factory attaches to `claim` (GAS_LOCKER_CLAIM, static)
const T_FACTORY_CLAIM: u64 = 180;
/// The floor for remove_liquidity. It is not a fixed allowance any more: the call also takes every
/// TGas `claim` arrives with beyond what the callback chain needs (unused-gas weight 1, the callback
/// 0), so whatever the factory attaches reaches the exchange without this account changing. A fixed
/// 90 was lock2's, and a fixed number is exactly what walled lock.nearlytrade.near's Deposit.
/// Measured: remove_liquidity burned 7.7 TGas on mainnet (lock2, tx 8NG5q6zQ, with its payout).
const GAS_REMOVE: Gas = Gas::from_tgas(20);
/// DCL `withdraw_asset`: burns ~4.3 and gives the ft_transfer to this account `prepaid - 16`, its
/// callback 10. It refuses anything under ~36 ("Exceeded the prepaid gas" at 35, fine at 38, on the
/// real dclv2 wasm), so 45 leaves the transfer 29 for a burn of ~1.7 (wrap.near).
const GAS_WITHDRAW: Gas = Gas::from_tgas(45);
/// wrap.near `near_withdraw`: burns ~1.8 and returns the NEAR transfer to this account
const GAS_UNWRAP: Gas = Gas::from_tgas(10);
/// ft_transfer of fees to the factory
const GAS_FORWARD: Gas = Gas::from_tgas(15);
/// on_delivered: return the booked amounts; creates nothing
const GAS_CB_DELIVERED: Gas = Gas::from_tgas(T_OWN + 2);
/// on_forwarded: read two results, book, and (NEAR pair) the NEAR transfer + on_delivered
const GAS_CB_FORWARDED: Gas = Gas::from_tgas(T_OWN + 2 * T_RECEIPT + tg(GAS_CB_DELIVERED));
/// a pair token's `get_tax` view (the tax tokens' own: a storage read and a short JSON). It takes the
/// place of the quote's withdraw leg on the first claim of that asset, so it fits that leg's budget.
/// 10-01 audit: 20, not 5. Its answer is cached for good, so a view that ran short must never be
/// read as "no tax"; unused gas is refunded.
const GAS_TAX_VIEW: Gas = Gas::from_tgas(20);
const _: () = assert!(tg(GAS_TAX_VIEW) <= tg(GAS_WITHDRAW), "the tax view replaces a withdraw leg");
/// on_withdrawn: read two results (or a withdrawal and the quote's tax config) and parse that config,
/// then the forward legs (worst case: an unwrap or a transfer, plus a token transfer) and on_forwarded
const GAS_CB_WITHDRAWN: Gas = Gas::from_tgas(T_OWN + 1 + 3 * T_RECEIPT + 2 * tg(GAS_FORWARD) + tg(GAS_CB_FORWARDED));
/// on_claimed: book the release, then two withdraw legs (or one and the tax view) and on_withdrawn
const GAS_CB_CLAIMED: Gas = Gas::from_tgas(T_OWN + 3 * T_RECEIPT + 2 * tg(GAS_WITHDRAW) + tg(GAS_CB_WITHDRAWN));
const _: () = assert!(tg(GAS_FORWARD) >= tg(GAS_UNWRAP), "the quote leg budget must cover an unwrap");
/// claim() = remove_liquidity floor + the callback chain, plus its own execution and two receipts;
/// the difference goes to remove_liquidity by weight
const T_CLAIM_NEEDS: u64 = T_OWN + 2 * T_RECEIPT + tg(GAS_REMOVE) + tg(GAS_CB_CLAIMED);
const _: () = assert!(T_CLAIM_NEEDS <= T_FACTORY_CLAIM, "claim no longer fits the factory's GAS_LOCKER_CLAIM");


const GAS_CB_ADMIN_SENT: Gas = Gas::from_tgas(T_OWN + 2);
const GAS_CB_ADMIN_WITHDRAWN: Gas = Gas::from_tgas(T_OWN + 2 * T_RECEIPT + tg(GAS_FORWARD) + tg(GAS_CB_ADMIN_SENT));
const GAS_CB_ADMIN_ASSET: Gas = Gas::from_tgas(T_OWN + 2 * T_RECEIPT + tg(GAS_WITHDRAW) + tg(GAS_CB_ADMIN_WITHDRAWN));
/// withdraw_dcl_asset() = get_user_asset + that chain; the factory has to attach at least this
const T_ADMIN_WITHDRAW: u64 = T_OWN + 2 * T_RECEIPT + tg(GAS_ASSET) + tg(GAS_CB_ADMIN_ASSET);
const _: () = assert!(T_ADMIN_WITHDRAW <= 300, "withdraw_dcl_asset no longer fits a transaction");

/// 10-01 audit: what `add`, `add_buy` and `claim` need free on this account (balance less storage stake
/// and the call's own deposit) before they start. Each launch costs this account ~0.0034 NEAR of its own
/// (a token registration for the exchange, the position record); a claim whose callbacks then run
/// short of storage stake would strand fees that already left the position.
const MIN_FREE: u128 = 100_000_000_000_000_000_000_000;
/// a claim's whole chain runs in a few blocks; a count older than this was left by a callback that died
const CLAIM_OPEN_EXPIRY_MS: u64 = 600_000;

const EVENT_STANDARD: &str = "nearpad";
const EVENT_VERSION: &str = "2.0.0";

/// What the factory sends to `add`. The JSON shape is lock2's and must stay so: the live factory
/// formats it by hand.
#[near(serializers = [json])]
#[derive(Clone, Debug)]
pub struct AddArgs {
    pub token: AccountId,
    pub pool_id: String,
    pub left_point: i32,
    pub right_point: i32,
    pub amount_x: U128,
    pub amount_y: U128,
}

/// The first buy that rides with `add_buy`: `amount` yoctoNEAR (attached to the call) are wrapped and
/// sent to the exchange with `msg`, the factory's own swap message (`Swap` with its floor and
/// `swap_out_recipient`, or `SwapByOutput`), untouched. The locker never decides what is bought.
#[near(serializers = [json])]
#[derive(Clone, Debug, PartialEq)]
pub struct BuyArgs {
    pub amount: U128,
    pub msg: String,
}

/// What `add_buy` returns to the factory.
#[near(serializers = [json])]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct AddBuyResult {
    /// the position, exactly as `add` returns it ("" = not yet)
    pub lpt_id: String,
    /// wNEAR the exchange kept for the buy
    pub used: U128,
    /// NEAR sent back to the factory; anything else of the buy is in `get_buy_held`
    pub refunded: U128,
}

/// The buy leg's own result, before it is joined with the position id.
#[near(serializers = [json])]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BuyOutcome {
    pub used: U128,
    pub refunded: U128,
}

/// What happened to a launch's first buy here, step by step (`get_first_buy`). The factory books a first buy
/// whose own callback was lost from this record alone (`settle_first_buy`), so every field is written by the
/// step that knows it, and `settled` only once nothing in the `add_buy` chain can move that NEAR any more.
#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct FirstBuy {
    /// NEAR attached to `add_buy`
    pub amount: U128,
    /// the hop wrapped it and sent it to the exchange
    pub sent: bool,
    /// the exchange's answer came back (`on_bought` ran)
    pub answered: bool,
    /// NEAR the exchange used
    pub used: U128,
    /// NEAR this chain sent back to the factory and saw arrive
    pub returned: U128,
    /// the chain is over: the rest is `get_buy_held`, for `refund_buy`
    pub settled: bool,
}

/// A buy's leftovers waiting here for the factory, per launch token.
#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BuyHeld {
    /// wNEAR the exchange refunded whose unwrap failed
    pub wnear: U128,
    /// NEAR whose transfer to the factory has not landed (or a hop that never ran)
    pub near: U128,
}

/// What this account knows about the position of one pool.
#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Position {
    /// the exchange's position id, once read back or returned by add_liquidity
    pub lpt_id: Option<String>,
    /// `list_liquidities` index this pool's position is searched from. Every row before it is either
    /// older than the HotZap or was already read and is not this pool's, so the search never
    /// restarts from zero and never needs more than a page or two.
    pub scan_from: u64,
    /// The last HotZap came back refunded in full: the exchange refused it and no position exists from
    /// it, so a retry goes straight to the balance check instead of searching the exchange.
    pub refunded: bool,
}

/// The fields of a `list_liquidities` row this contract reads. DCL returns more (owner_id, amount,
/// mft_id, v_liquidity, unclaimed_fee_x/y); unknown fields are ignored.
#[near(serializers = [json])]
#[derive(Clone, Debug)]
pub struct LiquidityRow {
    pub lpt_id: String,
    pub pool_id: String,
    pub left_point: i32,
    pub right_point: i32,
}

/// One launch's fees that the exchange released but the factory has not received yet, by where they
/// sit. Nothing here is ever paid out of anything but the amount it names.
#[near(serializers = [borsh, json])]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Carry {
    /// the pair's quote asset (wrap.near for a NEAR pair), known from the first claim
    pub quote: Option<AccountId>,
    /// quote-side fees still in this account's DCL balance (released, not withdrawn yet)
    pub quote_dcl: U128,
    /// token-side fees still in this account's DCL balance
    pub token_dcl: U128,
    /// quote side that reached this account and was not forwarded yet: wNEAR for a NEAR pair (not
    /// unwrapped yet), the FT otherwise
    pub quote_held: U128,
    /// token side that reached this account and was not forwarded yet
    pub token_held: U128,
    /// native NEAR unwrapped for this launch whose transfer to the factory did not land
    pub near_held: U128,
}

impl Carry {
    fn quote_total(&self) -> u128 {
        self.quote_dcl.0 + self.quote_held.0 + self.near_held.0
    }
    fn token_total(&self) -> u128 {
        self.token_dcl.0 + self.token_held.0
    }
    fn is_empty(&self) -> bool {
        self.quote_total() == 0 && self.token_total() == 0
    }
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Locker {
    factory: AccountId,
    dcl: AccountId,
    wnear: AccountId,
    /// Positions this account has confirmed it owns (read back or returned by add_liquidity). The
    /// exchange lists an account's positions in insertion order and this account never removes one,
    /// so every position opened from now on sits at an index >= this: it is where a new pool's
    /// search starts. Only confirmed positions count, so it can lag reality but never lead it.
    opened: u64,
    /// per pool id
    positions: LookupMap<String, Position>,
    /// Per launch token, never global: a launch only ever books its own fees. lock2's first carry
    ///    was one number, and on 2026-09-22 NEARLY's 9.42 NEAR was booked to NEARKAT.
    carries: LookupMap<AccountId, Carry>,
    /// sum of the NEAR-pair quote carries, so `get_owed` stays a single read
    owed_near_total: u128,
    /// Per exchange asset: how much of this account's DCL balance belongs to launch carries (released
    /// fees not withdrawn yet, including a withdrawal in flight). The factory's `withdraw_dcl_asset`
    /// can never take it.
    dcl_reserved: LookupMap<AccountId, u128>,
    /// 10-01 audit: per launch token, its supply from the moment a HotZap sends it until `record` books
    /// the position. Until then the whole supply may sit in this account's DCL balance (it reached the
    /// exchange and no position came of it, the case `on_asset` places with add_liquidity) and would
    /// read as loose; `withdraw_dcl_asset` never takes it. Only `release_supply` lets it go sooner.
    supply_reserved: LookupMap<AccountId, u128>,
    /// Per asset: what `withdraw_dcl_asset` brought here whose transfer on to the factory bounced.
    /// The next `withdraw_dcl_asset` for that asset (amount 0 is enough) sends it again.
    admin_held: LookupMap<AccountId, u128>,
    /// leftovers of the first buys, per launch token (see `BuyHeld`)
    buy_held: LookupMap<AccountId, BuyHeld>,
    /// Per FT pair asset: the buy tax (bps) its own token contract takes when the exchange pays this
    /// account, 0 when none. A pair asset can itself be a tax token whose tax-free list was fixed at its
    /// launch, before this account existed: what arrives from a withdrawal is then the amount less that
    /// tax, and booking the amount would make every forward of it bounce. Read once from the token's
    /// `get_tax` (its config never changes), on the first withdrawal of that asset. Keyed by exchange
    /// and asset (`tax_key`): after a `set_dcl` the new exchange is not on the token's pair list, so its
    /// answer there has to be read again.
    quote_tax: LookupMap<String, u16>,
    /// each launch's first buy, step by step (`get_first_buy`)
    first_buy: LookupMap<AccountId, FirstBuy>,
    /// 10-01 audit: claims whose `on_claimed` has not run yet. Their `remove_liquidity` may already have
    /// released fees into this account's DCL balance that `dcl_reserved` does not count yet, so the
    /// factory's `withdraw_dcl_asset` waits until this is 0.
    claims_open: u32,
    /// when the last claim was opened: a count left behind by a callback that died stops holding the owner
    /// withdrawal 10 minutes after it
    claims_open_at_ms: u64,
}

#[near]
impl Locker {
    /// Registers this account on the exchange and on wrap.near out of its own balance, so fund it
    /// with at least DCL_REGISTER + FT_STORAGE_REG before `new`. An existing registration is refunded.
    #[init]
    pub fn new(factory: AccountId, dcl: AccountId, wnear: AccountId) -> Self {
        let me = env::current_account_id();



        Promise::new(dcl.clone())
            .function_call(
                "storage_deposit".to_string(),
                format!(r#"{{"account_id":"{}","registration_only":true}}"#, me).into_bytes(),
                DCL_REGISTER,
                Gas::from_tgas(10),
            )
            .detach();

        Promise::new(wnear.clone())
            .function_call(
                "storage_deposit".to_string(),
                format!(r#"{{"account_id":"{}","registration_only":true}}"#, me).into_bytes(),
                FT_STORAGE_REG,
                Gas::from_tgas(10),
            )
            .detach();
        Self {
            factory, dcl, wnear,
            opened: 0,
            positions: LookupMap::new(b"p".to_vec()),
            carries: LookupMap::new(b"c".to_vec()),
            owed_near_total: 0,
            dcl_reserved: LookupMap::new(b"r".to_vec()),
            supply_reserved: LookupMap::new(b"s".to_vec()),
            admin_held: LookupMap::new(b"a".to_vec()),
            buy_held: LookupMap::new(b"b".to_vec()),
            quote_tax: LookupMap::new(b"t".to_vec()),
            first_buy: LookupMap::new(b"u".to_vec()),
            claims_open: 0,
            claims_open_at_ms: 0,
        }
    }



    /// FT-quote fees released by the exchange that the factory has not received yet, for this launch.
    pub fn get_owed_quote(&self, token: AccountId) -> U128 {
        let c = self.carry(&token);
        U128(if self.is_native(&c) { 0 } else { c.quote_total() })
    }

    /// Token-side fees released by the exchange that the factory has not received yet, for this launch.
    pub fn get_owed_token(&self, token: AccountId) -> U128 {
        U128(self.carry(&token).token_total())
    }

    /// NEAR-pair quote fees released by the exchange that the factory has not received yet, all launches.
    pub fn get_owed(&self) -> U128 { U128(self.owed_near_total) }

    /// The NEAR carry for one launch token.
    pub fn get_owed_near(&self, token: AccountId) -> U128 {
        let c = self.carry(&token);
        U128(if self.is_native(&c) { c.quote_total() } else { 0 })
    }



    /// The buy tax (bps) an FT pair asset takes when the current exchange pays this account; None until
    /// the first claim of that asset there read it.
    pub fn get_quote_tax(&self, quote: AccountId) -> Option<u16> { self.quote_tax.get(&self.tax_key(&quote)).copied() }

    /// Claims whose `remove_liquidity` result has not been booked yet (`withdraw_dcl_asset` waits for 0).
    pub fn get_claims_open(&self) -> u32 { self.claims_open }

    /// NEAR of the launch's first buy the exchange used (0: no first buy, refused, or no answer yet).
    pub fn get_bought(&self, token: AccountId) -> U128 { self.first_buy.get(&token).map(|b| b.used).unwrap_or(U128(0)) }

    /// The launch's first buy as this account saw it (None: `add_buy` never took NEAR for it).
    pub fn get_first_buy(&self, token: AccountId) -> Option<FirstBuy> { self.first_buy.get(&token).cloned() }

    /// Where one launch's undelivered fees sit.
    pub fn get_carry(&self, token: AccountId) -> Carry {
        self.carry(&token)
    }

    /// The part of this account's DCL balance of `asset` that belongs to launch carries.
    pub fn get_dcl_reserved(&self, asset: AccountId) -> U128 {
        U128(self.dcl_reserved.get(&asset).copied().unwrap_or(0))
    }

    /// The launch's supply while it is sent but not yet booked as a position (0: never sent, booked,
    /// or released). Held out of `withdraw_dcl_asset`'s reach whatever balance the exchange shows.
    pub fn get_supply_reserved(&self, token: AccountId) -> U128 {
        U128(self.supply_reserved.get(&token).copied().unwrap_or(0))
    }

    /// What this account knows about a pool's position (None: `add` was never called for it).
    pub fn get_position(&self, pool_id: String) -> Option<Position> {
        self.positions.get(&pool_id).cloned()
    }

    /// Positions this account has confirmed it owns on the exchange.
    pub fn get_opened(&self) -> u64 { self.opened }

    pub fn get_dcl(&self) -> AccountId { self.dcl.clone() }

    /// A buy's leftovers waiting here for the factory (`refund_buy` sends them).
    pub fn get_buy_held(&self, token: AccountId) -> BuyHeld {
        self.buy_held.get(&token).cloned().unwrap_or_default()
    }

    pub fn get_factory(&self) -> AccountId {
        self.factory.clone()
    }



    /// Open the launch's range position and return the exchange's `lpt_id` through the promise chain,
    /// exactly as lock2 did. An empty string means "not yet": the factory parks the launch and a
    /// `resume` calls this again, which carries on from where the last attempt stopped.
    ///
    /// Retry-safe: the supply is sent at most once per attempt and never when a position for this
    ///    pool is recorded or can be found on the exchange; tokens already sitting in this account's
    ///    DCL balance are placed with add_liquidity instead of being sent again.
    pub fn add(&mut self, args: AddArgs) -> PromiseOrValue<String> {
        self.assert_factory();
        self.assert_funded();
        let (_, _, supply) = sides(&args);
        require!(supply > 0, "nothing to add");
        let p = self.positions.get(&args.pool_id).cloned();
        match add_step(p.as_ref()) {
            AddStep::Known(lpt) => PromiseOrValue::Value(lpt),
            AddStep::Zap => {

                self.positions.insert(args.pool_id.clone(), Position { lpt_id: None, scan_from: self.opened, refunded: false });
                PromiseOrValue::Promise(self.zap(args))
            }
            AddStep::Lookup(from) => self.lookup(args, from, false),
            AddStep::CheckAsset => self.check_asset(args, false),
        }
    }

    /// The HotZap came back. `used` is what the exchange kept: all of it opened the position (the
    /// remainder of a rounding goes to this account's DCL balance), none of it means it was refused.
    #[private]
    pub fn on_zapped(&mut self, args: AddArgs, #[callback_result] used: Result<U128, PromiseError>) -> PromiseOrValue<String> {
        let mut p = self.positions.get(&args.pool_id).cloned().unwrap_or_default();



        p.refunded = matches!(used, Ok(U128(0)));
        let from = p.scan_from;
        self.positions.insert(args.pool_id.clone(), p.clone());
        if p.refunded {
            return pending(&args, "the exchange refused the HotZap");
        }
        self.lookup(args, from, true)
    }

    /// One page of this account's positions, read from `from`.
    #[private]
    pub fn on_listed(
        &mut self,
        args: AddArgs,
        from: u64,
        zapped_here: bool,
        #[callback_result] rows: Result<Vec<LiquidityRow>, PromiseError>,
    ) -> PromiseOrValue<String> {
        let Ok(rows) = rows else { return pending(&args, "list_liquidities failed") };
        let mut p = self.positions.get(&args.pool_id).cloned().unwrap_or_default();
        match scan_page(&rows, from, LIST_PAGE, &args) {
            Scan::Found(lpt) => PromiseOrValue::Value(self.record(&args, p, lpt, "hotzap")),
            Scan::More(next) => {
                p.scan_from = next;
                self.positions.insert(args.pool_id.clone(), p);
                self.lookup(args, next, zapped_here)
            }
            Scan::End(next) => {

                p.scan_from = next;
                self.positions.insert(args.pool_id.clone(), p);
                self.check_asset(args, zapped_here)
            }
        }
    }

    /// This account's DCL balance of the launch token. The supply sitting there (it reached the
    /// exchange but no position came of it) is placed with add_liquidity; otherwise the supply is
    /// still here and the HotZap is sent again, once per `add`.
    #[private]
    pub fn on_asset(&mut self, args: AddArgs, zapped_here: bool, #[callback_result] bal: Result<U128, PromiseError>) -> PromiseOrValue<String> {
        let Ok(bal) = bal else { return pending(&args, "get_user_asset failed") };
        let (_, _, supply) = sides(&args);


        let free = bal.0.saturating_sub(self.dcl_reserved.get(&args.token).copied().unwrap_or(0));
        match asset_step(free, supply, zapped_here) {
            AssetStep::AddOnly => {
                if !can_afford(GAS_ADD_LIQ.saturating_add(GAS_CB_ADDED)) {
                    return pending(&args, "out of gas before add_liquidity");
                }


                if let Some(mut p) = self.positions.get(&args.pool_id).cloned() {
                    if p.refunded { p.refunded = false; self.positions.insert(args.pool_id.clone(), p); }
                }
                PromiseOrValue::Promise(
                    Promise::new(self.dcl.clone())
                        .function_call("add_liquidity".to_string(), add_liquidity_args(&args).into_bytes(), NO_DEPOSIT, GAS_ADD_LIQ)
                        .then(Self::ext(env::current_account_id()).with_static_gas(GAS_CB_ADDED).with_unused_gas_weight(1).on_added(args)),
                )
            }
            AssetStep::Zap => {
                if !can_afford(GAS_FT_REG.saturating_add(GAS_ZAP).saturating_add(GAS_CB_ZAPPED)) {
                    return pending(&args, "out of gas before the HotZap");
                }
                let mut p = self.positions.get(&args.pool_id).cloned().unwrap_or_default();

                p.scan_from = p.scan_from.max(self.opened);
                p.refunded = false;
                self.positions.insert(args.pool_id.clone(), p);
                PromiseOrValue::Promise(self.zap(args))
            }
            AssetStep::Wait => pending(&args, "the HotZap already went out in this call"),
        }
    }

    /// add_liquidity returns the new position's id directly.
    #[private]
    pub fn on_added(&mut self, args: AddArgs, #[callback_result] lpt: Result<String, PromiseError>) -> String {
        match lpt {
            Ok(id) if !id.is_empty() => {
                let p = self.positions.get(&args.pool_id).cloned().unwrap_or_default();
                self.record(&args, p, id, "add_liquidity")
            }
            _ => {
                emit_pending(&args, "add_liquidity failed");
                String::new()
            }
        }
    }



    /// Open the position AND make the launch's first buy, one block apart, from this account.
    ///
    /// Why: the pool is public from the block the HotZap opens it, and a buy that arrives from
    /// anywhere else lands blocks later: on 2026-09-30 (PLAID, #1913) bots bought 20 N before the
    /// creator's 5 N, which its floor then refused. DCL's HotZap cannot carry the buy itself: its
    /// swap runs BEFORE its add and only spends the transferred asset (E101 otherwise; measured on
    /// the real dclv2 wasm), and the exchange has no swap call outside ft_on_transfer, so no single
    /// receipt can do both. What can be done is fixing the block: the HotZap reaches the exchange
    /// two blocks after this call and the swap three, through `buy_hop`, a hop on this account. A
    /// bot that sees the position open needs a block to see it and one to reach the exchange, so
    /// the earliest it can land is a block after ours.
    ///
    /// Only for a pool this account knows nothing about yet: a retry goes through `add`, and the
    /// factory makes the buy its own way then. The NEAR for the buy is attached; `buy.msg` is sent
    /// to the exchange as is. What the exchange refuses is unwrapped and returned to the factory
    /// before this returns; what could not be returned waits in `get_buy_held` for `refund_buy`.
    #[payable]
    pub fn add_buy(&mut self, args: AddArgs, buy: BuyArgs) -> PromiseOrValue<AddBuyResult> {
        self.assert_factory();
        self.assert_funded();
        let (_, _, supply) = sides(&args);
        require!(supply > 0, "nothing to add");
        require!(buy.amount.0 > 0 && env::attached_deposit().as_yoctonear() == buy.amount.0, "attach exactly the buy");
        require!(self.positions.get(&args.pool_id).is_none(), "this pool was placed or attempted: use add");
        require!(
            env::prepaid_gas().as_gas() >= Gas::from_tgas(T_ADD_BUY_NEEDS).as_gas(),
            format!("attach at least {} TGas", T_ADD_BUY_NEEDS)
        );

        self.hold(&args.token, |h| h.near = U128(h.near.0 + buy.amount.0));
        self.first_buy.insert(args.token.clone(), FirstBuy { amount: buy.amount, ..Default::default() });
        self.positions.insert(args.pool_id.clone(), Position { lpt_id: None, scan_from: self.opened, refunded: false });
        let me = env::current_account_id();
        let zap = self.zap(args.clone());

        let hop = Self::ext(me.clone()).with_static_gas(GAS_BUY_HOP).with_unused_gas_weight(3).buy_hop(args.token.clone(), buy);
        PromiseOrValue::Promise(zap.and(hop).then(Self::ext(me).with_static_gas(GAS_CB_ADD_BOUGHT).with_unused_gas_weight(0).on_add_bought(args)))
    }

    /// The hop: runs one block after `add_buy`, so its wrap.near batch (wrap the NEAR, send it to the
    /// exchange with the factory's swap message) reaches the exchange one block after the HotZap.
    #[private]
    pub fn buy_hop(&mut self, token: AccountId, buy: BuyArgs) -> Promise {


        self.hold(&token, |h| h.near = U128(h.near.0.saturating_sub(buy.amount.0)));
        self.note_buy(&token, |b| b.sent = true);
        Promise::new(self.wnear.clone())
            .function_call("near_deposit".to_string(), b"{}".to_vec(), NearToken::from_yoctonear(buy.amount.0), GAS_WRAP_DEPOSIT)
            .function_call_weight(
                "ft_transfer_call".to_string(),
                format!(r#"{{"receiver_id":"{}","amount":"{}","msg":{}}}"#, self.dcl, buy.amount.0, js(&buy.msg)).into_bytes(),
                ONE_YOCTO,
                GAS_BUY,
                GasWeight(1),
            )
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_CB_BOUGHT).with_unused_gas_weight(0).on_bought(token, buy.amount))
    }

    /// The swap came back. `used` is the wNEAR the exchange kept; the rest was refunded to this
    /// account's wNEAR balance and goes back to the factory as NEAR. An error means the batch
    /// failed as a whole, so the NEAR never left here: it is sent back, and held only if that bounces.
    #[private]
    pub fn on_bought(&mut self, token: AccountId, amount: U128, #[callback_result] used: Result<U128, PromiseError>) -> PromiseOrValue<BuyOutcome> {
        match used {
            Ok(u) => {
                let used = u.0.min(amount.0);
                let unused = amount.0 - used;
                self.note_buy(&token, |b| { b.answered = true; b.used = U128(used); b.settled = unused == 0; });

                emit("first_buy", &format!(r#"{{"token":"{}","amount":"{}","used":"{}"}}"#, token, amount.0, used));
                if unused == 0 {
                    return PromiseOrValue::Value(BuyOutcome { used: U128(used), refunded: U128(0) });
                }
                self.hold(&token, |h| h.wnear = U128(h.wnear.0 + unused));
                self.unwrap_held(token, used, unused)
            }
            Err(_) => {


                emit("first_buy", &format!(r#"{{"token":"{}","amount":"{}","used":"0","failed":true}}"#, token, amount.0));
                self.note_buy(&token, |b| b.answered = true);
                self.return_held(token, 0, amount.0)
            }
        }
    }

    /// The unwrap came back: its result is the NEAR transfer to this account, so on success the NEAR
    /// is here and goes on to the factory; on failure the wNEAR stays held.
    #[private]
    pub fn on_unwrapped(&mut self, token: AccountId, used: U128, unused: U128, #[callback_result] r: Result<(), PromiseError>) -> PromiseOrValue<BuyOutcome> {
        if r.is_err() {
            self.note_buy(&token, |b| b.settled = true);
            return PromiseOrValue::Value(BuyOutcome { used, refunded: U128(0) });
        }

        self.hold(&token, |h| h.wnear = U128(h.wnear.0.saturating_sub(unused.0)));
        self.return_held(token, used.0, unused.0)
    }

    /// The NEAR transfer to the factory came back.
    #[private]
    pub fn on_refunded(&mut self, token: AccountId, used: U128, unused: U128, #[callback_result] r: Result<(), PromiseError>) -> BuyOutcome {
        if r.is_err() {

            self.hold(&token, |h| h.near = U128(h.near.0 + unused.0));
            self.note_buy(&token, |b| b.settled = true);
            return BuyOutcome { used, refunded: U128(0) };
        }
        self.note_buy(&token, |b| { b.returned = U128(b.returned.0 + unused.0); b.settled = true; });
        BuyOutcome { used, refunded: unused }
    }

    /// Both legs are done: the position id from the HotZap chain, the buy from the hop.
    #[private]
    pub fn on_add_bought(
        &mut self,
        args: AddArgs,
        #[callback_result] lpt: Result<String, PromiseError>,
        #[callback_result] buy: Result<BuyOutcome, PromiseError>,
    ) -> AddBuyResult {

        self.note_buy(&args.token, |b| b.settled = true);
        let b = buy.unwrap_or_default();
        AddBuyResult { lpt_id: lpt.unwrap_or_default(), used: b.used, refunded: b.refunded }
    }

    /// Factory-only: send a buy's leftovers (see `get_buy_held`) to the factory. Held wNEAR is
    /// unwrapped first; held NEAR is sent when no wNEAR is waiting. What is sent is debited BEFORE it
    /// leaves and re-credited only by the callback that saw it fail, so two calls in flight cannot
    /// send the same leftover twice (as `on_claimed` does for carries).
    pub fn refund_buy(&mut self, token: AccountId) -> PromiseOrValue<BuyOutcome> {
        self.assert_factory();
        let h = self.get_buy_held(token.clone());
        let me = env::current_account_id();
        if h.wnear.0 > 0 {
            let w = h.wnear.0;
            self.hold(&token, |h| h.wnear = U128(0));
            return PromiseOrValue::Promise(
                Promise::new(self.wnear.clone())
                    .function_call("near_withdraw".to_string(), format!(r#"{{"amount":"{}"}}"#, w).into_bytes(), ONE_YOCTO, GAS_UNWRAP)
                    .then(Self::ext(me).with_static_gas(GAS_CB_UNWRAPPED).with_unused_gas_weight(0).on_refund_unwrapped(token, U128(w))),
            );
        }
        if h.near.0 > 0 {
            let n = h.near.0;
            self.hold(&token, |h| h.near = U128(0));
            return self.send_refund(token, n);
        }
        PromiseOrValue::Value(BuyOutcome::default())
    }

    /// `refund_buy`'s unwrap came back: on failure the wNEAR is held again; on success the NEAR is here
    /// and goes on to the factory.
    #[private]
    pub fn on_refund_unwrapped(&mut self, token: AccountId, amount: U128, #[callback_result] r: Result<(), PromiseError>) -> PromiseOrValue<BuyOutcome> {
        if r.is_err() {
            self.hold(&token, |h| h.wnear = U128(h.wnear.0 + amount.0));
            return PromiseOrValue::Value(BuyOutcome::default());
        }
        self.send_refund(token, amount.0)
    }

    /// `refund_buy`'s transfer came back: a bounce is held again as NEAR.
    #[private]
    pub fn on_refund_returned(&mut self, token: AccountId, amount: U128, #[callback_result] r: Result<(), PromiseError>) -> BuyOutcome {
        if r.is_err() {
            self.hold(&token, |h| h.near = U128(h.near.0 + amount.0));
            return BuyOutcome::default();
        }
        BuyOutcome { used: U128(0), refunded: amount }
    }



    /// Pull accrued fees off a position (remove_liquidity with amount 0) and forward them to the factory.
    /// Returns `[x, y]`: what the factory actually RECEIVED in this call for this launch, never merely
    /// what the exchange released. `quote` is the pair's other asset when it is an FT; `None` means the
    /// wNEAR pair, whose quote side reaches the factory as native NEAR.
    ///
    /// The fees stay in this account's DCL balance (`skip_refund_transfer`), are withdrawn per launch
    /// with `withdraw_asset`, whose result says whether they arrived, and only then are forwarded.
    pub fn claim(&mut self, lpt_id: String, token: AccountId, token_is_x: bool, quote: Option<AccountId>) -> Promise {
        self.assert_factory();
        self.assert_funded();

        self.claims_open += 1;
        self.claims_open_at_ms = env::block_timestamp_ms();
        Promise::new(self.dcl.clone())
            .function_call_weight(
                "remove_liquidity".to_string(),
                format!(r#"{{"lpt_id":{},"amount":"0","min_amount_x":"0","min_amount_y":"0","skip_refund_transfer":true}}"#, js(&lpt_id)).into_bytes(),
                NO_DEPOSIT,
                GAS_REMOVE,
                GasWeight(1),
            )
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_CB_CLAIMED).with_unused_gas_weight(0).on_claimed(token, token_is_x, quote))
    }

    /// The exchange released `[fee_x, fee_y]` into this account's DCL balance. Book it to this launch,
    /// then withdraw everything this launch has there.
    ///
    /// 10-01 audit: the result is read by hand, after `claims_open` counts down. A failed or unreadable
    ///    `remove_liquidity` returns `[0, 0]` (the factory books nothing, as for an error) instead of
    ///    panicking, because a panic would roll the count back and hold `withdraw_dcl_asset` for good.
    #[private]
    pub fn on_claimed(&mut self, token: AccountId, token_is_x: bool, quote: Option<AccountId>) -> PromiseOrValue<Vec<U128>> {
        self.claims_open = self.claims_open.saturating_sub(1);
        let Some(v) = claim_result() else {
            emit("claim_failed", &format!(r#"{{"token":"{}"}}"#, token));
            return PromiseOrValue::Value(vec![U128(0), U128(0)]);
        };
        let (fee_tok, fee_quote) = if token_is_x { (v[0].0, v[1].0) } else { (v[1].0, v[0].0) };
        let q = self.quote_asset(&quote);
        let (was, mut c) = self.carry_for(&token, &quote);
        c.quote_dcl = U128(c.quote_dcl.0 + fee_quote);
        c.token_dcl = U128(c.token_dcl.0 + fee_tok);
        self.reserve_add(&q, fee_quote);
        self.reserve_add(&token, fee_tok);



        let (mut wq, wt) = (c.quote_dcl.0, c.token_dcl.0);



        let probe = wq > 0 && quote.is_some() && !self.quote_tax.contains_key(&self.tax_key(&q));
        if probe { wq = 0; }
        c.quote_dcl = U128(c.quote_dcl.0 - wq);
        c.token_dcl = U128(0);
        self.put_carry(&token, &was, c);
        if wq == 0 && wt == 0 && !probe {
            return self.forward(token, token_is_x, quote);
        }
        let mut legs: Option<Promise> = None;
        for (asset, amount) in [(&q, wq), (&token, wt)] {
            if amount == 0 { continue; }
            let w = Promise::new(self.dcl.clone()).function_call(
                "withdraw_asset".to_string(),
                format!(r#"{{"token_id":"{}","amount":"{}","skip_unwrap_near":true}}"#, asset, amount).into_bytes(),
                NO_DEPOSIT,
                GAS_WITHDRAW,
            );
            legs = Some(match legs { Some(l) => l.and(w), None => w });
        }
        if probe {

            let v = Promise::new(q.clone()).function_call("get_tax".to_string(), b"{}".to_vec(), NO_DEPOSIT, GAS_TAX_VIEW);
            legs = Some(match legs { Some(l) => l.and(v), None => v });
        }
        PromiseOrValue::Promise(legs.expect("a leg").then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_CB_WITHDRAWN)
                .with_unused_gas_weight(0)
                .on_withdrawn(token, token_is_x, quote, U128(wq), U128(wt), Some(probe)),
        ))
    }

    /// The withdrawals came back, quote leg first (when it was non-zero), then the token leg. What
    /// arrived moves to `held`; what did not goes back to `dcl` for the next claim. Then forward.
    #[private]
    pub fn on_withdrawn(&mut self, token: AccountId, token_is_x: bool, quote: Option<AccountId>, wq: U128, wt: U128, probed: Option<bool>) -> PromiseOrValue<Vec<U128>> {
        let mut i = 0u64;
        let mut arrived = |amount: u128| -> bool {
            if amount == 0 { return true; }
            let ok = withdraw_arrived(env::promise_result_checked(i, 16));
            i += 1;
            ok
        };
        let (q_ok, t_ok) = (arrived(wq.0), arrived(wt.0));
        let q = self.quote_asset(&quote);
        if probed == Some(true) {


            let bps = env::promise_result_checked(i, 4096).ok()
                .and_then(|b| near_sdk::serde_json::from_slice::<near_sdk::serde_json::Value>(&b).ok())
                .map_or(0, |v| buy_tax_bps(&v, env::current_account_id().as_str(), self.dcl.as_str(), q.as_str()));
            self.quote_tax.insert(self.tax_key(&q), bps);
            emit("quote_tax_read", &format!(r#"{{"quote":"{}","buy_bps":{}}}"#, q, bps));
        }
        let (was, mut c) = self.carry_for(&token, &quote);
        let (q_left, q_dcl) = settle_withdrawal(q_ok, wq.0);
        let (t_held, t_dcl) = settle_withdrawal(t_ok, wt.0);

        let q_tax = if quote.is_some() { tax_of(q_left, self.quote_tax.get(&self.tax_key(&q)).copied().unwrap_or(0)) } else { 0 };
        let q_held = q_left - q_tax;
        if q_tax > 0 {
            emit("quote_tax_withheld", &format!(r#"{{"token":"{}","quote":"{}","withdrawn":"{}","tax":"{}"}}"#, token, q, q_left, q_tax));
        }
        c.quote_held = U128(c.quote_held.0 + q_held);
        c.quote_dcl = U128(c.quote_dcl.0 + q_dcl);
        c.token_held = U128(c.token_held.0 + t_held);
        c.token_dcl = U128(c.token_dcl.0 + t_dcl);

        self.reserve_sub(&q, q_left);
        self.reserve_sub(&token, t_held);
        self.put_carry(&token, &was, c);
        self.forward(token, token_is_x, quote)
    }

    /// The forward legs came back, quote leg first (when it was non-zero), then the token leg.
    #[private]
    pub fn on_forwarded(&mut self, token: AccountId, token_is_x: bool, quote: Option<AccountId>, fq: U128, ft: U128, fnear: U128) -> PromiseOrValue<Vec<U128>> {
        let mut i = 0u64;
        let mut landed = |amount: u128| -> bool {
            if amount == 0 { return true; }


            let ok = !matches!(env::promise_result_checked(i, 64), Err(PromiseError::Failed));
            i += 1;
            ok
        };
        let (q_ok, t_ok) = (landed(fq.0), landed(ft.0));
        let (was, mut c) = self.carry_for(&token, &quote);
        let (t_booked, t_back) = book(t_ok, ft.0);
        c.token_held = U128(c.token_held.0 + t_back);
        if quote.is_some() {

            let (q_booked, q_back) = book(q_ok, fq.0);
            c.quote_held = U128(c.quote_held.0 + q_back);
            self.put_carry(&token, &was, c);
            self.log_deferred(&token);
            return PromiseOrValue::Value(order(token_is_x, t_booked, q_booked));
        }


        let (unwrapped, q_back) = book(q_ok, fq.0);
        c.quote_held = U128(c.quote_held.0 + q_back);
        let send = unwrapped + fnear.0;
        self.put_carry(&token, &was, c);
        if send == 0 {
            self.log_deferred(&token);
            return PromiseOrValue::Value(order(token_is_x, t_booked, 0));
        }


        PromiseOrValue::Promise(Promise::new(self.factory.clone()).transfer(NearToken::from_yoctonear(send)).then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_CB_DELIVERED)
                .with_unused_gas_weight(0)
                .on_delivered(token, token_is_x, U128(t_booked), U128(send)),
        ))
    }

    /// The NEAR transfer to the factory came back.
    #[private]
    pub fn on_delivered(&mut self, token: AccountId, token_is_x: bool, token_booked: U128, near: U128, #[callback_result] r: Result<(), PromiseError>) -> Vec<U128> {
        let (booked, back) = book(r.is_ok(), near.0);
        if back > 0 {

            let (was, mut c) = self.carry_for(&token, &None);
            c.near_held = U128(c.near_held.0 + back);
            self.put_carry(&token, &was, c);
        }
        self.log_deferred(&token);
        order(token_is_x, token_booked.0, booked)
    }








    /// Send `amount` of this account's DCL balance of `token` to the factory. Refused if it would dip
    /// into what launch carries own there (`get_dcl_reserved`) or into a supply sent and not booked yet
    /// (`get_supply_reserved`). `amount` 0 only re-sends what an earlier call brought here but could
    /// not deliver. Returns whether the factory received it.
    pub fn withdraw_dcl_asset(&mut self, token: AccountId, amount: U128) -> PromiseOrValue<bool> {
        self.assert_factory();
        require!(
            env::prepaid_gas().as_gas() >= Gas::from_tgas(T_ADMIN_WITHDRAW).as_gas(),
            format!("attach at least {} TGas", T_ADMIN_WITHDRAW)
        );
        if amount.0 == 0 {
            return self.admin_send(token, 0);
        }
        PromiseOrValue::Promise(
            Promise::new(self.dcl.clone())
                .function_call(
                    "get_user_asset".to_string(),
                    format!(r#"{{"account_id":"{}","token_id":"{}"}}"#, env::current_account_id(), token).into_bytes(),
                    NO_DEPOSIT,
                    GAS_ASSET,
                )
                .then(Self::ext(env::current_account_id()).with_static_gas(GAS_CB_ADMIN_ASSET).with_unused_gas_weight(1).on_admin_asset(token, amount)),
        )
    }

    #[private]
    pub fn on_admin_asset(&mut self, token: AccountId, amount: U128, #[callback_result] bal: Result<U128, PromiseError>) -> PromiseOrValue<bool> {
        let bal = bal.unwrap_or_else(|_| env::panic_str("get_user_asset failed"));


        require!(self.claims_open == 0 || env::block_timestamp_ms() > self.claims_open_at_ms + CLAIM_OPEN_EXPIRY_MS, "a claim is in flight: retry");

        let reserved = self.dcl_reserved.get(&token).copied().unwrap_or(0).saturating_add(self.supply_reserved.get(&token).copied().unwrap_or(0));
        require!(amount.0 <= admin_free(bal.0, reserved), "that is more than the DCL balance not owned by launch fees or an unplaced supply");
        PromiseOrValue::Promise(
            Promise::new(self.dcl.clone())
                .function_call(
                    "withdraw_asset".to_string(),
                    format!(r#"{{"token_id":"{}","amount":"{}","skip_unwrap_near":true}}"#, token, amount.0).into_bytes(),
                    NO_DEPOSIT,
                    GAS_WITHDRAW,
                )
                .then(Self::ext(env::current_account_id()).with_static_gas(GAS_CB_ADMIN_WITHDRAWN).with_unused_gas_weight(1).on_admin_withdrawn(token, amount)),
        )
    }

    #[private]
    pub fn on_admin_withdrawn(&mut self, token: AccountId, amount: U128, #[callback_result] r: Result<bool, PromiseError>) -> PromiseOrValue<bool> {
        if !matches!(r, Ok(true)) {
            emit("dcl_asset_withdrawn", &format!(r#"{{"token":"{}","amount":"{}","arrived":false}}"#, token, amount.0));
            return PromiseOrValue::Value(false);
        }

        let net = amount.0 - tax_of(amount.0, self.quote_tax.get(&self.tax_key(&token)).copied().unwrap_or(0));
        self.admin_send(token, net)
    }

    #[private]
    pub fn on_admin_sent(&mut self, token: AccountId, amount: U128, #[callback_result] r: Result<(), PromiseError>) -> bool {
        let (sent, back) = book(r.is_ok(), amount.0);
        if back > 0 {
            let cur = self.admin_held.get(&token).copied().unwrap_or(0);
            self.admin_held.insert(token.clone(), cur + back);
        }
        emit("dcl_asset_withdrawn", &format!(r#"{{"token":"{}","amount":"{}","arrived":true,"sent":"{}","held":"{}"}}"#, token, amount.0, sent, back));
        back == 0
    }

    /// Point this account at another exchange account, for a Rhea migration. It registers here on the
    /// new one out of this account's balance, like `new`. Positions, position ids and the DCL balances
    /// behind launch carries must already live on the new account, or claims there will fail.
    pub fn set_dcl(&mut self, dcl: AccountId) {
        self.assert_factory();
        emit("dcl_set", &format!(r#"{{"old":"{}","new":"{}"}}"#, self.dcl, dcl));
        Promise::new(dcl.clone())
            .function_call(
                "storage_deposit".to_string(),
                format!(r#"{{"account_id":"{}","registration_only":true}}"#, env::current_account_id()).into_bytes(),
                DCL_REGISTER,
                Gas::from_tgas(10),
            )
            .detach();
        self.dcl = dcl;
    }

    /// Factory-only: the launch failed (the factory's `cancel_stuck_launch`, which never cancels a launch
    /// whose position may exist), so its supply is no longer waiting to become a position. Lets go of
    /// `get_supply_reserved` for `token`, and nothing else: whatever of the supply sits in this account's
    /// DCL balance is loose from now on and `withdraw_dcl_asset` reaches it. The only way that supply
    /// is ever released short of `record` booking the position.
    pub fn release_supply(&mut self, token: AccountId) {
        self.assert_factory();
        let amount = self.supply_reserved.remove(&token).unwrap_or(0);
        emit("supply_released", &format!(r#"{{"token":"{}","amount":"{}"}}"#, token, amount));
    }

    /// What `withdraw_dcl_asset` brought here for `token` but could not deliver yet.
    pub fn get_admin_held(&self, token: AccountId) -> U128 {
        U128(self.admin_held.get(&token).copied().unwrap_or(0))
    }

    /// Register this account on an FT quote asset. Without it the exchange's payout of the quote
    /// side has nowhere to land and the fees stay stuck in DCL's inner account. Factory-only, and
    /// it only ever calls `storage_deposit` — it still cannot move a position.
    #[payable]
    pub fn register(&mut self, token: AccountId) -> Promise {
        self.assert_factory();
        let d = env::attached_deposit();
        require!(d >= FT_STORAGE_REG, "attach at least the NEP-145 minimum");
        Promise::new(token).function_call(
            "storage_deposit".to_string(),
            format!(r#"{{"account_id":"{}","registration_only":true}}"#, env::current_account_id()).into_bytes(),
            d,
            Gas::from_tgas(10),
        )
    }
}


impl Locker {
    /// Change what is held for a launch's buy; an empty record is removed.
    fn hold(&mut self, token: &AccountId, f: impl FnOnce(&mut BuyHeld)) {
        let mut h = self.get_buy_held(token.clone());
        f(&mut h);
        if h.wnear.0 == 0 && h.near.0 == 0 { self.buy_held.remove(token); } else { self.buy_held.insert(token.clone(), h); }
    }

    /// Unwrap `unused` wNEAR held for a buy, then send it on (on_unwrapped).
    fn unwrap_held(&self, token: AccountId, used: u128, unused: u128) -> PromiseOrValue<BuyOutcome> {
        PromiseOrValue::Promise(
            Promise::new(self.wnear.clone())
                .function_call("near_withdraw".to_string(), format!(r#"{{"amount":"{}"}}"#, unused).into_bytes(), ONE_YOCTO, GAS_UNWRAP)
                .then(Self::ext(env::current_account_id()).with_static_gas(GAS_CB_UNWRAPPED).with_unused_gas_weight(0).on_unwrapped(token, U128(used), U128(unused))),
        )
    }

    /// `refund_buy`'s transfer of already-debited NEAR to the factory (on_refund_returned re-holds a bounce).
    fn send_refund(&self, token: AccountId, amount: u128) -> PromiseOrValue<BuyOutcome> {
        PromiseOrValue::Promise(
            Promise::new(self.factory.clone())
                .transfer(NearToken::from_yoctonear(amount))
                .then(Self::ext(env::current_account_id()).with_static_gas(GAS_CB_REFUNDED).with_unused_gas_weight(0).on_refund_returned(token, U128(amount))),
        )
    }

    /// Send `unused` NEAR held for a buy to the factory (on_refunded books it).
    fn return_held(&self, token: AccountId, used: u128, unused: u128) -> PromiseOrValue<BuyOutcome> {
        PromiseOrValue::Promise(
            Promise::new(self.factory.clone())
                .transfer(NearToken::from_yoctonear(unused))
                .then(Self::ext(env::current_account_id()).with_static_gas(GAS_CB_REFUNDED).with_unused_gas_weight(0).on_refunded(token, U128(used), U128(unused))),
        )
    }

    /// ft_transfer `amount` plus anything held from an earlier bounce of `token` to the factory.
    fn admin_send(&mut self, token: AccountId, amount: u128) -> PromiseOrValue<bool> {
        let total = amount + self.admin_held.remove(&token).unwrap_or(0);
        if total == 0 {
            return PromiseOrValue::Value(true);
        }
        PromiseOrValue::Promise(
            Promise::new(token.clone())
                .function_call(
                    "ft_transfer".to_string(),
                    format!(r#"{{"receiver_id":"{}","amount":"{}","memo":"dcl balance"}}"#, self.factory, total).into_bytes(),
                    ONE_YOCTO,
                    GAS_FORWARD,
                )
                .then(Self::ext(env::current_account_id()).with_static_gas(GAS_CB_ADMIN_SENT).with_unused_gas_weight(1).on_admin_sent(token, U128(total))),
        )
    }
    fn assert_factory(&self) {
        require!(env::predecessor_account_id() == self.factory, "factory only");
    }

    /// Update a launch's first-buy record (only one `add_buy` ever created).
    fn note_buy(&mut self, token: &AccountId, f: impl FnOnce(&mut FirstBuy)) {
        if let Some(mut b) = self.first_buy.get(token).cloned() {
            f(&mut b);
            self.first_buy.insert(token.clone(), b);
        }
    }

    /// 10-01 audit: refuse to start a launch's add or a claim on a nearly dry account (see MIN_FREE).
    /// The factory parks a launch whose add is refused, and a refused claim moves nothing.
    fn assert_funded(&self) {
        require!(free_balance() >= MIN_FREE, "locker NEAR low: top up");
    }

    /// `quote_tax` key: the exchange this account talks to, then the asset
    fn tax_key(&self, asset: &AccountId) -> String {
        format!("{}|{}", self.dcl, asset)
    }

    fn quote_asset(&self, quote: &Option<AccountId>) -> AccountId {
        quote.clone().unwrap_or_else(|| self.wnear.clone())
    }

    fn is_native(&self, c: &Carry) -> bool {
        c.quote.as_ref().map_or(true, |q| *q == self.wnear)
    }

    fn carry(&self, token: &AccountId) -> Carry {
        self.carries.get(token).cloned().unwrap_or_default()
    }

    /// A launch's carry inside the claim chain, with its quote asset pinned from the call's own args.
    fn carry_for(&self, token: &AccountId, quote: &Option<AccountId>) -> (Carry, Carry) {
        let was = self.carry(token);
        let mut c = was.clone();
        c.quote = Some(self.quote_asset(quote));
        (was, c)
    }

    /// Store a launch's carry and keep `owed_near_total` in step with it.
    fn put_carry(&mut self, token: &AccountId, was: &Carry, c: Carry) {
        if self.is_native(was) { self.owed_near_total = self.owed_near_total.saturating_sub(was.quote_total()); }
        if self.is_native(&c) { self.owed_near_total += c.quote_total(); }
        if c.is_empty() {
            self.carries.remove(token);
        } else {
            self.carries.insert(token.clone(), c);
        }
    }

    /// At the end of a claim: say what this launch still has on its way, if anything. lock2's event,
    /// with where each part sits.
    fn log_deferred(&self, token: &AccountId) {
        let c = self.carry(token);
        if c.is_empty() { return; }
        emit(
            "fee_forward_deferred",
            &format!(
                r#"{{"token":"{}","owed":"{}","owed_quote":"{}","quote_dcl":"{}","token_dcl":"{}","quote_held":"{}","token_held":"{}","near_held":"{}"}}"#,
                token, c.token_total(), c.quote_total(), c.quote_dcl.0, c.token_dcl.0, c.quote_held.0, c.token_held.0, c.near_held.0
            ),
        );
    }

    /// This account's claim on its own DCL balance of `asset` grows by what the exchange released...
    fn reserve_add(&mut self, asset: &AccountId, amount: u128) {
        if amount == 0 { return; }
        let cur = self.dcl_reserved.get(asset).copied().unwrap_or(0);
        self.dcl_reserved.insert(asset.clone(), cur + amount);
    }

    /// ...and shrinks by what actually left the exchange.
    fn reserve_sub(&mut self, asset: &AccountId, amount: u128) {
        if amount == 0 { return; }
        let next = self.dcl_reserved.get(asset).copied().unwrap_or(0).saturating_sub(amount);
        if next == 0 { self.dcl_reserved.remove(asset); } else { self.dcl_reserved.insert(asset.clone(), next); }
    }

    /// Send what has arrived here for this launch on to the factory: the quote leg (NEAR pair:
    /// unwrap the wNEAR first), the token leg, and any NEAR held from an earlier bounce. Each amount
    /// leaves the carry now and comes back in `on_forwarded` if its leg failed.
    fn forward(&mut self, token: AccountId, token_is_x: bool, quote: Option<AccountId>) -> PromiseOrValue<Vec<U128>> {
        let (was, mut c) = self.carry_for(&token, &quote);
        let (fq, ft, fnear) = (c.quote_held.0, c.token_held.0, if quote.is_none() { c.near_held.0 } else { 0 });
        if fq == 0 && ft == 0 && fnear == 0 {
            self.log_deferred(&token);
            return PromiseOrValue::Value(vec![U128(0), U128(0)]);
        }
        c.quote_held = U128(0);
        c.token_held = U128(0);
        if quote.is_none() { c.near_held = U128(0); }
        self.put_carry(&token, &was, c);
        let send = |asset: &AccountId, amount: u128| {
            Promise::new(asset.clone()).function_call(
                "ft_transfer".to_string(),
                format!(r#"{{"receiver_id":"{}","amount":"{}","memo":"lp fees"}}"#, self.factory, amount).into_bytes(),
                ONE_YOCTO,
                GAS_FORWARD,
            )
        };
        let mut legs: Option<Promise> = None;
        if fq > 0 {
            legs = Some(match &quote {
                Some(q) => send(q, fq),
                None => Promise::new(self.wnear.clone()).function_call(
                    "near_withdraw".to_string(),
                    format!(r#"{{"amount":"{}"}}"#, fq).into_bytes(),
                    ONE_YOCTO,
                    GAS_UNWRAP,
                ),
            });
        }
        if ft > 0 {
            let t = send(&token, ft);
            legs = Some(match legs { Some(l) => l.and(t), None => t });
        }
        match legs {
            Some(l) => PromiseOrValue::Promise(l.then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_CB_FORWARDED)
                    .with_unused_gas_weight(0)
                    .on_forwarded(token, token_is_x, quote, U128(fq), U128(ft), U128(fnear)),
            )),

            None => PromiseOrValue::Promise(Promise::new(self.factory.clone()).transfer(NearToken::from_yoctonear(fnear)).then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_CB_DELIVERED)
                    .with_unused_gas_weight(0)
                    .on_delivered(token, token_is_x, U128(0), U128(fnear)),
            )),
        }
    }

    /// storage_deposit for the exchange on the token, then the supply in ONE ft_transfer_call whose msg
    /// opens the position (HotZap). Every spare TGas goes to the callback, which reads the id back.
    /// The supply is reserved from here until `record` books the position (see `supply_reserved`).
    fn zap(&mut self, args: AddArgs) -> Promise {
        let (_, _, supply) = sides(&args);
        let other = other_token(&args.pool_id, &args.token);
        self.supply_reserved.insert(args.token.clone(), supply);
        Promise::new(args.token.clone())
            .function_call(
                "storage_deposit".to_string(),
                format!(r#"{{"account_id":"{}","registration_only":true}}"#, self.dcl).into_bytes(),
                FT_STORAGE_REG,
                GAS_FT_REG,
            )
            .function_call(
                "ft_transfer_call".to_string(),
                format!(r#"{{"receiver_id":"{}","amount":"{}","msg":{}}}"#, self.dcl, supply, js(&hotzap_msg(&args, &other))).into_bytes(),
                ONE_YOCTO,
                GAS_ZAP,
            )
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_CB_ZAPPED).with_unused_gas_weight(1).on_zapped(args))
    }

    /// Read one page of this account's positions from `from`, if the gas is there for it.
    fn lookup(&self, args: AddArgs, from: u64, zapped_here: bool) -> PromiseOrValue<String> {
        if !can_afford(GAS_LIST.saturating_add(GAS_CB_LISTED)) {
            return pending(&args, "out of gas before list_liquidities");
        }
        PromiseOrValue::Promise(
            Promise::new(self.dcl.clone())
                .function_call(
                    "list_liquidities".to_string(),
                    format!(r#"{{"account_id":"{}","from_index":{},"limit":{}}}"#, env::current_account_id(), from, LIST_PAGE).into_bytes(),
                    NO_DEPOSIT,
                    GAS_LIST,
                )
                .then(Self::ext(env::current_account_id()).with_static_gas(GAS_CB_LISTED).with_unused_gas_weight(1).on_listed(args, from, zapped_here)),
        )
    }

    /// Read this account's DCL balance of the launch token, if the gas is there for it.
    fn check_asset(&self, args: AddArgs, zapped_here: bool) -> PromiseOrValue<String> {
        if !can_afford(GAS_ASSET.saturating_add(GAS_CB_ASSET)) {
            return pending(&args, "out of gas before get_user_asset");
        }
        PromiseOrValue::Promise(
            Promise::new(self.dcl.clone())
                .function_call(
                    "get_user_asset".to_string(),
                    format!(r#"{{"account_id":"{}","token_id":"{}"}}"#, env::current_account_id(), args.token).into_bytes(),
                    NO_DEPOSIT,
                    GAS_ASSET,
                )
                .then(Self::ext(env::current_account_id()).with_static_gas(GAS_CB_ASSET).with_unused_gas_weight(1).on_asset(args, zapped_here)),
        )
    }

    /// Book a position for its pool, once. Returns its id for the factory.
    fn record(&mut self, args: &AddArgs, mut p: Position, lpt: String, via: &str) -> String {
        if p.lpt_id.is_none() {
            self.opened += 1;
        }
        p.lpt_id = Some(lpt.clone());
        p.refunded = false;
        self.positions.insert(args.pool_id.clone(), p);

        self.supply_reserved.remove(&args.token);
        emit(
            "position_opened",
            &format!(r#"{{"token":"{}","pool_id":{},"lpt_id":{},"via":"{}"}}"#, args.token, js(&args.pool_id), js(&lpt), via),
        );
        lpt
    }
}



/// `remove_liquidity`'s `[fee_x, fee_y]`, or None when it failed or answered anything else. Never panics.
fn claim_result() -> Option<Vec<U128>> {
    if env::promise_results_count() != 1 { return None; }
    env::promise_result_checked(0, 256).ok()
        .and_then(|b| near_sdk::serde_json::from_slice::<Vec<U128>>(&b).ok())
        .filter(|v| v.len() == 2)
}

/// This account's NEAR not locked for storage, less what the current call attached (already counted
/// in the balance and not ours: `add_buy`'s buy).
fn free_balance() -> u128 {
    let locked = env::storage_byte_cost().as_yoctonear() * env::storage_usage() as u128;
    env::account_balance().as_yoctonear().saturating_sub(locked).saturating_sub(env::attached_deposit().as_yoctonear())
}

/// What the factory may take out of this account's DCL balance of an asset: never the part launch
/// carries own.
fn admin_free(balance: u128, reserved: u128) -> u128 {
    balance.saturating_sub(reserved)
}

/// DCL's `withdraw_asset` resolves to its `callback_post_withdraw_asset`, which returns `true` only
/// when the transfer to this account went through. Anything else did not arrive.
fn withdraw_arrived(r: Result<Vec<u8>, PromiseError>) -> bool {
    matches!(r, Ok(v) if v == b"true")
}

/// `(held, back_to_dcl)` for a withdrawal of `amount`: what arrived is here now, what did not is
/// still in the exchange balance and is tried again by the next claim. Measured on the real dclv2
/// wasm: when the transfer to the owner fails, callback_post_withdraw_asset logs `lostfound` with
/// `locked: false`, credits the amount back to the owner's DCL balance and returns `false`.
fn settle_withdrawal(arrived: bool, amount: u128) -> (u128, u128) {
    if arrived { (amount, 0) } else { (0, amount) }
}

/// The buy tax a tax token takes on `amount` paid out of a pair to a non-exempt account: its own
/// `tax_for` (token-tax v1, v2 and the current code alike), `amount * bps / 10_000` rounded down,
/// bps capped at 400, and `amount / 10_000 * bps` when the product overflows.
fn tax_of(amount: u128, bps: u16) -> u128 {
    let bps = (bps as u128).min(MAX_TAX_BPS);
    if bps == 0 { return 0; }
    amount.checked_mul(bps).map(|x| x / 10_000).unwrap_or_else(|| amount / 10_000 * bps)
}
const MAX_TAX_BPS: u128 = 400;

/// From a tax token's `get_tax` answer (`{"tax":{"buy_bps","pairs","admin","exempt",…}|null,…}`, the
/// same shape in every version): the bps it takes when `dcl` pays `me`. None when it has no tax, the
/// exchange is not one of its pairs, or `me` is tax-free there (its admin, itself, or on its list).
fn buy_tax_bps(v: &near_sdk::serde_json::Value, me: &str, dcl: &str, token: &str) -> u16 {
    let t = &v["tax"];
    if !t.is_object() { return 0; }
    let listed = |k: &str, a: &str| t[k].as_array().map_or(false, |l| l.iter().any(|x| x.as_str() == Some(a)));
    if !listed("pairs", dcl) || t["admin"].as_str() == Some(me) || me == token || listed("exempt", me) { return 0; }
    let bps = t["buy_bps"].as_u64().or_else(|| t["buy_bps"].as_str().and_then(|x| x.parse().ok())).unwrap_or(0);
    bps.min(MAX_TAX_BPS as u64) as u16
}

/// `(booked, carried)`: the factory credits creators from what this contract RETURNS, so returning an
/// amount whose transfer bounced would credit someone funds that never left here. Booked is what landed.
fn book(delivered: bool, amount: u128) -> (u128, u128) {
    if delivered { (amount, 0) } else { (0, amount) }
}

/// `[x, y]` in the pool's order.
fn order(token_is_x: bool, tok: u128, quote: u128) -> Vec<U128> {
    if token_is_x { vec![U128(tok), U128(quote)] } else { vec![U128(quote), U128(tok)] }
}



/// `(amount_x, amount_y, supply)`: the factory places the whole supply on one side.
fn sides(args: &AddArgs) -> (u128, u128, u128) {
    let (x, y) = (args.amount_x.0, args.amount_y.0);
    require!(x == 0 || y == 0, "single-sided only");
    (x, y, x.max(y))
}

/// The pool's other asset. DCL pool ids are `token_x|token_y|fee`.
fn other_token(pool_id: &str, token: &AccountId) -> AccountId {
    let mut it = pool_id.split('|');
    let (a, b) = (it.next().unwrap_or(""), it.next().unwrap_or(""));
    let other = if a == token.as_str() {
        b
    } else if b == token.as_str() {
        a
    } else {
        env::panic_str("token is not in the pool")
    };
    other.parse().unwrap_or_else(|_| env::panic_str("pool id"))
}

/// The DCL msg that opens the position in the same call that delivers the supply.
///
/// `swap_infos` must not be empty (Rhea, 2026-09-29), so it carries one swap of ZERO: DCL runs it as
///    a no-op (the live launches that use this msg log `amount_in 0, amount_out 0`) and spends the
///    whole supply on the range.
/// The zero swap always goes from the pool's X to its Y, whichever side the launch token is on. On
///    the real dclv2 wasm a zero swap from Y to X fails the whole HotZap with
///    `ERR_TOKEN_<X>_NOT_ENOUGH`, so a token placed as Y must NOT be the swap's input.
fn hotzap_msg(args: &AddArgs, other: &AccountId) -> String {
    let p = js(&args.pool_id);
    let (x_token, y_token) = if args.amount_x.0 > 0 { (&args.token, other) } else { (other, &args.token) };
    format!(
        concat!(
            r#"{{"HotZap":{{"swap_infos":[{{"pool_ids":[{p}],"input_token":"{i}","output_token":"{o}","amount_in":"0","min_output_amount":"0"}}],"#,
            r#""add_liquidity_infos":[{{"pool_id":{p},"left_point":{l},"right_point":{r},"amount_x":"{x}","amount_y":"{y}","min_amount_x":"0","min_amount_y":"0"}}]}}}}"#
        ),
        p = p, i = x_token, o = y_token, l = args.left_point, r = args.right_point, x = args.amount_x.0, y = args.amount_y.0
    )
}

fn add_liquidity_args(args: &AddArgs) -> String {
    format!(
        r#"{{"pool_id":{},"left_point":{},"right_point":{},"amount_x":"{}","amount_y":"{}","min_amount_x":"0","min_amount_y":"0"}}"#,
        js(&args.pool_id), args.left_point, args.right_point, args.amount_x.0, args.amount_y.0
    )
}

/// What `add` does first for a pool, from what this account knows about it.
#[derive(Debug, PartialEq)]
enum AddStep {
    /// the position is recorded: return it, send nothing
    Known(String),
    /// never attempted: send the HotZap
    Zap,
    /// a HotZap went out and may have landed: search the exchange from this index before anything else
    Lookup(u64),
    /// the last HotZap was refunded in full: no position, so check the exchange balance, then resend
    CheckAsset,
}

fn add_step(p: Option<&Position>) -> AddStep {
    match p {
        None => AddStep::Zap,
        Some(Position { lpt_id: Some(l), .. }) => AddStep::Known(l.clone()),
        Some(Position { refunded: true, .. }) => AddStep::CheckAsset,
        Some(p) => AddStep::Lookup(p.scan_from),
    }
}

#[derive(Debug, PartialEq)]
enum AssetStep {
    /// the supply is already in this account's DCL balance: place it without sending anything
    AddOnly,
    /// the supply is still here: send the HotZap
    Zap,
    /// the HotZap already went out in this call and no position came of it: stop and let a resume look again
    Wait,
}

fn asset_step(dcl_balance: u128, supply: u128, zapped_here: bool) -> AssetStep {
    if dcl_balance >= supply {
        AssetStep::AddOnly
    } else if zapped_here {
        AssetStep::Wait
    } else {
        AssetStep::Zap
    }
}

#[derive(Debug, PartialEq)]
enum Scan {
    Found(String),
    /// a full page without it: read the next one from here
    More(u64),
    /// the list ended without it
    End(u64),
}

/// Find this launch's position in one page of `list_liquidities`. It is the first row on the same pool
/// with the same range: no one else can hold such a row before the supply is placed, because the whole
/// supply is in this account or in its DCL balance until then.
fn scan_page(rows: &[LiquidityRow], from: u64, page: u64, args: &AddArgs) -> Scan {
    if let Some(r) = rows
        .iter()
        .find(|r| r.pool_id == args.pool_id && r.left_point == args.left_point && r.right_point == args.right_point)
    {
        return Scan::Found(r.lpt_id.clone());
    }
    let next = from + rows.len() as u64;
    if rows.len() as u64 >= page { Scan::More(next) } else { Scan::End(next) }
}

/// Stop this attempt without undoing what it learned: an empty id tells the factory "not yet", so it
/// parks the launch at AddLiquidity and a `resume` continues from the recorded state. A panic here
/// would roll that state back.
fn pending(args: &AddArgs, reason: &str) -> PromiseOrValue<String> {
    emit_pending(args, reason);
    PromiseOrValue::Value(String::new())
}

fn emit_pending(args: &AddArgs, reason: &str) {
    emit("position_pending", &format!(r#"{{"token":"{}","pool_id":{},"reason":{}}}"#, args.token, js(&args.pool_id), js(reason)));
}

/// Whether this call still has `need` to attach, plus its own tail.
fn can_afford(need: Gas) -> bool {
    env::prepaid_gas().as_gas().saturating_sub(env::used_gas().as_gas()) >= need.as_gas().saturating_add(GAS_TAIL.as_gas())
}

fn emit(event: &str, data: &str) {
    env::log_str(&format!(r#"EVENT_JSON:{{"standard":"{}","version":"{}","event":"{}","data":[{}]}}"#, EVENT_STANDARD, EVENT_VERSION, event, data));
}

fn js(s: &str) -> String {
    near_sdk::serde_json::to_string(s).unwrap_or_else(|_| env::panic_str("json"))
}
#[cfg(test)]
mod add_tests {
    use super::*;
    use near_sdk::test_utils::{get_created_receipts, VMContextBuilder};
    use near_sdk::mock::MockAction;
    use near_sdk::{testing_env, RuntimeFeesConfig};
    use std::collections::HashMap;

    const SUPPLY: u128 = 1_000_000_000 * 1_000_000_000_000_000_000;

    fn me() -> AccountId { "lock3.near".parse().unwrap() }
    fn factory() -> AccountId { "f.near".parse().unwrap() }
    fn dcl() -> AccountId { "dclv2.ref-labs.near".parse().unwrap() }

    /// A call from `pred` with `tgas` of prepaid gas.
    fn ctx(pred: AccountId, tgas: u64) {
        let mut c = VMContextBuilder::new();
        c.current_account_id(me()).predecessor_account_id(pred).prepaid_gas(Gas::from_tgas(tgas)).storage_usage(1000);
        testing_env!(c.build(), near_sdk::test_vm_config(), RuntimeFeesConfig::test(), HashMap::default(), vec![]);
    }

    fn locker() -> Locker {
        ctx(me(), 300);
        Locker::new(factory(), dcl(), "wrap.near".parse().unwrap())
    }

    fn args_x(token: &str) -> AddArgs {
        AddArgs {
            token: token.parse().unwrap(),
            pool_id: format!("{}|wrap.near|10000", token),
            left_point: 200,
            right_point: 500000,
            amount_x: U128(SUPPLY),
            amount_y: U128(0),
        }
    }

    /// (receiver, method, args) of every function call created so far
    fn calls() -> Vec<(String, String, String)> {
        let mut out = vec![];
        for r in get_created_receipts() {
            for a in r.actions {
                if let MockAction::FunctionCallWeight { method_name, args, .. } = a {
                    out.push((r.receiver_id.to_string(), String::from_utf8(method_name).unwrap(), String::from_utf8(args).unwrap()));
                }
            }
        }
        out
    }

    fn methods() -> Vec<String> { calls().into_iter().map(|c| c.1).collect() }

    fn value(r: PromiseOrValue<String>) -> String {
        match r {
            PromiseOrValue::Value(v) => v,
            PromiseOrValue::Promise(_) => panic!("expected a value, got a promise"),
        }
    }

    /// The exact msg a third-party launchpad sent to dclv2 on mainnet (tx 73GLYtcHVHkHww8H3r6mo9tnm3cYTZjF7igSSm2w7n1J),
    /// which opened position `...#16066` with `paid_token_x` 999999999999999999999985342 and a 14658 remainder.
    #[test]
    fn the_hotzap_msg_matches_the_one_seen_live() {
        let t = "shoot-f41ea6.rhea.meme-launchpad.near";
        let mut a = args_x(t);
        a.left_point = 600;
        let live = r#"{"HotZap":{"swap_infos":[{"pool_ids":["shoot-f41ea6.rhea.meme-launchpad.near|wrap.near|10000"],"input_token":"shoot-f41ea6.rhea.meme-launchpad.near","output_token":"wrap.near","amount_in":"0","min_output_amount":"0"}],"add_liquidity_infos":[{"pool_id":"shoot-f41ea6.rhea.meme-launchpad.near|wrap.near|10000","left_point":600,"right_point":500000,"amount_x":"1000000000000000000000000000","amount_y":"0","min_amount_x":"0","min_amount_y":"0"}]}}"#;
        assert_eq!(hotzap_msg(&a, &"wrap.near".parse().unwrap()), live);

        let v: near_sdk::serde_json::Value = near_sdk::serde_json::from_str(live).unwrap();
        assert_eq!(v["HotZap"]["swap_infos"].as_array().unwrap().len(), 1, "swap_infos cannot be empty");
    }

    /// A launch whose token sorts second in the pool id (an FT pair such as `bnb-0x…omdep.near|dots…`)
    /// places the supply as Y, and its zero swap still goes X -> Y (a Y -> X zero swap fails on dclv2).
    #[test]
    fn a_token_on_the_y_side_zaps_y() {
        let a = AddArgs {
            token: "dots.nearlytrade.near".parse().unwrap(),
            pool_id: "bnb-0xa9ee28c80f960b889dfbd1902055218cba016f75.omdep.near|dots.nearlytrade.near|10000".into(),
            left_point: -273400,
            right_point: 176400,
            amount_x: U128(0),
            amount_y: U128(SUPPLY),
        };
        let other = other_token(&a.pool_id, &a.token);
        assert_eq!(other.as_str(), "bnb-0xa9ee28c80f960b889dfbd1902055218cba016f75.omdep.near");
        let v: near_sdk::serde_json::Value = near_sdk::serde_json::from_str(&hotzap_msg(&a, &other)).unwrap();
        let s = &v["HotZap"]["swap_infos"][0];
        assert_eq!(s["input_token"], other.as_str(), "the zero swap's input is the pool's X");
        assert_eq!(s["output_token"], "dots.nearlytrade.near");
        assert_eq!(s["amount_in"], "0");
        let l = &v["HotZap"]["add_liquidity_infos"][0];
        assert_eq!((l["amount_x"].as_str().unwrap(), l["amount_y"].as_str().unwrap()), ("0", "1000000000000000000000000000"));
        assert_eq!((l["left_point"].as_i64().unwrap(), l["right_point"].as_i64().unwrap()), (-273400, 176400));
    }

    #[test]
    #[should_panic(expected = "token is not in the pool")]
    fn a_token_outside_the_pool_is_refused() {
        other_token("a.near|wrap.near|10000", &"b.near".parse().unwrap());
    }

    /// Rows exactly as dclv2 v2.3.13 returns them (lock2.nearlytrade.near, from_index 1400, 2026-09-30).
    const LIVE_ROWS: &str = r#"[{"lpt_id":"girl.nearlytrade.near|wrap.near|10000#16272","owner_id":"lock2.nearlytrade.near","pool_id":"girl.nearlytrade.near|wrap.near|10000","left_point":200,"right_point":500000,"amount":"50498695734733140569374","mft_id":"","v_liquidity":"0","unclaimed_fee_x":"0","unclaimed_fee_y":"0"},{"lpt_id":"olivine-14.nearlytrade.near|wrap.near|10000#16273","owner_id":"lock2.nearlytrade.near","pool_id":"olivine-14.nearlytrade.near|wrap.near|10000","left_point":200,"right_point":500000,"amount":"50498695734733140569374","mft_id":"","v_liquidity":"0","unclaimed_fee_x":"12","unclaimed_fee_y":"34"},{"lpt_id":"bnb-0xa9ee28c80f960b889dfbd1902055218cba016f75.omdep.near|dots.nearlytrade.near|10000#16274","owner_id":"lock2.nearlytrade.near","pool_id":"bnb-0xa9ee28c80f960b889dfbd1902055218cba016f75.omdep.near|dots.nearlytrade.near|10000","left_point":-273400,"right_point":176400,"amount":"73904916","mft_id":"","v_liquidity":"0","unclaimed_fee_x":"0","unclaimed_fee_y":"0"}]"#;

    #[test]
    fn the_lpt_is_read_back_from_a_live_list_liquidities_page() {
        let rows: Vec<LiquidityRow> = near_sdk::serde_json::from_str(LIVE_ROWS).unwrap();
        assert_eq!(rows.len(), 3);

        assert_eq!(scan_page(&rows, 1400, 8, &args_x("olivine-14.nearlytrade.near")), Scan::Found("olivine-14.nearlytrade.near|wrap.near|10000#16273".into()));

        let mut other_range = args_x("olivine-14.nearlytrade.near");
        other_range.left_point = 400;
        assert_eq!(scan_page(&rows, 1400, 8, &other_range), Scan::End(1403), "a short page is the end of the list");

        assert_eq!(scan_page(&rows, 1400, 3, &args_x("nobody.near")), Scan::More(1403));

        assert_eq!(scan_page(&[], 1540, 8, &args_x("nobody.near")), Scan::End(1540));

        let mut twice = rows.clone();
        let mut dup = rows[1].clone();
        dup.lpt_id = "olivine-14.nearlytrade.near|wrap.near|10000#99999".into();
        twice.push(dup);
        assert_eq!(scan_page(&twice, 0, 8, &args_x("olivine-14.nearlytrade.near")), Scan::Found("olivine-14.nearlytrade.near|wrap.near|10000#16273".into()));
    }

    #[test]
    fn what_add_does_first_depends_only_on_what_is_recorded() {
        assert_eq!(add_step(None), AddStep::Zap);
        let known = Position { lpt_id: Some("p#1".into()), scan_from: 3, refunded: false };
        assert_eq!(add_step(Some(&known)), AddStep::Known("p#1".into()));
        assert_eq!(add_step(Some(&Position { lpt_id: None, scan_from: 7, refunded: false })), AddStep::Lookup(7));
        assert_eq!(add_step(Some(&Position { lpt_id: None, scan_from: 7, refunded: true })), AddStep::CheckAsset);

        assert_eq!(asset_step(SUPPLY, SUPPLY, false), AssetStep::AddOnly);
        assert_eq!(asset_step(SUPPLY + 5, SUPPLY, true), AssetStep::AddOnly);

        assert_eq!(asset_step(14_658, SUPPLY, false), AssetStep::Zap);

        assert_eq!(asset_step(0, SUPPLY, true), AssetStep::Wait);
    }

    /// First attempt: storage_deposit + ONE ft_transfer_call with the HotZap, nothing else.
    #[test]
    fn the_first_add_sends_the_supply_once_with_the_hotzap() {
        let mut l = locker();
        ctx(factory(), 207);
        let r = l.add(args_x("cat.nearlytrade.near"));
        drop(r);
        let c = calls();
        let m: Vec<&str> = c.iter().map(|c| c.1.as_str()).collect();
        assert_eq!(m, vec!["storage_deposit", "ft_transfer_call", "on_zapped"]);
        assert_eq!(c[1].0, "cat.nearlytrade.near");
        let ftc: near_sdk::serde_json::Value = near_sdk::serde_json::from_str(&c[1].2).unwrap();
        assert_eq!(ftc["receiver_id"], "dclv2.ref-labs.near");
        assert_eq!(ftc["amount"], SUPPLY.to_string());
        assert!(ftc["msg"].as_str().unwrap().starts_with(r#"{"HotZap":"#));
        assert!(!m.contains(&"add_liquidity"), "no Deposit + add_liquidity any more");
        assert_eq!(l.get_position("cat.nearlytrade.near|wrap.near|10000".into()), Some(Position { lpt_id: None, scan_from: 0, refunded: false }));
    }

    #[test]
    #[should_panic(expected = "factory only")]
    fn only_the_factory_adds() {
        let mut l = locker();
        ctx("mallory.near".parse().unwrap(), 207);
        let _ = l.add(args_x("cat.nearlytrade.near"));
    }

    /// The whole retry story for one pool, callback by callback.
    #[test]
    fn a_retry_never_sends_the_supply_twice() {
        let mut l = locker();
        let a = args_x("cat.nearlytrade.near");
        ctx(factory(), 207);
        drop(l.add(a.clone()));


        ctx(me(), 150);
        drop(l.on_zapped(a.clone(), Ok(U128(SUPPLY))));
        let c = calls();
        assert_eq!(c.iter().map(|c| c.1.as_str()).collect::<Vec<_>>(), vec!["list_liquidities", "on_listed"]);
        assert!(c[0].2.contains(r#""from_index":0"#) && c[0].2.contains(r#""limit":8"#));


        ctx(me(), 100);
        assert_eq!(value(l.on_listed(a.clone(), 0, true, Err(PromiseError::Failed))), "");


        ctx(factory(), 267);
        drop(l.add(a.clone()));
        let m = methods();
        assert_eq!(m, vec!["list_liquidities", "on_listed"]);
        assert!(!m.iter().any(|m| m == "ft_transfer_call"), "the supply must not be sent again");


        let rows = vec![
            LiquidityRow { lpt_id: "dog|wrap.near|10000#1".into(), pool_id: "dog|wrap.near|10000".into(), left_point: 200, right_point: 500000 },
            LiquidityRow { lpt_id: "cat.nearlytrade.near|wrap.near|10000#2".into(), pool_id: a.pool_id.clone(), left_point: 200, right_point: 500000 },
        ];
        ctx(me(), 100);
        assert_eq!(value(l.on_listed(a.clone(), 0, false, Ok(rows))), "cat.nearlytrade.near|wrap.near|10000#2");
        assert_eq!(l.get_opened(), 1);


        ctx(factory(), 267);
        assert_eq!(value(l.add(a.clone())), "cat.nearlytrade.near|wrap.near|10000#2");
        assert!(calls().is_empty(), "a recorded position creates no receipts");
        assert_eq!(l.get_opened(), 1, "counted once");
    }

    /// The HotZap was refused (refunded in full): nothing is searched, the balance is checked and,
    /// the supply being back here, the HotZap goes out again on the next resume.
    #[test]
    fn a_refunded_hotzap_is_resent_on_resume() {
        let mut l = locker();
        let a = args_x("cat.nearlytrade.near");
        ctx(factory(), 207);
        drop(l.add(a.clone()));
        ctx(me(), 150);
        assert_eq!(value(l.on_zapped(a.clone(), Ok(U128(0)))), "");
        assert!(l.get_position(a.pool_id.clone()).unwrap().refunded);

        ctx(factory(), 267);
        drop(l.add(a.clone()));
        assert_eq!(methods(), vec!["get_user_asset", "on_asset"], "a refund proves there is no position: go straight to the balance");

        ctx(me(), 200);
        drop(l.on_asset(a.clone(), false, Ok(U128(0))));
        assert_eq!(methods(), vec!["storage_deposit", "ft_transfer_call", "on_zapped"]);
    }

    /// 10-01 audit: after a HotZap refunded in full the pool reads "refunded"; once the supply is placed from the
    /// exchange balance instead, the flag is cleared, so a live position never reads as "proven none".
    #[test]
    fn placing_the_supply_from_the_exchange_clears_a_stale_refund_flag() {
        let mut l = locker();
        let a = args_x("cat.nearlytrade.near");
        ctx(factory(), 207);
        drop(l.add(a.clone()));
        ctx(me(), 150);
        assert_eq!(value(l.on_zapped(a.clone(), Ok(U128(0)))), "");
        assert!(l.get_position(a.pool_id.clone()).unwrap().refunded);
        ctx(me(), 200);
        drop(l.on_asset(a.clone(), false, Ok(U128(SUPPLY))));
        assert_eq!(methods(), vec!["add_liquidity", "on_added"]);
        assert!(!l.get_position(a.pool_id.clone()).unwrap().refunded);
    }

    /// The tokens reached the exchange but no position came of them: the supply sits in this account's
    /// DCL balance, and it is placed with add_liquidity instead of being sent again.
    #[test]
    fn supply_stranded_at_the_exchange_is_placed_without_resending() {
        let mut l = locker();
        let a = args_x("cat.nearlytrade.near");
        ctx(factory(), 207);
        drop(l.add(a.clone()));

        ctx(me(), 150);
        drop(l.on_zapped(a.clone(), Err(PromiseError::Failed)));
        assert_eq!(methods(), vec!["list_liquidities", "on_listed"]);

        ctx(me(), 150);
        drop(l.on_listed(a.clone(), 0, true, Ok(vec![])));
        assert_eq!(methods(), vec!["get_user_asset", "on_asset"]);

        ctx(me(), 120);
        drop(l.on_asset(a.clone(), true, Ok(U128(SUPPLY))));
        let c = calls();
        assert_eq!(c.iter().map(|c| c.1.as_str()).collect::<Vec<_>>(), vec!["add_liquidity", "on_added"]);
        assert_eq!(c[0].0, "dclv2.ref-labs.near");
        let v: near_sdk::serde_json::Value = near_sdk::serde_json::from_str(&c[0].2).unwrap();
        assert_eq!((v["amount_x"].as_str().unwrap(), v["left_point"].as_i64().unwrap()), (SUPPLY.to_string().as_str(), 200));

        ctx(me(), 20);
        assert_eq!(l.on_added(a.clone(), Ok("cat.nearlytrade.near|wrap.near|10000#5".into())), "cat.nearlytrade.near|wrap.near|10000#5");
        assert_eq!(l.get_opened(), 1);
    }

    /// 10-01 audit: the supply is reserved the moment the HotZap sends it and stays so through every retry
    /// (search, balance check, add_liquidity), until the position is booked by either route.
    #[test]
    fn the_supply_is_reserved_from_the_hotzap_until_the_position_is_booked() {
        let mut l = locker();
        let a = args_x("cat.nearlytrade.near");
        assert_eq!(l.get_supply_reserved(a.token.clone()).0, 0, "nothing sent yet");
        ctx(factory(), 207);
        drop(l.add(a.clone()));
        assert_eq!(l.get_supply_reserved(a.token.clone()).0, SUPPLY);

        ctx(me(), 150);
        drop(l.on_zapped(a.clone(), Err(PromiseError::Failed)));
        ctx(me(), 150);
        drop(l.on_listed(a.clone(), 0, true, Ok(vec![])));
        assert_eq!(l.get_supply_reserved(a.token.clone()).0, SUPPLY);
        ctx(me(), 120);
        drop(l.on_asset(a.clone(), true, Ok(U128(SUPPLY))));
        assert_eq!(methods(), vec!["add_liquidity", "on_added"]);
        assert_eq!(l.get_supply_reserved(a.token.clone()).0, SUPPLY, "still reserved while add_liquidity is in flight");

        ctx(me(), 20);
        assert_eq!(l.on_added(a.clone(), Err(PromiseError::Failed)), "");
        assert_eq!(l.get_supply_reserved(a.token.clone()).0, SUPPLY);
        ctx(me(), 20);
        assert_eq!(l.on_added(a.clone(), Ok("cat.nearlytrade.near|wrap.near|10000#5".into())), "cat.nearlytrade.near|wrap.near|10000#5");
        assert_eq!(l.get_supply_reserved(a.token.clone()).0, 0, "a position: what is left at the exchange is a remainder");


        let b = args_x("dog.nearlytrade.near");
        ctx(factory(), 207);
        drop(l.add(b.clone()));
        ctx(me(), 150);
        assert_eq!(value(l.on_zapped(b.clone(), Ok(U128(0)))), "");
        assert_eq!(l.get_supply_reserved(b.token.clone()).0, SUPPLY, "a refund is not a release: the supply is here and goes out again");
        ctx(me(), 200);
        drop(l.on_asset(b.clone(), false, Ok(U128(0))));
        assert_eq!(methods(), vec!["storage_deposit", "ft_transfer_call", "on_zapped"]);
        assert_eq!(l.get_supply_reserved(b.token.clone()).0, SUPPLY);
        let rows = vec![LiquidityRow { lpt_id: "dog.nearlytrade.near|wrap.near|10000#6".into(), pool_id: b.pool_id.clone(), left_point: 200, right_point: 500000 }];
        ctx(me(), 100);
        assert_eq!(value(l.on_listed(b.clone(), 1, true, Ok(rows))), "dog.nearlytrade.near|wrap.near|10000#6");
        assert_eq!(l.get_supply_reserved(b.token.clone()).0, 0);
        assert_eq!(l.get_supply_reserved(a.token.clone()).0, 0, "per token");
    }

    /// 10-01 audit: the factory reporting a failed launch is the one other way the supply is released.
    #[test]
    fn release_supply_lets_go_of_a_failed_launch_supply_only() {
        let mut l = locker();
        let a = args_x("cat.nearlytrade.near");
        let b = args_x("dog.nearlytrade.near");
        ctx(factory(), 207);
        drop(l.add(a.clone()));
        ctx(factory(), 207);
        drop(l.add(b.clone()));
        ctx(factory(), 30);
        l.release_supply(a.token.clone());
        assert_eq!(l.get_supply_reserved(a.token.clone()).0, 0);
        assert_eq!(l.get_supply_reserved(b.token.clone()).0, SUPPLY, "another launch's supply is untouched");

        ctx(factory(), 30);
        l.release_supply("eel.nearlytrade.near".parse().unwrap());
        assert!(calls().is_empty(), "it moves nothing");
    }

    #[test]
    #[should_panic(expected = "factory only")]
    fn only_the_factory_releases_a_supply() {
        let mut l = locker();
        ctx("mallory.near".parse().unwrap(), 30);
        l.release_supply("cat.nearlytrade.near".parse().unwrap());
    }

    /// Nothing at the exchange and the HotZap already went out in this call: stop instead of looping.
    #[test]
    fn one_call_sends_at_most_one_hotzap() {
        let mut l = locker();
        let a = args_x("cat.nearlytrade.near");
        ctx(factory(), 207);
        drop(l.add(a.clone()));
        ctx(me(), 150);
        assert_eq!(value(l.on_asset(a.clone(), true, Ok(U128(0)))), "");
        assert!(calls().is_empty());
    }

    /// A long page walk keeps what it learned: a resume continues from the last row read.
    #[test]
    fn the_search_resumes_where_it_stopped() {
        let mut l = locker();
        let a = args_x("cat.nearlytrade.near");
        ctx(factory(), 207);
        drop(l.add(a.clone()));
        let other = |i: u64| LiquidityRow { lpt_id: format!("x|wrap.near|10000#{}", i), pool_id: "x|wrap.near|10000".into(), left_point: 200, right_point: 500000 };

        ctx(me(), 12);
        let r = l.on_listed(a.clone(), 0, true, Ok((0..LIST_PAGE).map(other).collect()));
        assert_eq!(value(r), "");
        assert_eq!(l.get_position(a.pool_id.clone()).unwrap().scan_from, LIST_PAGE);
        ctx(factory(), 267);
        drop(l.add(a.clone()));
        assert!(calls()[0].2.contains(&format!(r#""from_index":{}"#, LIST_PAGE)));
    }

    /// Two launches in flight: the second one's search starts at the confirmed count, which is at or
    /// before its own row whatever order the callbacks run in.
    #[test]
    fn a_new_pool_is_searched_from_the_confirmed_count() {
        let mut l = locker();
        let a = args_x("cat.nearlytrade.near");
        let b = args_x("dog.nearlytrade.near");
        ctx(factory(), 207);
        drop(l.add(a.clone()));
        ctx(factory(), 207);
        drop(l.add(b.clone()));

        let rows = vec![
            LiquidityRow { lpt_id: "a#0".into(), pool_id: a.pool_id.clone(), left_point: 200, right_point: 500000 },
            LiquidityRow { lpt_id: "b#1".into(), pool_id: b.pool_id.clone(), left_point: 200, right_point: 500000 },
        ];
        ctx(me(), 100);
        assert_eq!(value(l.on_listed(b.clone(), 0, true, Ok(rows.clone()))), "b#1");
        ctx(me(), 100);
        assert_eq!(value(l.on_listed(a.clone(), 0, true, Ok(rows))), "a#0");
        assert_eq!(l.get_opened(), 2);

        ctx(factory(), 207);
        drop(l.add(args_x("eel.nearlytrade.near")));
        assert_eq!(l.get_position("eel.nearlytrade.near|wrap.near|10000".into()).unwrap().scan_from, 2);
    }

    /// The JSON the live factory formats by hand for `add` still deserializes into AddArgs.
    #[test]
    fn the_factory_add_json_still_parses() {
        let (ax, ay) = (SUPPLY, 0u128);
        let json = format!(
            r#"{{"args":{{"token":"{}","pool_id":{},"left_point":{},"right_point":{},"amount_x":"{}","amount_y":"{}"}}}}"#,
            "cat.nearlytrade.near", js("cat.nearlytrade.near|wrap.near|10000"), 200, 500000, ax, ay
        );
        #[derive(near_sdk::serde::Deserialize)]
        #[serde(crate = "near_sdk::serde")]
        struct In { args: AddArgs }
        let a: In = near_sdk::serde_json::from_str(&json).unwrap();
        assert_eq!(a.args.amount_x.0, SUPPLY);
        assert_eq!(a.args.pool_id, "cat.nearlytrade.near|wrap.near|10000");
    }
}

#[cfg(test)]
mod claim_gas_tests {
    use super::*;
    use near_sdk::mock::MockAction;
    use near_sdk::test_utils::{get_created_receipts, VMContextBuilder};
    use near_sdk::testing_env;

    /// remove_liquidity is scheduled with a floor and unused-gas weight 1; the callback with weight 0,
    /// so every TGas the factory attaches beyond the chain's needs reaches the exchange.
    #[test]
    fn remove_liquidity_takes_the_spare_gas_by_weight() {
        let me: AccountId = "lock3.near".parse().unwrap();
        let f: AccountId = "f.near".parse().unwrap();
        let mut c = VMContextBuilder::new();
        c.current_account_id(me.clone()).predecessor_account_id(me.clone());
        testing_env!(c.build());
        let mut l = Locker::new(f.clone(), "dclv2.ref-labs.near".parse().unwrap(), "wrap.near".parse().unwrap());

        c.predecessor_account_id(f).prepaid_gas(Gas::from_tgas(300));
        testing_env!(c.build());
        drop(l.claim("p#1".into(), "cat.near".parse().unwrap(), true, None));
        let mut seen = vec![];
        for r in get_created_receipts() {
            for a in r.actions {
                if let MockAction::FunctionCallWeight { method_name, prepaid_gas, gas_weight, .. } = a {
                    seen.push((String::from_utf8(method_name).unwrap(), prepaid_gas, gas_weight.0));
                }
            }
        }
        assert_eq!(seen[0], ("remove_liquidity".to_string(), GAS_REMOVE, 1));
        assert_eq!(seen[1].0, "on_claimed");
        assert_eq!(seen[1].2, 0, "the callback's gas is fixed; the spare goes to the exchange");
    }
}

#[cfg(test)]
mod claim_tests {
    use super::*;
    use near_sdk::mock::MockAction;
    use near_sdk::test_utils::{get_created_receipts, VMContextBuilder};
    use near_sdk::{testing_env, PromiseResult, RuntimeFeesConfig};
    use std::collections::HashMap;

    const N: u128 = 1_000_000_000_000_000_000_000_000;

    fn me() -> AccountId { "lock3.near".parse().unwrap() }
    fn factory() -> AccountId { "f.near".parse().unwrap() }
    fn wrap() -> AccountId { "wrap.near".parse().unwrap() }
    fn usdc() -> AccountId { "usdc.near".parse().unwrap() }
    fn cat() -> AccountId { "cat.near".parse().unwrap() }
    fn dog() -> AccountId { "dog.near".parse().unwrap() }

    /// A callback on this account with these promise results.
    fn cb(results: Vec<PromiseResult>) {
        let mut c = VMContextBuilder::new();
        c.current_account_id(me()).predecessor_account_id(me()).prepaid_gas(Gas::from_tgas(200)).storage_usage(1000);
        testing_env!(c.build(), near_sdk::test_vm_config(), RuntimeFeesConfig::test(), HashMap::default(), results);
    }

    fn locker() -> Locker {
        cb(vec![]);
        Locker::new(factory(), "dclv2.ref-labs.near".parse().unwrap(), wrap())
    }

    fn ok(v: &[u8]) -> PromiseResult { PromiseResult::Successful(v.to_vec()) }

    /// (receiver, method or "Transfer", args or amount) for every action created so far
    fn actions() -> Vec<(String, String, String)> {
        let mut out = vec![];
        for r in get_created_receipts() {
            for a in r.actions {
                match a {
                    MockAction::FunctionCallWeight { method_name, args, .. } => out.push((r.receiver_id.to_string(), String::from_utf8(method_name).unwrap(), String::from_utf8(args).unwrap())),
                    MockAction::Transfer { deposit, .. } => out.push((r.receiver_id.to_string(), "Transfer".into(), deposit.as_yoctonear().to_string())),
                    _ => {}
                }
            }
        }
        out
    }

    fn names() -> Vec<String> { actions().into_iter().map(|a| a.1).collect() }

    /// on_claimed with `remove_liquidity`'s answer `v`
    fn claimed(l: &mut Locker, token: AccountId, token_is_x: bool, quote: Option<AccountId>, v: Vec<U128>) -> PromiseOrValue<Vec<U128>> {
        cb(vec![ok(near_sdk::serde_json::to_string(&v).unwrap().as_bytes())]);
        l.on_claimed(token, token_is_x, quote)
    }

    fn value(r: PromiseOrValue<Vec<U128>>) -> Vec<U128> {
        match r {
            PromiseOrValue::Value(v) => v,
            PromiseOrValue::Promise(_) => panic!("expected a value"),
        }
    }

    /// The fees are kept at the exchange, not paid out in the same block.
    #[test]
    fn the_claim_keeps_the_fees_in_the_dcl_balance() {
        let mut l = locker();
        let mut c = VMContextBuilder::new();

        c.current_account_id(me()).predecessor_account_id(factory()).prepaid_gas(Gas::from_tgas(300));
        testing_env!(c.build());
        drop(l.claim("cat.near|wrap.near|10000#7".into(), cat(), true, None));
        let a = actions();
        assert_eq!(a[0].1, "remove_liquidity");
        let v: near_sdk::serde_json::Value = near_sdk::serde_json::from_str(&a[0].2).unwrap();
        assert_eq!(v["skip_refund_transfer"], true);
        assert_eq!(v["amount"], "0");
        assert_eq!(a[1].1, "on_claimed");
    }

    /// NEAR pair, end to end: release -> withdraw exactly this launch's wNEAR -> (arrived) unwrap ->
    /// (arrived) send the NEAR -> (landed) book it. Nothing leaves before the step that proves it is here.
    #[test]
    fn a_near_pair_forwards_only_what_arrived() {
        let mut l = locker();
        let fee = 1_400 * (N / 1000);

        cb(vec![]);
        drop(claimed(&mut l, cat(), true, None, vec![U128(0), U128(fee)]));
        let a = actions();
        assert_eq!(a.iter().map(|a| a.1.as_str()).collect::<Vec<_>>(), vec!["withdraw_asset", "on_withdrawn"]);
        assert_eq!(a[0].0, "dclv2.ref-labs.near");
        let w: near_sdk::serde_json::Value = near_sdk::serde_json::from_str(&a[0].2).unwrap();
        assert_eq!((w["token_id"].as_str().unwrap(), w["amount"].as_str().unwrap(), w["skip_unwrap_near"].as_bool().unwrap()), ("wrap.near", fee.to_string().as_str(), true));
        assert!(!a.iter().any(|a| a.1 == "Transfer"), "no NEAR moves before the withdrawal is confirmed");
        assert_eq!(l.get_dcl_reserved(wrap()).0, fee, "the fee is reserved in our DCL balance until it arrives");


        cb(vec![ok(b"true")]);
        drop(l.on_withdrawn(cat(), true, None, U128(fee), U128(0), None));
        let a = actions();
        assert_eq!(a.iter().map(|a| a.1.as_str()).collect::<Vec<_>>(), vec!["near_withdraw", "on_forwarded"]);
        assert_eq!(a[0].0, "wrap.near");
        assert_eq!(l.get_dcl_reserved(wrap()).0, 0);


        cb(vec![ok(b"")]);
        let r = l.on_forwarded(cat(), true, None, U128(fee), U128(0), U128(0));
        drop(r);
        let a = actions();
        assert_eq!(a[0], ("f.near".to_string(), "Transfer".to_string(), fee.to_string()));
        assert_eq!(a[1].1, "on_delivered");


        cb(vec![ok(b"")]);
        assert_eq!(l.on_delivered(cat(), true, U128(0), U128(fee), Ok(())), vec![U128(0), U128(fee)]);
        assert_eq!(l.get_owed().0, 0);
        assert_eq!(l.get_carry(cat()), Carry::default());
    }

    /// A withdrawal that did not arrive is booked as nothing, stays this launch's, and is retried by
    /// its next claim. lock2 would have sent the NEAR anyway, out of whatever else sat in the account.
    #[test]
    fn a_withdrawal_that_did_not_arrive_is_never_forwarded() {
        let mut l = locker();
        let fee = 3 * N;
        cb(vec![]);
        drop(claimed(&mut l, cat(), true, None, vec![U128(0), U128(fee)]));

        cb(vec![ok(b"false")]);
        let booked = value(l.on_withdrawn(cat(), true, None, U128(fee), U128(0), None));
        assert_eq!(booked, vec![U128(0), U128(0)]);
        assert!(actions().is_empty(), "nothing is sent when nothing arrived");
        assert_eq!(l.get_owed_near(cat()).0, fee);
        assert_eq!(l.get_owed().0, fee);
        assert_eq!(l.get_carry(cat()).quote_dcl.0, fee, "it is still in the exchange balance");
        assert_eq!(l.get_dcl_reserved(wrap()).0, fee);

        cb(vec![]);
        drop(claimed(&mut l, cat(), true, None, vec![U128(0), U128(N)]));
        let w: near_sdk::serde_json::Value = near_sdk::serde_json::from_str(&actions()[0].2).unwrap();
        assert_eq!(w["amount"], (fee + N).to_string(), "the next claim withdraws its own fee plus the carry");
        cb(vec![PromiseResult::Failed]);
        drop(l.on_withdrawn(cat(), true, None, U128(fee + N), U128(0), None));
        assert_eq!(l.get_owed_near(cat()).0, fee + N);
    }

    /// Replays 2026-09-22 against lock3: NEARLY's withdrawal fails while NEARKAT's arrives. Each launch
    /// only ever withdraws, forwards and books its own amount; a carry never follows another launch.
    #[test]
    fn a_carry_is_only_ever_booked_to_its_own_launch() {
        let mut l = locker();
        let (f_cat, f_dog) = (9_416 * (N / 1000), 3_848 * (N / 1000));
        cb(vec![]);
        drop(claimed(&mut l, cat(), true, None, vec![U128(0), U128(f_cat)]));
        cb(vec![ok(b"false")]);
        drop(l.on_withdrawn(cat(), true, None, U128(f_cat), U128(0), None));

        cb(vec![]);
        drop(claimed(&mut l, dog(), true, None, vec![U128(0), U128(f_dog)]));
        let w: near_sdk::serde_json::Value = near_sdk::serde_json::from_str(&actions()[0].2).unwrap();
        assert_eq!(w["amount"], f_dog.to_string());
        cb(vec![ok(b"true")]);
        drop(l.on_withdrawn(dog(), true, None, U128(f_dog), U128(0), None));
        cb(vec![ok(b"")]);
        drop(l.on_forwarded(dog(), true, None, U128(f_dog), U128(0), U128(0)));
        cb(vec![ok(b"")]);
        assert_eq!(l.on_delivered(dog(), true, U128(0), U128(f_dog), Ok(())), vec![U128(0), U128(f_dog)]);

        assert_eq!(l.get_owed_near(cat()).0, f_cat);
        assert_eq!(l.get_owed_near(dog()).0, 0);
        assert_eq!(l.get_owed().0, f_cat);
        assert_eq!(l.get_dcl_reserved(wrap()).0, f_cat);
    }

    /// Two claims for one launch in flight at once (a reset in-flight flag): the second finds the first
    /// one's amount already taken out, so the shared balance is never withdrawn twice for it.
    #[test]
    fn overlapping_claims_never_withdraw_the_same_fee_twice() {
        let mut l = locker();
        cb(vec![]);
        drop(claimed(&mut l, cat(), true, None, vec![U128(0), U128(5 * N)]));
        cb(vec![]);
        drop(claimed(&mut l, cat(), true, None, vec![U128(0), U128(N)]));
        let w: near_sdk::serde_json::Value = near_sdk::serde_json::from_str(&actions()[0].2).unwrap();
        assert_eq!(w["amount"], N.to_string(), "only the second claim's own fee");
        assert_eq!(l.get_dcl_reserved(wrap()).0, 6 * N, "both are still reserved until they arrive");
    }

    /// FT pair (token is y, quote USDC is x). The first claim of USDC here reads its tax config instead
    /// of withdrawing it (USDC has no `get_tax`: no tax); the next claim withdraws it. Both legs are
    /// forwarded as ft_transfers, and a bounced quote transfer is carried as HELD (already here), so the
    /// next claim forwards it without withdrawing it again.
    #[test]
    fn an_ft_pair_books_each_leg_only_if_it_landed() {
        let mut l = locker();
        let (fx_usdc, fy_tok) = (5_000_000u128, 7 * N);
        cb(vec![]);
        drop(claimed(&mut l, cat(), false, Some(usdc()), vec![U128(fx_usdc), U128(fy_tok)]));
        let a = actions();
        assert_eq!(a.iter().map(|a| (a.0.as_str(), a.1.as_str())).collect::<Vec<_>>(), vec![("dclv2.ref-labs.near", "withdraw_asset"), ("usdc.near", "get_tax"), ("lock3.near", "on_withdrawn")]);
        assert!(a[0].2.contains(r#""token_id":"cat.near""#) && a[0].2.contains(&format!(r#""amount":"{}""#, fy_tok)));
        assert_eq!(l.get_carry(cat()).quote_dcl.0, fx_usdc, "USDC waits at the exchange for the next claim");
        cb(vec![ok(b"true"), PromiseResult::Failed]);
        drop(l.on_withdrawn(cat(), false, Some(usdc()), U128(0), U128(fy_tok), Some(true)));
        assert_eq!(l.get_quote_tax(usdc()), Some(0), "an FT with no get_tax has no tax");
        assert_eq!(names(), vec!["ft_transfer", "on_forwarded"], "the token leg went on");
        cb(vec![ok(b"")]);
        assert_eq!(value(l.on_forwarded(cat(), false, Some(usdc()), U128(0), U128(fy_tok), U128(0))), vec![U128(0), U128(fy_tok)]);

        cb(vec![]);
        drop(claimed(&mut l, cat(), false, Some(usdc()), vec![U128(0), U128(0)]));
        let a = actions();
        assert_eq!(a.iter().map(|a| a.1.as_str()).collect::<Vec<_>>(), vec!["withdraw_asset", "on_withdrawn"]);
        assert!(a[0].2.contains(r#""token_id":"usdc.near""#) && a[0].2.contains(&format!(r#""amount":"{}""#, fx_usdc)));
        cb(vec![ok(b"true")]);
        drop(l.on_withdrawn(cat(), false, Some(usdc()), U128(fx_usdc), U128(0), Some(false)));
        let a = actions();
        assert_eq!((a[0].0.as_str(), a[0].1.as_str()), ("usdc.near", "ft_transfer"));
        assert!(a[0].2.contains(r#""receiver_id":"f.near""#) && a[0].2.contains(&format!(r#""amount":"{}""#, fx_usdc)), "untaxed: the whole amount");

        cb(vec![PromiseResult::Failed]);
        assert_eq!(value(l.on_forwarded(cat(), false, Some(usdc()), U128(fx_usdc), U128(0), U128(0))), vec![U128(0), U128(0)]);
        assert_eq!(l.get_owed_quote(cat()).0, fx_usdc);
        assert_eq!(l.get_owed_near(cat()).0, 0, "an FT pair never shows up as NEAR");
        assert_eq!(l.get_owed().0, 0);
        assert_eq!(l.get_carry(cat()).quote_held.0, fx_usdc);

        cb(vec![]);
        drop(claimed(&mut l, cat(), false, Some(usdc()), vec![U128(0), U128(0)]));
        assert_eq!(names(), vec!["ft_transfer", "on_forwarded"]);
        cb(vec![ok(b"")]);
        assert_eq!(value(l.on_forwarded(cat(), false, Some(usdc()), U128(fx_usdc), U128(0), U128(0))), vec![U128(fx_usdc), U128(0)]);
        assert_eq!(l.get_carry(cat()), Carry::default());
    }

    fn ninu() -> AccountId { "ninu-4.nearlytrade.near".parse().unwrap() }
    fn tax_json(buy: u64, exempt: &[&str], admin: &str, pairs: &[&str]) -> Vec<u8> {
        near_sdk::serde_json::json!({"tax": {"buy_bps": buy, "sell_bps": 300, "pairs": pairs, "admin": admin, "exempt": exempt}, "pending": "5"}).to_string().into_bytes()
    }
    const DCL: &str = "dclv2.ref-labs.near";

    /// A pair asset that is itself a tax token whose tax-free list predates this locker (NINU: 2 %
    /// buy, lock2 exempt): what arrives is the withdrawal less that tax, and that is what is forwarded.
    #[test]
    fn a_taxed_pair_asset_is_booked_and_forwarded_as_it_arrives() {
        let mut l = locker();
        let g = 1_234_567_891_234_567_891_234u128;
        cb(vec![]);
        drop(claimed(&mut l, cat(), true, Some(ninu()), vec![U128(0), U128(g)]));
        assert_eq!(names(), vec!["get_tax", "on_withdrawn"], "first claim: only the read");
        assert_eq!(l.get_dcl_reserved(ninu()).0, g, "still owned by the launch at the exchange");
        cb(vec![ok(&tax_json(200, &["lock2.nearlytrade.near"], "nearlytrade.near", &[DCL]))]);
        drop(l.on_withdrawn(cat(), true, Some(ninu()), U128(0), U128(0), Some(true)));
        assert_eq!(l.get_quote_tax(ninu()), Some(200));
        cb(vec![]);
        drop(claimed(&mut l, cat(), true, Some(ninu()), vec![U128(0), U128(0)]));
        assert_eq!(names(), vec!["withdraw_asset", "on_withdrawn"]);
        cb(vec![ok(b"true")]);
        drop(l.on_withdrawn(cat(), true, Some(ninu()), U128(g), U128(0), Some(false)));
        let tax = g * 200 / 10_000;
        let a = actions();
        assert_eq!((a[0].0.as_str(), a[0].1.as_str()), ("ninu-4.nearlytrade.near", "ft_transfer"));
        assert!(a[0].2.contains(&format!(r#""amount":"{}""#, g - tax)), "forwards what arrived: {}", a[0].2);
        assert_eq!(l.get_dcl_reserved(ninu()).0, 0, "the whole withdrawal left the exchange, tax included");
        let ev = near_sdk::test_utils::get_logs().into_iter().find(|x| x.contains("quote_tax_withheld")).expect("event");
        assert!(ev.contains(&format!(r#""withdrawn":"{}","tax":"{}""#, g, tax)), "{}", ev);
        cb(vec![ok(b"")]);
        assert_eq!(value(l.on_forwarded(cat(), true, Some(ninu()), U128(g - tax), U128(0), U128(0))), vec![U128(0), U128(g - tax)]);
        assert_eq!(l.get_carry(cat()), Carry::default());
    }

    /// No tax for this locker when the token has none, the exchange is not its pair, or this account is
    /// tax-free there (on its list, its admin, or itself).
    #[test]
    fn a_pair_asset_where_this_locker_is_tax_free_books_the_whole_amount() {
        let me = "lock3.near";
        for (case, body, want) in [
            ("exempt", tax_json(200, &["lock2.nearlytrade.near", me], "nearlytrade.near", &[DCL]), 0u16),
            ("admin", tax_json(200, &[], me, &[DCL]), 0),
            ("other pair", tax_json(200, &[], "nearlytrade.near", &["other.near"]), 0),
            ("no tax", br#"{"tax":null,"pending":"0"}"#.to_vec(), 0),
            ("garbage", b"not json".to_vec(), 0),
            ("capped", tax_json(900, &[], "nearlytrade.near", &[DCL]), 400),
            ("taxed", tax_json(100, &["lock2.nearlytrade.near"], "nearlytrade.near", &[DCL]), 100),
        ] {
            let mut l = locker();
            cb(vec![]);
            drop(claimed(&mut l, cat(), true, Some(ninu()), vec![U128(0), U128(10_000)]));
            cb(vec![ok(&body)]);
            drop(l.on_withdrawn(cat(), true, Some(ninu()), U128(0), U128(0), Some(true)));
            assert_eq!(l.get_quote_tax(ninu()), Some(want), "{case}");
        }
        assert_eq!(buy_tax_bps(&near_sdk::serde_json::json!({"tax": {"buy_bps": 200, "pairs": [DCL], "admin": "a", "exempt": []}}), "ninu.near", DCL, "ninu.near"), 0, "the token itself");
        assert_eq!(buy_tax_bps(&near_sdk::serde_json::json!({"tax": {"buy_bps": "150", "pairs": [DCL], "admin": "a"}}), me, DCL, "t.near"), 150, "a string bps, no exempt list");
    }

    /// The token's own formula, on the edges: rounded down, capped at 400, and the overflow branch.
    #[test]
    fn the_tax_matches_the_tokens_formula() {
        assert_eq!(tax_of(1, 200), 0);
        assert_eq!(tax_of(99, 200), 1);
        assert_eq!(tax_of(10_001, 200), 200);
        assert_eq!(tax_of(10_000, 100), 100);
        assert_eq!(tax_of(5_000_000, 0), 0);
        assert_eq!(tax_of(1_000_000, 900), 40_000, "capped at 400 like the token");
        let big = u128::MAX - 7;
        assert_eq!(tax_of(big, 300), big / 10_000 * 300, "the token's overflow branch");
        let near_max = u128::MAX / 300;
        assert_eq!(tax_of(near_max, 300), near_max * 300 / 10_000, "the largest exact product");
    }

    /// The factory's own withdrawal of loose balance sends what arrived too, once the asset's tax is known.
    #[test]
    fn an_admin_withdrawal_of_a_taxed_asset_sends_what_arrived() {
        let mut l = locker();
        cb(vec![]);
        drop(claimed(&mut l, cat(), true, Some(ninu()), vec![U128(0), U128(1)]));
        cb(vec![ok(&tax_json(200, &[], "nearlytrade.near", &[DCL]))]);
        drop(l.on_withdrawn(cat(), true, Some(ninu()), U128(0), U128(0), Some(true)));
        cb(vec![]);
        drop(l.on_admin_withdrawn(ninu(), U128(1_000_000), Ok(true)));
        let a = actions();
        assert!(a.iter().any(|x| x.0 == "ninu-4.nearlytrade.near" && x.1 == "ft_transfer" && x.2.contains(r#""amount":"980000""#)), "{a:?}");
    }

    /// A failed unwrap leaves wNEAR here: it is carried as held and unwrapped by the next claim.
    #[test]
    fn a_failed_unwrap_is_retried_without_a_new_withdrawal() {
        let mut l = locker();
        cb(vec![]);
        drop(claimed(&mut l, cat(), true, None, vec![U128(0), U128(2 * N)]));
        cb(vec![ok(b"true")]);
        drop(l.on_withdrawn(cat(), true, None, U128(2 * N), U128(0), None));
        cb(vec![PromiseResult::Failed]);
        assert_eq!(value(l.on_forwarded(cat(), true, None, U128(2 * N), U128(0), U128(0))), vec![U128(0), U128(0)]);
        assert_eq!(l.get_carry(cat()).quote_held.0, 2 * N);
        assert_eq!(l.get_owed().0, 2 * N);
        cb(vec![]);
        drop(claimed(&mut l, cat(), true, None, vec![U128(0), U128(0)]));
        assert_eq!(names(), vec!["near_withdraw", "on_forwarded"]);
    }

    /// NEAR that bounced off the factory is sent again by the next claim, on its own.
    #[test]
    fn near_that_bounced_is_resent() {
        let mut l = locker();
        cb(vec![]);
        assert_eq!(l.on_delivered(cat(), true, U128(0), U128(N), Err(PromiseError::Failed)), vec![U128(0), U128(0)]);
        assert_eq!(l.get_carry(cat()).near_held.0, N);
        assert_eq!(l.get_owed().0, N);
        cb(vec![]);
        drop(claimed(&mut l, cat(), true, None, vec![U128(0), U128(0)]));
        let a = actions();
        assert_eq!(a[0], ("f.near".to_string(), "Transfer".to_string(), N.to_string()));
        assert_eq!(a[1].1, "on_delivered");
    }

    #[test]
    fn nothing_released_and_nothing_owed_books_zero_and_sends_nothing() {
        let mut l = locker();
        cb(vec![]);
        assert_eq!(value(claimed(&mut l, cat(), true, None, vec![U128(0), U128(0)])), vec![U128(0), U128(0)]);
        assert!(actions().is_empty());
    }

    #[test]
    fn withdraw_results_are_read_strictly() {
        assert!(withdraw_arrived(Ok(b"true".to_vec())));
        assert!(!withdraw_arrived(Ok(b"false".to_vec())));
        assert!(!withdraw_arrived(Ok(vec![])));
        assert!(!withdraw_arrived(Err(PromiseError::Failed)));
        assert_eq!(settle_withdrawal(true, 9), (9, 0));
        assert_eq!(settle_withdrawal(false, 9), (0, 9));
        assert_eq!(book(true, 4), (4, 0));
        assert_eq!(book(false, 4), (0, 4));
        assert_eq!(order(true, 1, 2), vec![U128(1), U128(2)]);
        assert_eq!(order(false, 1, 2), vec![U128(2), U128(1)]);
    }

    /// The JSON the live factory formats by hand for `claim` still deserializes into claim's args.
    #[test]
    fn the_factory_claim_json_still_parses() {
        #[derive(near_sdk::serde::Deserialize)]
        #[serde(crate = "near_sdk::serde")]
        #[allow(dead_code)]
        struct In { lpt_id: String, token: AccountId, token_is_x: bool, quote: Option<AccountId> }
        let near_pair = format!(r#"{{"lpt_id":{},"token":"{}","token_is_x":{},"quote":{}}}"#, js("cat.near|wrap.near|10000#7"), "cat.near", true, "null");
        let a: In = near_sdk::serde_json::from_str(&near_pair).unwrap();
        assert!(a.quote.is_none());
        let ft_pair = format!(r#"{{"lpt_id":{},"token":"{}","token_is_x":{},"quote":{}}}"#, js("usdc.near|cat.near|10000#8"), "cat.near", false, "\"usdc.near\"");
        let a: In = near_sdk::serde_json::from_str(&ft_pair).unwrap();
        assert_eq!(a.quote.unwrap().as_str(), "usdc.near");
    }

    /// `new` registers this account on the exchange (0.5 N minimum) and on wrap.near.
    #[test]
    fn init_registers_on_the_exchange_and_wrap() {
        let _ = locker();
        let a = actions();
        assert_eq!(a.iter().map(|a| (a.0.as_str(), a.1.as_str())).collect::<Vec<_>>(), vec![("dclv2.ref-labs.near", "storage_deposit"), ("wrap.near", "storage_deposit")]);
        assert!(a.iter().all(|a| a.2.contains(r#""account_id":"lock3.near""#) && a.2.contains(r#""registration_only":true"#)));
    }
}

#[cfg(test)]
mod admin_tests {
    use super::*;
    use near_sdk::mock::MockAction;
    use near_sdk::test_utils::{get_created_receipts, VMContextBuilder};
    use near_sdk::testing_env;

    const N: u128 = 1_000_000_000_000_000_000_000_000;

    fn me() -> AccountId { "lock3.near".parse().unwrap() }
    fn factory() -> AccountId { "f.near".parse().unwrap() }
    fn wrap() -> AccountId { "wrap.near".parse().unwrap() }

    fn call(pred: AccountId, tgas: u64) {
        let mut c = VMContextBuilder::new();
        c.current_account_id(me()).predecessor_account_id(pred).prepaid_gas(Gas::from_tgas(tgas));
        testing_env!(c.build());
    }

    /// on_claimed with `remove_liquidity`'s answer `v`
    fn claimed(l: &mut Locker, token: AccountId, token_is_x: bool, quote: Option<AccountId>, v: Vec<U128>) -> PromiseOrValue<Vec<U128>> {
        let mut c = VMContextBuilder::new();
        c.current_account_id(me()).predecessor_account_id(me()).prepaid_gas(Gas::from_tgas(200));
        testing_env!(c.build(), near_sdk::test_vm_config(), near_sdk::RuntimeFeesConfig::test(), Default::default(),
            vec![near_sdk::PromiseResult::Successful(near_sdk::serde_json::to_string(&v).unwrap().into_bytes())]);
        l.on_claimed(token, token_is_x, quote)
    }

    fn locker() -> Locker {
        call(me(), 300);
        Locker::new(factory(), "dclv2.ref-labs.near".parse().unwrap(), wrap())
    }

    fn actions() -> Vec<(String, String, String)> {
        let mut out = vec![];
        for r in get_created_receipts() {
            for a in r.actions {
                if let MockAction::FunctionCallWeight { method_name, args, .. } = a {
                    out.push((r.receiver_id.to_string(), String::from_utf8(method_name).unwrap(), String::from_utf8(args).unwrap()));
                }
            }
        }
        out
    }

    #[test]
    #[should_panic(expected = "factory only")]
    fn only_the_factory_withdraws_dcl_balance() {
        let mut l = locker();
        call("owner.near".parse().unwrap(), 300);
        let _ = l.withdraw_dcl_asset(wrap(), U128(1));
    }

    #[test]
    #[should_panic(expected = "factory only")]
    fn only_the_factory_sets_the_exchange() {
        let mut l = locker();
        call("owner.near".parse().unwrap(), 300);
        l.set_dcl("dclv3.ref-labs.near".parse().unwrap());
    }

    #[test]
    fn set_dcl_points_every_later_call_at_the_new_exchange() {
        let mut l = locker();
        call(factory(), 300);
        l.set_dcl("dclv3.ref-labs.near".parse().unwrap());
        assert_eq!(l.get_dcl().as_str(), "dclv3.ref-labs.near");
        assert_eq!(actions()[0].0, "dclv3.ref-labs.near", "it registers on the new exchange");
        call(factory(), 300);
        drop(l.claim("p#1".into(), "cat.near".parse().unwrap(), true, None));
        assert_eq!(actions()[0].0, "dclv3.ref-labs.near");
    }

    /// The factory can take loose DCL balance, never what launch carries own there.
    #[test]
    fn the_factory_never_takes_launch_fees_out_of_the_dcl_balance() {
        let mut l = locker();

        call(me(), 200);
        drop(claimed(&mut l, "cat.near".parse().unwrap(), true, None, vec![U128(0), U128(5 * N)]));
        assert_eq!(l.get_dcl_reserved(wrap()).0, 5 * N);
        assert_eq!(admin_free(7 * N, 5 * N), 2 * N);
        assert_eq!(admin_free(4 * N, 5 * N), 0);

        call(me(), 200);
        drop(l.on_admin_asset(wrap(), U128(2 * N), Ok(U128(7 * N))));
        let a = actions();
        assert_eq!(a[0].1, "withdraw_asset");
        assert!(a[0].2.contains(&format!(r#""amount":"{}""#, 2 * N)) && a[0].2.contains(r#""skip_unwrap_near":true"#));
    }

    #[test]
    #[should_panic(expected = "not owned by launch fees")]
    fn a_withdrawal_that_would_dip_into_launch_fees_is_refused() {
        let mut l = locker();
        call(me(), 200);
        drop(claimed(&mut l, "cat.near".parse().unwrap(), true, None, vec![U128(0), U128(5 * N)]));
        call(me(), 200);
        let _ = l.on_admin_asset(wrap(), U128(3 * N), Ok(U128(7 * N)));
    }

    /// 10-01 audit: a supply that reached the exchange without a position is the whole DCL balance of
    /// its token and no carry owns it. It is not loose: only the HotZap remainder above it may go, the
    /// rest only once the factory released it (the launch failed) or the position is booked.
    #[test]
    fn the_factory_never_takes_an_unplaced_supply_out_of_the_dcl_balance() {
        const SUPPLY: u128 = 1_000_000_000 * N;
        let mut l = locker();
        let cat: AccountId = "cat.nearlytrade.near".parse().unwrap();
        let a = AddArgs { token: cat.clone(), pool_id: "cat.nearlytrade.near|wrap.near|10000".into(), left_point: 200, right_point: 500000, amount_x: U128(SUPPLY), amount_y: U128(0) };
        call(factory(), 207);
        drop(l.add(a.clone()));
        assert_eq!(l.get_supply_reserved(cat.clone()).0, SUPPLY);
        assert_eq!(l.get_dcl_reserved(cat.clone()).0, 0, "no carry: the supply is not fees");

        call(me(), 200);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(l.on_admin_asset(cat.clone(), U128(SUPPLY), Ok(U128(SUPPLY + 14_658))))));
        assert!(r.is_err(), "the supply is not loose balance");
        call(me(), 200);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(l.on_admin_asset(cat.clone(), U128(14_659), Ok(U128(SUPPLY + 14_658)))))).is_err(), "one yocto into the supply");

        call(me(), 200);
        drop(l.on_admin_asset(cat.clone(), U128(14_658), Ok(U128(SUPPLY + 14_658))));
        let x = actions();
        assert_eq!(x[0].1, "withdraw_asset");
        assert!(x[0].2.contains(r#""amount":"14658""#));

        call(me(), 20);
        assert_eq!(l.on_added(a.clone(), Ok("cat.nearlytrade.near|wrap.near|10000#5".into())), "cat.nearlytrade.near|wrap.near|10000#5");
        assert_eq!(l.get_supply_reserved(cat.clone()).0, 0);
        call(me(), 200);
        drop(l.on_admin_asset(cat.clone(), U128(14_658), Ok(U128(14_658))));
        assert_eq!(actions()[0].1, "withdraw_asset");


        let dog: AccountId = "dog.nearlytrade.near".parse().unwrap();
        let b = AddArgs { token: dog.clone(), pool_id: "dog.nearlytrade.near|wrap.near|10000".into(), left_point: 200, right_point: 500000, amount_x: U128(SUPPLY), amount_y: U128(0) };
        call(factory(), 207);
        drop(l.add(b));
        call(me(), 200);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(l.on_admin_asset(dog.clone(), U128(1), Ok(U128(SUPPLY)))))).is_err());
        call(factory(), 30);
        l.release_supply(dog.clone());
        call(me(), 200);
        drop(l.on_admin_asset(dog.clone(), U128(SUPPLY), Ok(U128(SUPPLY))));
        assert_eq!(actions()[0].1, "withdraw_asset");
    }

    /// Arrived -> sent to the factory; bounced -> held here and re-sent by `amount` 0.
    #[test]
    fn a_bounced_admin_transfer_is_resent() {
        let mut l = locker();
        let t: AccountId = "cat.near".parse().unwrap();
        call(me(), 200);
        drop(l.on_admin_withdrawn(t.clone(), U128(14_658), Ok(true)));
        let a = actions();
        assert_eq!((a[0].0.as_str(), a[0].1.as_str()), ("cat.near", "ft_transfer"));
        assert!(a[0].2.contains(r#""receiver_id":"f.near""#) && a[0].2.contains(r#""amount":"14658""#));

        call(me(), 200);
        match l.on_admin_withdrawn(t.clone(), U128(1), Ok(false)) {
            PromiseOrValue::Value(v) => assert!(!v),
            _ => panic!("nothing may be sent"),
        }
        assert!(actions().is_empty());

        call(me(), 200);
        assert!(!l.on_admin_sent(t.clone(), U128(14_658), Err(PromiseError::Failed)));
        assert_eq!(l.get_admin_held(t.clone()).0, 14_658);
        call(factory(), 300);
        drop(l.withdraw_dcl_asset(t.clone(), U128(0)));
        let a = actions();
        assert!(a[0].2.contains(r#""amount":"14658""#));
        assert_eq!(l.get_admin_held(t).0, 0, "taken out while in flight, put back only if it bounces again");
    }

    /// The whole point of the locker: no call it can make removes liquidity or moves a position. The
    /// only remove_liquidity it sends is the fee claim, with amount "0".
    /// 10-01 audit: while a claim's `on_claimed` has not run, the fees its `remove_liquidity` released sit in
    /// the DCL balance without being reserved yet, so the factory's withdrawal of loose balance waits.
    /// review 10-01: a count left by an `on_claimed` that died (rolled back after counting down) holds the
    /// factory's withdrawal for 10 minutes after the last claim opened, not for ever.
    #[test]
    fn an_open_claim_count_left_by_a_dead_callback_expires() {
        let mut l = locker();
        call(factory(), 300);
        drop(l.claim("p#1".into(), "cat.near".parse().unwrap(), true, None));
        assert_eq!(l.get_claims_open(), 1);
        let t0 = l.claims_open_at_ms;
        let at = |ms: u64| {
            let mut c = VMContextBuilder::new();
            c.current_account_id(me()).predecessor_account_id(me()).prepaid_gas(Gas::from_tgas(200)).block_timestamp(ms * 1_000_000);
            testing_env!(c.build());
        };
        at(t0 + CLAIM_OPEN_EXPIRY_MS);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(l.on_admin_asset(wrap(), U128(N), Ok(U128(5 * N)))))).is_err(), "still within the window");
        at(t0 + CLAIM_OPEN_EXPIRY_MS + 1);
        drop(l.on_admin_asset(wrap(), U128(N), Ok(U128(5 * N))));
    }

    #[test]
    fn the_factory_withdrawal_waits_for_open_claims() {
        let mut l = locker();
        call(factory(), 300);
        drop(l.claim("p#1".into(), "cat.near".parse().unwrap(), true, None));
        call(factory(), 300);
        drop(l.claim("p#2".into(), "dog.near".parse().unwrap(), true, None));
        assert_eq!(l.get_claims_open(), 2);

        call(me(), 200);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(l.on_admin_asset(wrap(), U128(5 * N), Ok(U128(5 * N))))));
        assert!(r.is_err(), "refused while a claim is open");
        drop(claimed(&mut l, "cat.near".parse().unwrap(), true, None, vec![U128(0), U128(5 * N)]));
        assert_eq!(l.get_claims_open(), 1);
        call(me(), 200);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(l.on_admin_asset(wrap(), U128(1), Ok(U128(6 * N)))))).is_err());

        let mut c = VMContextBuilder::new();
        c.current_account_id(me()).predecessor_account_id(me()).prepaid_gas(Gas::from_tgas(200));
        testing_env!(c.build(), near_sdk::test_vm_config(), near_sdk::RuntimeFeesConfig::test(), Default::default(), vec![near_sdk::PromiseResult::Failed]);
        match l.on_claimed("dog.near".parse().unwrap(), true, None) {
            PromiseOrValue::Value(v) => assert_eq!(v, vec![U128(0), U128(0)]),
            _ => panic!("no legs for a failed claim"),
        }
        assert_eq!(l.get_claims_open(), 0);
        assert_eq!(l.get_carry("dog.near".parse().unwrap()), Carry::default());

        call(me(), 200);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(l.on_admin_asset(wrap(), U128(2 * N), Ok(U128(6 * N)))))).is_err());
        call(me(), 200);
        drop(l.on_admin_asset(wrap(), U128(N), Ok(U128(6 * N))));
        assert_eq!(actions()[0].1, "withdraw_asset");
    }

    /// An answer on_claimed cannot read never panics it (a panic would roll the count back for good).
    #[test]
    fn an_unreadable_claim_result_counts_down_and_books_nothing() {
        for body in [b"not json".to_vec(), b"[\"1\"]".to_vec(), b"[\"1\",\"2\",\"3\"]".to_vec(), vec![b'1'; 300]] {
            let mut l = locker();
            call(factory(), 300);
            drop(l.claim("p#1".into(), "cat.near".parse().unwrap(), true, None));
            let mut c = VMContextBuilder::new();
            c.current_account_id(me()).predecessor_account_id(me()).prepaid_gas(Gas::from_tgas(200));
            testing_env!(c.build(), near_sdk::test_vm_config(), near_sdk::RuntimeFeesConfig::test(), Default::default(), vec![near_sdk::PromiseResult::Successful(body)]);
            match l.on_claimed("cat.near".parse().unwrap(), true, None) {
                PromiseOrValue::Value(v) => assert_eq!(v, vec![U128(0), U128(0)]),
                _ => panic!("no legs"),
            }
            assert_eq!(l.get_claims_open(), 0);
            assert_eq!(l.get_dcl_reserved(wrap()).0, 0);
        }
    }

    /// 10-01 audit: a nearly dry account starts no claim (its callbacks could not pay for their storage).
    #[test]
    fn a_dry_locker_refuses_to_claim() {
        let mut l = locker();
        let stake = env::storage_byte_cost().as_yoctonear() * 1000;
        let mut c = VMContextBuilder::new();
        c.current_account_id(me()).predecessor_account_id(factory()).prepaid_gas(Gas::from_tgas(300)).storage_usage(1000).account_balance(NearToken::from_yoctonear(stake + MIN_FREE - 1));
        testing_env!(c.build());
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(l.claim("p#1".into(), "cat.near".parse().unwrap(), true, None))));
        assert!(r.is_err());
        assert_eq!(l.get_claims_open(), 0);
        assert!(actions().is_empty());
    }

    /// 10-01 audit: the tax read is per exchange. After `set_dcl` the new exchange is not on the token's
    /// pair list, so the cached bps no longer applies and the next claim reads it again.
    #[test]
    fn the_tax_cache_follows_the_exchange() {
        let mut l = locker();
        let ninu: AccountId = "ninu-4.nearlytrade.near".parse().unwrap();
        let cat: AccountId = "cat.near".parse().unwrap();
        let tax = near_sdk::serde_json::json!({"tax": {"buy_bps": 200, "pairs": ["dclv2.ref-labs.near"], "admin": "nearlytrade.near", "exempt": []}}).to_string().into_bytes();
        drop(claimed(&mut l, cat.clone(), true, Some(ninu.clone()), vec![U128(0), U128(10_000)]));
        let mut c = VMContextBuilder::new();
        c.current_account_id(me()).predecessor_account_id(me()).prepaid_gas(Gas::from_tgas(200));
        testing_env!(c.build(), near_sdk::test_vm_config(), near_sdk::RuntimeFeesConfig::test(), Default::default(), vec![near_sdk::PromiseResult::Successful(tax)]);
        drop(l.on_withdrawn(cat.clone(), true, Some(ninu.clone()), U128(0), U128(0), Some(true)));
        assert_eq!(l.get_quote_tax(ninu.clone()), Some(200));
        call(factory(), 300);
        l.set_dcl("dclv3.ref-labs.near".parse().unwrap());
        assert_eq!(l.get_quote_tax(ninu.clone()), None, "not read for the new exchange");
        drop(claimed(&mut l, cat.clone(), true, Some(ninu.clone()), vec![U128(0), U128(0)]));
        let a = actions();
        assert!(a.iter().any(|x| x.0 == "ninu-4.nearlytrade.near" && x.1 == "get_tax"), "read again: {a:?}");
    }

    #[test]
    fn no_code_path_moves_a_position_or_removes_liquidity() {
        let src = include_str!("lib.rs");
        let code = src.split("#[cfg(test)]").next().unwrap();
        for forbidden in ["nft_transfer", "nft_approve", "mft_transfer", "mft_register", "batch_remove_liquidity", "batch_update_liquidity", "merge_liquidity", "burn_v_liquidity", "mint_v_liquidity", "DeleteKey", "add_full_access_key", "add_access_key", "deploy_contract", "delete_account"] {
            assert!(!code.contains(forbidden), "{} must never appear in the locker", forbidden);
        }
        assert_eq!(code.matches(r#""remove_liquidity""#).count(), 1, "one call site: the claim");
        assert_eq!(code.matches(r#""remove_liquidity".to_string()"#).count(), 1);
        assert!(code.contains(r#"{{"lpt_id":{},"amount":"0","min_amount_x":"0","min_amount_y":"0","skip_refund_transfer":true}}"#));
    }
}

#[cfg(test)]
mod budget_tests {
    use super::*;

    /// The numbers the factory's fixed budgets have to hold, spelled out.
    #[test]
    fn the_chains_fit_the_factory_budgets() {
        assert_eq!(T_ADD_NEEDS, 103, "add: of the factory's 180 (207 inside the launch tx)");
        assert_eq!(tg(GAS_CB_CLAIMED), 143);
        assert_eq!(T_CLAIM_NEEDS, 168, "claim: of the factory's 180; remove_liquidity gets the rest by weight");
        assert!(T_FACTORY_CLAIM - T_CLAIM_NEEDS + tg(GAS_REMOVE) >= 30, "remove_liquidity burned 7.75 on the real wasm");
        assert!(tg(GAS_WITHDRAW) - 16 >= 20, "dclv2 refuses a withdraw_asset whose transfer would get under ~20");
        assert_eq!(T_ADMIN_WITHDRAW, 90);
    }
}

#[cfg(test)]
mod add_buy_tests {
    use super::*;
    use near_sdk::mock::MockAction;
    use near_sdk::test_utils::{get_created_receipts, VMContextBuilder};
    use near_sdk::testing_env;

    const N: u128 = 1_000_000_000_000_000_000_000_000;
    const SUPPLY: u128 = 1_000_000_000 * 1_000_000_000_000_000_000;

    fn me() -> AccountId { "lock3.near".parse().unwrap() }
    fn factory() -> AccountId { "f.near".parse().unwrap() }
    fn cat() -> AccountId { "cat.nearlytrade.near".parse().unwrap() }

    fn call(pred: AccountId, tgas: u64, deposit: u128) {
        let mut c = VMContextBuilder::new();
        c.current_account_id(me()).predecessor_account_id(pred).prepaid_gas(Gas::from_tgas(tgas)).attached_deposit(NearToken::from_yoctonear(deposit));
        testing_env!(c.build());
    }

    fn locker() -> Locker {
        call(me(), 300, 0);
        Locker::new(factory(), "dclv2.ref-labs.near".parse().unwrap(), "wrap.near".parse().unwrap())
    }

    fn args() -> AddArgs {
        AddArgs { token: cat(), pool_id: "cat.nearlytrade.near|wrap.near|10000".into(), left_point: 200, right_point: 500000, amount_x: U128(SUPPLY), amount_y: U128(0) }
    }

    /// the factory's own message, passed through untouched
    fn buy() -> BuyArgs {
        BuyArgs { amount: U128(5 * N), msg: r#"{"Swap":{"pool_ids":["cat.nearlytrade.near|wrap.near|10000"],"output_token":"cat.nearlytrade.near","min_output_amount":"48000000000000000000000000","swap_out_recipient":"creator.near"}}"#.into() }
    }

    /// (receiver, method or Transfer, args or amount, deposit, gas weight)
    fn actions() -> Vec<(String, String, String, u128, u64)> {
        let mut out = vec![];
        for r in get_created_receipts() {
            for a in r.actions {
                match a {
                    MockAction::FunctionCallWeight { method_name, args, attached_deposit, gas_weight, .. } => out.push((r.receiver_id.to_string(), String::from_utf8(method_name).unwrap(), String::from_utf8(args).unwrap(), attached_deposit.as_yoctonear(), gas_weight.0)),
                    MockAction::Transfer { deposit, .. } => out.push((r.receiver_id.to_string(), "Transfer".into(), deposit.as_yoctonear().to_string(), 0, 0)),
                    _ => {}
                }
            }
        }
        out
    }

    /// the hop runs (one block after add_buy): the NEAR leaves as wNEAR for the swap
    fn hop(l: &mut Locker) {
        call(me(), 300, 0);
        drop(l.buy_hop(cat(), buy()));
    }

    fn value(r: PromiseOrValue<BuyOutcome>) -> BuyOutcome {
        match r { PromiseOrValue::Value(v) => v, _ => panic!("expected a value") }
    }

    /// One call creates the HotZap batch AND the hop; nothing goes to wrap.near yet.
    #[test]
    fn add_buy_schedules_the_hotzap_and_a_hop_on_this_account() {
        let mut l = locker();
        call(factory(), 300, 5 * N);
        drop(l.add_buy(args(), buy()));
        let a = actions();
        let m: Vec<(&str, &str)> = a.iter().map(|a| (a.0.as_str(), a.1.as_str())).collect();
        assert_eq!(m, vec![("cat.nearlytrade.near", "storage_deposit"), ("cat.nearlytrade.near", "ft_transfer_call"), ("lock3.near", "on_zapped"), ("lock3.near", "buy_hop"), ("lock3.near", "on_add_bought")]);
        assert!(a[1].2.contains(r#"{\"HotZap\":"#));
        assert_eq!(a[3].4, 3, "the hop takes most of the spare gas for the swap");
        assert_eq!(a[4].4, 0);
        assert_eq!(l.get_buy_held(cat()).near.0, 5 * N, "the NEAR is here until the hop wraps it");
        assert_eq!(l.get_position(args().pool_id).unwrap().scan_from, 0);
    }

    /// The hop wraps the NEAR and sends it with the factory's message, in one wrap.near batch.
    #[test]
    fn the_hop_wraps_and_swaps_with_the_factory_message() {
        let mut l = locker();

        call(me(), 200, 0);
        drop(l.buy_hop(cat(), buy()));
        let a = actions();
        assert_eq!((a[0].0.as_str(), a[0].1.as_str(), a[0].3), ("wrap.near", "near_deposit", 5 * N));
        assert_eq!((a[1].0.as_str(), a[1].1.as_str(), a[1].3, a[1].4), ("wrap.near", "ft_transfer_call", 1, 1));
        let v: near_sdk::serde_json::Value = near_sdk::serde_json::from_str(&a[1].2).unwrap();
        assert_eq!(v["receiver_id"], "dclv2.ref-labs.near");
        assert_eq!(v["amount"], (5 * N).to_string());
        assert_eq!(v["msg"], buy().msg, "the message is the factory's, byte for byte");
        assert_eq!(a[2].1, "on_bought");
    }

    #[test]
    #[should_panic(expected = "attach exactly the buy")]
    fn the_deposit_must_match_the_buy() {
        let mut l = locker();
        call(factory(), 300, 4 * N);
        let _ = l.add_buy(args(), buy());
    }

    #[test]
    #[should_panic(expected = "factory only")]
    fn only_the_factory_adds_with_a_buy() {
        let mut l = locker();
        call("mallory.near".parse().unwrap(), 300, 5 * N);
        let _ = l.add_buy(args(), buy());
    }

    #[test]
    #[should_panic(expected = "use add")]
    fn a_retry_never_goes_through_add_buy() {
        let mut l = locker();
        call(factory(), 300, 5 * N);
        drop(l.add_buy(args(), buy()));
        call(factory(), 300, 5 * N);
        let _ = l.add_buy(args(), buy());
    }

    #[test]
    #[should_panic(expected = "attach at least")]
    fn too_little_gas_is_refused_before_anything_moves() {
        let mut l = locker();
        call(factory(), 150, 5 * N);
        let _ = l.add_buy(args(), buy());
    }

    /// A full fill: the exchange kept everything, nothing to return.
    #[test]
    fn a_full_fill_books_the_whole_amount() {
        let mut l = locker();
        call(factory(), 300, 5 * N);
        drop(l.add_buy(args(), buy()));
        hop(&mut l);
        call(me(), 100, 0);
        assert_eq!(value(l.on_bought(cat(), U128(5 * N), Ok(U128(5 * N)))), BuyOutcome { used: U128(5 * N), refunded: U128(0) });
        assert_eq!(l.get_bought(cat()).0, 5 * N, "the fill is on record for the factory's owner");
        assert_eq!(l.get_buy_held(cat()), BuyHeld::default());
        assert!(actions().is_empty());
        call(me(), 20, 0);
        assert_eq!(l.on_add_bought(args(), Ok("p#7".into()), Ok(BuyOutcome { used: U128(5 * N), refunded: U128(0) })), AddBuyResult { lpt_id: "p#7".into(), used: U128(5 * N), refunded: U128(0) });
    }

    /// The floor refused the swap: the wNEAR is unwrapped and the NEAR goes back to the factory,
    /// step by step, and the result says what came back.
    #[test]
    fn a_refused_buy_is_unwrapped_and_returned() {
        let mut l = locker();
        call(factory(), 300, 5 * N);
        drop(l.add_buy(args(), buy()));
        hop(&mut l);
        call(me(), 100, 0);
        drop(l.on_bought(cat(), U128(5 * N), Ok(U128(0))));
        assert_eq!(l.get_bought(cat()).0, 0, "a refused buy bought nothing");
        let a = actions();
        assert_eq!((a[0].0.as_str(), a[0].1.as_str()), ("wrap.near", "near_withdraw"));
        assert!(a[0].2.contains(&format!(r#""amount":"{}""#, 5 * N)));
        assert_eq!(l.get_buy_held(cat()), BuyHeld { wnear: U128(5 * N), near: U128(0) });
        call(me(), 100, 0);
        drop(l.on_unwrapped(cat(), U128(0), U128(5 * N), Ok(())));
        let a = actions();
        assert_eq!((a[0].0.as_str(), a[0].1.as_str(), a[0].2.as_str()), ("f.near", "Transfer", (5 * N).to_string().as_str()));
        assert_eq!(l.get_buy_held(cat()), BuyHeld::default(), "on its way back: not held, so refund_buy cannot send it too");
        call(me(), 20, 0);
        assert_eq!(l.on_refunded(cat(), U128(0), U128(5 * N), Ok(())), BuyOutcome { used: U128(0), refunded: U128(5 * N) });
        assert_eq!(l.get_buy_held(cat()), BuyHeld::default());

        call(me(), 100, 0);
        drop(l.on_bought(cat(), U128(5 * N), Ok(U128(3 * N))));
        assert_eq!(l.get_buy_held(cat()), BuyHeld { wnear: U128(2 * N), near: U128(0) });
    }

    /// The first-buy record the factory settles from: every step writes what it knows, `settled` only at the end.
    #[test]
    fn the_first_buy_record_follows_every_step() {
        let mut l = locker();
        assert_eq!(l.get_first_buy(cat()), None, "never took NEAR");
        call(factory(), 300, 5 * N);
        drop(l.add_buy(args(), buy()));
        assert_eq!(l.get_first_buy(cat()), Some(FirstBuy { amount: U128(5 * N), ..Default::default() }));
        hop(&mut l);
        assert!(l.get_first_buy(cat()).unwrap().sent);

        call(me(), 100, 0);
        drop(l.on_bought(cat(), U128(5 * N), Ok(U128(3 * N))));
        let b = l.get_first_buy(cat()).unwrap();
        assert_eq!((b.answered, b.used.0, b.settled), (true, 3 * N, false), "the rest is still on its way");
        call(me(), 100, 0);
        drop(l.on_unwrapped(cat(), U128(3 * N), U128(2 * N), Ok(())));
        assert!(!l.get_first_buy(cat()).unwrap().settled);
        call(me(), 20, 0);
        drop(l.on_refunded(cat(), U128(3 * N), U128(2 * N), Ok(())));
        assert_eq!(l.get_first_buy(cat()), Some(FirstBuy { amount: U128(5 * N), sent: true, answered: true, used: U128(3 * N), returned: U128(2 * N), settled: true }));
        assert_eq!(l.get_bought(cat()).0, 3 * N);

        let mut l = locker();
        call(factory(), 300, 5 * N);
        drop(l.add_buy(args(), buy()));
        hop(&mut l);
        call(me(), 100, 0);
        drop(l.on_bought(cat(), U128(5 * N), Err(PromiseError::Failed)));
        call(me(), 20, 0);
        drop(l.on_refunded(cat(), U128(0), U128(5 * N), Err(PromiseError::Failed)));
        let b = l.get_first_buy(cat()).unwrap();
        assert_eq!((b.answered, b.used.0, b.returned.0, b.settled), (true, 0, 0, true));
        assert_eq!(l.get_buy_held(cat()).near.0, 5 * N);
    }

    /// Whatever fails on the way back stays held, and refund_buy sends it again.
    #[test]
    fn leftovers_stay_held_until_refund_buy_sends_them() {
        let mut l = locker();
        call(factory(), 300, 5 * N);
        drop(l.add_buy(args(), buy()));
        hop(&mut l);

        call(me(), 100, 0);
        drop(l.on_bought(cat(), U128(5 * N), Err(PromiseError::Failed)));
        assert_eq!(actions()[0].1, "Transfer");
        call(me(), 20, 0);
        assert_eq!(l.on_refunded(cat(), U128(0), U128(5 * N), Err(PromiseError::Failed)), BuyOutcome { used: U128(0), refunded: U128(0) });
        assert_eq!(l.get_buy_held(cat()), BuyHeld { wnear: U128(0), near: U128(5 * N) });

        call(factory(), 100, 0);
        drop(l.refund_buy(cat()));
        assert_eq!((actions()[0].1.as_str(), actions()[0].2.as_str()), ("Transfer", (5 * N).to_string().as_str()));
        assert_eq!(l.get_buy_held(cat()), BuyHeld::default(), "debited before it leaves");
        call(factory(), 100, 0);
        assert_eq!(value(l.refund_buy(cat())), BuyOutcome::default(), "nothing to send twice");

        call(me(), 20, 0);
        assert_eq!(l.on_refund_returned(cat(), U128(5 * N), Err(PromiseError::Failed)), BuyOutcome::default());
        assert_eq!(l.get_buy_held(cat()), BuyHeld { wnear: U128(0), near: U128(5 * N) });
        call(me(), 20, 0);
        assert_eq!(l.on_refund_returned(cat(), U128(5 * N), Ok(())), BuyOutcome { used: U128(0), refunded: U128(5 * N) });

        call(me(), 100, 0);
        drop(l.on_bought(cat(), U128(5 * N), Ok(U128(0))));
        call(me(), 20, 0);
        assert_eq!(value(l.on_unwrapped(cat(), U128(0), U128(5 * N), Err(PromiseError::Failed))), BuyOutcome { used: U128(0), refunded: U128(0) });
        assert_eq!(l.get_buy_held(cat()).wnear.0, 5 * N);
        call(factory(), 100, 0);
        drop(l.refund_buy(cat()));
        assert_eq!(actions()[0].1, "near_withdraw");
        assert_eq!(l.get_buy_held(cat()).wnear.0, 0, "debited before the unwrap");
        call(me(), 20, 0);
        assert_eq!(value(l.on_refund_unwrapped(cat(), U128(5 * N), Err(PromiseError::Failed))), BuyOutcome::default());
        assert_eq!(l.get_buy_held(cat()).wnear.0, 5 * N, "held again after a failed unwrap");
        call("mallory.near".parse().unwrap(), 100, 0);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(l.refund_buy(cat())))).is_err(), "factory only");
    }

    /// A hop that errored gives used 0 and the position still comes back.
    #[test]
    fn a_failed_buy_leg_never_hides_the_position() {
        let mut l = locker();
        call(me(), 20, 0);
        assert_eq!(l.on_add_bought(args(), Ok("p#7".into()), Err(PromiseError::Failed)), AddBuyResult { lpt_id: "p#7".into(), used: U128(0), refunded: U128(0) });
        assert_eq!(l.on_add_bought(args(), Err(PromiseError::Failed), Ok(BuyOutcome { used: U128(N), refunded: U128(0) })), AddBuyResult { lpt_id: "".into(), used: U128(N), refunded: U128(0) });
    }

    /// 10-01 audit: the buy is debited when the hop sends it. If `on_bought` never runs after a fill,
    /// nothing reads as held and `refund_buy` pays nothing; a batch that failed is held again.
    #[test]
    fn the_buy_is_debited_when_it_leaves() {
        let mut l = locker();
        call(factory(), 300, 5 * N);
        drop(l.add_buy(args(), buy()));
        assert_eq!(l.get_buy_held(cat()).near.0, 5 * N, "here until the hop wraps it");
        hop(&mut l);
        assert_eq!(l.get_buy_held(cat()), BuyHeld::default(), "gone with the wrap");

        call(factory(), 100, 0);
        assert_eq!(value(l.refund_buy(cat())), BuyOutcome::default());
        assert!(actions().is_empty());

        call(me(), 100, 0);
        drop(l.on_bought(cat(), U128(5 * N), Err(PromiseError::Failed)));
        assert_eq!(l.get_bought(cat()).0, 0, "no fill on record");
        assert_eq!((actions()[0].1.as_str(), actions()[0].2.as_str()), ("Transfer", (5 * N).to_string().as_str()));

        assert_eq!(l.get_buy_held(cat()), BuyHeld::default());
        call(factory(), 100, 0);
        assert_eq!(value(l.refund_buy(cat())), BuyOutcome::default());
        assert!(actions().is_empty());
        call(me(), 20, 0);
        assert_eq!(l.on_refunded(cat(), U128(0), U128(5 * N), Ok(())), BuyOutcome { used: U128(0), refunded: U128(5 * N) });
        assert_eq!(l.get_buy_held(cat()), BuyHeld::default());
    }

    /// 10-01 audit: a nearly dry account starts no launch (the factory parks it); the buy's own deposit
    /// is not counted as ours.
    #[test]
    fn a_dry_locker_refuses_to_add() {
        for with_buy in [false, true] {
            let mut l = locker();
            let mut c = VMContextBuilder::new();
            let dep = if with_buy { 5 * N } else { 0 };

            let stake = env::storage_byte_cost().as_yoctonear() * 1000;
            let mut a = args();
            a.token = format!("t{}.nearlytrade.near", with_buy as u8).parse().unwrap();
            a.pool_id = format!("{}|wrap.near|10000", a.token);
            let args = || a.clone();
            c.current_account_id(me()).predecessor_account_id(factory()).prepaid_gas(Gas::from_tgas(300))
                .storage_usage(1000).attached_deposit(NearToken::from_yoctonear(dep)).account_balance(NearToken::from_yoctonear(stake + 90 * N / 1000));
            testing_env!(c.build());
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if with_buy { drop(l.add_buy(args(), buy())) } else { drop(l.add(args())) }
            }));
            assert!(r.is_err(), "refused (buy: {with_buy})");
            assert!(l.get_position(args().pool_id).is_none());

            let mut c = VMContextBuilder::new();
            c.current_account_id(me()).predecessor_account_id(factory()).prepaid_gas(Gas::from_tgas(300))
                .storage_usage(1000).attached_deposit(NearToken::from_yoctonear(dep)).account_balance(NearToken::from_yoctonear(stake + MIN_FREE));
            testing_env!(c.build());
            if with_buy { drop(l.add_buy(args(), buy())) } else { drop(l.add(args())) }
            assert!(l.get_position(args().pool_id).is_some());
        }
    }

    /// 10-01 audit: add_buy's HotZap reserves the supply like add's, until the position is booked.
    #[test]
    fn add_buy_reserves_the_supply_until_the_position_is_booked() {
        let mut l = locker();
        call(factory(), 300, 5 * N);
        drop(l.add_buy(args(), buy()));
        assert_eq!(l.get_supply_reserved(cat()).0, SUPPLY);
        call(me(), 150, 0);
        let rows = vec![LiquidityRow { lpt_id: "cat.nearlytrade.near|wrap.near|10000#1".into(), pool_id: args().pool_id, left_point: 200, right_point: 500000 }];
        drop(l.on_listed(args(), 0, true, Ok(rows)));
        assert_eq!(l.get_supply_reserved(cat()).0, 0);
    }

    #[test]
    fn the_add_buy_budget() {
        assert_eq!(T_ADD_BUY_NEEDS, 229, "add_buy: of the 230 the launch transaction can attach");
        assert_eq!(tg(GAS_BUY_HOP), 120);
    }
}

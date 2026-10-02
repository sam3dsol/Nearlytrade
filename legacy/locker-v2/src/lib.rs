//! Liquidity locker — the account that OWNS every launch's DCL position. It has exactly two jobs:
//! place the supply as a single-sided range (`add`) and pull LP fees off a position (`claim`), forwarding
//! them to the factory. There is no method that removes liquidity, and after deployment the account's
//! access keys are deleted, so the code can never change: liquidity is provably locked while the
//! factory next door stays upgradable.
use near_sdk::json_types::U128;
use near_sdk::store::LookupMap;
use near_sdk::{env, near, require, AccountId, Gas, GasWeight, NearToken, PanicOnDefault, Promise, PromiseError, PromiseOrValue};

const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);
const NO_DEPOSIT: NearToken = NearToken::from_yoctonear(0);
const FT_STORAGE_REG: NearToken = NearToken::from_yoctonear(1_250_000_000_000_000_000_000);





const GAS_DEPOSIT: Gas = Gas::from_tgas(90);













const GAS_ADD_LIQ: Gas = Gas::from_tgas(62);
const GAS_CLAIM: Gas = Gas::from_tgas(90);
const GAS_FORWARD: Gas = Gas::from_tgas(20);

const GAS_CB: Gas = Gas::from_tgas(66);





const GAS_CB_SMALL: Gas = Gas::from_tgas(72);
/// `on_token_forwarded` reads one callback result and touches one map entry
const GAS_CB_TOKEN: Gas = Gas::from_tgas(10);




const fn tg(g: Gas) -> u64 { g.as_gas() / 1_000_000_000_000 }
/// what contracts/factory sends as GAS_LOCKER_ADD
/// 180 since the dev buy left the launch transaction. The whole class of failure here is a
/// venue quietly needing more gas than a number measured months ago, so the margin is the fix.
const T_FACTORY_BUDGET: u64 = 180;
/// add() = storage_deposit(5) + ft_transfer_call(GAS_DEPOSIT) + .then(GAS_CB), plus ~2.3 own
/// execution and the action-creation fees of the three promises
const T_ADD_NEEDS: u64 = 5 + tg(GAS_DEPOSIT) + tg(GAS_CB) + 4;
const _: () = assert!(T_ADD_NEEDS <= T_FACTORY_BUDGET, "locker.add no longer fits the factory's GAS_LOCKER_ADD");
/// our token forwards `prepaid - used(~3.2) - 15 - 5` into the exchange, which burns 21.3 there
const _: () = assert!(tg(GAS_DEPOSIT) >= 4 + 15 + 5 + 52, "GAS_DEPOSIT no longer forwards enough for dclv2 ft_on_transfer");
/// on_deposited burns ~1.6 of its own before attaching GAS_ADD_LIQ to add_liquidity
const _: () = assert!(tg(GAS_CB) >= tg(GAS_ADD_LIQ) + 4, "GAS_CB cannot attach GAS_ADD_LIQ");
/// worst case in on_claimed: an FT quote pair forwards two ft_transfers plus on_forwarded(8)
const _: () = assert!(tg(GAS_CB_SMALL) >= 2 * tg(GAS_FORWARD) + tg(GAS_CB_TOKEN) + 4, "GAS_CB_SMALL cannot forward BOTH sides and still book the token leg");
/// and the whole callback still has to fit under what the factory attaches to `claim`
const _: () = assert!(tg(GAS_CLAIM) + tg(GAS_CB_SMALL) + 5 <= 180, "claim no longer fits the factory's GAS_LOCKER_CLAIM");

#[near(serializers = [json])]
pub struct AddArgs {
    pub token: AccountId,
    pub pool_id: String,
    pub left_point: i32,
    pub right_point: i32,
    pub amount_x: U128,
    pub amount_y: U128,
}

/// Gas money this account keeps back so a forward can never leave it unable to pay for its own state.
const SAFETY_FLOOR: NearToken = NearToken::from_millinear(50);

/// Raw storage prefix for the FT-quote carry, per launch token. Kept outside the state struct so
/// adding it changed no layout and needed no migration. `Q` collides with neither map prefix.
const OWED_QUOTE_PREFIX: &[u8] = b"Q";

fn owed_quote_key(token: &AccountId) -> Vec<u8> {
    [OWED_QUOTE_PREFIX, token.as_bytes()].concat()
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Locker {
    factory: AccountId,
    dcl: AccountId,
    wnear: AccountId,
    /// Quote-side fees the exchange released but this account could NOT forward at the time.
    ///    `remove_liquidity` returns the fee amounts immediately, but DCL's actual payout (unwrap
    ///    through wrap.near, then a transfer) settles in LATER receipts — so at callback time the
    ///    money may not be here yet. Forwarding it out of our own balance is what failed on mainnet
    ///    2026-09-22 with `LackBalanceForState` once a claim (1.196 Ⓝ) exceeded our free balance,
    ///    stranding the fees with nobody credited. We now forward only what we can actually afford
    ///    and carry the remainder here, so the next claim pays it out. Nothing is ever fronted.
    /// Keyed PER LAUNCH TOKEN. It used to be one global number, so the carry was paid out by
    ///    whichever launch claimed next and booked to THAT launch: on 2026-09-22 NEARLY's 9.42 Ⓝ
    ///    was booked to NEARKAT, 70% of it landed in NEARKAT's creator bucket and the
    ///    protocol half came up ~1.95 Ⓝ short. Now a launch only ever books its own fees; the
    ///    balance is still shared, so one launch's payout may fund another's forward, but each
    ///    launch's cumulative booking equals exactly what the exchange released for it.
    owed_near: LookupMap<AccountId, U128>,
    /// sum of `owed_near`, so `get_owed` stays a single read
    owed_near_total: U128,
    /// The same race on the token side, per launch token. We cannot read our own FT balance
    ///    synchronously, so unlike the NEAR leg this cannot be clamped up front: the transfer is
    ///    attempted and the CALLBACK records whatever bounced. Before this, a failed `ft_transfer`
    ///    still had the factory booking the fee as received, which credited a creator tokens that
    ///    never left this account.
    owed_token: LookupMap<AccountId, U128>,
}

#[near]
impl Locker {
    #[init]
    pub fn new(factory: AccountId, dcl: AccountId, wnear: AccountId) -> Self {
        Self {
            factory, dcl, wnear,
            owed_near: LookupMap::new(b"n".to_vec()),
            owed_near_total: U128(0),
            owed_token: LookupMap::new(b"o".to_vec()),
        }
    }

    /// FT-quote fees released by the exchange that are still waiting here for this launch's token.
    pub fn get_owed_quote(&self, token: AccountId) -> U128 {
        U128(read_owed_quote(&token))
    }

    /// Token-side fees released by the exchange that are still waiting here for this launch's token.
    pub fn get_owed_token(&self, token: AccountId) -> U128 {
        self.owed_token.get(&token).copied().unwrap_or(U128(0))
    }

    /// Quote-side fees released by the exchange that are still waiting here to be forwarded, all launches.
    pub fn get_owed(&self) -> U128 { self.owed_near_total }

    /// The NEAR carry for one launch token.
    pub fn get_owed_near(&self, token: AccountId) -> U128 {
        self.owed_near.get(&token).copied().unwrap_or(U128(0))
    }

    /// What this account can part with right now without dropping under its own storage stake.
    fn spendable(&self) -> u128 {
        let stake = env::storage_byte_cost().as_yoctonear().saturating_mul(env::storage_usage() as u128);
        env::account_balance()
            .as_yoctonear()
            .saturating_sub(stake)
            .saturating_sub(SAFETY_FLOOR.as_yoctonear())
    }

    /// Deposit the tokens this account holds into the exchange and open the range position.
    /// Returns the exchange's `lpt_id` to the caller (the factory) through the promise chain.
    pub fn add(&mut self, args: AddArgs) -> Promise {
        self.assert_factory();
        let supply = if args.amount_x.0 > 0 { args.amount_x.0 } else { args.amount_y.0 };
        Promise::new(args.token.clone())
            .function_call(
                "storage_deposit".to_string(),
                format!(r#"{{"account_id":"{}","registration_only":true}}"#, self.dcl).into_bytes(),
                FT_STORAGE_REG,
                Gas::from_tgas(5),
            )
            .function_call_weight(
                "ft_transfer_call".to_string(),
                format!(r#"{{"receiver_id":"{}","amount":"{}","msg":"\"Deposit\""}}"#, self.dcl, supply).into_bytes(),
                ONE_YOCTO,
                GAS_DEPOSIT,
                GasWeight(1),
            )
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_CB).with_unused_gas_weight(0).on_deposited(args))
    }

    #[private]
    pub fn on_deposited(&mut self, args: AddArgs, #[callback_result] used: Result<U128, PromiseError>) -> Promise {
        let supply = if args.amount_x.0 > 0 { args.amount_x.0 } else { args.amount_y.0 };
        match used {
            Ok(u) if u.0 == supply => {}
            _ => env::panic_str("exchange did not accept the deposit"),
        }
        Promise::new(self.dcl.clone()).function_call(
            "add_liquidity".to_string(),
            format!(
                r#"{{"pool_id":{},"left_point":{},"right_point":{},"amount_x":"{}","amount_y":"{}","min_amount_x":"0","min_amount_y":"0"}}"#,
                near_sdk::serde_json::to_string(&args.pool_id).unwrap(), args.left_point, args.right_point, args.amount_x.0, args.amount_y.0
            )
            .into_bytes(),
            NO_DEPOSIT,
            GAS_ADD_LIQ,
        )
    }

    /// Pull accrued fees off a position (remove_liquidity with amount 0) and forward them to the factory.
    /// Returns the exchange's `[fee_x, fee_y]`.
    /// `quote` is the pair's other asset when it is an FT the exchange pays out as tokens. `None`
    /// means the wNEAR pair, whose quote side arrives here already unwrapped as native NEAR.
    pub fn claim(&mut self, lpt_id: String, token: AccountId, token_is_x: bool, quote: Option<AccountId>) -> Promise {
        self.assert_factory();
        Promise::new(self.dcl.clone())
            .function_call(
                "remove_liquidity".to_string(),
                format!(r#"{{"lpt_id":{},"amount":"0","min_amount_x":"0","min_amount_y":"0"}}"#, near_sdk::serde_json::to_string(&lpt_id).unwrap()).into_bytes(),
                NO_DEPOSIT,
                GAS_CLAIM,
            )
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_CB_SMALL).on_claimed(token, token_is_x, quote))
    }

    #[private]
    pub fn on_claimed(&mut self, token: AccountId, token_is_x: bool, quote: Option<AccountId>, #[callback_result] r: Result<Vec<U128>, PromiseError>) -> PromiseOrValue<Vec<U128>> {
        let v = r.unwrap_or_else(|_| env::panic_str("claim failed"));
        require!(v.len() == 2, "unexpected claim result");
        let (fee_tok, fee_quote) = if token_is_x { (v[0].0, v[1].0) } else { (v[1].0, v[0].0) };
        if let Some(q) = quote {
            return self.forward_ft_legs(token, token_is_x, q, fee_quote, fee_tok);
        }

        let mut p: Option<Promise> = None;




        let owed = self.get_owed_near(token.clone()).0;
        let (sent_quote, carry) = forward_split(fee_quote, owed, self.spendable());
        if carry == 0 { self.owed_near.remove(&token); } else { self.owed_near.insert(token.clone(), U128(carry)); }
        self.owed_near_total = U128(self.owed_near_total.0 - owed + carry);
        if carry > 0 {
            env::log_str(&format!(
                r#"EVENT_JSON:{{"standard":"nearpad","version":"2.0.0","event":"fee_forward_deferred","data":[{{"token":"{}","owed":"{}","sent":"{}"}}]}}"#,
                token, carry, sent_quote
            ));
        }
        if sent_quote > 0 {
            p = Some(Promise::new(self.factory.clone()).transfer(NearToken::from_yoctonear(sent_quote)));
        }

        let want_tok = fee_tok.saturating_add(self.get_owed_token(token.clone()).0);
        if want_tok == 0 {
            let booked: Vec<U128> = if token_is_x { vec![U128(0), U128(sent_quote)] } else { vec![U128(sent_quote), U128(0)] };
            return match p {
                Some(p) => PromiseOrValue::Promise(p.then(Self::ext(env::current_account_id()).with_static_gas(Gas::from_tgas(8)).on_forwarded(booked))),
                None => PromiseOrValue::Value(booked),
            };
        }



        let t = Promise::new(token.clone()).function_call(
            "ft_transfer".to_string(),
            format!(r#"{{"receiver_id":"{}","amount":"{}","memo":"lp fees"}}"#, self.factory, want_tok).into_bytes(),
            ONE_YOCTO,
            GAS_FORWARD,
        );
        let chain = match p { Some(q) => q.then(t), None => t };
        PromiseOrValue::Promise(chain.then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_CB_TOKEN)
                .on_token_forwarded(token, U128(want_tok), U128(sent_quote), token_is_x),
        ))
    }

    /// Book the token leg only if it actually landed; carry it otherwise.
    #[private]
    pub fn on_token_forwarded(
        &mut self,
        token: AccountId,
        amount: U128,
        quote_sent: U128,
        token_is_x: bool,
        #[callback_result] r: Result<(), PromiseError>,
    ) -> Vec<U128> {
        let (landed, carry) = book_token(r.is_ok(), amount.0);
        if carry == 0 {
            self.owed_token.remove(&token);
        } else {
            self.owed_token.insert(token.clone(), U128(carry));
            env::log_str(&format!(
                r#"EVENT_JSON:{{"standard":"nearpad","version":"2.0.0","event":"fee_forward_deferred","data":[{{"token":"{}","owed":"{}"}}]}}"#,
                token, carry
            ));
        }
        if token_is_x { vec![U128(landed), quote_sent] } else { vec![quote_sent, U128(landed)] }
    }

    /// FT-quote pair: book each leg only if its transfer landed; carry whatever bounced.
    /// The promise results arrive in the order the legs were joined: quote first (when `quote_amount`
    /// is non-zero), then the token (when `token_amount` is non-zero).
    #[private]
    pub fn on_ft_legs_forwarded(&mut self, token: AccountId, token_is_x: bool, quote_amount: U128, token_amount: U128) -> Vec<U128> {
        let mut i = 0u64;
        let mut landed = |amount: u128| -> bool {
            if amount == 0 { return true; }

            let ok = !matches!(env::promise_result_checked(i, 64), Err(PromiseError::Failed));
            i += 1;
            ok
        };
        let (q_booked, q_carry) = book_token(landed(quote_amount.0), quote_amount.0);
        let (t_booked, t_carry) = book_token(landed(token_amount.0), token_amount.0);
        write_owed_quote(&token, q_carry);
        if t_carry == 0 { self.owed_token.remove(&token); } else { self.owed_token.insert(token.clone(), U128(t_carry)); }
        if q_carry > 0 || t_carry > 0 {
            env::log_str(&format!(
                r#"EVENT_JSON:{{"standard":"nearpad","version":"2.0.0","event":"fee_forward_deferred","data":[{{"token":"{}","owed_quote":"{}","owed":"{}"}}]}}"#,
                token, q_carry, t_carry
            ));
        }
        if token_is_x { vec![U128(t_booked), U128(q_booked)] } else { vec![U128(q_booked), U128(t_booked)] }
    }

    #[private]
    pub fn on_forwarded(&mut self, amounts: Vec<U128>) -> Vec<U128> {
        amounts
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

    pub fn get_factory(&self) -> AccountId {
        self.factory.clone()
    }
}

impl Locker {
    fn assert_factory(&self) {
        require!(env::predecessor_account_id() == self.factory, "factory only");
    }

    /// An FT-quote pair used to return the quote amount as booked whether or not its
    ///    `ft_transfer` landed — the same defect the token leg had. Both legs now carry what bounced,
    ///    and the callback reads each transfer's own result, which is why they are joined, not chained.
    fn forward_ft_legs(&mut self, token: AccountId, token_is_x: bool, quote: AccountId, fee_quote: u128, fee_tok: u128) -> PromiseOrValue<Vec<U128>> {
        let want_q = fee_quote.saturating_add(read_owed_quote(&token));
        let want_tok = fee_tok.saturating_add(self.get_owed_token(token.clone()).0);
        let send = |asset: &AccountId, amount: u128| {
            Promise::new(asset.clone()).function_call(
                "ft_transfer".to_string(),
                format!(r#"{{"receiver_id":"{}","amount":"{}","memo":"lp fees"}}"#, self.factory, amount).into_bytes(),
                ONE_YOCTO,
                GAS_FORWARD,
            )
        };
        let mut legs: Option<Promise> = None;
        if want_q > 0 { legs = Some(send(&quote, want_q)); }
        if want_tok > 0 {
            let t = send(&token, want_tok);
            legs = Some(match legs { Some(l) => l.and(t), None => t });
        }
        match legs {
            None => PromiseOrValue::Value(vec![U128(0), U128(0)]),
            Some(l) => PromiseOrValue::Promise(l.then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_CB_TOKEN)
                    .on_ft_legs_forwarded(token, token_is_x, U128(want_q), U128(want_tok)),
            )),
        }
    }
}

fn read_owed_quote(token: &AccountId) -> u128 {
    env::storage_read(&owed_quote_key(token))
        .map(|b| u128::from_le_bytes(b.try_into().unwrap_or_else(|_| env::panic_str("owed quote"))))
        .unwrap_or(0)
}

fn write_owed_quote(token: &AccountId, amount: u128) {
    let k = owed_quote_key(token);
    if amount == 0 { env::storage_remove(&k); } else { env::storage_write(&k, &amount.to_le_bytes()); }
}

/// Decide how much of the quote side can leave now and how much has to wait.
///
/// Pure so it can be tested without a VM: `sent + carry` always equals everything owed, nothing is
/// invented, and we never promise more than `spendable`.
/// What the factory may book for the token leg, and what has to wait.
///
/// The factory credits creators from what this contract RETURNS, so returning a fee whose
/// `ft_transfer` bounced would credit someone tokens that never left here. Booked is what landed.
fn book_token(delivered: bool, amount: u128) -> (u128, u128) {
    if delivered { (amount, 0) } else { (0, amount) }
}

fn forward_split(fee: u128, owed: u128, spendable: u128) -> (u128, u128) {
    let want = fee.saturating_add(owed);
    let sent = want.min(spendable);
    (sent, want - sent)
}

#[cfg(test)]
mod tests {
    use super::forward_split;

    const N: u128 = 1_000_000_000_000_000_000_000_000;

    #[test]
    fn a_forward_never_exceeds_what_we_can_afford() {

        let (sent, carry) = forward_split(1_195_864 * (N / 1_000_000), 0, 213 * (N / 1000));
        assert_eq!(sent, 213 * (N / 1000), "must send exactly what is affordable, not what DCL reported");
        assert!(carry > 0, "the rest has to be carried, not silently dropped");
        assert_eq!(sent + carry, 1_195_864 * (N / 1_000_000), "nothing may be lost or invented");
    }

    #[test]
    fn the_carry_is_paid_out_by_later_claims() {
        let (sent1, carry1) = forward_split(N, 0, N / 4);
        assert_eq!(sent1, N / 4);

        let (sent2, carry2) = forward_split(N / 2, carry1, 10 * N);
        assert_eq!(carry2, 0, "with room available the debt clears");
        assert_eq!(sent1 + sent2, N + N / 2, "total forwarded equals total earned");
    }

    #[test]
    fn a_bounced_token_transfer_is_never_booked() {
        use super::book_token;

        assert_eq!(book_token(true, 3_395 * N), (3_395 * N, 0));

        assert_eq!(book_token(false, 3_395 * N), (0, 3_395 * N));

        for delivered in [true, false] {
            let (booked, carry) = book_token(delivered, 7 * N);
            assert_eq!(booked + carry, 7 * N, "nothing may be lost or invented");
        }
    }

    #[test]
    fn the_happy_path_is_unchanged() {
        let (sent, carry) = forward_split(N, 0, 10 * N);
        assert_eq!((sent, carry), (N, 0));
        assert_eq!(forward_split(0, 0, 10 * N), (0, 0));
    }

    /// An FT-quote pair books each leg only if its own transfer landed, and a bounce is retried by the
    /// next claim. Before this, a bounced quote transfer was still returned as booked.
    #[test]
    fn an_ft_quote_leg_is_only_booked_if_it_landed() {
        use super::*;
        use near_sdk::test_utils::VMContextBuilder;
        use near_sdk::{testing_env, PromiseResult, RuntimeFeesConfig};
        use std::collections::HashMap;

        let me: AccountId = "lock.near".parse().unwrap();
        let tok: AccountId = "cat.near".parse().unwrap();
        let usdc: AccountId = "usdc.near".parse().unwrap();
        let env_with = |results: Vec<PromiseResult>| {
            let mut c = VMContextBuilder::new();
            c.current_account_id(me.clone()).predecessor_account_id(me.clone()).storage_usage(1000);
            testing_env!(c.build(), near_sdk::test_vm_config(), RuntimeFeesConfig::test(), HashMap::default(), results);
        };
        let ok = || PromiseResult::Successful(vec![]);
        env_with(vec![]);
        let mut l = Locker::new("f.near".parse().unwrap(), "dcl.near".parse().unwrap(), "wrap.near".parse().unwrap());


        env_with(vec![ok(), ok()]);
        assert_eq!(l.on_ft_legs_forwarded(tok.clone(), true, U128(5), U128(7)), vec![U128(7), U128(5)]);
        assert_eq!((l.get_owed_quote(tok.clone()).0, l.get_owed_token(tok.clone()).0), (0, 0));


        env_with(vec![PromiseResult::Failed, ok()]);
        assert_eq!(l.on_ft_legs_forwarded(tok.clone(), true, U128(5), U128(7)), vec![U128(7), U128(0)]);
        assert_eq!(l.get_owed_quote(tok.clone()).0, 5);


        env_with(vec![]);
        let _ = l.forward_ft_legs(tok.clone(), true, usdc.clone(), 3, 0);
        env_with(vec![ok()]);
        assert_eq!(l.on_ft_legs_forwarded(tok.clone(), true, U128(8), U128(0)), vec![U128(0), U128(8)]);
        assert_eq!(l.get_owed_quote(tok.clone()).0, 0);


        env_with(vec![ok(), PromiseResult::Failed]);
        assert_eq!(l.on_ft_legs_forwarded(tok.clone(), false, U128(2), U128(9)), vec![U128(2), U128(0)]);
        assert_eq!(l.get_owed_token(tok.clone()).0, 9);
        assert_eq!(l.get_owed_quote(tok.clone()).0, 0);
    }

    /// Replays 2026-09-22: NEARLY claims 9.42 Ⓝ with no room to forward, then NEARKAT claims 3.85 Ⓝ
    /// once NEARLY's payout has landed. The old global carry booked NEARLY's money to NEARKAT.
    #[test]
    fn a_carry_is_only_ever_booked_to_its_own_launch() {
        use super::*;
        use near_sdk::test_utils::VMContextBuilder;
        use near_sdk::{testing_env, PromiseResult, RuntimeFeesConfig};
        use std::collections::HashMap;

        let me: AccountId = "lock.near".parse().unwrap();
        let nearly: AccountId = "nearly.near".parse().unwrap();
        let kat: AccountId = "nearkat.near".parse().unwrap();
        let claim = |l: &mut Locker, tok: &AccountId, fee: u128, balance: u128| -> u128 {
            let mut c = VMContextBuilder::new();
            c.current_account_id(me.clone()).predecessor_account_id(me.clone())
                .account_balance(NearToken::from_yoctonear(balance)).storage_usage(1000);
            let res = near_sdk::serde_json::to_vec(&vec![U128(0), U128(fee)]).unwrap();
            testing_env!(c.build(), near_sdk::test_vm_config(), RuntimeFeesConfig::test(), HashMap::default(),
                vec![PromiseResult::Successful(res)]);
            let before = l.get_owed_near(tok.clone()).0;
            let _ = l.on_claimed(tok.clone(), true, None, Ok(vec![U128(0), U128(fee)]));
            fee + before - l.get_owed_near(tok.clone()).0
        };
        let stake = 1000 * env::storage_byte_cost().as_yoctonear();
        let floor = SAFETY_FLOOR.as_yoctonear();
        let mut l = Locker::new("f.near".parse().unwrap(), "dcl.near".parse().unwrap(), "wrap.near".parse().unwrap());

        let f_nearly = 9_416 * (N / 1000);
        let f_kat = 3_848 * (N / 1000);

        let b1 = claim(&mut l, &nearly, f_nearly, stake + floor + 4_256 * (N / 10_000));

        let b2 = claim(&mut l, &kat, f_kat, stake + floor + f_nearly);
        assert!(b2 <= f_kat, "NEARKAT booked {} but only earned {}", b2, f_kat);
        assert_eq!(b2, f_kat);
        assert_eq!(l.get_owed_near(nearly.clone()).0 + b1, f_nearly, "NEARLY's remainder stays NEARLY's");
        assert_eq!(l.get_owed().0, f_nearly - b1);

        let b3 = claim(&mut l, &nearly, 0, stake + floor + 100 * N);
        assert_eq!(b1 + b3, f_nearly);
        assert_eq!(l.get_owed().0, 0);
    }
}

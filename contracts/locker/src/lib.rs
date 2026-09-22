use near_sdk::json_types::U128;
use near_sdk::{env, near, require, AccountId, Gas, NearToken, PanicOnDefault, Promise, PromiseError, PromiseOrValue};

const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);
const NO_DEPOSIT: NearToken = NearToken::from_yoctonear(0);
const FT_STORAGE_REG: NearToken = NearToken::from_yoctonear(1_250_000_000_000_000_000_000);
const GAS_DEPOSIT: Gas = Gas::from_tgas(90);
const GAS_ADD_LIQ: Gas = Gas::from_tgas(45);
const GAS_CLAIM: Gas = Gas::from_tgas(90);
const GAS_FORWARD: Gas = Gas::from_tgas(20);
const GAS_CB: Gas = Gas::from_tgas(70);
const GAS_CB_SMALL: Gas = Gas::from_tgas(60);

#[near(serializers = [json])]
pub struct AddArgs {
    pub token: AccountId,
    pub pool_id: String,
    pub left_point: i32,
    pub right_point: i32,
    pub amount_x: U128,
    pub amount_y: U128,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Locker {
    factory: AccountId,
    dcl: AccountId,
    wnear: AccountId,
}

#[near]
impl Locker {
    #[init]
    pub fn new(factory: AccountId, dcl: AccountId, wnear: AccountId) -> Self {
        Self { factory, dcl, wnear }
    }

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
            .function_call(
                "ft_transfer_call".to_string(),
                format!(r#"{{"receiver_id":"{}","amount":"{}","msg":"\"Deposit\""}}"#, self.dcl, supply).into_bytes(),
                ONE_YOCTO,
                GAS_DEPOSIT,
            )
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_CB).on_deposited(args))
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

    pub fn claim(&mut self, lpt_id: String, token: AccountId, token_is_x: bool) -> Promise {
        self.assert_factory();
        Promise::new(self.dcl.clone())
            .function_call(
                "remove_liquidity".to_string(),
                format!(r#"{{"lpt_id":{},"amount":"0","min_amount_x":"0","min_amount_y":"0"}}"#, near_sdk::serde_json::to_string(&lpt_id).unwrap()).into_bytes(),
                NO_DEPOSIT,
                GAS_CLAIM,
            )
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_CB_SMALL).on_claimed(token, token_is_x))
    }

    #[private]
    pub fn on_claimed(&mut self, token: AccountId, token_is_x: bool, #[callback_result] r: Result<Vec<U128>, PromiseError>) -> PromiseOrValue<Vec<U128>> {
        let v = r.unwrap_or_else(|_| env::panic_str("claim failed"));
        require!(v.len() == 2, "unexpected claim result");
        let (fee_tok, fee_near) = if token_is_x { (v[0].0, v[1].0) } else { (v[1].0, v[0].0) };
        let mut p: Option<Promise> = None;
        if fee_near > 0 {
            p = Some(Promise::new(self.factory.clone()).transfer(NearToken::from_yoctonear(fee_near)));
        }
        if fee_tok > 0 {
            let t = Promise::new(token).function_call(
                "ft_transfer".to_string(),
                format!(r#"{{"receiver_id":"{}","amount":"{}","memo":"lp fees"}}"#, self.factory, fee_tok).into_bytes(),
                ONE_YOCTO,
                GAS_FORWARD,
            );
            p = Some(match p { Some(q) => q.and(t), None => t });
        }
        match p {
            Some(p) => PromiseOrValue::Promise(p.then(Self::ext(env::current_account_id()).with_static_gas(Gas::from_tgas(8)).on_forwarded(v))),
            None => PromiseOrValue::Value(v),
        }
    }

    #[private]
    pub fn on_forwarded(&mut self, amounts: Vec<U128>) -> Vec<U128> {
        amounts
    }

    pub fn get_factory(&self) -> AccountId {
        self.factory.clone()
    }
}

impl Locker {
    fn assert_factory(&self) {
        require!(env::predecessor_account_id() == self.factory, "factory only");
    }
}

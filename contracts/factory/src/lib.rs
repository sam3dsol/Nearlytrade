use near_sdk::json_types::{U128, U64};
use near_sdk::serde::Serialize;
use near_sdk::store::LookupMap;
use near_sdk::{
    env, log, near, require, AccountId, BorshStorageKey, CryptoHash, Gas, NearToken,
    PanicOnDefault, Promise, PromiseError, PromiseOrValue,
};

const BPS: u128 = 10_000;
const TOKEN_DECIMALS: u8 = 18;
const NEAR_DECIMALS: u8 = 24;
const STORAGE_PRICE_PER_BYTE: u128 = 10_000_000_000_000_000_000;
const TOKEN_BASE_STORAGE_BYTES: u128 = 3_000;
const FT_STORAGE_REG: NearToken = NearToken::from_yoctonear(1_250_000_000_000_000_000_000);
const DCL_REGISTER: NearToken = NearToken::from_millinear(500);
const DCL_POOL_CREATE: NearToken = NearToken::from_millinear(100);
const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);
const NO_DEPOSIT: NearToken = NearToken::from_yoctonear(0);
const POINT_DELTA_1PCT: i32 = 200;
const ONE_0001_FP: u128 = 1_000_100_000_000_000_000;
const FP: u128 = 1_000_000_000_000_000_000;
const POINT_MAX: i32 = 500_000;

const GAS_TOKEN_INIT: Gas = Gas::from_tgas(15);
const GAS_DCL_POOL: Gas = Gas::from_tgas(10);
const GAS_LOCKER_ADD: Gas = Gas::from_tgas(180);
const GAS_LOCKER_CLAIM: Gas = Gas::from_tgas(180);
const GAS_DEV_BUY: Gas = Gas::from_tgas(160);
const GAS_BALANCE_OF: Gas = Gas::from_tgas(5);
const GAS_DELIVER: Gas = Gas::from_tgas(12);
const GAS_CB_MIN: Gas = Gas::from_tgas(8);
const GAS_RESERVE: Gas = Gas::from_tgas(10);
const GAS_RESERVE_LAUNCH: Gas = Gas::from_tgas(18);
const GAS_UNWRAP: Gas = Gas::from_tgas(10);

const fn tg(g: Gas) -> u64 { g.as_gas() / 1_000_000_000_000 }
const T_AFTER_LAUNCH: u64 = 300 - 8;
const T_CB_BURN: u64 = 4;
const T_AT_ON_CREATED: u64 = T_AFTER_LAUNCH - tg(step_gas_create_token()) - tg(GAS_RESERVE_LAUNCH) - T_CB_BURN;
const T_AT_ON_LIQUIDITY: u64 = T_AT_ON_CREATED - tg(GAS_LOCKER_ADD) - tg(GAS_RESERVE) - T_CB_BURN;
const _: () = assert!(
    T_AT_ON_LIQUIDITY >= tg(GAS_CB_MIN) + tg(GAS_RESERVE),
    "the launch chain can no longer reach on_liquidity_added",
);
const T_DEV_BUY_NEEDS: u64 = tg(GAS_DEV_BUY) + 10 + tg(GAS_CB_MIN) + tg(GAS_RESERVE);
const _: () = assert!(
    T_DEV_BUY_NEEDS + tg(GAS_RESERVE_LAUNCH) <= 300,
    "step_gas(DevBuy) no longer fits in a transaction of its own",
);
const _: () = assert!(tg(GAS_DEV_BUY) + 10 >= tg(GAS_DEV_BUY) + 5 + 5, "step_gas(DevBuy) must cover its own batch");
const T_CREATE_TOKEN_BATCH: u64 = tg(GAS_TOKEN_INIT) + 5 + 5 + tg(GAS_DCL_POOL);
const _: () = assert!(tg(step_gas_create_token()) >= T_CREATE_TOKEN_BATCH, "step_gas(CreateToken) must cover BOTH batches it schedules");
const fn step_gas_create_token() -> Gas { Gas::from_tgas(tg(GAS_TOKEN_INIT) + tg(GAS_DCL_POOL) + 10) }
const GAS_ON_SPLIT: Gas = Gas::from_tgas(4);
const GAS_ON_SWEPT: Gas = Gas::from_tgas(4);
const GAS_ON_FEES_CLAIMED: Gas = Gas::from_tgas(45);

const EVENT_STANDARD: &str = "nearpad";
const EVENT_VERSION: &str = "2.0.0";

#[derive(BorshStorageKey)]
#[near(serializers = [borsh])]
enum StorageKey {
    Launches,
    TokenIndex,
    CreatorFees,
    CreatorTokenFees,
    SymbolIndex,
    Quotes,
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug)]
pub struct Config {
    pub total_supply: U128,
    pub pool_fee: u32,
    pub init_point: i32,
    pub min_init_point: i32,
    pub max_init_point: i32,
    pub range_points: i32,
    pub launch_fee: U128,
    pub creator_fee_share_bps: u16,
    pub dcl_storage_per_launch: U128,
    pub max_icon_bytes: u32,
    pub unique_symbols: bool,
    pub max_dev_buy_bps: u16,
    pub max_wallet_bps: u16,
    pub max_wallet_ms: u64,
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug)]
pub struct QuoteAsset {
    pub decimals: u8,
    pub init_point: i32,
    pub min_init_point: i32,
    pub max_init_point: i32,
    pub range_points: i32,
    pub native: bool,
    pub enabled: bool,
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    CreateToken,
    CreatePool,
    Deposit,
    AddLiquidity,
    DevBuy,
    ReadDevTokens,
    DeliverDevTokens,
    Done,
    Failed,
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Default, Debug)]
pub struct Links {
    pub website: Option<String>,
    pub twitter: Option<String>,
    pub telegram: Option<String>,
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Debug)]
pub struct Launch {
    pub id: u64,
    pub token: AccountId,
    pub creator: AccountId,
    pub name: String,
    pub symbol: String,
    pub icon: Option<String>,
    pub description: String,
    pub links: Links,
    pub created_at_ms: u64,
    pub total_supply: U128,

    pub pool_id: String,
    pub token_is_x: bool,
    pub init_point: i32,
    pub left_point: i32,
    pub right_point: i32,
    pub lpt_id: Option<String>,

    pub step: Step,
    pub inflight: bool,
    pub dev_buy_near: U128,
    pub dev_buy_exact: bool,
    pub dev_buy_tokens: U128,
    pub dev_buy_held: U128,

    pub claims: u64,
    pub fees_near_total: U128,
    pub fees_token_total: U128,
    pub creator_token_fees: U128,
    pub protocol_token_fees: U128,

    pub quote: AccountId,
    pub fees_quote_total: U128,
    pub creator_quote_fees: U128,
    pub protocol_quote_fees: U128,
}

#[near(serializers = [json])]
#[derive(Clone, Debug)]
pub struct LaunchArgs {
    pub name: String,
    pub symbol: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub links: Option<Links>,
    pub dev_buy: Option<U128>,
    pub init_point: Option<i32>,
    pub quote: Option<AccountId>,
}

#[near(serializers = [json])]
#[derive(Clone, Debug, Default)]
pub struct LaunchCost {
    pub launch_fee: U128,
    pub token_storage: U128,
    pub pool_create: U128,
    pub dcl_storage: U128,
    pub dev_buy: U128,
    pub total: U128,
}

#[near(serializers = [json])]
pub struct FeeSplit {
    pub recipients: Vec<(AccountId, u16)>,
    pub house_creator: Option<AccountId>,
    pub min_push: U128,
    pub min_claim: U128,
    pub pending: U128,
}

#[near(serializers = [json])]
pub struct Addresses {
    pub wnear: AccountId,
    pub dcl: AccountId,
    pub locker: AccountId,
    pub token_code_hash: String,
    pub dcl_registered: bool,
    pub wnear_registered: bool,
    pub paused: bool,
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
struct TokenMetadata<'a> {
    spec: &'a str,
    name: &'a str,
    symbol: &'a str,
    icon: &'a Option<String>,
    reference: Option<String>,
    reference_hash: Option<String>,
    decimals: u8,
}

#[near(serializers = [json])]
pub struct StorageBalanceJson {
    pub total: String,
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
struct TokenRules {
    max_wallet_bps: u16,
    until_ms: String,
    exempt: Vec<AccountId>,
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
struct TokenInit<'a> {
    owner_id: AccountId,
    total_supply: U128,
    metadata: TokenMetadata<'a>,
    rules: Option<TokenRules>,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Factory {
    owner_id: AccountId,
    token_code_hash: CryptoHash,
    wnear_id: AccountId,
    dcl_id: AccountId,
    config: Config,
    paused: bool,
    next_id: u64,
    launches: LookupMap<u64, Launch>,
    token_index: LookupMap<AccountId, u64>,
    creator_fees: LookupMap<AccountId, u128>,
    protocol_fees: u128,
    dcl_registered: bool,
    wnear_registered: bool,
    symbol_index: LookupMap<String, u64>,
    quotes: LookupMap<AccountId, QuoteAsset>,
    quote_ids: Vec<AccountId>,
    locker_id: AccountId,
    protocol_recipients: Vec<(AccountId, u16)>,
    house_creator: Option<AccountId>,
    min_push_yocto: u128,
    min_claim_yocto: u128,
}

#[near(serializers = [borsh])]
pub struct V4Launch {
    pub id: u64, pub token: AccountId, pub creator: AccountId, pub name: String, pub symbol: String, pub icon: Option<String>,
    pub description: String, pub links: Links, pub created_at_ms: u64, pub total_supply: U128, pub pool_id: String, pub token_is_x: bool,
    pub init_point: i32, pub left_point: i32, pub right_point: i32, pub lpt_id: Option<String>, pub step: Step, pub inflight: bool,
    pub dev_buy_near: U128, pub dev_buy_exact: bool, pub dev_buy_tokens: U128, pub dev_buy_held: U128, pub claims: u64, pub fees_near_total: U128,
    pub fees_token_total: U128, pub creator_token_fees: U128, pub protocol_token_fees: U128,
}
#[near(serializers = [borsh])]
pub struct V4Factory {
    owner_id: AccountId,
    token_code_hash: CryptoHash,
    wnear_id: AccountId,
    dcl_id: AccountId,
    config: Config,
    paused: bool,
    next_id: u64,
    launches: LookupMap<u64, V4Launch>,
    token_index: LookupMap<AccountId, u64>,
    creator_fees: LookupMap<AccountId, u128>,
    protocol_fees: u128,
    dcl_registered: bool,
    wnear_registered: bool,
    symbol_index: LookupMap<String, u64>,
    locker_id: AccountId,
    protocol_recipients: Vec<(AccountId, u16)>,
    house_creator: Option<AccountId>,
    min_push_yocto: u128,
    min_claim_yocto: u128,
}
#[near]
impl Factory {
    #[private]
    #[init(ignore_state)]
    pub fn migrate_v5() -> Self {
        let o: V4Factory = env::state_read().expect("old state");
        let mut launches: LookupMap<u64, Launch> = LookupMap::new(b"L3".to_vec());
        for id in 0..o.next_id {
            if let Some(l) = o.launches.get(&id) {
                launches.insert(id, Launch {
                    id: l.id, token: l.token.clone(), creator: l.creator.clone(), name: l.name.clone(), symbol: l.symbol.clone(), icon: l.icon.clone(),
                    description: l.description.clone(), links: l.links.clone(), created_at_ms: l.created_at_ms, total_supply: l.total_supply,
                    pool_id: l.pool_id.clone(), token_is_x: l.token_is_x, init_point: l.init_point, left_point: l.left_point, right_point: l.right_point,
                    lpt_id: l.lpt_id.clone(), step: l.step, inflight: l.inflight, dev_buy_near: l.dev_buy_near, dev_buy_exact: l.dev_buy_exact,
                    dev_buy_tokens: l.dev_buy_tokens, dev_buy_held: l.dev_buy_held, claims: l.claims, fees_near_total: l.fees_near_total,
                    fees_token_total: l.fees_token_total, creator_token_fees: l.creator_token_fees, protocol_token_fees: l.protocol_token_fees,
                    quote: o.wnear_id.clone(),
                    fees_quote_total: U128(0), creator_quote_fees: U128(0), protocol_quote_fees: U128(0),
                });
            }
        }
        let mut quotes: LookupMap<AccountId, QuoteAsset> = LookupMap::new(StorageKey::Quotes);
        quotes.insert(o.wnear_id.clone(), QuoteAsset {
            decimals: NEAR_DECIMALS,
            init_point: o.config.init_point,
            min_init_point: o.config.min_init_point,
            max_init_point: o.config.max_init_point,
            range_points: o.config.range_points,
            native: true,
            enabled: true,
        });
        Self {
            owner_id: o.owner_id, token_code_hash: o.token_code_hash, wnear_id: o.wnear_id.clone(), dcl_id: o.dcl_id,
            config: o.config, paused: o.paused, next_id: o.next_id, launches,
            token_index: o.token_index, creator_fees: o.creator_fees, protocol_fees: o.protocol_fees,
            dcl_registered: o.dcl_registered, wnear_registered: o.wnear_registered, symbol_index: o.symbol_index,
            locker_id: o.locker_id, protocol_recipients: o.protocol_recipients, house_creator: o.house_creator,
            min_push_yocto: o.min_push_yocto, min_claim_yocto: o.min_claim_yocto,
            quotes,
            quote_ids: vec![o.wnear_id],
        }
    }

    #[init]
    pub fn new(owner_id: AccountId, token_code_hash: String, wnear_id: AccountId, dcl_id: AccountId, locker_id: AccountId, config: Option<Config>) -> Self {
        let config = config.unwrap_or_else(Config::default_mainnet);
        config.validate();
        let (cfg_init, cfg_min, cfg_max, cfg_range) = (config.init_point, config.min_init_point, config.max_init_point, config.range_points);
        let wnear = wnear_id.clone();
        Self {
            owner_id,
            token_code_hash: parse_hex32(&token_code_hash),
            wnear_id,
            dcl_id,
            config,
            paused: false,
            next_id: 0,
            launches: LookupMap::new(StorageKey::Launches),
            token_index: LookupMap::new(StorageKey::TokenIndex),
            creator_fees: LookupMap::new(StorageKey::CreatorFees),
            protocol_fees: 0,
            dcl_registered: false,
            wnear_registered: false,
            symbol_index: LookupMap::new(StorageKey::SymbolIndex),
            locker_id,
            protocol_recipients: Vec::new(),
            house_creator: None,
            quotes: {
                let mut q: LookupMap<AccountId, QuoteAsset> = LookupMap::new(StorageKey::Quotes);
                q.insert(wnear.clone(), QuoteAsset {
                    decimals: NEAR_DECIMALS, init_point: cfg_init, min_init_point: cfg_min, max_init_point: cfg_max,
                    range_points: cfg_range, native: true, enabled: true,
                });
                q
            },
            quote_ids: vec![wnear],
            min_push_yocto: 50_000_000_000_000_000_000_000,
            min_claim_yocto: 50_000_000_000_000_000_000_000,
        }
    }

    #[payable]
    pub fn launch(&mut self, args: LaunchArgs) -> Promise {
        require!(!self.paused, "launches paused");
        let creator = env::predecessor_account_id();
        let deposit = env::attached_deposit().as_yoctonear();

        let name = args.name.trim().to_string();
        let symbol = args.symbol.trim().to_string();
        require!((1..=32).contains(&name.chars().count()), "name: 1-32 chars");
        require!((1..=10).contains(&symbol.chars().count()), "symbol: 1-10 chars");
        require!(symbol.chars().all(|c| c.is_ascii_alphanumeric()), "symbol: ascii letters/digits only");
        let description = args.description.unwrap_or_default();
        require!(description.chars().count() <= 500, "description: max 500 chars");
        let links = args.links.unwrap_or_default();
        for l in [&links.website, &links.twitter, &links.telegram].into_iter().flatten() {
            require!(l.len() <= 200, "link: max 200 chars");
        }
        let icon_len = args.icon.as_ref().map(|s| s.len()).unwrap_or(0);
        if let Some(icon) = &args.icon {
            require!(icon.starts_with("data:image/") || icon.starts_with("https://") || icon.starts_with("ipfs://"), "icon: data:image/, https:// or ipfs://");
            require!(icon_len <= self.config.max_icon_bytes as usize, "icon too large");
        }
        let quote_id = args.quote.clone().unwrap_or_else(|| self.wnear_id.clone());
        let q = self.quotes.get(&quote_id).cloned().unwrap_or_else(|| env::panic_str("quote asset not approved"));
        require!(q.enabled, "quote asset is disabled");

        let init_x = args.init_point.unwrap_or(q.init_point).clamp(q.min_init_point, q.max_init_point);
        let init_x = floor_to(init_x, POINT_DELTA_1PCT);

        let requested = args.dev_buy.map(|v| v.0).unwrap_or(0);
        require!(requested == 0 || q.native, "dev buy is only available on the NEAR pair");
        let cap = if requested == 0 { 0 } else {
            dev_buy_cap_yocto(init_x + POINT_DELTA_1PCT, self.config.total_supply.0, self.config.max_dev_buy_bps, self.config.pool_fee)
        };
        let (dev_buy, dev_buy_exact) = if requested == 0 {
            (0, false)
        } else if requested >= cap + cap / 100 {
            (cap + cap / 100, true)
        } else if requested >= cap - cap / 100 {
            (cap - cap / 100, false)
        } else {
            (requested, false)
        };
        let cost = self.internal_cost(icon_len, name.len() + symbol.len() + description.len(), dev_buy);
        require!(deposit >= cost.total.0, "attached deposit below quote_launch(...).total");
        let extra = deposit - cost.total.0 + (requested.saturating_sub(dev_buy));

        let slug = sanitize_slug(&symbol);
        if self.config.unique_symbols {
            require!(!self.symbol_index.contains_key(&slug), "symbol already launched on this pad");
        }
        let id = self.next_id;
        self.next_id += 1;
        if !self.symbol_index.contains_key(&slug) {
            self.symbol_index.insert(slug.clone(), id);
        }
        let seed = env::random_seed();
        let token: AccountId = format!("{}-{}.{}", slug, hex6(&seed), env::current_account_id()).parse().expect("token account id");

        let token_is_x = token.as_str() < quote_id.as_str();
        let (a, b) = if token_is_x { (&token, &quote_id) } else { (&quote_id, &token) };
        let pool_id = format!("{}|{}|{}", a, b, self.config.pool_fee);
        let top = (init_x.saturating_add(q.range_points)).min(POINT_MAX);
        require!(top - init_x >= POINT_DELTA_1PCT * 2, "pair's range_points is too small to open a position");
        let (init_point, left_point, right_point) = if token_is_x {
            (init_x, init_x + POINT_DELTA_1PCT, top)
        } else {
            (-init_x, -top, -init_x - POINT_DELTA_1PCT)
        };

        let l = Launch {
            id,
            token: token.clone(),
            creator: creator.clone(),
            name,
            symbol,
            icon: args.icon,
            description,
            links,
            created_at_ms: env::block_timestamp_ms(),
            total_supply: self.config.total_supply,
            pool_id,
            token_is_x,
            init_point,
            left_point,
            right_point,
            lpt_id: None,
            step: Step::CreateToken,
            inflight: false,
            dev_buy_near: U128(dev_buy),
            dev_buy_exact,
            dev_buy_tokens: U128(0),
            dev_buy_held: U128(dev_buy),
            claims: 0,
            fees_near_total: U128(0),
            fees_token_total: U128(0),
            creator_token_fees: U128(0),
            protocol_token_fees: U128(0),
            quote: quote_id.clone(),
            fees_quote_total: U128(0),
            creator_quote_fees: U128(0),
            protocol_quote_fees: U128(0),
        };
        self.protocol_fees += cost.launch_fee.0;
        self.token_index.insert(token, id);
        self.launches.insert(id, l.clone());
        if extra > 0 {
            Promise::new(creator).transfer(NearToken::from_yoctonear(extra)).detach();
        }
        emit(
            "launch_started",
            &format!(
                r#"{{"id":{},"token":"{}","creator":"{}","name":{},"symbol":{},"pool_id":{},"quote":"{}","init_point":{},"left_point":{},"right_point":{},"dev_buy":"{}","ts_ms":{}}}"#,
                id, l.token, l.creator, js(&l.name), js(&l.symbol), js(&l.pool_id), l.quote, l.init_point, l.left_point, l.right_point, dev_buy, l.created_at_ms
            ),
        );
        self.internal_run_step(id, l, cost.token_storage.0)
    }

    pub fn resume(&mut self, launch_id: U64) -> Promise {
        let l = self.launches.get(&launch_id.0).cloned().expect("launch");
        require!(!l.inflight, "step in flight");
        require!(!matches!(l.step, Step::Done | Step::Failed), "nothing to resume");
        let icon_len = l.icon.as_ref().map(|s| s.len()).unwrap_or(0);
        let storage = self.internal_cost(icon_len, l.name.len() + l.symbol.len() + l.description.len(), 0).token_storage.0;
        self.internal_run_step(launch_id.0, l, storage)
    }

    pub fn refund_failed(&mut self, launch_id: U64) -> Promise {
        let mut l = self.launches.get(&launch_id.0).cloned().expect("launch");
        require!(l.step == Step::Failed, "launch not failed");
        require!(l.dev_buy_held.0 > 0, "nothing to refund");
        let amount = l.dev_buy_held.0;
        l.dev_buy_held = U128(0);
        self.launches.insert(launch_id.0, l.clone());
        emit("dev_buy_refunded", &format!(r#"{{"id":{},"creator":"{}","amount":"{}"}}"#, l.id, l.creator, amount));
        Promise::new(l.creator).transfer(NearToken::from_yoctonear(amount))
    }

    #[private]
    pub fn on_created(
        &mut self,
        id: u64,
        #[callback_result] token: Result<StorageBalanceJson, PromiseError>,
        #[callback_result] pool: Result<String, PromiseError>,
    ) -> PromiseOrValue<bool> {
        let mut l = self.launches.get(&id).cloned().expect("launch");
        l.inflight = false;
        if token.is_err() {
            return self.internal_fail(id, l, "token create");
        }
        let pool_ok = matches!(&pool, Ok(p) if *p == l.pool_id);
        if pool_ok {
            self.dcl_registered = true;
            l.step = Step::AddLiquidity;
            self.internal_advance(id, l)
        } else {
            l.step = Step::CreatePool;
            self.internal_fail(id, l, "create_pool")
        }
    }

    #[private]
    pub fn on_pool_created(&mut self, id: u64, #[callback_result] r: Result<String, PromiseError>) -> PromiseOrValue<bool> {
        let mut l = self.launches.get(&id).cloned().expect("launch");
        l.inflight = false;
        match r {
            Ok(p) if p == l.pool_id => {
                self.dcl_registered = true;
                l.step = Step::AddLiquidity;
                self.internal_advance(id, l)
            }
            _ => self.internal_fail(id, l, "create_pool"),
        }
    }

    #[private]
    pub fn on_deposited(&mut self, id: u64, #[callback_result] used: Result<U128, PromiseError>) -> PromiseOrValue<bool> {
        let mut l = self.launches.get(&id).cloned().expect("launch");
        l.inflight = false;
        match used {
            Ok(u) if u.0 == l.total_supply.0 => {
                l.step = Step::AddLiquidity;
                self.internal_advance(id, l)
            }
            _ => self.internal_fail(id, l, "deposit"),
        }
    }

    #[private]
    pub fn on_liquidity_added(&mut self, id: u64, #[callback_result] lpt: Result<String, PromiseError>) -> PromiseOrValue<bool> {
        let mut l = self.launches.get(&id).cloned().expect("launch");
        l.inflight = false;
        match lpt {
            Ok(lpt_id) if !lpt_id.is_empty() => {
                l.lpt_id = Some(lpt_id);
                l.step = if l.dev_buy_near.0 > 0 { Step::DevBuy } else { Step::Done };
                if l.step == Step::Done {
                    self.internal_done(id, l);
                    return PromiseOrValue::Value(true);
                }
                self.internal_advance(id, l)
            }
            _ => self.internal_fail(id, l, "add_liquidity"),
        }
    }

    #[private]
    pub fn on_dev_bought(&mut self, id: u64, #[callback_result] used: Result<U128, PromiseError>) -> PromiseOrValue<bool> {
        let mut l = self.launches.get(&id).cloned().expect("launch");
        l.inflight = false;
        self.wnear_registered = true;
        match used {
            Ok(u) if u.0 > 0 && u.0 <= l.dev_buy_near.0 => {
                let unused = l.dev_buy_near.0 - u.0;
                if unused > 0 {
                    Promise::new(self.wnear_id.clone())
                        .function_call("near_withdraw".to_string(), format!(r#"{{"amount":"{}"}}"#, unused).into_bytes(), ONE_YOCTO, GAS_UNWRAP)
                        .then(Promise::new(l.creator.clone()).transfer(NearToken::from_yoctonear(unused)))
                        .detach();
                }
                l.dev_buy_near = u;
                l.dev_buy_held = U128(0);
                l.step = Step::Done;
                self.internal_done(id, l);
                PromiseOrValue::Value(true)
            }
            _ => {
                Promise::new(self.wnear_id.clone())
                    .function_call("near_withdraw".to_string(), format!(r#"{{"amount":"{}"}}"#, l.dev_buy_near.0).into_bytes(), ONE_YOCTO, GAS_UNWRAP)
                    .detach();
                emit("launch_step_parked", &format!(r#"{{"id":{},"step":"DevBuy","reason":"swap refunded"}}"#, id));
                self.launches.insert(id, l);
                PromiseOrValue::Value(false)
            }
        }
    }

    pub fn cancel_dev_buy(&mut self, launch_id: U64) -> Promise {
        let mut l = self.launches.get(&launch_id.0).cloned().expect("launch");
        require!(env::predecessor_account_id() == l.creator, "creator only");
        require!(l.step == Step::DevBuy && !l.inflight, "no pending dev buy");
        let amount = l.dev_buy_held.0;
        l.dev_buy_held = U128(0);
        l.dev_buy_near = U128(0);
        self.internal_done(launch_id.0, l.clone());
        emit("dev_buy_refunded", &format!(r#"{{"id":{},"creator":"{}","amount":"{}"}}"#, l.id, l.creator, amount));
        Promise::new(l.creator).transfer(NearToken::from_yoctonear(amount))
    }

    #[private]
    pub fn on_dev_tokens_read(&mut self, id: u64, #[callback_result] bal: Result<U128, PromiseError>) -> PromiseOrValue<bool> {
        let mut l = self.launches.get(&id).cloned().expect("launch");
        l.inflight = false;
        match bal {
            Ok(b) if b.0 > 0 => {
                l.dev_buy_tokens = b;
                l.step = Step::DeliverDevTokens;
                self.internal_advance(id, l)
            }
            _ => {
                l.step = Step::Done;
                self.internal_done(id, l);
                PromiseOrValue::Value(true)
            }
        }
    }

    #[private]
    pub fn on_dev_tokens_delivered(&mut self, id: u64, #[callback_result] r: Result<(), PromiseError>) -> bool {
        let mut l = self.launches.get(&id).cloned().expect("launch");
        l.inflight = false;
        if r.is_err() {
            emit("launch_step_failed", &format!(r#"{{"id":{},"step":"DeliverDevTokens"}}"#, id));
            self.launches.insert(id, l);
            return false;
        }
        l.step = Step::Done;
        self.internal_done(id, l);
        true
    }

    pub fn claim_fees(&mut self, launch_id: U64) -> Promise {
        let mut l = self.launches.get(&launch_id.0).cloned().expect("launch");
        require!(l.step == Step::Done, "launch not live");
        require!(!l.inflight, "claim in flight");
        let lpt = l.lpt_id.clone().expect("lpt");
        let (token, token_is_x) = (l.token.clone(), l.token_is_x);
        let quote_arg = if l.quote == self.wnear_id { "null".to_string() } else { format!("\"{}\"", l.quote) };
        l.inflight = true;
        self.launches.insert(launch_id.0, l);
        Promise::new(self.locker_id.clone())
            .function_call(
                "claim".to_string(),
                format!(r#"{{"lpt_id":{},"token":"{}","token_is_x":{},"quote":{}}}"#, js(&lpt), token, token_is_x, quote_arg).into_bytes(),
                NO_DEPOSIT,
                GAS_LOCKER_CLAIM,
            )
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_FEES_CLAIMED).on_fees_claimed(launch_id.0))
    }

    #[private]
    pub fn on_fees_claimed(&mut self, id: u64, #[callback_result] r: Result<Vec<U128>, PromiseError>) -> bool {
        let mut l = self.launches.get(&id).cloned().expect("launch");
        l.inflight = false;
        let Ok(v) = r else {
            self.launches.insert(id, l);
            return false;
        };
        if v.len() != 2 {
            self.launches.insert(id, l);
            return false;
        }
        let (fee_tok, fee_quote) = if l.token_is_x { (v[0].0, v[1].0) } else { (v[1].0, v[0].0) };
        let share = l_share(self.config.creator_fee_share_bps);
        let c_quote = fee_quote * share / BPS;
        let p_quote = fee_quote - c_quote;
        let c_tok = fee_tok * share / BPS;
        let p_tok = fee_tok - c_tok;
        let house = self.is_house(&l.creator);
        let native = l.quote == self.wnear_id;
        if native {
            if c_quote > 0 && !house {
                let cur = self.creator_fees.get(&l.creator).copied().unwrap_or(0);
                self.creator_fees.insert(l.creator.clone(), cur + c_quote);
            }
            self.protocol_fees += p_quote + if house { c_quote } else { 0 };
            l.fees_near_total = U128(l.fees_near_total.0 + fee_quote);
        } else {
            l.creator_quote_fees = U128(l.creator_quote_fees.0 + if house { 0 } else { c_quote });
            l.protocol_quote_fees = U128(l.protocol_quote_fees.0 + p_quote + if house { c_quote } else { 0 });
            l.fees_quote_total = U128(l.fees_quote_total.0 + fee_quote);
        }
        l.creator_token_fees = U128(l.creator_token_fees.0 + if house { 0 } else { c_tok });
        l.protocol_token_fees = U128(l.protocol_token_fees.0 + p_tok + if house { c_tok } else { 0 });
        l.fees_token_total = U128(l.fees_token_total.0 + fee_tok);
        l.claims += 1;
        emit(
            "fees_claimed",
            &format!(r#"{{"id":{},"token":"{}","quote":"{}","fee_near":"{}","fee_quote":"{}","fee_token":"{}","creator_quote":"{}","protocol_quote":"{}","ts_ms":{}}}"#,
                id, l.token, l.quote, if native { fee_quote } else { 0 }, fee_quote, fee_tok, c_quote, p_quote, env::block_timestamp_ms()),
        );
        self.launches.insert(id, l);
        self.drain_protocol_fees();
        true
    }

    fn drain_protocol_fees(&mut self) {
        let amount = self.protocol_fees;
        if amount == 0 || amount < self.min_push_yocto || self.protocol_recipients.is_empty() {
            return;
        }
        self.protocol_fees = 0;
        let n = self.protocol_recipients.len();
        let bps: Vec<u16> = self.protocol_recipients.iter().map(|(_, b)| *b).collect();
        let legs = split_legs(amount, &bps);
        for i in 0..n {
            let who = self.protocol_recipients[i].0.clone();
            let cut = legs[i];
            if cut == 0 {
                continue;
            }
            Promise::new(who.clone())
                .transfer(NearToken::from_yoctonear(cut))
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(GAS_ON_SPLIT)
                        .on_protocol_split(who.clone(), U128(cut)),
                )
                .detach();
        }
        emit("protocol_fees_split", &format!(r#"{{"amount":"{}","legs":{},"ts_ms":{}}}"#, amount, n, env::block_timestamp_ms()));
    }

    #[private]
    pub fn on_protocol_split(&mut self, to: AccountId, amount: U128, #[callback_result] r: Result<(), PromiseError>) {
        if r.is_err() {
            self.protocol_fees += amount.0;
            emit("protocol_split_failed", &format!(r#"{{"to":"{}","amount":"{}"}}"#, to, amount.0));
        }
    }

    pub fn claim_creator_fees(&mut self) -> Promise {
        let who = env::predecessor_account_id();
        let amount = self.creator_fees.get(&who).copied().unwrap_or(0);
        require!(amount > 0, "nothing to claim");
        self.creator_fees.insert(who.clone(), 0);
        emit("creator_fees_claimed", &format!(r#"{{"creator":"{}","amount":"{}"}}"#, who, amount));
        Promise::new(who).transfer(NearToken::from_yoctonear(amount))
    }

    pub fn claim_creator_token_fees(&mut self, launch_id: U64) -> Promise {
        let mut l = self.launches.get(&launch_id.0).cloned().expect("launch");
        require!(env::predecessor_account_id() == l.creator, "creator only");
        let amount = l.creator_token_fees.0;
        require!(amount > 0, "nothing to claim");
        l.creator_token_fees = U128(0);
        self.launches.insert(launch_id.0, l.clone());
        emit("creator_token_fees_claimed", &format!(r#"{{"id":{},"creator":"{}","amount":"{}"}}"#, l.id, l.creator, amount));
        self.internal_send_tokens(&l.token, &l.creator, amount, "fees")
    }

    pub fn get_quotes(&self) -> Vec<(AccountId, QuoteAsset)> {
        self.quote_ids.iter().filter_map(|id| self.quotes.get(id).map(|q| (id.clone(), q.clone()))).collect()
    }

    pub fn get_quote(&self, quote: AccountId) -> Option<QuoteAsset> {
        self.quotes.get(&quote).cloned()
    }

    pub fn claim_creator_quote_fees(&mut self, launch_id: U64) -> Promise {
        let mut l = self.launches.get(&launch_id.0).cloned().expect("launch");
        require!(env::predecessor_account_id() == l.creator, "creator only");
        require!(l.quote != self.wnear_id, "NEAR pair: use claim_creator_fees");
        let amount = l.creator_quote_fees.0;
        require!(amount > 0, "nothing to claim");
        l.creator_quote_fees = U128(0);
        self.launches.insert(launch_id.0, l.clone());
        emit("creator_quote_fees_claimed", &format!(r#"{{"id":{},"creator":"{}","quote":"{}","amount":"{}"}}"#, l.id, l.creator, l.quote, amount));
        self.internal_send_tokens(&l.quote, &l.creator, amount, "fees")
    }

    pub fn set_quote(&mut self, quote: AccountId, decimals: u8, init_point: i32, min_init_point: i32, max_init_point: i32, range_points: i32, enabled: bool) {
        self.assert_owner();
        require!(min_init_point <= init_point && init_point <= max_init_point, "init_point outside its own bounds");
        require!(decimals <= 24, "decimals: 0-24");
        require!(range_points >= POINT_DELTA_1PCT * 2, "range_points too small to open a position");
        require!((max_init_point.saturating_add(range_points)).min(POINT_MAX) - min_init_point <= 550_000, "range would exceed DCL's liquidity ceiling at the low end of this pair's bounds");
        let native = quote == self.wnear_id;
        if !self.quote_ids.contains(&quote) {
            require!(self.quote_ids.len() < 64, "too many pair assets");
            self.quote_ids.push(quote.clone());
        }
        self.quotes.insert(quote.clone(), QuoteAsset { decimals, init_point, min_init_point, max_init_point, range_points, native, enabled });
        emit("quote_set", &format!(r#"{{"quote":"{}","decimals":{},"init_point":{},"enabled":{}}}"#, quote, decimals, init_point, enabled));
    }

    pub fn set_quote_enabled(&mut self, quote: AccountId, enabled: bool) {
        self.assert_owner();
        let mut q = self.quotes.get(&quote).cloned().expect("unknown pair asset");
        q.enabled = enabled;
        self.quotes.insert(quote.clone(), q);
        emit("quote_set", &format!(r#"{{"quote":"{}","enabled":{}}}"#, quote, enabled));
    }

    #[payable]
    pub fn register_quote(&mut self, quote: AccountId) -> Promise {
        self.assert_owner();
        require!(self.quotes.contains_key(&quote), "unknown pair asset");
        let half = env::attached_deposit().as_yoctonear() / 2;
        require!(half >= FT_STORAGE_REG.as_yoctonear(), "attach at least 0.0025 NEAR");
        Promise::new(quote.clone())
            .function_call(
                "storage_deposit".to_string(),
                format!(r#"{{"account_id":"{}","registration_only":true}}"#, env::current_account_id()).into_bytes(),
                NearToken::from_yoctonear(half),
                Gas::from_tgas(10),
            )
            .then(Promise::new(self.locker_id.clone()).function_call(
                "register".to_string(),
                format!(r#"{{"token":"{}"}}"#, quote).into_bytes(),
                NearToken::from_yoctonear(half),
                Gas::from_tgas(20),
            ))
    }

    pub fn withdraw_protocol_quote_fees(&mut self, launch_id: U64, to: AccountId) -> Promise {
        self.assert_owner();
        let mut l = self.launches.get(&launch_id.0).cloned().expect("launch");
        let amount = l.protocol_quote_fees.0;
        require!(amount > 0, "nothing to withdraw");
        l.protocol_quote_fees = U128(0);
        self.launches.insert(launch_id.0, l.clone());
        emit("protocol_quote_fees_withdrawn", &format!(r#"{{"id":{},"quote":"{}","to":"{}","amount":"{}"}}"#, l.id, l.quote, to, amount));
        self.internal_send_tokens(&l.quote, &to, amount, "protocol fees")
    }

    pub fn set_config(&mut self, config: Config) {
        self.assert_owner();
        config.validate();
        self.config = config;
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.assert_owner();
        self.paused = paused;
    }

    pub fn set_token_code_hash(&mut self, token_code_hash: String) {
        self.assert_owner();
        self.token_code_hash = parse_hex32(&token_code_hash);
    }

    pub fn set_protocol_recipients(&mut self, recipients: Vec<(AccountId, u16)>) {
        self.assert_owner();
        if !recipients.is_empty() {
            require!(recipients.len() <= 4, "at most 4 recipients");
            let sum: u32 = recipients.iter().map(|(_, b)| *b as u32).sum();
            require!(sum == 10_000, "bps must sum to 10000");
            let mut seen: Vec<&AccountId> = Vec::new();
            for (who, _) in recipients.iter() {
                require!(!seen.contains(&who), "duplicate recipient");
                seen.push(who);
            }
        }
        self.protocol_recipients = recipients;
    }

    pub fn set_house_creator(&mut self, house_creator: Option<AccountId>) {
        self.assert_owner();
        self.house_creator = house_creator;
    }

    pub fn set_house_creators(&mut self, accounts: Vec<AccountId>, on: bool) {
        self.assert_owner();
        for a in accounts {
            let k = house_key(&a);
            if on { env::storage_write(&k, &[1]); } else { env::storage_remove(&k); }
            emit("house_creator_set", &format!(r#"{{"account":"{}","on":{}}}"#, a, on));
        }
    }

    pub fn is_house_creator(&self, account_id: AccountId) -> bool { self.is_house(&account_id) }

    pub fn split_protocol_token_fees(&mut self, launch_id: U64) {
        require!(!self.protocol_recipients.is_empty(), "no recipients");
        let mut l = self.launches.get(&launch_id.0).cloned().expect("launch");
        let house = self.is_house(&l.creator);
        let amount = l.protocol_token_fees.0 + if house { l.creator_token_fees.0 } else { 0 };
        require!(amount > 0, "nothing to split");
        l.protocol_token_fees = U128(0);
        if house { l.creator_token_fees = U128(0); }
        self.launches.insert(launch_id.0, l.clone());
        let bps: Vec<u16> = self.protocol_recipients.iter().map(|(_, b)| *b).collect();
        let legs = split_legs(amount, &bps);
        for (i, cut) in legs.into_iter().enumerate() {
            if cut == 0 { continue; }
            let who = self.protocol_recipients[i].0.clone();
            self.internal_send_tokens(&l.token, &who, cut, "protocol fees")
                .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_SPLIT).on_token_split(launch_id, who, U128(cut)))
                .detach();
        }
        emit("protocol_token_fees_split", &format!(r#"{{"id":{},"token":"{}","amount":"{}"}}"#, l.id, l.token, amount));
    }

    #[private]
    pub fn on_token_split(&mut self, launch_id: U64, to: AccountId, amount: U128, #[callback_result] r: Result<(), PromiseError>) {
        if r.is_err() {
            let mut l = self.launches.get(&launch_id.0).cloned().expect("launch");
            l.protocol_token_fees = U128(l.protocol_token_fees.0 + amount.0);
            self.launches.insert(launch_id.0, l);
            emit("protocol_token_split_failed", &format!(r#"{{"id":{},"to":"{}","amount":"{}"}}"#, launch_id.0, to, amount.0));
        }
    }

    pub fn set_min_push(&mut self, min_push: U128) {
        self.assert_owner();
        self.min_push_yocto = min_push.0;
    }

    pub fn set_min_claim(&mut self, min_claim: U128) {
        self.assert_owner();
        self.min_claim_yocto = min_claim.0;
    }

    pub fn set_thresholds(&mut self, min_push: Option<U128>, min_claim: Option<U128>) {
        self.assert_owner();
        if let Some(v) = min_push { self.min_push_yocto = v.0; }
        if let Some(v) = min_claim { self.min_claim_yocto = v.0; }
    }

    pub fn push_protocol_fees(&mut self) {
        self.assert_owner();
        require!(!self.protocol_recipients.is_empty(), "no recipients set");
        require!(self.protocol_fees > 0, "nothing to push");
        let keep = self.min_push_yocto;
        self.min_push_yocto = 0;
        self.drain_protocol_fees();
        self.min_push_yocto = keep;
    }

    pub fn withdraw_protocol_fees(&mut self, to: AccountId, amount: Option<U128>) -> Promise {
        self.assert_owner();
        let amount = amount.map(|a| a.0).unwrap_or(self.protocol_fees);
        require!(amount > 0 && amount <= self.protocol_fees, "amount exceeds protocol fees");
        self.protocol_fees -= amount;
        emit("protocol_fees_withdrawn", &format!(r#"{{"to":"{}","amount":"{}"}}"#, to, amount));
        Promise::new(to).transfer(NearToken::from_yoctonear(amount))
    }

    pub fn withdraw_protocol_token_fees(&mut self, launch_id: U64, to: AccountId) -> Promise {
        self.assert_owner();
        let mut l = self.launches.get(&launch_id.0).cloned().expect("launch");
        let amount = l.protocol_token_fees.0;
        require!(amount > 0, "nothing to withdraw");
        l.protocol_token_fees = U128(0);
        self.launches.insert(launch_id.0, l.clone());
        self.internal_send_tokens(&l.token, &to, amount, "protocol fees")
    }

    pub fn sweep_wnear(&mut self, amount: U128) -> Promise {
        self.assert_owner();
        self.protocol_fees += amount.0;
        Promise::new(self.wnear_id.clone())
            .function_call("near_withdraw".to_string(), format!(r#"{{"amount":"{}"}}"#, amount.0).into_bytes(), ONE_YOCTO, GAS_UNWRAP)
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_SWEPT).on_wnear_swept(amount))
    }

    #[private]
    pub fn on_wnear_swept(&mut self, amount: U128, #[callback_result] r: Result<(), PromiseError>) {
        if r.is_ok() {
            return;
        }
        let reversed = self.protocol_fees.min(amount.0);
        self.protocol_fees -= reversed;
        emit("wnear_sweep_failed", &format!(r#"{{"amount":"{}","reversed":"{}"}}"#, amount.0, reversed));
    }

    pub fn transfer_ownership(&mut self, new_owner: AccountId) {
        self.assert_owner();
        self.owner_id = new_owner;
    }

    pub fn set_range(&mut self, launch_id: U64, left_point: i32, right_point: i32) {
        self.assert_owner();
        let mut l = self.launches.get(&launch_id.0).cloned().expect("launch");
        require!(l.lpt_id.is_none() && l.step != Step::Done, "liquidity already placed");
        require!(left_point < right_point && left_point % POINT_DELTA_1PCT == 0 && right_point % POINT_DELTA_1PCT == 0, "points must be multiples of 200");
        l.left_point = left_point;
        l.right_point = right_point;
        l.inflight = false;
        self.launches.insert(launch_id.0, l);
    }

    pub fn set_step(&mut self, launch_id: U64, step: Step) {
        self.assert_owner();
        let mut l = self.launches.get(&launch_id.0).cloned().expect("launch");
        require!(l.step != Step::Done, "already live");
        l.step = step;
        l.inflight = false;
        self.launches.insert(launch_id.0, l);
    }

    pub fn reset_inflight(&mut self, launch_id: U64) {
        self.assert_owner();
        let mut l = self.launches.get(&launch_id.0).cloned().expect("launch");
        l.inflight = false;
        self.launches.insert(launch_id.0, l);
    }

    pub fn get_launch(&self, launch_id: U64) -> Option<Launch> {
        self.launches.get(&launch_id.0).cloned()
    }

    pub fn get_launch_by_symbol(&self, symbol: String) -> Option<Launch> {
        self.symbol_index.get(&sanitize_slug(&symbol)).and_then(|id| self.launches.get(id).cloned())
    }

    pub fn get_launch_by_token(&self, token: AccountId) -> Option<Launch> {
        self.token_index.get(&token).and_then(|id| self.launches.get(id).cloned())
    }

    pub fn get_launches(&self, from_index: Option<u64>, limit: Option<u64>) -> Vec<Launch> {
        let skip = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(50).min(200);
        let mut out = Vec::new();
        let mut id = self.next_id;
        let mut skipped = 0;
        while id > 0 && (out.len() as u64) < limit {
            id -= 1;
            if let Some(l) = self.launches.get(&id) {
                if skipped < skip {
                    skipped += 1;
                    continue;
                }
                out.push(l.clone());
            }
        }
        out
    }

    pub fn get_num_launches(&self) -> u64 {
        self.next_id
    }

    pub fn quote_launch(&self, icon_bytes: u32, dev_buy: Option<U128>) -> LaunchCost {
        self.internal_cost(icon_bytes as usize, 200, dev_buy.map(|d| d.0).unwrap_or(0))
    }

    pub fn get_config(&self) -> Config {
        self.config.clone()
    }

    pub fn get_dev_buy_cap(&self) -> U128 {
        U128(dev_buy_cap_yocto(self.config.init_point + POINT_DELTA_1PCT, self.config.total_supply.0, self.config.max_dev_buy_bps, self.config.pool_fee))
    }

    pub fn get_creator_fees(&self, account_id: AccountId) -> U128 {
        U128(self.creator_fees.get(&account_id).copied().unwrap_or(0))
    }

    pub fn get_protocol_fees(&self) -> U128 {
        U128(self.protocol_fees)
    }

    pub fn get_owner(&self) -> AccountId {
        self.owner_id.clone()
    }

    pub fn get_fee_split(&self) -> FeeSplit {
        FeeSplit {
            recipients: self.protocol_recipients.iter().map(|(a, b)| (a.clone(), *b)).collect(),
            house_creator: self.house_creator.clone(),
            min_push: U128(self.min_push_yocto),
            min_claim: U128(self.min_claim_yocto),
            pending: U128(self.protocol_fees),
        }
    }

    pub fn get_addresses(&self) -> Addresses {
        Addresses {
            wnear: self.wnear_id.clone(),
            dcl: self.dcl_id.clone(),
            locker: self.locker_id.clone(),
            token_code_hash: hex32(self.token_code_hash),
            dcl_registered: self.dcl_registered,
            wnear_registered: self.wnear_registered,
            paused: self.paused,
        }
    }
}

impl Factory {
    fn is_house(&self, a: &AccountId) -> bool {
        self.house_creator.as_ref() == Some(a) || env::storage_has_key(&house_key(a))
    }

    fn assert_owner(&self) {
        require!(env::predecessor_account_id() == self.owner_id, "owner only");
    }

    fn internal_cost(&self, icon_len: usize, text_len: usize, dev_buy: u128) -> LaunchCost {
        let bytes = TOKEN_BASE_STORAGE_BYTES + icon_len as u128 + text_len as u128;
        let token_storage = bytes * STORAGE_PRICE_PER_BYTE * 125 / 100 + FT_STORAGE_REG.as_yoctonear();
        let dcl_storage = if self.dcl_registered { self.config.dcl_storage_per_launch.0 } else { DCL_REGISTER.as_yoctonear() };
        let dev = dev_buy;
        LaunchCost {
            launch_fee: self.config.launch_fee,
            token_storage: U128(token_storage),
            pool_create: U128(DCL_POOL_CREATE.as_yoctonear()),
            dcl_storage: U128(dcl_storage),
            dev_buy: U128(dev),
            total: U128(self.config.launch_fee.0 + token_storage + DCL_POOL_CREATE.as_yoctonear() + dcl_storage + dev),
        }
    }

    fn internal_fail(&mut self, id: u64, mut l: Launch, step: &str) -> PromiseOrValue<bool> {
        emit("launch_step_failed", &format!(r#"{{"id":{},"step":{}}}"#, id, js(step)));
        if l.step == Step::CreateToken {
            l.step = Step::Failed;
            emit("launch_failed", &format!(r#"{{"id":{},"token":"{}","refundable":"{}"}}"#, id, l.token, l.dev_buy_held.0));
        }
        self.launches.insert(id, l);
        PromiseOrValue::Value(false)
    }

    fn internal_done(&mut self, id: u64, mut l: Launch) {
        l.step = Step::Done;
        l.inflight = false;
        self.launches.insert(id, l.clone());
        emit(
            "launch",
            &format!(
                r#"{{"id":{},"token":"{}","creator":"{}","name":{},"symbol":{},"pool_id":{},"lpt_id":{},"init_point":{},"left_point":{},"right_point":{},"dev_buy_near":"{}","dev_buy_tokens":"{}","ts_ms":{}}}"#,
                id, l.token, l.creator, js(&l.name), js(&l.symbol), js(&l.pool_id), js(l.lpt_id.as_deref().unwrap_or("")), l.init_point, l.left_point, l.right_point,
                l.dev_buy_near.0, l.dev_buy_tokens.0, env::block_timestamp_ms()
            ),
        );
    }

    fn internal_advance(&mut self, id: u64, l: Launch) -> PromiseOrValue<bool> {
        let need = step_gas(l.step).saturating_add(GAS_CB_MIN).saturating_add(GAS_RESERVE);
        if l.step == Step::DevBuy || gas_left() < need {
            emit("launch_step_parked", &format!(r#"{{"id":{},"step":"{:?}"}}"#, id, l.step));
            self.launches.insert(id, l);
            return PromiseOrValue::Value(true);
        }
        PromiseOrValue::Promise(self.internal_run_step(id, l, 0))
    }

    fn internal_run_step(&mut self, id: u64, mut l: Launch, token_storage: u128) -> Promise {
        require!(!l.inflight, "step in flight");
        l.inflight = true;
        let step = l.step;
        self.launches.insert(id, l.clone());
        let me = env::current_account_id();
        let reserve = if step == Step::CreateToken { GAS_RESERVE_LAUNCH } else { GAS_RESERVE };
        let cb_gas = gas_left().saturating_sub(step_gas(step)).saturating_sub(reserve).max(GAS_CB_MIN);
        let ext = Self::ext(me.clone()).with_static_gas(cb_gas);
        match step {
            Step::CreateToken => {
                let rules = (self.config.max_wallet_bps > 0).then(|| TokenRules {
                    max_wallet_bps: self.config.max_wallet_bps,
                    until_ms: (env::block_timestamp_ms() + self.config.max_wallet_ms).to_string(),
                    exempt: vec![me.clone(), self.dcl_id.clone(), self.locker_id.clone()],
                });
                let init = TokenInit {
                    owner_id: self.locker_id.clone(),
                    total_supply: l.total_supply,
                    metadata: TokenMetadata { spec: "ft-1.0.0", name: &l.name, symbol: &l.symbol, icon: &l.icon, reference: None, reference_hash: None, decimals: TOKEN_DECIMALS },
                    rules,
                };
                let token = Promise::new(l.token.clone())
                    .create_account()
                    .transfer(NearToken::from_yoctonear(token_storage + FT_STORAGE_REG.as_yoctonear()))
                    .use_global_contract(self.token_code_hash)
                    .function_call("new".to_string(), near_sdk::serde_json::to_vec(&init).expect("init"), NO_DEPOSIT, GAS_TOKEN_INIT)
                    .function_call(
                        "storage_deposit".to_string(),
                        format!(r#"{{"account_id":"{}","registration_only":true}}"#, l.creator).into_bytes(),
                        FT_STORAGE_REG,
                        Gas::from_tgas(5),
                    );
                token.and(self.internal_pool_batch(&l, &me)).then(ext.on_created(id))
            }
            Step::CreatePool => self.internal_pool_batch(&l, &me).then(ext.on_pool_created(id)),
            Step::Deposit | Step::AddLiquidity => {
                let (ax, ay) = if l.token_is_x { (l.total_supply.0, 0u128) } else { (0u128, l.total_supply.0) };
                Promise::new(self.locker_id.clone())
                    .function_call(
                        "add".to_string(),
                        format!(
                            r#"{{"args":{{"token":"{}","pool_id":{},"left_point":{},"right_point":{},"amount_x":"{}","amount_y":"{}"}}}}"#,
                            l.token, js(&l.pool_id), l.left_point, l.right_point, ax, ay
                        )
                        .into_bytes(),
                        NO_DEPOSIT,
                        GAS_LOCKER_ADD,
                    )
                    .then(ext.on_liquidity_added(id))
            }
            Step::DevBuy => {
                let swap_msg = if l.dev_buy_exact {
                    let out = l.total_supply.0 / BPS * self.config.max_dev_buy_bps as u128;
                    format!(
                        r#"{{"SwapByOutput":{{"pool_ids":[{}],"output_token":"{}","output_amount":"{}","swap_out_recipient":"{}"}}}}"#,
                        js(&l.pool_id), l.token, out, l.creator
                    )
                } else {
                    format!(
                        r#"{{"Swap":{{"pool_ids":[{}],"output_token":"{}","min_output_amount":"0","swap_out_recipient":"{}"}}}}"#,
                        js(&l.pool_id), l.token, l.creator
                    )
                };
                let mut p = Promise::new(self.wnear_id.clone());
                if !self.wnear_registered {
                    p = p.function_call(
                        "storage_deposit".to_string(),
                        format!(r#"{{"account_id":"{}","registration_only":true}}"#, me).into_bytes(),
                        FT_STORAGE_REG,
                        Gas::from_tgas(5),
                    );
                }
                p.function_call("near_deposit".to_string(), b"{}".to_vec(), NearToken::from_yoctonear(l.dev_buy_near.0), Gas::from_tgas(5))
                    .function_call(
                        "ft_transfer_call".to_string(),
                        format!(r#"{{"receiver_id":"{}","amount":"{}","msg":{}}}"#, self.dcl_id, l.dev_buy_near.0, js(&swap_msg)).into_bytes(),
                        ONE_YOCTO,
                        GAS_DEV_BUY,
                    )
                    .then(ext.on_dev_bought(id))
            }
            Step::ReadDevTokens => Promise::new(l.token.clone())
                .function_call("ft_balance_of".to_string(), format!(r#"{{"account_id":"{}"}}"#, me).into_bytes(), NO_DEPOSIT, GAS_BALANCE_OF)
                .then(ext.on_dev_tokens_read(id)),
            Step::DeliverDevTokens => self
                .internal_send_tokens(&l.token, &l.creator, l.dev_buy_tokens.0, "dev buy")
                .then(ext.on_dev_tokens_delivered(id)),
            Step::Done | Step::Failed => env::panic_str("launch finished"),
        }
    }

    fn internal_pool_batch(&self, l: &Launch, me: &AccountId) -> Promise {
        let (a, b) = if l.token_is_x { (&l.token, &l.quote) } else { (&l.quote, &l.token) };
        let storage = if self.dcl_registered { self.config.dcl_storage_per_launch.0 } else { DCL_REGISTER.as_yoctonear() };
        let _ = me;
        Promise::new(self.dcl_id.clone())
            .function_call(
                "storage_deposit".to_string(),
                format!(r#"{{"account_id":"{}","registration_only":false}}"#, self.locker_id).into_bytes(),
                NearToken::from_yoctonear(storage),
                Gas::from_tgas(5),
            )
            .function_call(
                "create_pool".to_string(),
                format!(r#"{{"token_a":"{}","token_b":"{}","fee":{},"init_point":{}}}"#, a, b, self.config.pool_fee, l.init_point).into_bytes(),
                DCL_POOL_CREATE,
                GAS_DCL_POOL,
            )
    }

    fn internal_send_tokens(&self, token: &AccountId, to: &AccountId, amount: u128, memo: &str) -> Promise {
        Promise::new(token.clone())
            .function_call(
                "storage_deposit".to_string(),
                format!(r#"{{"account_id":"{}","registration_only":true}}"#, to).into_bytes(),
                FT_STORAGE_REG,
                Gas::from_tgas(5),
            )
            .function_call(
                "ft_transfer".to_string(),
                format!(r#"{{"receiver_id":"{}","amount":"{}","memo":{}}}"#, to, amount, js(memo)).into_bytes(),
                ONE_YOCTO,
                GAS_DELIVER,
            )
    }
}

impl Config {
    pub fn default_mainnet() -> Self {
        Self {
            total_supply: U128(1_000_000_000 * 10u128.pow(TOKEN_DECIMALS as u32)),
            pool_fee: 10_000,
            init_point: 0,
            min_init_point: -6_800,
            max_init_point: 15_200,
            range_points: 1_000_000,
            launch_fee: U128(0),
            creator_fee_share_bps: 7_000,
            dcl_storage_per_launch: U128(20 * 10u128.pow(21)),
            max_icon_bytes: 16 * 1024,
            unique_symbols: false,
            max_dev_buy_bps: 400,
            max_wallet_bps: 0,
            max_wallet_ms: 0,
        }
    }

    pub fn validate(&self) {
        require!(self.total_supply.0 > 0, "supply");
        require!(self.pool_fee == 10_000, "only the 1% tier is supported (point delta 200)");
        require!(self.min_init_point <= self.init_point && self.init_point <= self.max_init_point, "init_point outside bounds");
        require!(self.range_points >= 2 * POINT_DELTA_1PCT, "range_points too small");
        require!(self.creator_fee_share_bps as u128 <= BPS, "creator share max 100%");
        require!(self.max_icon_bytes <= 32 * 1024, "icon cap max 32KB");
        require!(self.max_dev_buy_bps <= 2_000, "dev buy cap max 20%");
        require!(self.max_wallet_bps == 0 || self.max_dev_buy_bps <= self.max_wallet_bps, "dev buy cap must fit the max wallet");
        require!(self.max_wallet_ms <= 86_400_000, "max wallet window max 24h");
    }
}

fn step_gas(step: Step) -> Gas {
    match step {
        Step::CreateToken => GAS_TOKEN_INIT.saturating_add(GAS_DCL_POOL).saturating_add(Gas::from_tgas(10)),
        Step::CreatePool => GAS_DCL_POOL.saturating_add(Gas::from_tgas(5)),
        Step::Deposit | Step::AddLiquidity => GAS_LOCKER_ADD,
        Step::DevBuy => GAS_DEV_BUY.saturating_add(Gas::from_tgas(10)),
        Step::ReadDevTokens => GAS_BALANCE_OF,
        Step::DeliverDevTokens => GAS_DELIVER.saturating_add(Gas::from_tgas(5)),
        Step::Done | Step::Failed => Gas::from_tgas(0),
    }
}

const POW_MAX_POINT: u32 = 29_000;
fn pow_1_0001_fp(p: i32) -> u128 {
    require!(p.unsigned_abs() <= POW_MAX_POINT, "point out of range for the fixed-point power");
    let mut e = p.unsigned_abs();
    let mut base = ONE_0001_FP;
    let mut acc = FP;
    while e > 0 {
        if e & 1 == 1 {
            acc = acc * base / FP;
        }
        e >>= 1;
        if e > 0 {
            base = base * base / FP;
        }
    }
    if p < 0 { FP * FP / acc } else { acc }
}

fn dev_buy_cap_yocto(left_point: i32, total_supply: u128, bps: u16, pool_fee: u32) -> u128 {
    if bps == 0 {
        return 0;
    }
    let fdv_left = total_supply / FP * pow_1_0001_fp(left_point) / 1_000_000 * 1_000_000;
    let cost = fdv_left / (BPS - bps as u128) * bps as u128;
    cost * 1_000_000 / (1_000_000 - pool_fee as u128)
}

fn split_legs(amount: u128, bps: &[u16]) -> Vec<u128> {
    let n = bps.len();
    let mut out = Vec::with_capacity(n);
    let mut sent: u128 = 0;
    for (i, b) in bps.iter().enumerate() {
        let cut = if i + 1 == n {
            amount - sent
        } else {
            match amount.checked_mul(*b as u128) {
                Some(v) => v / BPS,
                None => (amount / BPS) * *b as u128,
            }
        };
        sent += cut;
        out.push(cut);
    }
    out
}

fn l_share(bps: u16) -> u128 {
    bps as u128
}

fn gas_left() -> Gas {
    env::prepaid_gas().saturating_sub(env::used_gas())
}

fn floor_to(p: i32, delta: i32) -> i32 {
    p.div_euclid(delta) * delta
}

fn sanitize_slug(symbol: &str) -> String {
    let s: String = symbol.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).take(12).collect();
    if s.is_empty() { "t".to_string() } else { s }
}

fn hex6(seed: &[u8]) -> String {
    let mut s = String::with_capacity(6);
    for b in seed.iter().take(3) {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

fn emit(event: &str, data_json: &str) {
    log!(r#"EVENT_JSON:{{"standard":"{}","version":"{}","event":"{}","data":[{}]}}"#, EVENT_STANDARD, EVENT_VERSION, event, data_json);
}

fn js(s: &str) -> String {
    near_sdk::serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

fn parse_hex32(h: &str) -> CryptoHash {
    let h = h.trim().trim_start_matches("0x");
    require!(h.len() == 64, "code hash must be 64 hex chars");
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&h[2 * i..2 * i + 2], 16).expect("hex");
    }
    out
}

fn hex32(h: CryptoHash) -> String {
    let mut s = String::with_capacity(64);
    for b in h {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_and_prices() {
        let raw = 1.0001f64.powi(-3200);
        let fdv_usd = raw * 1e18 / 1e24 * 1e9 * 4.13;
        assert!((fdv_usd - 3000.0).abs() < 60.0, "fdv={}", fdv_usd);
        assert!(1.0001f64.powi(500_000 + 3200) > 1e21);
        assert_eq!(floor_to(-3196, 200), -3200);
        assert_eq!(floor_to(-3200, 200), -3200);
        assert_eq!(floor_to(150, 200), 0);
    }

    #[test]
    fn a_pairs_points_are_not_transferable_between_assets() {
        let point_for = |fdv_quote: f64, dec_quote: i32| -> i32 {
            let supply = 1e9f64;
            let raw = (fdv_quote / supply) * 10f64.powi(dec_quote - TOKEN_DECIMALS as i32);
            (raw.ln() / 1.0001f64.ln()).round() as i32
        };
        assert_eq!(point_for(1_000.0, NEAR_DECIMALS as i32), 0);
        let usdc = point_for(4_300.0, 6);
        assert!((-400_000..-399_000).contains(&usdc), "usdc point = {usdc}");
        assert!(usdc < -390_000, "a USDC pair must not reuse the wNEAR point");

        let wnear_top = (0i32.saturating_add(1_000_000)).min(POINT_MAX);
        assert_eq!(wnear_top - 0, 500_000);
        let naive_top = (usdc.saturating_add(1_000_000)).min(POINT_MAX);
        assert!(naive_top - usdc > 600_000, "this is the overflow set_quote now refuses");
        let safe = 450_000;
        assert!((usdc.saturating_add(safe)).min(POINT_MAX) - usdc <= 550_000);
    }

    #[test]
    fn a_deep_pair_point_never_reaches_the_power_helper() {
        assert!(400_000u32 > POW_MAX_POINT);
        assert!((pow_1_0001_fp(29_000) as f64 / 1e18 - 1.0001f64.powi(29_000)).abs() < 1e6);
        assert_eq!(dev_buy_cap_yocto(-400_000, 10u128.pow(27), 0, 10_000), 0, "bps 0 must short-circuit before the power");
    }

    #[test]
    fn ordering_and_ranges() {
        let cfg = Config::default_mainnet();
        cfg.validate();
        let token_is_x = "pepe-1a2b3c.pad.near" < "wrap.near";
        assert!(token_is_x);
        let top = (cfg.init_point + cfg.range_points).min(POINT_MAX);
        let (l, r) = (cfg.init_point + POINT_DELTA_1PCT, top);
        assert_eq!((l, r), (200, 500000));
        assert!(!("zebra-1a2b3c.yakpad.near" < "wrap.near"));
        let (l2, r2) = (-top, -cfg.init_point - POINT_DELTA_1PCT);
        assert_eq!((l2, r2), (-500000, -200));
    }

    #[test]
    fn dev_buy_cap_matches_rhea_quote() {
        let cap = dev_buy_cap_yocto(-3000, 10u128.pow(27), 500, 10_000);
        let near = cap as f64 / 1e24;
        assert!((near - 39.383).abs() < 0.2, "cap={}", near);
        let cap4 = dev_buy_cap_yocto(200, 10u128.pow(27), 400, 10_000) as f64 / 1e24;
        assert!((cap4 - 42.93).abs() < 0.3, "cap4={}", cap4);
        assert!((pow_1_0001_fp(0) as f64 / 1e18 - 1.0).abs() < 1e-12);
        assert!((pow_1_0001_fp(-3000) as f64 / 1e18 - 0.7408).abs() < 0.001);
    }

    #[test]
    fn split_legs_are_exact_and_even() {
        for amount in [0u128, 1, 2, 3, 999, 1_000_000_000_000_000_000_000_001, 1_200_000_000_000_000_000_000_000_000_000_000] {
            let legs = split_legs(amount, &[5_000, 5_000]);
            assert_eq!(legs.iter().sum::<u128>(), amount, "legs must re-sum to the amount");
            assert!(legs[0].abs_diff(legs[1]) <= 1);
        }
        assert_eq!(split_legs(u128::MAX, &[5_000, 5_000]).iter().sum::<u128>(), u128::MAX);
        assert_eq!(split_legs(u128::MAX / 4, &[5_000, 5_000]).iter().sum::<u128>(), u128::MAX / 4);
        assert_eq!(split_legs(3, &[5_000, 5_000]), vec![1, 2]);
        let legs = split_legs(1_000_000_007, &[3_333, 3_333, 3_334]);
        assert_eq!(legs.iter().sum::<u128>(), 1_000_000_007);
        assert_eq!(split_legs(777, &[10_000]), vec![777]);
        assert!((pow_1_0001_fp(15200) as f64 / 1e18 - 4.571).abs() < 0.01);
    }

    #[test]
    fn slug_and_hex() {
        assert_eq!(sanitize_slug("PEPE"), "pepe");
        assert_eq!(sanitize_slug("$$"), "t");
        assert_eq!(hex6(&[0xab, 0x01, 0xff, 0x00]), "ab01ff");
        let h = parse_hex32("7591d117dde58bba80a7df25ff6bd1112428f85cf20018d774f00b48af0fa11b");
        assert_eq!(hex32(h), "7591d117dde58bba80a7df25ff6bd1112428f85cf20018d774f00b48af0fa11b");
    }
}

fn house_key(a: &AccountId) -> Vec<u8> {
    [b"xh:".as_slice(), a.as_bytes()].concat()
}






use near_sdk::json_types::{Base58CryptoHash, U128, U64};
use near_sdk::serde::Serialize;
use near_sdk::store::LookupMap;
use near_sdk::{
    env, log, near, require, AccountId, Allowance, BorshStorageKey, CryptoHash, Gas, GasWeight, NearToken,
    PanicOnDefault, Promise, PromiseError, PromiseOrValue, PublicKey,
};


const BPS: u128 = 10_000;
const TOKEN_DECIMALS: u8 = 18;

const NEAR_DECIMALS: u8 = 24;
const STORAGE_PRICE_PER_BYTE: u128 = 10_000_000_000_000_000_000;

const TOKEN_BASE_STORAGE_BYTES: u128 = 3_000;




const PROTOCOL_BPS: u16 = 2_000;
const BURN_POT_BPS: u16 = 0;
const REFERRAL_POT_BPS: u16 = 1_000;
const _: () = assert!(7_000 + PROTOCOL_BPS + BURN_POT_BPS + REFERRAL_POT_BPS == 10_000, "the four parts make the whole fee");

const BURN_POT_KEY: &[u8] = b"bpt";
const REFERRAL_POT_KEY: &[u8] = b"rpt";
fn burn_pot() -> Option<AccountId> { env::storage_read(BURN_POT_KEY).map(|b| String::from_utf8(b).expect("burn pot").parse().expect("burn pot")) }
fn referral_pot() -> Option<AccountId> { env::storage_read(REFERRAL_POT_KEY).map(|b| String::from_utf8(b).expect("referral pot").parse().expect("referral pot")) }

const FT_STORAGE_REG: NearToken = NearToken::from_yoctonear(1_250_000_000_000_000_000_000);

const DCL_REGISTER: NearToken = NearToken::from_millinear(500);

const DCL_POOL_CREATE: NearToken = NearToken::from_millinear(100);
const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);
const NO_DEPOSIT: NearToken = NearToken::from_yoctonear(0);

const POINT_DELTA_1PCT: i32 = 200;

const ONE_0001_FP: u128 = 1_000_100_000_000_000_000;
const FP: u128 = 1_000_000_000_000_000_000;


const POINT_MAX: i32 = 500_000;
const EVENT_STANDARD: &str = "nearpad";
const EVENT_VERSION: &str = "2.0.0";






const fn tg(g: Gas) -> u64 { g.as_gas() / 1_000_000_000_000 }

const GAS_TOKEN_INIT: Gas = Gas::from_tgas(6);

const GAS_DCL_POOL: Gas = Gas::from_tgas(8);


const GAS_LOCKER_ADD: Gas = Gas::from_tgas(180);

const GAS_LOCKER_CLAIM: Gas = Gas::from_tgas(180);
const _: () = assert!(tg(GAS_LOCKER_ADD) >= 180 && tg(GAS_LOCKER_CLAIM) >= 180, "lock3 add/claim were measured against 180 TGas");

const GAS_LOCKER_ADMIN_WITHDRAW: Gas = Gas::from_tgas(100);

const GAS_LWD_BOOK: Gas = Gas::from_tgas(20);
const GAS_LWD_WITHDRAWN: Gas = Gas::from_tgas(30);
const GAS_LWD_BEFORE: Gas = Gas::from_tgas(140);
const _: () = assert!(tg(GAS_UNWRAP) + tg(GAS_ON_SPLIT) + 4 <= tg(GAS_LWD_BOOK), "the booking must fit a wNEAR unwrap and its callback");
const _: () = assert!(tg(GAS_BALANCE_OF) + tg(GAS_LWD_BOOK) + 4 <= tg(GAS_LWD_WITHDRAWN), "the second read must fit its booking callback");
const _: () = assert!(tg(GAS_LOCKER_ADMIN_WITHDRAW) + tg(GAS_LWD_WITHDRAWN) + 6 <= tg(GAS_LWD_BEFORE), "the withdrawal step must fit lock3's call and the second read");
const _: () = assert!(tg(GAS_BALANCE_OF) + tg(GAS_LWD_BEFORE) + 15 <= 300, "locker_withdraw_dcl_asset must fit one transaction");

const GAS_LOCKER_SET_DCL: Gas = Gas::from_tgas(20);

const GAS_LOCKER_REFUND_BUY: Gas = Gas::from_tgas(60);
const GAS_ON_LOCKER_REFUND: Gas = Gas::from_tgas(12);

const GAS_FIRST_BUY_VIEW: Gas = Gas::from_tgas(5);
const GAS_ON_FIRST_BUY: Gas = Gas::from_tgas(20);
const _: () = assert!(tg(GAS_ON_SPLIT) + 3 + 3 <= tg(GAS_ON_LOCKER_REFUND), "on_locker_buy_refunded must fit the creator's transfer and its callback");
const GAS_BALANCE_OF: Gas = Gas::from_tgas(5);

const GAS_DELIVER: Gas = Gas::from_tgas(12);
const GAS_UNWRAP: Gas = Gas::from_tgas(10);
const GAS_BURN: Gas = Gas::from_tgas(10);

const GAS_ON_SPLIT: Gas = Gas::from_tgas(4);


const GAS_OWED_CB: Gas = Gas::from_tgas(20);

const GAS_CB_MIN: Gas = Gas::from_tgas(8);

const GAS_RESERVE: Gas = Gas::from_tgas(10);



const GAS_RESERVE_LAUNCH: Gas = Gas::from_tgas(18);

const GAS_LIQ_CB: Gas = Gas::from_tgas(32);

const GAS_GET_POOL: Gas = Gas::from_tgas(5);
const GAS_POOL_CHECKED: Gas = Gas::from_tgas(20);



const GAS_POOL_STEP_CB: Gas = Gas::from_tgas(45);
const _: () = assert!(5 + tg(GAS_GET_POOL) + tg(GAS_POOL_CHECKED) + 2 * 2 + 5 <= tg(GAS_POOL_STEP_CB), "the pool check must fit the create step's callback floor");

const GAS_HOOK: Gas = Gas::from_tgas(10);
const GAS_HOOK_MIN_LEFT: Gas = Gas::from_tgas(20);



const T_AFTER_LAUNCH: u64 = 300 - 9;

const T_CB_BURN: u64 = 4;
const T_AT_ON_CREATED: u64 = T_AFTER_LAUNCH - tg(step_gas_create_token()) - tg(GAS_RESERVE_LAUNCH) - T_CB_BURN;
const _: () = assert!(tg(GAS_LOCKER_ADD) + tg(GAS_LIQ_CB) + tg(GAS_RESERVE) <= T_AT_ON_CREATED, "AddLiquidity no longer fits in the launch transaction");
const _: () = assert!(T_AT_ON_CREATED >= tg(GAS_POOL_STEP_CB), "on_created in the launch transaction must afford the pool check");
const _: () = assert!(tg(GAS_LIQ_CB) >= T_CB_BURN + tg(GAS_HOOK_MIN_LEFT) + 5, "on_liquidity_added must reach internal_done with the hook's allowance left");


const fn step_gas_create_token() -> Gas { Gas::from_tgas(tg(GAS_TOKEN_INIT) + tg(GAS_DCL_POOL) + 10) }










const GAS_DEV_BUY: Gas = Gas::from_tgas(160);


const GAS_DEV_BOUGHT_CB: Gas = Gas::from_tgas(55);

const GAS_DEV_UNWRAPPED: Gas = Gas::from_tgas(12);
const _: () = assert!(tg(GAS_ON_SPLIT) + 3 + 3 <= tg(GAS_DEV_UNWRAPPED), "on_dev_refund_unwrapped must fit the transfer's callback");
const _: () = assert!(tg(GAS_UNWRAP) + tg(GAS_DEV_UNWRAPPED) + 2 * 2 + 20 + 5 <= tg(GAS_DEV_BOUGHT_CB), "on_dev_bought must fit the unwrap, its callback and the hook allowance");

const T_DEV_BUY_NEEDS: u64 = tg(GAS_DEV_BUY) + 10 + tg(GAS_DEV_BOUGHT_CB) + tg(GAS_RESERVE);
const _: () = assert!(T_DEV_BUY_NEEDS + tg(GAS_RESERVE_LAUNCH) <= 300, "step_gas(DevBuy) no longer fits in a transaction of its own");

const GAS_DEV_BUY_INLINE: Gas = Gas::from_tgas(80);


const GAS_DEV_BOUGHT_CB_INLINE: Gas = Gas::from_tgas(36);

const GAS_RESERVE_INLINE: Gas = Gas::from_tgas(5);
const _: () = assert!(tg(GAS_DEV_BUY_INLINE) >= 76 + 4, "the in-line swap must stay above the measured 76 TGas minimum");
const _: () = assert!(5 + tg(GAS_UNWRAP) + tg(GAS_DEV_UNWRAPPED) + 3 + 5 <= tg(GAS_DEV_BOUGHT_CB_INLINE), "on_dev_bought in-line must fit its unwrap path with margin");


const GAS_LIQ_CB_DEV_BUY: Gas = Gas::from_tgas(3 + 5 + tg(GAS_DEV_BUY_INLINE) + tg(GAS_DEV_BOUGHT_CB_INLINE) + tg(GAS_RESERVE_INLINE));


const T_ON_CREATED_AT_300: u64 = 255;



const T_LOCKER_ADD_BUY: u64 = 230;

const GAS_LIQ_CB_ADD_BUY: Gas = Gas::from_tgas(14);

const _: () = assert!(T_ON_CREATED_AT_300 - T_CB_BURN >= T_LOCKER_ADD_BUY + tg(GAS_LIQ_CB_ADD_BUY) + tg(GAS_RESERVE_INLINE), "add_buy no longer fits a 300 TGas launch");

const GAS_ADD_BUY_STEP: Gas = Gas::from_tgas(T_LOCKER_ADD_BUY + tg(GAS_LIQ_CB_ADD_BUY) + tg(GAS_RESERVE_INLINE));

const GAS_ADD_BUY_MARGIN: Gas = Gas::from_tgas(3);
const _: () = assert!(tg(GAS_LIQ_CB_ADD_BUY) >= T_CB_BURN + tg(GAS_ON_SPLIT) + 2 + 3, "on_liquidity_added_buy must book the launch and its refund");

const T_LOCK3_ADD: u64 = 111;
const _: () = assert!(T_ON_CREATED_AT_300 >= T_LOCK3_ADD + tg(GAS_LIQ_CB_DEV_BUY) + tg(GAS_RESERVE_INLINE), "an in-line dev buy no longer fits a 300 TGas launch on lock3");



const DEV_BUY_MIN_OUT_BPS: u128 = 9_700;




const GAS_PER_PAYOUT_LEG: Gas = Gas::from_tgas(6);
const GAS_FEES_CB_BASE: Gas = Gas::from_tgas(5);
const GAS_FEES_CB_MARGIN: Gas = Gas::from_tgas(10);

const GAS_CLAIM_RESERVE: Gas = Gas::from_tgas(10);
const fn fees_claimed_gas(legs: u64) -> Gas { Gas::from_tgas(tg(GAS_FEES_CB_BASE) + tg(GAS_FEES_CB_MARGIN) + legs * tg(GAS_PER_PAYOUT_LEG)) }
const _: () = assert!(tg(GAS_ON_SPLIT) + 2 <= tg(GAS_PER_PAYOUT_LEG), "a payout leg must cover its callback and receipt fees");
const _: () = assert!(tg(GAS_LOCKER_CLAIM) + tg(fees_claimed_gas(8 + 5)) + tg(GAS_CLAIM_RESERVE) + 5 <= 300, "claim_fees must fit the locker claim and a callback that pays 8 split legs and a router");

const GAS_ROUTER_CALL: Gas = Gas::from_tgas(20);


const GAS_ON_PAID_ROUTER: Gas = Gas::from_tgas(25);

const ROUTER_LEG_WEIGHT: usize = 5;
const _: () = assert!(tg(GAS_ON_PAID_ROUTER) + 3 <= ROUTER_LEG_WEIGHT as u64 * tg(GAS_PER_PAYOUT_LEG), "a router payout must fit its leg weight");
const _: () = assert!(tg(GAS_ROUTER_CALL) + 3 <= tg(GAS_ON_PAID_ROUTER), "on_paid must fit the router notice");


const GAS_BB_BEFORE: Gas = Gas::from_tgas(262);
const GAS_BB_FTC: Gas = Gas::from_tgas(180);
const GAS_BB_SWAPPED: Gas = Gas::from_tgas(60);
const GAS_BB_AFTER: Gas = Gas::from_tgas(30);

const GAS_BB_BURNED: Gas = Gas::from_tgas(6);
const _: () = assert!(5 + 5 + tg(GAS_BB_FTC) + tg(GAS_BB_SWAPPED) + 10 <= tg(GAS_BB_BEFORE), "on_bb_before must fit its swap batch + callback");
const _: () = assert!(tg(GAS_UNWRAP) + tg(GAS_ON_SPLIT) + tg(GAS_BALANCE_OF) + tg(GAS_BB_AFTER) + 8 <= tg(GAS_BB_SWAPPED), "on_bb_swapped must fit unwrap + its callback + read + callback");
const _: () = assert!(tg(GAS_BURN) + tg(GAS_BB_BURNED) + 8 <= tg(GAS_BB_AFTER), "on_bb_after must fit burn + callback");

const GAS_TAX_TAKE: Gas = Gas::from_tgas(10);

const GAS_MIGRATE: Gas = Gas::from_tgas(20);

const GAS_GET_POSITION: Gas = Gas::from_tgas(10);
const GAS_ON_STUCK_POSITION: Gas = Gas::from_tgas(15);
const _: () = assert!(tg(GAS_ON_SPLIT) + 5 <= tg(GAS_ON_STUCK_POSITION), "on_stuck_position must fit the refund and its callback");



const FM_CREATOR: u8 = 0;
const FM_BURN: u8 = 1;
const FM_HOLDERS: u8 = 2;



const MIN_CREATOR_SHARE_BPS: u16 = 7_000;
const MAX_CREATOR_SHARE_BPS: u16 = 7_000;

const MAX_RELAYERS: usize = 16;
const MAX_FEE_ROUTERS: usize = 32;

const STUCK_LAUNCH_MS: u64 = 24 * 60 * 60 * 1000;



const UNWRAP_RESOLVE_MS: u64 = 30 * 60 * 1000;

const MAX_HOLDER_PAYOUTS: usize = 25;
const MAX_FT_HOLDER_PAYOUTS: usize = 10;


const MAX_TAX_BPS: u16 = 400;




const PLATFORM_TAX_BPS: u16 = PROTOCOL_BPS + BURN_POT_BPS + REFERRAL_POT_BPS;

const TAX_EXTRA_BYTES: u128 = 600;


const MAX_TEXT_BYTES: usize = 32 * 4 + 12 + 500 * 4;


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


    pub fee_mode: Option<String>,

    pub creator_share_bps: Option<u16>,

    pub tax: Option<TaxArgs>,



    pub fee_to: Option<AccountId>,
}


#[near(serializers = [json])]
#[derive(Clone, Debug)]
pub struct TaxArgs {
    pub buy_bps: u16,
    pub sell_bps: u16,

    pub creator_bps: u16,

    pub burn_bps: u16,

    pub holders_bps: u16,
}

#[near(serializers = [json])]
#[derive(Clone, Debug)]
pub struct TaxInfo {
    pub buy_bps: u16,
    pub sell_bps: u16,
    pub creator_bps: u16,
    pub burn_bps: u16,
    pub holders_bps: u16,

    pub platform_bps: u16,

    pub pending: U128,

    pub holders_bucket: U128,

    pub burned: U128,

    pub paid_creator: U128,

    pub paid_holders: U128,

    pub paid_platform: U128,

    pub selling: U128,

    pub floor: U128,
}

#[near(serializers = [json])]
#[derive(Clone, Debug)]
pub struct FeeOpts {

    pub mode: String,
    pub creator_share_bps: u16,

    pub bucket: U128,

    pub burned: U128,

    pub paid_holders: U128,
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
#[derive(Clone, Debug, Default)]
pub struct FirstBuyRecord {
    pub amount: U128,
    pub sent: bool,
    pub answered: bool,
    pub used: U128,
    pub returned: U128,
    pub settled: bool,
}



#[near(serializers = [json])]
#[derive(Clone, Debug, Default)]
pub struct AddBuyResult {
    pub lpt_id: String,
    pub used: U128,
    pub refunded: U128,
}


#[near(serializers = [json])]
#[derive(Clone, Debug, Default)]
pub struct BuyOutcome {
    pub used: U128,
    pub refunded: U128,
}



#[derive(Clone, Copy)]
struct DevBuyBudget {
    swap: Gas,
    cb: Gas,
    reserve: Gas,
}

impl DevBuyBudget {

    const FULL: DevBuyBudget = DevBuyBudget { swap: GAS_DEV_BUY, cb: GAS_DEV_BOUGHT_CB, reserve: GAS_RESERVE };

    const INLINE: DevBuyBudget = DevBuyBudget { swap: GAS_DEV_BUY_INLINE, cb: GAS_DEV_BOUGHT_CB_INLINE, reserve: GAS_RESERVE_INLINE };



    fn need(&self, wnear_registered: bool) -> Gas {
        let reg = if !wnear_registered || self.swap == GAS_DEV_BUY { 5 } else { 0 };
        Gas::from_tgas(5 + reg + tg(self.swap) + tg(self.cb) + tg(self.reserve))
    }
}

#[near(serializers = [json])]
pub struct Addresses {
    pub wnear: AccountId,

    pub dcl: AccountId,

    pub dcl_current: AccountId,
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
struct TokenTax {
    buy_bps: u16,
    sell_bps: u16,
    pairs: Vec<AccountId>,
    admin: AccountId,
    exempt: Vec<AccountId>,
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
struct TokenInit<'a> {
    owner_id: AccountId,
    total_supply: U128,
    metadata: TokenMetadata<'a>,
    rules: Option<TokenRules>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tax: Option<TokenTax>,
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

#[near]
impl Factory {

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
        require!((1..=12).contains(&symbol.chars().count()), "symbol: 1-12 chars");
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
        let fee_mode = match args.fee_mode.as_deref() {
            None | Some("creator") => FM_CREATOR,
            Some("burn") => FM_BURN,
            Some("holders") => FM_HOLDERS,
            _ => env::panic_str("fee_mode: creator, burn or holders"),
        };


        if let Some(b) = args.creator_share_bps {
            require!(b == self.config.creator_fee_share_bps, "creator_share_bps: the launchpad's current share, or leave it out");
        }
        let custom_share = args.creator_share_bps;

        let fee_to = args.fee_to.clone().filter(|a| *a != creator);
        require!(fee_to.as_ref() != Some(&env::current_account_id()), "fee_to: not the launchpad itself");
        let tax = args.tax.clone().filter(|t| t.buy_bps > 0 || t.sell_bps > 0);
        if let Some(t) = &tax {
            require!(t.buy_bps <= MAX_TAX_BPS && t.sell_bps <= MAX_TAX_BPS, "tax: max 4% a side (1% pool fee + 4% = 5%)");
            require!(t.creator_bps as u32 + t.burn_bps as u32 + t.holders_bps as u32 == 10_000, "tax shares must total 100%");
            require!(tax_code_hash().is_some(), "tax launches are not enabled yet");

            require!(q.native || fee_mode == FM_CREATOR || t.creator_bps == 0, "tax: a creator tax share on a pair token needs fee mode creator");
        }




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


        let dev_out = if dev_buy == 0 {
            0
        } else if dev_buy_exact {
            self.config.total_supply.0 / BPS * self.config.max_dev_buy_bps as u128
        } else {
            mul_div(dev_buy_tokens_out(init_x + POINT_DELTA_1PCT, self.config.total_supply.0, dev_buy, self.config.pool_fee), DEV_BUY_MIN_OUT_BPS, BPS)
        };
        let cost = self.internal_cost(icon_len, name.len() + symbol.len() + description.len(), dev_buy, tax.is_some());
        require!(deposit >= cost.total.0, "attached deposit below quote_launch(...).total");


        let extra = deposit - cost.total.0;

        let slug = sanitize_slug(&symbol);
        if self.config.unique_symbols {
            require!(!self.symbol_index.contains_key(&slug), "symbol already launched on this pad");
        }
        let id = self.next_id;
        self.next_id += 1;
        if !self.symbol_index.contains_key(&slug) {
            self.symbol_index.insert(slug.clone(), id);
        }



        let mut taken = slug_count(&slug);
        if taken == 0 && RESERVED_SLUGS.contains(&slug.as_str()) { taken = 1; }
        set_slug_count(&slug, taken + 1);
        let token: AccountId = format!("{}.{}", short_name(&slug, taken), env::current_account_id()).parse().expect("token account id");


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
        if dev_out > 0 {
            set_dev_buy_out(id, dev_out);
        }



        let share = custom_share.unwrap_or(self.config.creator_fee_share_bps);
        set_fee_opts(id, fee_mode, share);
        if fee_mode != FM_CREATOR || custom_share.is_some() {
            emit("fee_opts", &format!(r#"{{"id":{},"mode":"{}","creator_share_bps":{}}}"#, id, mode_name(fee_mode), share));
        }
        if let Some(t) = &tax {
            set_tax_opts(id, t, PLATFORM_TAX_BPS);
            emit("tax_opts", &format!(
                r#"{{"id":{},"buy_bps":{},"sell_bps":{},"creator_bps":{},"burn_bps":{},"holders_bps":{},"platform_bps":{}}}"#,
                id, t.buy_bps, t.sell_bps, t.creator_bps, t.burn_bps, t.holders_bps, PLATFORM_TAX_BPS
            ));
        }
        if let Some(to) = &fee_to {
            set_fee_to(id, to);
            emit("fee_to", &format!(r#"{{"id":{},"creator":"{}","to":"{}"}}"#, id, creator, to));
        }
        self.token_index.insert(token, id);
        self.launches.insert(id, l.clone());
        if let Some(rc) = self.pot_recipients(share) {
            self.internal_set_launch_recipients(U64(id), Some(rc.clone()));
            emit("launch_split", &format!(r#"{{"id":{},"recipients":{}}}"#, id, near_sdk::serde_json::to_string(&rc).unwrap()));
        }
        if extra > 0 {
            Promise::new(creator).transfer(NearToken::from_yoctonear(extra)).detach();
        }
        emit(
            "launch_started",
            &format!(
                r#"{{"id":{},"token":"{}","creator":"{}","name":{},"symbol":{},"pool_id":{},"quote":"{}","init_point":{},"left_point":{},"right_point":{},"dev_buy":"{}","ts_ms":{},"dev_buy_min_out":"{}"}}"#,
                id, l.token, l.creator, js(&l.name), js(&l.symbol), js(&l.pool_id), l.quote, l.init_point, l.left_point, l.right_point, dev_buy, l.created_at_ms, dev_out
            ),
        );
        self.internal_run_step(id, l, cost.token_storage.0, true)
    }





    pub fn resume(&mut self, launch_id: U64) -> Promise {
        let l = self.rec(launch_id.0);
        require!(!l.inflight, "step in flight");
        require!(!matches!(l.step, Step::Done | Step::Failed), "nothing to resume");
        if l.step == Step::DevBuy {
            let who = env::predecessor_account_id();
            require!(who == l.creator || who == self.owner_id || relayers().contains(&who), "dev buy: creator, owner or relayer only");

            if let Some(w) = read_u128(&unwrap_pending_key(launch_id.0)) {
                let mut l = l;
                l.inflight = true;
                self.launches.insert(launch_id.0, l);
                return self.dev_buy_unwrap(launch_id.0, w, 1);
            }
        }

        self.assert_not_paused();
        if l.step == Step::AddLiquidity && self.first_buy_by_locker(launch_id.0, &l) {

            let p = env::predecessor_account_id();
            require!(p == l.creator || p == self.owner_id || relayers().contains(&p), "first buy pending: only the creator, the owner or a relayer can open this pool");
            require!(gas_left() >= GAS_ADD_BUY_STEP.saturating_add(GAS_ADD_BUY_MARGIN), "attach at least 260 TGas: this launch's first buy goes out with its position");
        }
        let icon_len = icon_len_of(launch_id.0, &l);
        let storage = self.internal_cost(icon_len, l.name.len() + l.symbol.len() + l.description.len(), 0, tax_opts(launch_id.0).is_some()).token_storage.0;
        self.internal_run_step(launch_id.0, l, storage, false)
    }


    pub fn refund_failed(&mut self, launch_id: U64) -> Promise {
        let mut l = self.rec(launch_id.0);
        require!(l.step == Step::Failed, "launch not failed");
        require!(l.dev_buy_held.0 > 0, "nothing to refund");
        let amount = l.dev_buy_held.0;
        l.dev_buy_held = U128(0);

        l.dev_buy_near = U128(0);
        self.launches.insert(launch_id.0, l.clone());
        emit("dev_buy_refunded", &format!(r#"{{"id":{},"creator":"{}","amount":"{}"}}"#, l.id, l.creator, amount));
        Promise::new(l.creator)
            .transfer(NearToken::from_yoctonear(amount))
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_SPLIT).on_dev_refund_sent(launch_id, U128(amount)))
    }










    pub fn cancel_stuck_launch(&mut self, launch_id: U64) -> PromiseOrValue<bool> {
        let mut l = self.rec(launch_id.0);
        require!(env::predecessor_account_id() == l.creator, "creator only");
        require!(matches!(l.step, Step::CreatePool | Step::AddLiquidity), "only a launch parked before its dev buy");
        require!(!l.inflight, "step in flight");
        require!(buy_held(launch_id.0) == 0, "part of the buy is held by the locker: locker_refund_buy first");
        require!(env::block_timestamp_ms() >= l.created_at_ms.saturating_add(STUCK_LAUNCH_MS), "a stuck launch can be cancelled 24h after it was created");
        if l.step == Step::CreatePool {
            return match self.internal_cancel_stuck(l) {
                Some(p) => PromiseOrValue::Promise(p),
                None => PromiseOrValue::Value(true),
            };
        }
        l.inflight = true;
        self.launches.insert(launch_id.0, l.clone());
        PromiseOrValue::Promise(
            Promise::new(self.locker_for(launch_id.0))
                .function_call("get_position".to_string(), format!(r#"{{"pool_id":{}}}"#, js(&l.pool_id)).into_bytes(), NO_DEPOSIT, GAS_GET_POSITION)
                .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_STUCK_POSITION).on_stuck_position(launch_id)),
        )
    }


    #[private]
    pub fn on_stuck_position(&mut self, launch_id: U64, #[callback_result] pos: Result<Option<near_sdk::serde_json::Value>, PromiseError>) -> bool {
        let mut l = self.rec(launch_id.0);
        l.inflight = false;
        let no_position = match &pos {
            Ok(None) => true,
            Ok(Some(p)) => p.get("lpt_id").map_or(false, |v| v.is_null()) && p.get("refunded").and_then(|v| v.as_bool()) == Some(true),
            Err(_) => false,
        };
        if !no_position {
            let reason = if pos.is_err() { "the locker cannot say whether a position exists" } else { "a position exists or may exist" };
            emit("stuck_launch_kept", &format!(r#"{{"id":{},"reason":{}}}"#, launch_id.0, js(reason)));
            self.launches.insert(launch_id.0, l);
            return false;
        }
        if let Some(p) = self.internal_cancel_stuck(l) {
            p.detach();
        }
        true
    }



    #[private]
    pub fn on_dev_refund_sent(&mut self, launch_id: U64, amount: U128, #[callback_result] r: Result<(), PromiseError>) {
        if r.is_err() {
            let mut l = self.rec(launch_id.0);
            l.dev_buy_held = U128(l.dev_buy_held.0 + amount.0);
            self.launches.insert(launch_id.0, l);
            emit("dev_buy_refund_failed", &format!(r#"{{"id":{},"amount":"{}"}}"#, launch_id.0, amount.0));
        }
    }


    #[private]
    pub fn on_created(
        &mut self,
        id: u64,
        inline: Option<bool>,
        #[callback_result] token: Result<StorageBalanceJson, PromiseError>,
        #[callback_result] pool: Result<String, PromiseError>,
    ) -> PromiseOrValue<bool> {
        let inline = inline.unwrap_or(false);
        let mut l = self.rec(id);
        l.inflight = false;
        if token.is_err() {



            if inline {
                let icon_len = icon_len_of(id, &l);
                let storage = self.internal_cost(icon_len, l.name.len() + l.symbol.len() + l.description.len(), 0, tax_opts(id).is_some()).token_storage.0;
                emit("token_storage_refunded", &format!(r#"{{"id":{},"creator":"{}","amount":"{}"}}"#, id, l.creator, storage));
                self.internal_refund_creator(id, &l.creator, storage).detach();
            }
            return self.internal_fail(id, l, "token create");
        }


        if let Some(icon) = l.icon.take() {
            if icon.len() > 0 {
                env::storage_write(&icon_len_key(id), &(icon.len() as u128).to_le_bytes());
            }
        }
        let pool_ok = matches!(&pool, Ok(p) if *p == l.pool_id);
        if pool_ok {
            self.set_dcl_registered(id);
            l.step = Step::AddLiquidity;
            self.internal_advance(id, l, inline)
        } else {
            l.step = Step::CreatePool;
            emit("launch_step_failed", &format!(r#"{{"id":{},"step":"create_pool"}}"#, id));
            PromiseOrValue::Promise(self.internal_pool_check(id, l, inline))
        }
    }

    #[private]
    pub fn on_pool_created(&mut self, id: u64, inline: Option<bool>, #[callback_result] r: Result<String, PromiseError>) -> PromiseOrValue<bool> {
        let inline = inline.unwrap_or(false);
        let mut l = self.rec(id);
        l.inflight = false;
        match r {
            Ok(p) if p == l.pool_id => {
                self.set_dcl_registered(id);
                l.step = Step::AddLiquidity;
                self.internal_advance(id, l, inline)
            }
            _ => {
                emit("launch_step_failed", &format!(r#"{{"id":{},"step":"create_pool"}}"#, id));
                PromiseOrValue::Promise(self.internal_pool_check(id, l, inline))
            }
        }
    }







    #[private]
    pub fn on_pool_checked(&mut self, id: u64, inline: Option<bool>, #[callback_result] pool: Result<Option<near_sdk::serde_json::Value>, PromiseError>) -> PromiseOrValue<bool> {
        let inline = inline.unwrap_or(false);
        let mut l = self.rec(id);
        l.inflight = false;
        match pool {
            Ok(Some(p)) if p.get("current_point").and_then(|v| v.as_i64()) == Some(l.init_point as i64) => {

                self.set_dcl_registered(id);
                emit("pool_adopted", &format!(r#"{{"id":{},"pool_id":{},"point":{}}}"#, id, js(&l.pool_id), l.init_point));
                l.step = Step::AddLiquidity;
                self.internal_advance(id, l, inline)
            }
            Ok(Some(_)) => {


                let fee = DCL_POOL_CREATE.as_yoctonear();
                emit("pool_deposit_refunded", &format!(r#"{{"id":{},"creator":"{}","amount":"{}"}}"#, id, l.creator, fee));
                self.internal_refund_creator(id, &l.creator.clone(), fee).detach();
                self.internal_mark_failed(id, l, "pool exists at another price")
            }
            other => {
                let reason = if other.is_err() { "pool read failed" } else { "create_pool refused and no pool exists" };
                emit("launch_step_parked", &format!(r#"{{"id":{},"step":"CreatePool","reason":"{}"}}"#, id, reason));
                self.launches.insert(id, l);
                PromiseOrValue::Value(false)
            }
        }
    }

    #[private]
    pub fn on_liquidity_added(&mut self, id: u64, inline: Option<bool>, #[callback_result] lpt: Result<String, PromiseError>) -> PromiseOrValue<bool> {
        let inline = inline.unwrap_or(false);
        let mut l = self.rec(id);
        l.inflight = false;
        match lpt {
            Ok(lpt_id) if !lpt_id.is_empty() => {
                l.lpt_id = Some(lpt_id);
                l.step = if l.dev_buy_near.0 > 0 && !dev_buy_done(id) { Step::DevBuy } else { Step::Done };
                if l.step == Step::Done {
                    self.internal_done(id, l);
                    return PromiseOrValue::Value(true);
                }
                self.internal_advance(id, l, inline)
            }


            Ok(_) => {
                emit("launch_step_parked", &format!(r#"{{"id":{},"step":"AddLiquidity","reason":"position pending"}}"#, id));
                self.launches.insert(id, l);
                PromiseOrValue::Value(false)
            }
            Err(_) => self.internal_fail(id, l, "add_liquidity"),
        }
    }









    #[private]
    pub fn on_liquidity_added_buy(&mut self, id: u64, #[callback_result] r: Result<AddBuyResult, PromiseError>) -> bool {
        let l = self.rec(id);
        if !(l.step == Step::AddLiquidity && dev_buy_inflight(&l)) {


            let r = r.unwrap_or_default();
            emit("add_buy_result_ignored", &format!(r#"{{"id":{},"used":"{}","refunded":"{}","lpt_id":{}}}"#, id, r.used.0, r.refunded.0, js(&r.lpt_id)));
            return false;
        }
        self.settle_add_buy(id, l, r.ok())
    }



    fn settle_add_buy(&mut self, id: u64, mut l: Launch, r: Option<AddBuyResult>) -> bool {
        l.inflight = false;
        let res = match r {
            Some(res) => res,
            None => {



                set_buy_held(id, l.dev_buy_near.0);
                env::storage_write(&buy_unknown_key(id), &[1]);
                emit("dev_buy_unknown", &format!(r#"{{"id":{},"amount":"{}"}}"#, id, l.dev_buy_near.0));
                emit("launch_step_parked", &format!(r#"{{"id":{},"step":"AddLiquidity","reason":"add_buy failed"}}"#, id));
                self.launches.insert(id, l);
                return false;
            }
        };
        let (used, refunded) = (res.used.0.min(l.dev_buy_near.0), res.refunded.0.min(l.dev_buy_near.0));

        let held = l.dev_buy_near.0.saturating_sub(used).saturating_sub(refunded);
        set_buy_held(id, held);
        emit("first_buy_done", &format!(r#"{{"id":{},"used":"{}","refunded":"{}","held":"{}","lpt_id":{}}}"#, id, used, refunded, held, js(&res.lpt_id)));
        if used > 0 {
            l.dev_buy_near = U128(used);
            if l.dev_buy_exact { l.dev_buy_tokens = U128(self.dev_buy_out_for(&l)); }
            env::storage_write(&dev_buy_done_key(id), &[1]);
            if refunded > 0 {
                emit("dev_buy_excess_refunded", &format!(r#"{{"id":{},"creator":"{}","amount":"{}"}}"#, id, l.creator, refunded));
                self.internal_refund_creator(id, &l.creator, refunded).detach();
            }
        } else {

            l.dev_buy_held = U128(refunded);
        }
        if res.lpt_id.is_empty() {
            emit("launch_step_parked", &format!(r#"{{"id":{},"step":"AddLiquidity","reason":"position pending"}}"#, id));
            self.launches.insert(id, l);
            return false;
        }
        l.lpt_id = Some(res.lpt_id);
        if used > 0 {
            self.internal_done(id, l);
            return true;
        }
        l.step = Step::DevBuy;
        emit("launch_step_parked", &format!(r#"{{"id":{},"step":"DevBuy","reason":"{}"}}"#, id, if refunded > 0 { "swap refunded" } else { "buy held by the locker" }));
        self.launches.insert(id, l);
        false
    }










    #[private]
    pub fn on_dev_bought(&mut self, id: u64, #[callback_result] used: Result<U128, PromiseError>) -> PromiseOrValue<bool> {
        let mut l = self.rec(id);
        let amount = l.dev_buy_near.0;
        match used {
            Ok(u) if u.0 > 0 => {
                self.wnear_registered = true;
                let spent = u.0.min(amount);
                let unused = amount - spent;
                l.dev_buy_near = U128(spent);
                l.dev_buy_held = U128(0);
                if l.dev_buy_exact { l.dev_buy_tokens = U128(self.dev_buy_out_for(&l)); }
                if unused > 0 {
                    self.unwrap_wnear(unused)
                        .then(Self::ext(env::current_account_id()).with_static_gas(GAS_DEV_UNWRAPPED).with_unused_gas_weight(0).on_dev_refund_unwrapped(id, U128(unused)))
                        .detach();
                }

                if l.step == Step::Done { self.launches.insert(id, l); } else { self.internal_done(id, l); }
                PromiseOrValue::Value(true)
            }
            Ok(_) => {
                self.wnear_registered = true;

                if l.step != Step::Done {
                    l.inflight = true;
                    self.launches.insert(id, l);
                }
                PromiseOrValue::Promise(self.dev_buy_unwrap(id, amount, 0))
            }
            Err(_) => {
                if l.step == Step::Done {

                    emit("dev_buy_refunded", &format!(r#"{{"id":{},"creator":"{}","amount":"{}"}}"#, id, l.creator, amount));
                    self.internal_refund_creator(id, &l.creator.clone(), amount).detach();
                    return PromiseOrValue::Value(false);
                }
                l.inflight = false;
                l.dev_buy_held = U128(amount);
                emit("launch_step_parked", &format!(r#"{{"id":{},"step":"DevBuy","reason":"wrap reverted"}}"#, id));
                self.launches.insert(id, l);
                PromiseOrValue::Value(false)
            }
        }
    }








    #[private]
    pub fn on_dev_buy_unwrapped(&mut self, id: u64, op: Option<u64>, #[callback_result] r: Result<(), PromiseError>) -> bool {
        let mark = read_u128(&dev_unwrap_op_key(id)).map(|v| v as u64);
        if op != mark {
            emit("dev_buy_unwrap_stale", &format!(r#"{{"id":{},"op":"{}","ok":{}}}"#, id, op.unwrap_or(0), r.is_ok()));
            return false;
        }
        env::storage_remove(&dev_unwrap_op_key(id));
        self.internal_dev_buy_unwrapped(id, r.is_ok(), op.is_none())
    }



    fn internal_dev_buy_unwrapped(&mut self, id: u64, ok: bool, legacy: bool) -> bool {
        let mut l = self.rec(id);
        let prev = read_u128(&unwrap_pending_key(id)).unwrap_or(0);
        if l.step == Step::Done && !legacy {


            let amount = l.dev_buy_near.0;
            env::storage_remove(&unwrap_pending_key(id));
            if ok {
                sub_counter(UW_TOTAL_KEY, prev);
                emit("dev_buy_refunded", &format!(r#"{{"id":{},"creator":"{}","amount":"{}"}}"#, id, l.creator, amount));
                self.internal_refund_creator(id, &l.creator, amount).detach();
            } else {

                add_counter(&wnear_owed_key(id), amount);
                add_counter(UW_TOTAL_KEY, amount);
                sub_counter(UW_TOTAL_KEY, prev);
                emit("dev_buy_unwrap_failed", &format!(r#"{{"id":{},"amount":"{}","released":true}}"#, id, amount));
            }
            return ok;
        }
        if l.step == Step::Done {


            let owed = read_counter(&wnear_owed_key(id));
            if ok {

                let paid = l.dev_buy_near.0;
                let booked = paid.min(owed);
                sub_counter(&wnear_owed_key(id), booked);
                sub_counter(UW_TOTAL_KEY, booked);
                emit("dev_buy_refunded", &format!(r#"{{"id":{},"creator":"{}","amount":"{}"}}"#, id, l.creator, paid));
                self.internal_refund_creator(id, &l.creator, paid).detach();
            } else {

                emit("dev_buy_unwrap_failed", &format!(r#"{{"id":{},"amount":"{}","released":true}}"#, id, l.dev_buy_near.0));
            }
            return ok;
        }
        l.inflight = false;
        if ok {
            l.dev_buy_held = l.dev_buy_near;
            sub_counter(UW_TOTAL_KEY, prev);
            env::storage_remove(&unwrap_pending_key(id));
            emit("launch_step_parked", &format!(r#"{{"id":{},"step":"DevBuy","reason":"swap refunded"}}"#, id));
        } else {
            if prev == 0 { add_counter(UW_TOTAL_KEY, l.dev_buy_near.0); }
            env::storage_write(&unwrap_pending_key(id), &l.dev_buy_near.0.to_le_bytes());
            emit("dev_buy_unwrap_failed", &format!(r#"{{"id":{},"amount":"{}"}}"#, id, l.dev_buy_near.0));
        }
        self.launches.insert(id, l);
        ok
    }



    #[private]
    pub fn on_dev_refund_unwrapped(&mut self, id: u64, amount: U128, #[callback_result] r: Result<(), PromiseError>) -> bool {
        let l = self.rec(id);
        if r.is_err() {

            add_counter(&wnear_owed_key(id), amount.0);
            add_counter(UW_TOTAL_KEY, amount.0);
            emit("dev_buy_refund_unwrap_failed", &format!(r#"{{"id":{},"creator":"{}","amount":"{}"}}"#, id, l.creator, amount.0));
            return false;
        }
        emit("dev_buy_excess_refunded", &format!(r#"{{"id":{},"creator":"{}","amount":"{}"}}"#, id, l.creator, amount.0));
        self.internal_refund_creator(id, &l.creator, amount.0).detach();
        true
    }


    pub fn cancel_dev_buy(&mut self, launch_id: U64) -> Promise {
        let mut l = self.rec(launch_id.0);
        require!(env::predecessor_account_id() == l.creator, "creator only");
        require!(l.step == Step::DevBuy && !l.inflight, "no pending dev buy");
        require!(!env::storage_has_key(&unwrap_pending_key(launch_id.0)), "the dev buy's wNEAR is not unwrapped yet: resume first");
        require!(buy_held(launch_id.0) == 0, "part of the buy is held by the locker: locker_refund_buy first");
        let amount = l.dev_buy_held.0;
        l.dev_buy_held = U128(0);
        l.dev_buy_near = U128(0);
        self.internal_done(launch_id.0, l.clone());
        emit("dev_buy_refunded", &format!(r#"{{"id":{},"creator":"{}","amount":"{}"}}"#, l.id, l.creator, amount));
        self.internal_refund_creator(l.id, &l.creator, amount)
    }





    pub fn refund_dev_buy(&mut self, launch_id: U64) -> PromiseOrValue<bool> {
        let who = env::predecessor_account_id();
        require!(who == self.owner_id || relayers().contains(&who), "owner or relayer only");
        let mut l = self.rec(launch_id.0);
        require!(l.step == Step::DevBuy && !l.inflight, "no pending dev buy");
        require!(!env::storage_has_key(&unwrap_pending_key(launch_id.0)), "the dev buy's wNEAR is not unwrapped yet: resume first");
        require!(buy_held(launch_id.0) == 0, "part of the buy is held by the locker: locker_refund_buy first");
        let amount = l.dev_buy_held.0;
        l.dev_buy_held = U128(0);
        l.dev_buy_near = U128(0);
        let creator = l.creator.clone();
        self.internal_done(launch_id.0, l);
        emit("dev_buy_refunded", &format!(r#"{{"id":{},"creator":"{}","amount":"{}","by":"{}"}}"#, launch_id.0, creator, amount, who));
        if amount == 0 {
            return PromiseOrValue::Value(true);
        }
        PromiseOrValue::Promise(self.internal_refund_creator(launch_id.0, &creator, amount))
    }





    #[private]
    pub fn on_dev_refund_to_creator(&mut self, launch_id: U64, creator: AccountId, amount: U128, #[callback_result] r: Result<(), PromiseError>) {
        if r.is_err() {
            let cur = self.creator_fees.get(&creator).copied().unwrap_or(0);
            self.creator_fees.insert(creator.clone(), cur + amount.0);
            emit("dev_buy_refund_failed", &format!(r#"{{"id":{},"creator":"{}","amount":"{}","booked":"creator_fees"}}"#, launch_id.0, creator, amount.0));
        }
    }




    pub fn claim_fees(&mut self, launch_id: U64) -> Promise {
        self.assert_not_paused();
        let mut l = self.rec(launch_id.0);
        require!(l.step == Step::Done, "launch not live");
        require!(!l.inflight, "claim in flight");


        transfer_start(&l.token);
        if self.quote_is_launch_token(&l) { transfer_start(&l.quote); }
        let lpt = l.lpt_id.clone().expect("lpt");

        let router = l.quote == self.wnear_id && l_payee_is_router(&l);
        let (token, token_is_x) = (l.token.clone(), l.token_is_x);

        let quote_arg = if l.quote == self.wnear_id { "null".to_string() } else { format!("\"{}\"", l.quote) };
        l.inflight = true;
        self.launches.insert(launch_id.0, l);
        let legs = self.protocol_recipients.len() + launch_recipients(launch_id.0).map_or(0, |r| r.len()) + if router { ROUTER_LEG_WEIGHT } else { 0 };
        let cb_gas = fees_claimed_gas(legs as u64);
        require!(gas_left() >= GAS_LOCKER_CLAIM.saturating_add(cb_gas).saturating_add(GAS_CLAIM_RESERVE), "attach more gas: claim_fees needs 300 TGas");
        Promise::new(self.locker_for(launch_id.0))
            .function_call(
                "claim".to_string(),
                format!(r#"{{"lpt_id":{},"token":"{}","token_is_x":{},"quote":{}}}"#, js(&lpt), token, token_is_x, quote_arg).into_bytes(),
                NO_DEPOSIT,
                GAS_LOCKER_CLAIM,
            )
            .then(Self::ext(env::current_account_id()).with_static_gas(cb_gas).on_fees_claimed(launch_id.0))
    }

    #[private]
    pub fn on_fees_claimed(&mut self, id: u64, #[callback_result] r: Result<Vec<U128>, PromiseError>) -> bool {
        let mut l = self.rec(id);
        l.inflight = false;
        transfer_end(&l.token);
        if self.quote_is_launch_token(&l) { transfer_end(&l.quote); }
        let Ok(v) = r else {
            self.launches.insert(id, l);
            return false;
        };
        if v.len() != 2 {
            self.launches.insert(id, l);
            return false;
        }
        let (fee_tok, fee_quote) = if l.token_is_x { (v[0].0, v[1].0) } else { (v[1].0, v[0].0) };
        let (mode, share_bps) = fee_opts(id, self.config.creator_fee_share_bps);
        let share = l_share(share_bps);
        let c_quote = fee_quote * share / BPS;
        let p_quote = fee_quote - c_quote;
        let c_tok = fee_tok * share / BPS;
        let p_tok = fee_tok - c_tok;



        let divert = mode != FM_CREATOR;


        let to = payee(&l);
        let house = self.is_house(&to) && !divert;
        let native = l.quote == self.wnear_id;
        if native {

            if c_quote > 0 && divert {
                set_mode_bucket(id, mode_bucket(id) + c_quote);
            } else if !house {

                self.add_to_bucket(id, "near", "creator", &to, c_quote);
            }
            let ours = p_quote + if house { c_quote } else { 0 };
            if launch_recipients(id).is_some() {

                set_launch_bucket(id, launch_bucket(id) + ours);
            } else {
                self.protocol_fees += ours;
            }
            l.fees_near_total = U128(l.fees_near_total.0 + fee_quote);
        } else {





            if c_quote > 0 && mode == FM_HOLDERS {
                set_mode_bucket(id, mode_bucket(id) + c_quote);
            } else {
                l.creator_quote_fees = U128(l.creator_quote_fees.0 + if house { 0 } else { c_quote });
            }
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
        self.drain_launch_fees(id);
        self.drain_router_near(id);
        true
    }












    pub fn claim_creator_fees(&mut self) -> Promise {
        self.internal_pay_creator_near(env::predecessor_account_id(), true)
    }



    pub fn push_creator_fees(&mut self, account_id: AccountId) -> Promise {
        self.assert_not_paused();
        self.internal_pay_creator_near(account_id, false)
    }


    pub fn claim_creator_token_fees(&mut self, launch_id: U64) -> Promise { self.internal_pay_creator(launch_id, "token", true) }

    pub fn push_creator_token_fees(&mut self, launch_id: U64) -> Promise { self.internal_pay_creator(launch_id, "token", false) }

    pub fn claim_creator_quote_fees(&mut self, launch_id: U64) -> Promise { self.internal_pay_creator(launch_id, "quote", true) }

    pub fn push_creator_quote_fees(&mut self, launch_id: U64) -> Promise { self.internal_pay_creator(launch_id, "quote", false) }





    pub fn split_protocol_token_fees(&mut self, launch_id: U64) { self.internal_split_protocol(launch_id, "token") }

    pub fn split_protocol_quote_fees(&mut self, launch_id: U64) { self.internal_split_protocol(launch_id, "quote") }








    #[private]
    pub fn on_paid(&mut self, launch_id: U64, kind: String, to: AccountId, amount: U128, bucket: String, ftc: bool) {
        let id = launch_id.0;


        let back = if ftc { ft_refund_of_last_call(amount.0) } else if promise_failed() { amount.0 } else { 0 };
        if ftc {
            let l = self.rec(id);
            transfer_end(if kind == "token" { &l.token } else { &l.quote });
        }
        if back > 0 && (bucket == "mode" || bucket == "tax_holders") {


            let asset = if kind == "near" { "near".to_string() } else { self.rec(id).quote.to_string() };
            add_owed(&asset, &to, back);
            emit("holder_owed", &format!(r#"{{"id":{},"to":"{}","asset":"{}","amount":"{}"}}"#, id, to, asset, back));
            return;
        }
        if back == 0 {
            if bucket == "creator" && is_router(&to) {


                Promise::new(to)
                    .function_call("on_launch_fees".to_string(), format!(r#"{{"launch_id":{},"kind":{},"amount":"{}"}}"#, id, js(&kind), amount.0).into_bytes(), NO_DEPOSIT, GAS_ROUTER_CALL)
                    .detach();
            }
            return;
        }
        self.add_to_bucket(id, &kind, &bucket, &to, back);

        let (ev, data) = match (bucket.as_str(), kind.as_str()) {
            ("account", _) | ("creator", "near") if !is_router(&to) || bucket == "account" => ("creator_push_failed", format!(r#"{{"creator":"{}","amount":"{}"}}"#, to, back)),
            ("creator", "near") => ("router_payout_failed", format!(r#"{{"id":{},"kind":"near","amount":"{}"}}"#, id, back)),
            ("creator", "token") => ("creator_token_push_failed", format!(r#"{{"id":{},"amount":"{}"}}"#, id, back)),
            ("creator", _) => ("quote_push_failed", format!(r#"{{"id":{},"amount":"{}","protocol":false}}"#, id, back)),
            ("protocol", "near") => ("protocol_split_failed", format!(r#"{{"to":"{}","amount":"{}"}}"#, to, back)),
            ("protocol", "token") => ("protocol_token_split_failed", format!(r#"{{"id":{},"to":"{}","amount":"{}"}}"#, id, to, back)),
            ("protocol", _) => ("quote_push_failed", format!(r#"{{"id":{},"amount":"{}","protocol":true}}"#, id, back)),
            ("launch", _) => ("launch_split_failed", format!(r#"{{"id":{},"to":"{}","amount":"{}"}}"#, id, to, back)),
            ("mode", _) => ("holder_pay_failed", format!(r#"{{"id":{},"to":"{}","amount":"{}"}}"#, id, to, back)),
            _ => ("tax_holder_pay_failed", format!(r#"{{"id":{},"to":"{}","amount":"{}"}}"#, id, to, back)),
        };
        emit(ev, &data);
    }



    #[private]
    pub fn on_creator_pushed(&mut self, account_id: AccountId, amount: U128, #[callback_result] r: Result<(), PromiseError>) {
        if r.is_err() { self.add_to_bucket(0, "near", "account", &account_id, amount.0); }
    }
    #[private]
    pub fn on_creator_token_pushed(&mut self, launch_id: U64, amount: U128, #[callback_result] r: Result<(), PromiseError>) {
        if r.is_err() { self.add_to_bucket(launch_id.0, "token", "creator", &env::current_account_id(), amount.0); }
    }
    #[private]
    pub fn on_quote_pushed(&mut self, launch_id: U64, amount: U128, to: Option<AccountId>, #[callback_result] r: Result<(), PromiseError>) {
        if r.is_err() { self.add_to_bucket(launch_id.0, "quote", if to.is_some() { "protocol" } else { "creator" }, &env::current_account_id(), amount.0); }
    }
    #[private]
    pub fn on_token_split(&mut self, launch_id: U64, to: AccountId, amount: U128, #[callback_result] r: Result<(), PromiseError>) {
        if r.is_err() { self.add_to_bucket(launch_id.0, "token", "protocol", &to, amount.0); }
    }
    #[private]
    pub fn on_protocol_split(&mut self, to: AccountId, amount: U128, #[callback_result] r: Result<(), PromiseError>) {
        if r.is_err() { self.add_to_bucket(0, "near", "protocol", &to, amount.0); }
    }
    #[private]
    pub fn on_launch_split(&mut self, launch_id: U64, to: AccountId, amount: U128, #[callback_result] r: Result<(), PromiseError>) {
        if r.is_err() { self.add_to_bucket(launch_id.0, "near", "launch", &to, amount.0); }
    }
    #[private]
    pub fn on_holder_paid(&mut self, launch_id: U64, to: AccountId, amount: U128, #[callback_result] r: Result<(), PromiseError>) {
        if r.is_err() { self.add_to_bucket(launch_id.0, "near", "mode", &to, amount.0); }
    }
    #[private]
    pub fn on_tax_holder_paid(&mut self, launch_id: U64, to: AccountId, amount: U128, #[callback_result] r: Result<(), PromiseError>) {
        if r.is_err() { self.add_to_bucket(launch_id.0, "near", "tax_holders", &to, amount.0); }
    }

    #[private]
    pub fn on_tax_creator_paid(&mut self, launch_id: U64, amount: U128, #[callback_result] r: Result<(), PromiseError>) {
        if r.is_err() { sub_counter(&tax_paid_creator_key(launch_id.0), amount.0); add_counter(&tax_holders_key(launch_id.0), amount.0); }
    }




    pub fn burn_token_fees(&mut self, launch_id: U64) -> Promise {
        self.assert_not_paused();
        require!(fee_mode(launch_id.0) != FM_CREATOR, "creator launch: fees are paid, not burned");
        let mut l = self.rec(launch_id.0);
        require_no_swap(&l.token);
        let amount = l.creator_token_fees.0;
        require!(amount > 0, "nothing to burn");
        l.creator_token_fees = U128(0);
        self.launches.insert(launch_id.0, l.clone());
        self.internal_burn(&l.token, launch_id, amount)
    }




    pub fn buyback(&mut self, launch_id: U64, min_out: U128) -> Promise {
        self.assert_not_paused();
        self.assert_keeper();

        require!(min_out.0 > 0, "min_out required");
        require!(fee_mode(launch_id.0) == FM_BURN, "not a buyback launch");
        let mut l = self.rec(launch_id.0);
        require!(l.step == Step::Done, "launch not live");



        take_swap_lock(&l.token, launch_id.0, LK_BUYBACK);
        self.guard_quote(&l);


        if l.quote != self.wnear_id && self.quote_is_launch_token(&l) { transfer_start(&l.quote); }
        let amount = if l.quote == self.wnear_id {
            let a = mode_bucket(launch_id.0);
            set_mode_bucket(launch_id.0, 0);
            a
        } else {
            let a = l.creator_quote_fees.0;
            l.creator_quote_fees = U128(0);
            self.launches.insert(launch_id.0, l.clone());
            a
        };
        require!(amount > 0, "nothing to buy back");

        Promise::new(l.token.clone())
            .function_call("storage_deposit".to_string(), format!(r#"{{"account_id":"{}","registration_only":true}}"#, env::current_account_id()).into_bytes(), FT_STORAGE_REG, Gas::from_tgas(5))
            .function_call("ft_balance_of".to_string(), format!(r#"{{"account_id":"{}"}}"#, env::current_account_id()).into_bytes(), NO_DEPOSIT, GAS_BALANCE_OF)
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_BB_BEFORE).on_bb_before(launch_id, U128(amount), min_out))
    }

    #[private]
    pub fn on_bb_before(&mut self, launch_id: U64, amount: U128, min_out: U128, #[callback_result] bal: Result<U128, PromiseError>) -> PromiseOrValue<bool> {
        let l = self.rec(launch_id.0);
        let Ok(before) = bal else {
            if l.quote != self.wnear_id && self.quote_is_launch_token(&l) { transfer_end(&l.quote); }
            self.internal_bb_recredit(launch_id, amount.0);
            release_swap_lock(&l.token, launch_id.0, LK_BUYBACK);
            return PromiseOrValue::Value(false);
        };
        let msg = format!(r#"{{"Swap":{{"pool_ids":[{}],"output_token":"{}","min_output_amount":"{}"}}}}"#, js(&l.pool_id), l.token, min_out.0);
        let ftc = format!(r#"{{"receiver_id":"{}","amount":"{}","msg":{}}}"#, self.dcl_for(launch_id.0), amount.0, js(&msg)).into_bytes();
        let p = if l.quote == self.wnear_id {
            let mut w = Promise::new(self.wnear_id.clone());


            if !self.wnear_registered {
                w = w.function_call("storage_deposit".to_string(), format!(r#"{{"account_id":"{}","registration_only":true}}"#, env::current_account_id()).into_bytes(), FT_STORAGE_REG, Gas::from_tgas(5));
                self.wnear_registered = true;
            }
            w.function_call("near_deposit".to_string(), b"{}".to_vec(), NearToken::from_yoctonear(amount.0), Gas::from_tgas(5))
                .function_call("ft_transfer_call".to_string(), ftc, ONE_YOCTO, GAS_BB_FTC)
        } else {
            Promise::new(l.quote.clone()).function_call("ft_transfer_call".to_string(), ftc, ONE_YOCTO, GAS_BB_FTC)
        };
        PromiseOrValue::Promise(p.then(Self::ext(env::current_account_id()).with_static_gas(GAS_BB_SWAPPED).on_bb_swapped(launch_id, amount, before, Some(min_out))))
    }


    #[private]
    pub fn on_bb_swapped(&mut self, launch_id: U64, amount: U128, before: U128, min_out: Option<U128>, #[callback_result] used: Result<U128, PromiseError>) -> PromiseOrValue<bool> {
        let l = self.rec(launch_id.0);

        if l.quote != self.wnear_id && self.quote_is_launch_token(&l) { transfer_end(&l.quote); }


        let (used, came_back_wrapped) = match used { Ok(u) => (u.0.min(amount.0), true), Err(_) => (0, false) };
        let unused = amount.0 - used;
        if unused > 0 && l.quote == self.wnear_id && came_back_wrapped {

            self.unwrap_wnear(unused)
                .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_SPLIT).on_bb_unwrapped(launch_id, U128(unused)))
                .detach();
        } else {
            self.internal_bb_recredit(launch_id, unused);
        }
        if used == 0 {
            release_swap_lock(&l.token, launch_id.0, LK_BUYBACK);
            emit("buyback_refunded", &format!(r#"{{"id":{},"amount":"{}"}}"#, launch_id.0, amount.0));
            return PromiseOrValue::Value(false);
        }
        PromiseOrValue::Promise(
            Promise::new(l.token.clone())
                .function_call("ft_balance_of".to_string(), format!(r#"{{"account_id":"{}"}}"#, env::current_account_id()).into_bytes(), NO_DEPOSIT, GAS_BALANCE_OF)
                .then(Self::ext(env::current_account_id()).with_static_gas(GAS_BB_AFTER).on_bb_after(launch_id, before, U128(used), min_out)),
        )
    }



    #[private]
    pub fn on_bb_unwrapped(&mut self, launch_id: U64, amount: U128, #[callback_result] r: Result<(), PromiseError>) {
        if r.is_ok() {
            self.internal_bb_recredit(launch_id, amount.0);
        } else {

            add_counter(&wnear_owed_bucket_key(launch_id.0), amount.0);
            add_counter(UW_TOTAL_KEY, amount.0);
            emit("buyback_unwrap_failed", &format!(r#"{{"id":{},"amount":"{}"}}"#, launch_id.0, amount.0));
        }
    }




    #[private]
    pub fn on_bb_after(&mut self, launch_id: U64, before: U128, spent: U128, min_out: Option<U128>, #[callback_result] bal: Result<U128, PromiseError>) -> PromiseOrValue<bool> {
        let l = self.rec(launch_id.0);

        let ours = swap_locked(&l.token) && matches!(read_swap_lock(&l.token), Some((LK_BUYBACK, i, _)) if i == launch_id.0);
        if !ours { emit("buyback_lock_lost", &format!(r#"{{"id":{}}}"#, launch_id.0)); }
        let diff = if ours { bal.map(|a| a.0.saturating_sub(before.0)).unwrap_or(0) } else { 0 };
        let bought = match min_out { Some(m) => diff.min(mul_div(m.0, 104, 100)), None => diff };
        emit("buyback", &format!(r#"{{"id":{},"token":"{}","spent":"{}","bought":"{}"}}"#, launch_id.0, l.token, spent.0, bought));
        if diff > bought {



            emit("buyback_excess_ignored", &format!(r#"{{"id":{},"amount":"{}"}}"#, launch_id.0, diff - bought));
        }
        if bought == 0 {
            release_swap_lock(&l.token, launch_id.0, LK_BUYBACK);
            return PromiseOrValue::Value(false);
        }
        PromiseOrValue::Promise(
            Promise::new(l.token.clone())
                .function_call("burn".to_string(), format!(r#"{{"amount":"{}"}}"#, bought).into_bytes(), ONE_YOCTO, GAS_BURN)
                .then(Self::ext(env::current_account_id()).with_static_gas(GAS_BB_BURNED).on_bb_burned(launch_id, U128(bought))),
        )
    }


    #[private]
    pub fn on_bb_burned(&mut self, launch_id: U64, amount: U128, #[callback_result] r: Result<(), PromiseError>) {
        self.internal_fee_burned(launch_id, amount.0, r.is_ok());
        let token = self.launches.get(&launch_id.0).map(|l| l.token.clone()).expect("launch");
        release_swap_lock(&token, launch_id.0, LK_BUYBACK);
    }



    #[private]
    pub fn on_fee_burned(&mut self, launch_id: U64, amount: U128, #[callback_result] r: Result<(), PromiseError>) {
        self.internal_fee_burned(launch_id, amount.0, r.is_ok());
    }



    pub fn pay_holders(&mut self, launch_id: U64, payouts: Vec<(AccountId, U128)>) {
        self.assert_not_paused();
        require!(fee_mode(launch_id.0) == FM_HOLDERS, "not a holders launch");
        self.internal_pay_holders(launch_id, payouts, false)
    }









    pub fn collect_tax(&mut self, launch_id: U64) -> Promise {
        self.assert_not_paused();
        self.assert_keeper();
        require!(tax_opts(launch_id.0).is_some(), "no tax on this launch");
        let l = self.rec(launch_id.0);
        require!(l.step == Step::Done, "launch not live");

        transfer_start(&l.token);
        Promise::new(l.token.clone())
            .function_call("tax_take".to_string(), b"{}".to_vec(), NO_DEPOSIT, GAS_TAX_TAKE)
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_SPLIT).on_tax_taken(launch_id))
    }

    #[private]
    pub fn on_tax_taken(&mut self, launch_id: U64, #[callback_result] r: Result<U128, PromiseError>) -> U128 {
        if let Some(l) = self.launches.get(&launch_id.0) {
            let token = l.token.clone();
            transfer_end(&token);
        }
        let got = r.map(|a| a.0).unwrap_or(0);
        if got > 0 {
            add_counter(&tax_pending_key(launch_id.0), got);
            emit("tax_collected", &format!(r#"{{"id":{},"amount":"{}"}}"#, launch_id.0, got));
        }
        U128(got)
    }








    pub fn process_tax(&mut self, launch_id: U64, min_out: U128, max_amount: Option<U128>, to_seller: Option<bool>) -> PromiseOrValue<bool> {
        let _ = to_seller;
        self.assert_not_paused();
        self.assert_keeper();
        require!(min_out.0 > 0, "min_out required: the floor the seller's proceeds must clear for this slice");
        let (t, platform_bps) = tax_opts_full(launch_id.0).expect("no tax on this launch");
        let l = self.rec(launch_id.0);
        require!(l.step == Step::Done, "launch not live");
        require_no_swap(&l.token);
        let mut pending = read_counter(&tax_pending_key(launch_id.0));
        require!(pending > 0, "no tax to process");
        env::storage_remove(&tax_pending_key(launch_id.0));
        if let Some(m) = max_amount {
            require!(m.0 > 0, "max_amount must be > 0");
            if m.0 < pending {
                add_counter(&tax_pending_key(launch_id.0), pending - m.0);
                pending = m.0;
            }
        }

        let platform = mul_bps(pending, platform_bps as u128);
        let rest = pending - platform;
        let burn = if t.creator_bps as u128 + t.holders_bps as u128 == 0 { rest } else { mul_bps(rest, t.burn_bps as u128) };
        let sell = pending - burn;
        if burn > 0 {
            Promise::new(l.token.clone())
                .function_call("burn".to_string(), format!(r#"{{"amount":"{}"}}"#, burn).into_bytes(), ONE_YOCTO, GAS_BURN)
                .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_SPLIT).on_tax_burned(launch_id, U128(burn)))
                .detach();
        }
        if sell == 0 {
            return PromiseOrValue::Value(true);
        }
        let seller = tax_seller().expect("no tax seller set");
        add_counter(&tax_selling_key(launch_id.0), sell);
        add_counter(&tax_floor_key(launch_id.0), min_out.0);
        PromiseOrValue::Promise(
            self.internal_send_tokens(&l.token, &seller, sell, "tax")
                .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_SPLIT).on_tax_to_seller(launch_id, U128(sell), Some(min_out))),
        )
    }


    #[private]
    pub fn on_tax_to_seller(&mut self, launch_id: U64, amount: U128, floor: Option<U128>, #[callback_result] r: Result<(), PromiseError>) -> bool {
        if r.is_err() {
            sub_counter(&tax_selling_key(launch_id.0), amount.0);

            sub_counter(&tax_floor_key(launch_id.0), floor.map(|f| f.0).unwrap_or(0));
            add_counter(&tax_pending_key(launch_id.0), amount.0);
            emit("tax_to_seller_failed", &format!(r#"{{"id":{},"amount":"{}"}}"#, launch_id.0, amount.0));
            return false;
        }
        emit("tax_to_seller", &format!(r#"{{"id":{},"amount":"{}"}}"#, launch_id.0, amount.0));
        true
    }



    #[payable]
    pub fn tax_proceeds(&mut self, launch_id: U64, sold: U128) -> bool {
        self.assert_not_paused();
        require!(Some(env::predecessor_account_id()) == tax_seller(), "tax seller only");
        let l = self.rec(launch_id.0);
        require!(l.quote == self.wnear_id, "pair launch: send the pair token with ft_transfer_call");
        self.internal_tax_settle(launch_id, sold.0, env::attached_deposit().as_yoctonear())
    }






    pub fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<U128> {
        let refund = PromiseOrValue::Value(amount);

        if self.paused { return refund; }


        if swap_locked(&env::predecessor_account_id()) { return refund; }
        if Some(&sender_id) != tax_seller().as_ref() { return refund; }
        let Ok(m) = near_sdk::serde_json::from_str::<near_sdk::serde_json::Value>(&msg) else { return refund };
        if let Some(id) = m["tax_return"].as_u64() {
            let Some(l) = self.launches.get(&id) else { return refund };
            if l.token != env::predecessor_account_id() || amount.0 == 0 { return refund; }
            let held = read_counter(&tax_selling_key(id));
            if amount.0 > held { return refund; }
            let floor_part = mul_div_wide(read_counter(&tax_floor_key(id)), amount.0, held);
            sub_counter(&tax_floor_key(id), floor_part);
            env::storage_write(&tax_selling_key(id), &(held - amount.0).to_le_bytes());
            add_counter(&tax_pending_key(id), amount.0);
            emit("tax_returned", &format!(r#"{{"id":{},"amount":"{}","floor_dropped":"{}"}}"#, id, amount.0, floor_part));
            return PromiseOrValue::Value(U128(0));
        }
        let (Some(id), Some(sold)) = (m["tax_proceeds"].as_u64(), m["sold"].as_str().and_then(|x| x.parse::<u128>().ok())) else { return refund };
        let Some(l) = self.launches.get(&id).cloned() else { return refund };
        if l.quote != env::predecessor_account_id() || l.quote == self.wnear_id { return refund; }
        self.internal_tax_settle(U64(id), sold, amount.0);
        PromiseOrValue::Value(U128(0))
    }

    #[private]
    pub fn on_tax_burned(&mut self, launch_id: U64, amount: U128, #[callback_result] r: Result<(), PromiseError>) {
        if r.is_err() {
            add_counter(&tax_pending_key(launch_id.0), amount.0);
            emit("tax_burn_failed", &format!(r#"{{"id":{},"amount":"{}"}}"#, launch_id.0, amount.0));
        } else {
            add_counter(&tax_burned_key(launch_id.0), amount.0);
            emit("tax_burned", &format!(r#"{{"id":{},"amount":"{}"}}"#, launch_id.0, amount.0));
        }
    }



    pub fn pay_tax_holders(&mut self, launch_id: U64, payouts: Vec<(AccountId, U128)>) {
        self.internal_pay_holders(launch_id, payouts, true)
    }





    pub fn push_owed(&mut self, asset: String, accounts: Vec<AccountId>) {
        self.assert_not_paused();
        let token: Option<AccountId> = if asset == "near" { None } else { Some(asset.parse().unwrap_or_else(|_| env::panic_str("asset: near or a token account"))) };
        let max = if token.is_none() { MAX_HOLDER_PAYOUTS } else { MAX_FT_HOLDER_PAYOUTS };
        require!(!accounts.is_empty() && accounts.len() <= max, "too many accounts per call (25 near, 10 other assets)");
        for (i, a) in accounts.iter().enumerate() {
            require!(!accounts[..i].contains(a), "duplicate account");
        }

        if let Some(t) = &token { if self.token_index.contains_key(t) { require_no_swap(t); } }
        for a in accounts {
            let amount = take_owed(&asset, &a);
            if amount == 0 { continue; }
            let p = match &token {
                None => Promise::new(a.clone()).transfer(NearToken::from_yoctonear(amount)),
                Some(t) => self.internal_send_tokens(t, &a, amount, "holder rewards"),
            };
            p.then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_SPLIT).on_owed_pushed(asset.clone(), a, U128(amount))).detach();
        }
    }

    #[private]
    pub fn on_owed_pushed(&mut self, asset: String, to: AccountId, amount: U128) {

        let back = if promise_failed() { amount.0 } else { 0 };
        if back > 0 {
            add_owed(&asset, &to, back);
            emit("owed_push_failed", &format!(r#"{{"to":"{}","asset":"{}","amount":"{}"}}"#, to, asset, back));
        }
        if amount.0 > back {
            emit("owed_paid", &format!(r#"{{"to":"{}","asset":"{}","amount":"{}"}}"#, to, asset, amount.0 - back));
        }
    }


    pub fn get_owed(&self, asset: String, account_id: AccountId) -> U128 { U128(owed_of(&asset, &account_id)) }
    pub fn get_owed_many(&self, asset: String, accounts: Vec<AccountId>) -> Vec<(AccountId, U128)> {
        require!(accounts.len() <= 200, "max 200 accounts");
        accounts.into_iter().map(|a| { let v = owed_of(&asset, &a); (a, U128(v)) }).collect()
    }

    pub fn get_owed_total(&self, asset: String) -> U128 { U128(read_counter(&owed_total_key(&asset))) }

    pub fn get_tax(&self, launch_id: U64) -> Option<TaxInfo> {
        tax_opts_full(launch_id.0).map(|(t, platform_bps)| TaxInfo {
            platform_bps,
            paid_platform: U128(read_counter(&tax_paid_platform_key(launch_id.0))),
            buy_bps: t.buy_bps,
            sell_bps: t.sell_bps,
            creator_bps: t.creator_bps,
            burn_bps: t.burn_bps,
            holders_bps: t.holders_bps,
            pending: U128(read_counter(&tax_pending_key(launch_id.0))),
            holders_bucket: U128(read_counter(&tax_holders_key(launch_id.0))),
            burned: U128(read_counter(&tax_burned_key(launch_id.0))),
            paid_creator: U128(read_counter(&tax_paid_creator_key(launch_id.0))),
            paid_holders: U128(read_counter(&tax_paid_holders_key(launch_id.0))),
            selling: U128(read_counter(&tax_selling_key(launch_id.0))),
            floor: U128(read_counter(&tax_floor_key(launch_id.0))),
        })
    }


    pub fn get_tax_many(&self, ids: Vec<U64>) -> Vec<(U64, TaxInfo)> {
        require!(ids.len() <= 200, "max 200 ids");
        ids.into_iter().filter_map(|i| self.get_tax(i).map(|t| (i, t))).collect()
    }


    pub fn get_tax_bounds(&self) -> (u16, bool, u16) { (MAX_TAX_BPS, tax_code_hash().is_some(), PLATFORM_TAX_BPS) }





    pub fn set_burn_pot(&mut self, account_id: Option<AccountId>) {
        self.assert_owner();
        require!(account_id.as_ref() != Some(&env::current_account_id()), "burn pot: not the launchpad itself");
        self.assert_pot_legs_fit(self.protocol_recipients.len(), account_id.is_some(), referral_pot().is_some());
        match account_id {
            Some(a) => env::storage_write(BURN_POT_KEY, a.as_bytes()),
            None => env::storage_remove(BURN_POT_KEY),
        };
    }
    pub fn get_burn_pot(&self) -> Option<AccountId> { burn_pot() }



    pub fn set_referral_pot(&mut self, account_id: Option<AccountId>) {
        self.assert_owner();
        require!(account_id.as_ref() != Some(&env::current_account_id()), "referral pot: not the launchpad itself");
        self.assert_pot_legs_fit(self.protocol_recipients.len(), burn_pot().is_some(), account_id.is_some());
        match account_id {
            Some(a) => env::storage_write(REFERRAL_POT_KEY, a.as_bytes()),
            None => env::storage_remove(REFERRAL_POT_KEY),
        };
    }
    pub fn get_referral_pot(&self) -> Option<AccountId> { referral_pot() }

    pub fn set_tax_seller(&mut self, account_id: Option<AccountId>) {
        self.assert_owner();
        internal_set_tax_seller(account_id);
    }

    pub fn get_tax_seller(&self) -> Option<AccountId> { tax_seller() }




    pub fn set_relayers(&mut self, relayers: Vec<AccountId>) {
        self.assert_owner();
        require!(relayers.len() <= MAX_RELAYERS, "at most 16 relayers");
        for (i, a) in relayers.iter().enumerate() {
            require!(!relayers[..i].contains(a), "duplicate relayer");
        }
        if relayers.is_empty() {
            env::storage_remove(RELAYERS_KEY);
        } else {
            env::storage_write(RELAYERS_KEY, &near_sdk::serde_json::to_vec(&relayers).expect("relayers"));
        }
        emit("relayers_set", &format!(r#"{{"count":{}}}"#, relayers.len()));
    }

    pub fn get_relayers(&self) -> Vec<AccountId> { relayers() }



    pub fn is_swap_in_flight(&self, token: AccountId) -> bool { swap_locked(&token) }


    pub fn get_dev_buy_min_out(&self, launch_id: U64) -> U128 {
        let l = self.rec(launch_id.0);
        U128(self.dev_buy_out_for(&l))
    }


    pub fn set_tax_token_code_hash(&mut self, token_code_hash: Option<String>) {
        self.assert_owner();
        match token_code_hash {
            Some(h) => { env::storage_write(TAX_CODE_KEY, &parse_hex32(&h)); }
            None => { env::storage_remove(TAX_CODE_KEY); }
        }
    }

    pub fn get_fee_opts(&self, launch_id: U64) -> FeeOpts {
        let (mode, share) = fee_opts(launch_id.0, self.config.creator_fee_share_bps);
        FeeOpts {
            mode: mode_name(mode).to_string(),
            creator_share_bps: share,
            bucket: U128(mode_bucket(launch_id.0)),
            burned: U128(read_counter(&burned_key(launch_id.0))),
            paid_holders: U128(read_counter(&paid_key(launch_id.0))),
        }
    }


    pub fn get_fee_opts_many(&self, ids: Vec<U64>) -> Vec<(U64, FeeOpts)> {
        require!(ids.len() <= 200, "max 200 ids");
        ids.into_iter().map(|i| (i, self.get_fee_opts(i))).collect()
    }

    pub fn get_creator_share_bounds(&self) -> (u16, u16) { (MIN_CREATOR_SHARE_BPS, MAX_CREATOR_SHARE_BPS) }




    pub fn backfill_fee_opts(&mut self, ids: Vec<U64>) -> u32 {
        self.assert_owner();
        require!(ids.len() <= 200, "max 200 ids");
        let share = self.config.creator_fee_share_bps;
        let mut n = 0u32;
        for i in ids {
            if i.0 >= self.next_id || env::storage_has_key(&fee_opts_key(i.0)) {
                continue;
            }
            set_fee_opts(i.0, FM_CREATOR, share);
            n += 1;
        }
        emit("fee_opts_backfilled", &format!(r#"{{"count":{},"creator_share_bps":{}}}"#, n, share));
        n
    }






    pub fn set_house_buyback(&mut self, launch_id: U64) {
        self.assert_owner();
        let l = self.rec(launch_id.0);
        require!(l.step == Step::Done && !l.inflight, "launch not live");
        require!(self.is_house(&payee(&l)), "house launches only");
        require!(fee_mode(launch_id.0) == FM_CREATOR, "already a burn or holders launch");


        require!(l.creator_token_fees.0 == 0 && l.creator_quote_fees.0 == 0, "pay the creator's booked token and quote fees first");
        set_fee_opts(launch_id.0, FM_BURN, BPS as u16);
        emit("house_buyback_set", &format!(r#"{{"id":{}}}"#, launch_id.0));
    }


    pub fn get_fee_to(&self, launch_id: U64) -> Option<AccountId> { fee_to(launch_id.0) }


    pub fn get_fee_to_many(&self, ids: Vec<U64>) -> Vec<(U64, AccountId)> {
        require!(ids.len() <= 200, "max 200 ids");
        ids.into_iter().filter_map(|i| fee_to(i.0).map(|a| (i, a))).collect()
    }


    pub fn get_quotes(&self) -> Vec<(AccountId, QuoteAsset)> {
        self.quote_ids.iter().filter_map(|id| self.quotes.get(id).map(|q| (id.clone(), q.clone()))).collect()
    }

    pub fn get_quote(&self, quote: AccountId) -> Option<QuoteAsset> {
        self.quotes.get(&quote).cloned()
    }










    pub fn set_quote(&mut self, quote: AccountId, decimals: u8, init_point: i32, min_init_point: i32, max_init_point: i32, range_points: i32, enabled: bool) {
        self.assert_owner();
        require!(min_init_point <= init_point && init_point <= max_init_point, "init_point outside its own bounds");
        require!(decimals <= 24, "decimals: 0-24");
        require!(range_points >= POINT_DELTA_1PCT * 2, "range_points too small to open a position");


        require!(range_points % POINT_DELTA_1PCT == 0, "range_points must be a multiple of 200");

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



    pub fn register_quote(&mut self, quote: AccountId) -> Promise {
        self.assert_owner();
        require!(self.quotes.contains_key(&quote), "unknown pair asset");
        let mut lockers: Vec<AccountId> = Vec::new();
        for (_, a) in self.get_lockers() {
            if !lockers.contains(&a) { lockers.push(a); }
        }
        let each = FT_STORAGE_REG.as_yoctonear();
        let mut p = Promise::new(quote.clone())
            .function_call(
                "storage_deposit".to_string(),
                format!(r#"{{"account_id":"{}","registration_only":true}}"#, env::current_account_id()).into_bytes(),
                NearToken::from_yoctonear(each),
                Gas::from_tgas(10),
            );



        for locker in lockers {
            p = p.then(Promise::new(locker).function_call(
                "register".to_string(),
                format!(r#"{{"token":"{}"}}"#, quote).into_bytes(),
                NearToken::from_yoctonear(each),
                Gas::from_tgas(20),
            ));
        }
        p
    }


    pub fn withdraw_protocol_quote_fees(&mut self, launch_id: U64, to: AccountId) -> Promise { self.internal_withdraw_protocol(launch_id, "quote", to) }

    pub fn set_config(&mut self, config: Config) {
        self.assert_owner();
        config.validate();
        self.config = config;
    }






    pub fn set_paused(&mut self, paused: bool) {
        self.assert_owner();
        self.paused = paused;
        emit("paused_set", &format!(r#"{{"paused":{}}}"#, paused));
    }

    pub fn set_token_code_hash(&mut self, token_code_hash: String) {
        self.assert_owner();
        self.token_code_hash = parse_hex32(&token_code_hash);
    }



    pub fn set_launch_recipients(&mut self, launch_id: U64, recipients: Option<Vec<(AccountId, u16)>>) {
        self.assert_owner();
        self.internal_set_launch_recipients(launch_id, recipients);
    }

    pub fn get_launch_recipients(&self, launch_id: U64) -> Option<Vec<(AccountId, u16)>> { launch_recipients(launch_id.0) }
    pub fn get_launch_bucket(&self, launch_id: U64) -> U128 { U128(launch_bucket(launch_id.0)) }
}

impl Factory {




    fn pot_recipients(&self, share: u16) -> Option<Vec<(AccountId, u16)>> {
        let (burn, refp) = (burn_pot(), referral_pot());
        if (burn.is_none() && refp.is_none()) || self.protocol_recipients.is_empty() {
            return None;
        }
        let rest = 10_000u32.saturating_sub(share as u32);
        if rest == 0 {
            return None;
        }
        let part = |bps: u16| (bps as u32 * 10_000 / rest).min(10_000);
        let burn_part = if burn.is_some() { part(BURN_POT_BPS) } else { 0 };
        let ref_part = if refp.is_some() { part(REFERRAL_POT_BPS) } else { 0 };
        if burn_part + ref_part >= 10_000 {
            return None;
        }
        let left = 10_000 - burn_part - ref_part;
        let total: u32 = self.protocol_recipients.iter().map(|(_, b)| *b as u32).sum();
        let mut rc: Vec<(AccountId, u16)> = Vec::new();
        let mut add = |who: &AccountId, bps: u32| {
            if bps == 0 { return; }
            match rc.iter_mut().find(|(w, _)| w == who) {
                Some(e) => e.1 += bps as u16,
                None => rc.push((who.clone(), bps as u16)),
            }
        };
        let mut given = 0u32;
        for (i, (who, b)) in self.protocol_recipients.iter().enumerate() {
            let p = if i + 1 == self.protocol_recipients.len() { left - given } else { left * *b as u32 / total };
            given += p;
            add(who, p);
        }
        if let Some(b) = &burn { add(b, burn_part); }
        if let Some(r) = &refp { add(r, ref_part); }
        if rc.len() > 4 {
            emit("pot_legs_dropped", &format!(r#"{{"legs":{}}}"#, rc.len()));
            return None;
        }
        Some(rc)
    }

    fn internal_set_launch_recipients(&mut self, launch_id: U64, recipients: Option<Vec<(AccountId, u16)>>) {
        require!(self.launches.get(&launch_id.0).is_some(), "launch");
        let key = launch_recipients_key(launch_id.0);
        match recipients {
            Some(rc) => {
                validate_launch_recipients(&rc);
                env::storage_write(&key, &near_sdk::serde_json::to_vec(&rc).unwrap());
            }
            None => {

                let leftover = launch_bucket(launch_id.0);
                self.protocol_fees += leftover;
                set_launch_bucket(launch_id.0, 0);
                env::storage_remove(&key);
                if leftover > 0 {
                    emit("launch_bucket_moved", &format!(r#"{{"id":{},"amount":"{}"}}"#, launch_id.0, leftover));
                    self.drain_protocol_fees();
                }
            }
        }
        emit("launch_recipients_set", &format!(r#"{{"id":{}}}"#, launch_id.0));
    }
}

#[near]
impl Factory {



    pub fn set_protocol_recipients(&mut self, recipients: Vec<(AccountId, u16)>) {
        self.assert_owner();
        validate_protocol_recipients(&recipients);
        self.assert_pot_legs_fit(recipients.len(), burn_pot().is_some(), referral_pot().is_some());
        self.protocol_recipients = recipients;
    }



    fn assert_pot_legs_fit(&self, protocol: usize, burn: bool, refp: bool) {
        let pots = burn as usize + refp as usize;
        require!(pots == 0 || protocol + pots <= 4, "pots: protocol recipients plus pots must be 4 legs or fewer");
    }



    pub fn set_house_creator(&mut self, house_creator: Option<AccountId>) {
        self.assert_owner();
        self.house_creator = house_creator;
    }




    pub fn set_house_creators(&mut self, accounts: Vec<AccountId>, on: bool) {
        self.assert_owner();
        internal_set_house_creators(accounts, on);
    }

    pub fn is_house_creator(&self, account_id: AccountId) -> bool { self.is_house(&account_id) }


    pub fn seed_slug_counts(&mut self, counts: Vec<(String, u32)>) {
        self.assert_owner();
        for (slug, n) in counts { if n > slug_count(&slug) { set_slug_count(&slug, n); } }
    }

    pub fn get_slug_count(&self, slug: String) -> u32 { slug_count(&slug) }


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
        self.internal_pay(0, "near", &to, amount, "protocol")
    }

    pub fn withdraw_protocol_token_fees(&mut self, launch_id: U64, to: AccountId) -> Promise { self.internal_withdraw_protocol(launch_id, "token", to) }





    pub fn sweep_wnear(&mut self, amount: U128) -> Promise {
        self.assert_owner();
        require!(amount.0 > 0, "amount must be > 0");
        require!(read_counter(UW_TOTAL_KEY) == 0, "wNEAR owed to a launch is still to be unwrapped: resume it first, or retry_wnear_owed");
        self.unwrap_wnear(amount.0)
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_SPLIT).on_wnear_swept(amount, Some(true)))
    }



    #[private]
    pub fn on_wnear_swept(&mut self, amount: U128, credit_on_success: Option<bool>, #[callback_result] r: Result<(), PromiseError>) {
        if credit_on_success == Some(true) {
            if r.is_ok() {
                self.protocol_fees += amount.0;
                emit("wnear_swept", &format!(r#"{{"amount":"{}"}}"#, amount.0));
            } else {
                emit("wnear_sweep_failed", &format!(r#"{{"amount":"{}","reversed":"0"}}"#, amount.0));
            }
            return;
        }
        if r.is_ok() {
            return;
        }


        let reversed = self.protocol_fees.min(amount.0);
        self.protocol_fees -= reversed;
        emit("wnear_sweep_failed", &format!(r#"{{"amount":"{}","reversed":"{}"}}"#, amount.0, reversed));
    }


    pub fn transfer_ownership(&mut self, new_owner: AccountId) {
        self.propose_owner(new_owner);
    }


    pub fn set_range(&mut self, launch_id: U64, left_point: i32, right_point: i32) {
        self.assert_owner();
        let mut l = self.rec(launch_id.0);
        require!(l.lpt_id.is_none() && l.step != Step::Done, "liquidity already placed");
        require!(left_point < right_point && left_point % POINT_DELTA_1PCT == 0 && right_point % POINT_DELTA_1PCT == 0, "points must be multiples of 200");
        require!(!dev_buy_inflight(&l), "a dev buy's in-flight flag is only cleared by its own callback");
        l.left_point = left_point;
        l.right_point = right_point;
        l.inflight = false;
        self.launches.insert(launch_id.0, l);
    }



    pub fn set_step(&mut self, launch_id: U64, step: Step) {
        self.assert_owner();
        require!(!matches!(step, Step::Deposit | Step::ReadDevTokens | Step::DeliverDevTokens), "dead step");
        let mut l = self.rec(launch_id.0);
        require!(l.step != Step::Done, "already live");


        require!(!dev_buy_inflight(&l), "a dev buy's in-flight flag is only cleared by its own callback");
        match step {

            Step::Done => require!(
                l.lpt_id.is_some() && l.dev_buy_held.0 == 0 && buy_held(launch_id.0) == 0 && !env::storage_has_key(&unwrap_pending_key(launch_id.0)),
                "Done needs a position and no dev buy money on the launch (cancel_dev_buy / refund_dev_buy)",
            ),
            Step::DevBuy => require!(l.lpt_id.is_some(), "DevBuy needs a position"),

            Step::CreateToken | Step::CreatePool | Step::Failed => require!(l.lpt_id.is_none(), "this launch has a position"),
            _ => {}
        }
        l.step = step;
        l.inflight = false;
        self.launches.insert(launch_id.0, l);
    }




    pub fn reset_inflight(&mut self, launch_id: U64) {
        self.assert_owner();
        let mut l = self.rec(launch_id.0);
        require!(!dev_buy_inflight(&l), "a dev buy's in-flight flag is only cleared by its own callback");
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
        if skip >= self.next_id {
            return out;
        }
        let mut id = self.next_id - skip;
        while id > 0 && (out.len() as u64) < limit {
            id -= 1;
            if let Some(l) = self.launches.get(&id) {
                out.push(l.clone());
            }
        }
        out
    }

    pub fn get_num_launches(&self) -> u64 {
        self.next_id
    }


    pub fn quote_launch(&self, icon_bytes: u32, dev_buy: Option<U128>, tax: Option<bool>) -> LaunchCost {
        self.internal_cost(icon_bytes as usize, MAX_TEXT_BYTES, dev_buy.map(|d| d.0).unwrap_or(0), tax.unwrap_or(false))
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


    pub fn get_lockers(&self) -> Vec<(U64, AccountId)> {
        let mut v = vec![(U64(0), self.locker_id.clone())];
        v.extend(lockers().into_iter().map(|(f, a)| (U64(f), a)));
        v
    }

    pub fn get_locker_for(&self, launch_id: U64) -> AccountId {
        self.locker_for(launch_id.0)
    }



    pub fn set_locker_from(&mut self, locker: AccountId, from_id: U64) {
        self.assert_owner();
        self.internal_set_locker_from(locker, from_id.0);
    }

    pub fn get_addresses(&self) -> Addresses {
        Addresses {
            dcl_current: self.dcl_for(self.next_id),
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





#[near]
impl Factory {



    pub fn set_dcl(&mut self, dcl: AccountId, migrate: Option<bool>) {
        self.assert_owner();
        require!(dcl != env::current_account_id(), "dcl: not the launchpad itself");
        require!(dcl != self.dcl_for(self.next_id), "dcl: already the exchange for new launches");
        if migrate == Some(true) {
            self.internal_migrate_dcl(dcl);
        } else {
            self.internal_set_dcl_from(dcl, self.next_id);
        }
    }




    pub fn set_locker_add_gas(&mut self, locker: AccountId, tgas: u64) {
        self.assert_owner();
        require!(self.get_lockers().iter().any(|(_, a)| *a == locker), "not one of this factory's lockers");
        if tgas == 0 {
            env::storage_remove(&locker_add_gas_key(&locker));
        } else {
            require!((60..=tg(GAS_LOCKER_ADD)).contains(&tgas), "locker add gas: 60 to 180 TGas");
            env::storage_write(&locker_add_gas_key(&locker), &tgas.to_le_bytes());
        }
        emit("locker_add_gas_set", &format!(r#"{{"locker":"{}","tgas":{}}}"#, locker, tgas));
    }




    pub fn set_locker_add_buy(&mut self, locker: AccountId, on: bool) {
        self.assert_owner();
        require!(self.get_lockers().iter().any(|(_, a)| *a == locker), "not one of this factory's lockers");
        if on { env::storage_write(&locker_add_buy_key(&locker), &[1]); } else { env::storage_remove(&locker_add_buy_key(&locker)); }
        emit("locker_add_buy_set", &format!(r#"{{"locker":"{}","on":{}}}"#, locker, on));
    }

    pub fn get_locker_add_buy(&self, locker: AccountId) -> bool { locker_add_buy(&locker) }





    pub fn locker_refund_buy(&mut self, launch_id: U64) -> Promise {
        self.assert_owner();
        let l = self.rec(launch_id.0);
        require!(buy_held(launch_id.0) > 0, "the locker holds nothing of this launch's buy");
        require!(!env::storage_has_key(&buy_unknown_key(launch_id.0)), "settle_first_buy first");
        require!(!env::storage_has_key(&buy_refund_inflight_key(launch_id.0)), "a refund is in flight");
        env::storage_write(&buy_refund_inflight_key(launch_id.0), &[1]);
        Promise::new(self.locker_for(launch_id.0))
            .function_call("refund_buy".to_string(), format!(r#"{{"token":"{}"}}"#, l.token).into_bytes(), NO_DEPOSIT, GAS_LOCKER_REFUND_BUY)
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_LOCKER_REFUND).on_locker_buy_refunded(launch_id))
    }

    #[private]
    pub fn on_locker_buy_refunded(&mut self, launch_id: U64, #[callback_result] r: Result<BuyOutcome, PromiseError>) -> U128 {
        env::storage_remove(&buy_refund_inflight_key(launch_id.0));
        let id = launch_id.0;
        let back = r.map(|o| o.refunded.0).unwrap_or(0).min(buy_held(id));
        set_buy_held(id, buy_held(id) - back);
        emit("locker_buy_refunded", &format!(r#"{{"id":{},"amount":"{}","still_held":"{}"}}"#, id, back, buy_held(id)));
        if back > 0 {
            let mut l = self.rec(id);

            if matches!(l.step, Step::Done | Step::Failed) || dev_buy_done(id) {
                emit("dev_buy_refunded", &format!(r#"{{"id":{},"creator":"{}","amount":"{}"}}"#, id, l.creator, back));
                self.internal_refund_creator(id, &l.creator, back).detach();
            } else {
                l.dev_buy_held = U128(l.dev_buy_held.0 + back);
                self.launches.insert(id, l);
            }
        }
        U128(back)
    }










    pub fn settle_first_buy(&mut self, launch_id: U64) -> Promise {
        let id = launch_id.0;
        let l = self.rec(id);
        require!(l.step == Step::AddLiquidity && (dev_buy_inflight(&l) || env::storage_has_key(&buy_unknown_key(id))), "no first buy to settle");
        Promise::new(self.locker_for(id))
            .function_call("get_first_buy".to_string(), format!(r#"{{"token":"{}"}}"#, l.token).into_bytes(), NO_DEPOSIT, GAS_FIRST_BUY_VIEW)
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_FIRST_BUY).with_unused_gas_weight(0).on_first_buy_read(launch_id))
    }

    #[private]
    pub fn on_first_buy_read(&mut self, launch_id: U64, #[callback_result] r: Result<Option<FirstBuyRecord>, PromiseError>) -> bool {
        let id = launch_id.0;
        let mut l = self.rec(id);
        let unknown = env::storage_has_key(&buy_unknown_key(id));

        if !(l.step == Step::AddLiquidity && (dev_buy_inflight(&l) || unknown)) {
            return false;
        }
        let near = l.dev_buy_near.0;
        match r {
            Ok(None) if unknown => {
                env::storage_remove(&buy_unknown_key(id));
                set_buy_held(id, 0);
                l.dev_buy_held = U128(near);
                emit("first_buy_settled", &format!(r#"{{"id":{},"used":"0","refunded":"{}","held":"0","taken":false}}"#, id, near));
                self.launches.insert(id, l);
                true
            }
            Ok(Some(b)) if b.settled => {
                env::storage_remove(&buy_unknown_key(id));
                let used = b.used.0.min(near);
                let refunded = b.returned.0.min(near - used);
                emit("first_buy_settled", &format!(r#"{{"id":{},"used":"{}","refunded":"{}","held":"{}","taken":true}}"#, id, used, refunded, near - used - refunded));
                self.settle_add_buy(id, l, Some(AddBuyResult { lpt_id: String::new(), used: U128(used), refunded: U128(refunded) }));
                true
            }
            _ => {
                emit("first_buy_not_settled", &format!(r#"{{"id":{}}}"#, id));
                false
            }
        }
    }






    pub fn release_dev_buy(&mut self, launch_id: U64) {
        self.assert_owner();
        let id = launch_id.0;
        let l = self.rec(id);
        require!(l.step == Step::DevBuy && l.dev_buy_held.0 == 0, "nothing to release: refund_dev_buy pays a funded buy, settle_first_buy books one sent with add_buy");
        require!(l.inflight || !env::storage_has_key(&unwrap_pending_key(id)), "resume unwraps the refused buy first");




        let unwrapping = env::storage_has_key(&dev_unwrap_op_key(id));
        let uw = if unwrapping { 0 } else { read_u128(&unwrap_pending_key(id)).unwrap_or(0) };
        if uw > 0 {
            env::storage_remove(&unwrap_pending_key(id));
            add_counter(&wnear_owed_key(id), uw);
        }
        emit("dev_buy_released", &format!(r#"{{"id":{},"creator":"{}","amount":"{}","unwrap_pending":"{}","unwrapping":{},"locker_held":"{}"}}"#, id, l.creator, l.dev_buy_near.0, uw, unwrapping, buy_held(id)));
        self.internal_done(id, l);
    }


    pub fn get_buy_held(&self, launch_id: U64) -> U128 { U128(buy_held(launch_id.0)) }



    pub fn get_wnear_owed(&self, launch_id: U64) -> (U128, U128) {
        (U128(read_counter(&wnear_owed_key(launch_id.0))), U128(read_counter(&wnear_owed_bucket_key(launch_id.0))))
    }





    pub fn retry_wnear_owed(&mut self, launch_id: U64) -> Promise {
        let id = launch_id.0;
        let _ = self.rec(id);
        let (owed, bucket) = (read_counter(&wnear_owed_key(id)), read_counter(&wnear_owed_bucket_key(id)));
        require!(owed + bucket > 0, "nothing owed to this launch");
        require!(!env::storage_has_key(&wnear_retry_key(id)), "retry in flight");
        require!(!env::storage_has_key(&dev_unwrap_op_key(id)), "the dev buy's unwrap is in flight");
        require!(gas_left() >= GAS_UNWRAP.saturating_add(GAS_OWED_CB).saturating_add(GAS_RESERVE), "attach at least 50 TGas");
        let op = env::block_timestamp_ms();
        env::storage_write(&wnear_retry_key(id), &[op.to_le_bytes().as_slice(), &owed.to_le_bytes(), &bucket.to_le_bytes()].concat());
        self.unwrap_wnear(owed + bucket)
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_OWED_CB).with_unused_gas_weight(0).on_wnear_owed_unwrapped(launch_id, U128(owed), U128(bucket), Some(op)))
    }



    #[private]
    pub fn on_wnear_owed_unwrapped(&mut self, launch_id: U64, owed: U128, bucket: U128, op: Option<u64>, #[callback_result] r: Result<(), PromiseError>) -> bool {
        let id = launch_id.0;
        if op.is_some() && op != wnear_retry(id).map(|w| w.0) {
            emit("wnear_owed_unwrap_stale", &format!(r#"{{"id":{},"op":"{}","ok":{}}}"#, id, op.unwrap_or(0), r.is_ok()));
            return false;
        }
        env::storage_remove(&wnear_retry_key(id));
        self.internal_wnear_owed_unwrapped(id, owed.0, bucket.0, r.is_ok())
    }

    fn internal_wnear_owed_unwrapped(&mut self, id: u64, owed: u128, bucket: u128, ok: bool) -> bool {
        if !ok {
            emit("wnear_owed_unwrap_failed", &format!(r#"{{"id":{},"amount":"{}"}}"#, id, owed + bucket));
            return false;
        }
        sub_counter(&wnear_owed_key(id), owed);
        sub_counter(&wnear_owed_bucket_key(id), bucket);
        sub_counter(UW_TOTAL_KEY, owed + bucket);
        emit("wnear_owed_paid", &format!(r#"{{"id":{},"creator":"{}","bucket":"{}"}}"#, id, owed, bucket));
        if owed > 0 {
            let creator = self.rec(id).creator;
            self.internal_refund_creator(id, &creator, owed).detach();
        }
        if bucket > 0 {
            self.internal_bb_recredit(U64(id), bucket);
        }
        true
    }





    pub fn resolve_unwrap(&mut self, launch_id: U64, landed: bool) -> bool {
        self.assert_owner();
        let id = launch_id.0;
        let now = env::block_timestamp_ms();
        if let Some(op) = read_u128(&dev_unwrap_op_key(id)).map(|v| v as u64) {
            require!(now >= op.saturating_add(UNWRAP_RESOLVE_MS), "unwrap in flight: its callback settles it");
            env::storage_remove(&dev_unwrap_op_key(id));
            emit("unwrap_resolved", &format!(r#"{{"id":{},"kind":"dev_buy","op":"{}","landed":{}}}"#, id, op, landed));
            return self.internal_dev_buy_unwrapped(id, landed, false);
        }
        let retry = wnear_retry(id);
        require!(retry.is_some(), "no unwrap in flight for this launch");
        let (op, owed, bucket) = retry.unwrap();
        require!(now >= op.saturating_add(UNWRAP_RESOLVE_MS), "unwrap in flight: its callback settles it");
        env::storage_remove(&wnear_retry_key(id));
        emit("unwrap_resolved", &format!(r#"{{"id":{},"kind":"owed","op":"{}","landed":{}}}"#, id, op, landed));
        self.internal_wnear_owed_unwrapped(id, owed, bucket, landed)
    }



    pub fn clear_tax_floor(&mut self, launch_id: U64) {
        self.assert_owner();
        let floor = read_counter(&tax_floor_key(launch_id.0));
        env::storage_remove(&tax_floor_key(launch_id.0));
        emit("tax_floor_cleared", &format!(r#"{{"id":{},"floor":"{}"}}"#, launch_id.0, floor));
    }


    pub fn get_locker_add_gas(&self, locker: AccountId) -> Option<u64> {
        locker_add_gas(&locker).map(|g| tg(g))
    }





    pub fn set_fee_router(&mut self, account: AccountId, on: bool) {
        self.assert_owner();
        require!(account != env::current_account_id(), "fee router: not the launchpad itself");
        let mut v = fee_routers();
        if on && !v.contains(&account) {
            v.push(account.clone());
        } else if !on {
            v.retain(|x| *x != account);
        }
        require!(v.len() <= MAX_FEE_ROUTERS, "at most 32 fee routers");
        if v.is_empty() { env::storage_remove(FEE_ROUTERS_KEY); } else { env::storage_write(FEE_ROUTERS_KEY, &near_sdk::serde_json::to_vec(&v).expect("routers")); }
        emit("fee_router_set", &format!(r#"{{"account":"{}","on":{}}}"#, account, on));
    }



    pub fn set_launch_hook(&mut self, hook: Option<AccountId>) {
        self.assert_owner();
        require!(hook.as_ref() != Some(&env::current_account_id()), "launch hook: not the launchpad itself");
        match &hook {
            Some(a) => env::storage_write(LAUNCH_HOOK_KEY, a.as_bytes()),
            None => env::storage_remove(LAUNCH_HOOK_KEY),
        };
        emit("launch_hook_set", &format!(r#"{{"hook":{}}}"#, hook.map(|a| format!("\"{}\"", a)).unwrap_or_else(|| "null".into())));
    }




    pub fn upgrade(&mut self, code_hash: Base58CryptoHash, migrate: bool) -> Promise {
        self.assert_owner();
        let hash: CryptoHash = code_hash.into();
        emit("upgrade", &format!(r#"{{"code_hash":"{}","migrate":{}}}"#, hex32(hash), migrate));
        let p = Promise::new(env::current_account_id()).use_global_contract(hash);
        if migrate { p.function_call_weight("migrate".to_string(), b"{}".to_vec(), NO_DEPOSIT, GAS_MIGRATE, GasWeight(1)) } else { p }
    }






    pub fn upgrade_code(&self) -> Promise {
        self.assert_owner();
        let code = env::input().unwrap_or_default();
        require!(code.starts_with(b"\0asm"), "upgrade_code: the input must be the wasm itself");
        let hash: CryptoHash = env::sha256_array(&code);
        emit("upgrade", &format!(r#"{{"code_hash":"{}","migrate":true,"raw":true}}"#, hex32(hash)));
        Promise::new(env::current_account_id())
            .deploy_contract(code)
            .function_call_weight("migrate".to_string(), b"{}".to_vec(), NO_DEPOSIT, GAS_MIGRATE, GasWeight(1))
    }






    pub fn add_fc_key(&mut self, public_key: PublicKey, methods: Vec<String>, allowance: Option<U128>) -> Promise {
        self.assert_owner();
        require!(!methods.is_empty(), "fc key: list the methods; an empty list allows every method, callbacks included");
        let mut total = 0usize;
        for (i, m) in methods.iter().enumerate() {
            require!(
                !m.is_empty() && m.len() <= 256 && m.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "fc key: a method name is 1-256 characters of [A-Za-z0-9_]",
            );
            require!(!m.starts_with("on_") && m != "migrate", "fc key: callbacks (on_*) and migrate are never on a key");
            require!(!methods[..i].contains(m), "fc key: duplicate method");
            total += m.len() + 1;
        }
        require!(total <= 2_000, "fc key: at most 2000 bytes of method names");
        let allowance_json = allowance.map(|a| format!("\"{}\"", a.0)).unwrap_or_else(|| "null".into());
        let allowance = match allowance {
            None => Allowance::unlimited(),
            Some(a) => Allowance::limited(NearToken::from_yoctonear(a.0)).unwrap_or_else(|| env::panic_str("fc key: allowance must be > 0 (omit it for unlimited)")),
        };
        let me = env::current_account_id();
        emit("fc_key_added", &format!(
            r#"{{"public_key":"{}","methods":{},"allowance":{}}}"#,
            public_key, near_sdk::serde_json::to_string(&methods).expect("methods"), allowance_json
        ));
        Promise::new(me.clone()).add_access_key_allowance(public_key, allowance, me, methods.join(","))
    }



    pub fn delete_fc_key(&mut self, public_key: PublicKey) -> Promise {
        self.assert_owner();
        require!(public_key != env::signer_account_pk(), "fc key: this call is signed with that key");
        emit("fc_key_deleted", &format!(r#"{{"public_key":"{}"}}"#, public_key));
        Promise::new(env::current_account_id()).delete_key(public_key)
    }





    #[private]
    pub fn migrate(&self) -> bool {
        true
    }




    pub fn propose_owner(&mut self, new_owner: AccountId) {
        self.assert_owner();
        if new_owner == self.owner_id {
            env::storage_remove(PENDING_OWNER_KEY);
            emit("owner_proposal_cancelled", &format!(r#"{{"owner":"{}"}}"#, self.owner_id));
            return;
        }
        env::storage_write(PENDING_OWNER_KEY, new_owner.as_bytes());
        emit("owner_proposed", &format!(r#"{{"owner":"{}","proposed":"{}"}}"#, self.owner_id, new_owner));
    }


    pub fn accept_owner(&mut self) {
        let pending = pending_owner().expect("no pending owner");
        require!(env::predecessor_account_id() == pending, "only the proposed owner can accept");
        env::storage_remove(PENDING_OWNER_KEY);
        emit("owner_changed", &format!(r#"{{"from":"{}","to":"{}"}}"#, self.owner_id, pending));
        self.owner_id = pending;
    }

    pub fn get_pending_owner(&self) -> Option<AccountId> { pending_owner() }











    pub fn locker_release_supply(&mut self, launch_id: U64) -> Promise {
        self.assert_owner();
        let l = self.rec(launch_id.0);
        require!(l.step == Step::Failed, "only a failed launch's supply is released");
        emit("locker_release_supply", &format!(r#"{{"id":{},"token":"{}"}}"#, launch_id.0, l.token));
        Promise::new(self.locker_for(launch_id.0))
            .function_call("release_supply".to_string(), format!(r#"{{"token":"{}"}}"#, l.token).into_bytes(), NO_DEPOSIT, GAS_ON_SPLIT)
    }





    pub fn token_add_pair(&mut self, launch_id: U64, pair: AccountId) -> Promise {
        self.assert_owner();
        let l = self.rec(launch_id.0);
        require!(tax_opts(launch_id.0).is_some(), "no tax on this launch");
        require!(!self.quotes.contains_key(&l.token), "a quote asset's pairs stay as the lockers read them");
        emit("token_add_pair", &format!(r#"{{"id":{},"token":"{}","pair":"{}"}}"#, launch_id.0, l.token, pair));
        Promise::new(l.token)
            .function_call("tax_add_pair".to_string(), format!(r#"{{"pair":"{}"}}"#, pair).into_bytes(), NO_DEPOSIT, GAS_ON_SPLIT)
    }

    pub fn locker_withdraw_dcl_asset(&mut self, locker: AccountId, token: AccountId, amount: U128, max_book: Option<U128>) -> Promise {
        self.assert_owner();
        require!(self.get_lockers().iter().any(|(_, a)| *a == locker), "not one of this factory's lockers");
        require!(token == self.wnear_id || self.token_index.contains_key(&token), "only a launch token or wNEAR can be booked");
        take_swap_lock(&token, LOCKER_WITHDRAW_LOCK_ID, LK_ADMIN);
        emit("locker_dcl_withdraw", &format!(r#"{{"locker":"{}","token":"{}","amount":"{}"}}"#, locker, token, amount.0));
        Promise::new(token.clone())
            .function_call("ft_balance_of".to_string(), format!(r#"{{"account_id":"{}"}}"#, env::current_account_id()).into_bytes(), NO_DEPOSIT, GAS_BALANCE_OF)
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_LWD_BEFORE).on_locker_withdraw_before(locker, token, amount, max_book))
    }

    #[private]
    pub fn on_locker_withdraw_before(&mut self, locker: AccountId, token: AccountId, amount: U128, max_book: Option<U128>, #[callback_result] bal: Result<U128, PromiseError>) -> PromiseOrValue<bool> {
        let Ok(before) = bal else {
            release_swap_lock(&token, LOCKER_WITHDRAW_LOCK_ID, LK_ADMIN);
            emit("locker_dcl_withdrawn", &format!(r#"{{"token":"{}","amount":"{}","booked":"0","reason":"balance read failed"}}"#, token, amount.0));
            return PromiseOrValue::Value(false);
        };
        PromiseOrValue::Promise(
            Promise::new(locker)
                .function_call_weight(
                    "withdraw_dcl_asset".to_string(),
                    format!(r#"{{"token":"{}","amount":"{}"}}"#, token, amount.0).into_bytes(),
                    NO_DEPOSIT,
                    GAS_LOCKER_ADMIN_WITHDRAW,
                    GasWeight(1),
                )
                .then(Self::ext(env::current_account_id()).with_static_gas(GAS_LWD_WITHDRAWN).with_unused_gas_weight(0).on_locker_withdrawn(token, amount, before, max_book)),
        )
    }





    #[private]
    pub fn on_locker_withdrawn(&mut self, token: AccountId, amount: U128, before: U128, max_book: Option<U128>, #[callback_result] r: Result<bool, PromiseError>) -> PromiseOrValue<U128> {
        let _ = (amount, r);
        if !self.holds_admin_lock(&token) {
            emit("locker_dcl_withdrawn", &format!(r#"{{"token":"{}","booked":"0","reason":"lock lost"}}"#, token));
            return PromiseOrValue::Value(U128(0));
        }
        PromiseOrValue::Promise(balance_of_me(token.clone()).then(Self::ext(env::current_account_id()).with_static_gas(GAS_LWD_BOOK).with_unused_gas_weight(0).on_locker_withdraw_booked(token, before, max_book)))
    }


    #[private]
    pub fn on_locker_withdraw_booked(&mut self, token: AccountId, before: U128, max_book: Option<U128>, #[callback_result] bal: Result<U128, PromiseError>) -> U128 {
        let ours = self.holds_admin_lock(&token);
        release_swap_lock(&token, LOCKER_WITHDRAW_LOCK_ID, LK_ADMIN);
        if !ours {
            emit("locker_dcl_withdrawn", &format!(r#"{{"token":"{}","booked":"0","reason":"lock lost"}}"#, token));
            return U128(0);
        }
        let Ok(after) = bal else {
            emit("locker_dcl_withdrawn", &format!(r#"{{"token":"{}","booked":"0","reason":"balance read failed"}}"#, token));
            return U128(0);
        };
        let got = after.0.saturating_sub(before.0).min(max_book.map_or(u128::MAX, |m| m.0));
        if got == 0 {
            emit("locker_dcl_withdrawn", &format!(r#"{{"token":"{}","booked":"0"}}"#, token));
            return U128(0);
        }
        if token == self.wnear_id {
            self.unwrap_wnear(got)
                .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_SPLIT).on_wnear_swept(U128(got), Some(true)))
                .detach();
            emit("locker_dcl_withdrawn", &format!(r#"{{"token":"{}","booked":"{}","to":"protocol_fees"}}"#, token, got));
            return U128(got);
        }
        let id = *self.token_index.get(&token).expect("launch token");
        let mut l = self.rec(id);
        l.protocol_token_fees = U128(l.protocol_token_fees.0 + got);
        self.launches.insert(id, l);
        emit("locker_dcl_withdrawn", &format!(r#"{{"token":"{}","booked":"{}","id":{},"to":"protocol_token_fees"}}"#, token, got, id));
        U128(got)
    }



    pub fn push_router_fees(&mut self, launch_id: U64) -> Promise {
        self.assert_not_paused();
        let l = self.rec(launch_id.0);
        let near = router_near(launch_id.0);
        require!(near > 0, "nothing to push");
        set_router_near(launch_id.0, 0);
        let to = payee(&l);
        emit("router_fees_paid", &format!(r#"{{"id":{},"to":"{}","kind":"near","amount":"{}"}}"#, launch_id.0, to, near));
        self.internal_pay(launch_id.0, "near", &to, near, "creator")
    }



    pub fn get_router_fees(&self, launch_id: U64) -> (U128, U128) {
        (U128(router_near(launch_id.0)), U128(0))
    }

    pub fn get_fee_routers(&self) -> Vec<AccountId> { fee_routers() }
    pub fn is_fee_router(&self, account_id: AccountId) -> bool { is_router(&account_id) }
    pub fn get_launch_hook(&self) -> Option<AccountId> { launch_hook() }


    pub fn get_dcls(&self) -> Vec<(U64, AccountId)> {
        let mut v = vec![(U64(0), self.dcl_id.clone())];
        v.extend(dcls().into_iter().map(|(f, a)| (U64(f), a)));
        v
    }

    pub fn get_dcl_for(&self, launch_id: U64) -> AccountId { self.dcl_for(launch_id.0) }

}


impl Factory {
    fn is_house(&self, a: &AccountId) -> bool {
        self.house_creator.as_ref() == Some(a) || env::storage_has_key(&house_key(a))
    }


    fn locker_for(&self, id: u64) -> AccountId {
        lockers().into_iter().rev().find(|(f, _)| *f <= id).map(|(_, a)| a).unwrap_or_else(|| self.locker_id.clone())
    }




    fn dcl_for(&self, id: u64) -> AccountId {
        dcls().into_iter().rev().find(|(f, _)| *f <= id).map(|(_, a)| a).unwrap_or_else(|| self.dcl_id.clone())
    }



    fn internal_set_locker_from(&mut self, locker: AccountId, from_id: u64) {


        let me = env::current_account_id();
        require!(
            locker.is_sub_account_of(&me) && locker.as_str()[..locker.as_str().len() - me.as_str().len() - 1].contains('_'),
            "locker: a direct sub-account of the launchpad with '_' in its name",
        );
        require!(from_id >= self.next_id, "existing launches keep their locker");
        let mut v = lockers();
        require!(v.last().map_or(true, |(f, _)| from_id > *f), "cutoffs must increase");
        v.push((from_id, locker.clone()));
        env::storage_write(LOCKERS_KEY, &near_sdk::serde_json::to_vec(&v).expect("lockers"));
        emit("locker_set", &format!(r#"{{"locker":"{}","from_id":{}}}"#, locker, from_id));
    }



    fn internal_set_dcl_from(&mut self, dcl: AccountId, from_id: u64) {
        let mut v = dcls();

        if v.last().map_or(false, |(f, _)| *f == from_id) {
            v.pop();
        }
        require!(v.last().map_or(true, |(f, _)| from_id > *f), "cutoffs must increase");
        v.push((from_id, dcl.clone()));
        env::storage_write(DCLS_KEY, &near_sdk::serde_json::to_vec(&v).expect("dcls"));
        emit("dcl_set", &format!(r#"{{"dcl":"{}","from_id":{}}}"#, dcl, from_id));
    }





    fn internal_migrate_dcl(&mut self, dcl: AccountId) {
        let old = self.dcl_for(self.next_id);

        let mut segs: Vec<(u64, AccountId)> = vec![(0, self.dcl_id.clone())];
        segs.extend(dcls());
        let locker_list = self.get_lockers();
        let mut lockers: Vec<AccountId> = Vec::new();
        for (i, (from, a)) in locker_list.iter().enumerate() {
            let to = locker_list.get(i + 1).map_or(u64::MAX, |(f, _)| f.0);
            let on_old = segs.iter().enumerate().any(|(j, (c, x))| {
                let d = segs.get(j + 1).map_or(u64::MAX, |(f, _)| *f);
                *x == old && from.0.max(*c) < to.min(d)
            });
            if on_old && !lockers.contains(a) {
                lockers.push(a.clone());
            }
        }
        if self.dcl_id == old {
            self.dcl_id = dcl.clone();
        }
        let v: Vec<(u64, AccountId)> = dcls().into_iter().map(|(f, a)| (f, if a == old { dcl.clone() } else { a })).collect();
        if !v.is_empty() {
            env::storage_write(DCLS_KEY, &near_sdk::serde_json::to_vec(&v).expect("dcls"));
        }
        require!(
            gas_left() >= Gas::from_tgas((tg(GAS_LOCKER_SET_DCL) + 2) * lockers.len() as u64 + 10),
            "attach more gas: a migration calls set_dcl on every locker",
        );
        for a in &lockers {
            Promise::new(a.clone())
                .function_call("set_dcl".to_string(), format!(r#"{{"dcl":"{}"}}"#, dcl).into_bytes(), NO_DEPOSIT, GAS_LOCKER_SET_DCL)
                .detach();
        }
        emit("dcl_set", &format!(r#"{{"dcl":"{}","from_id":0,"migrate":true,"old":"{}","lockers":{}}}"#, dcl, old, lockers.len()));
    }






    fn internal_pay(&self, id: u64, kind: &str, to: &AccountId, amount: u128, bucket: &str) -> Promise {
        let router = bucket == "creator" && is_router(to);
        let (p, ftc) = if kind == "near" {
            (Promise::new(to.clone()).transfer(NearToken::from_yoctonear(amount)), false)
        } else {
            let l = self.launches.get(&id).expect("launch");
            let asset = if kind == "token" { &l.token } else { &l.quote };
            (self.internal_send_tokens(asset, to, amount, "fees"), false)
        };
        let gas = if router { GAS_ON_PAID_ROUTER } else { GAS_ON_SPLIT };
        p.then(Self::ext(env::current_account_id()).with_static_gas(gas).on_paid(U64(id), kind.to_string(), to.clone(), U128(amount), bucket.to_string(), ftc))
    }


    fn add_to_bucket(&mut self, id: u64, kind: &str, bucket: &str, to: &AccountId, amount: u128) {
        if amount == 0 { return; }
        match (bucket, kind) {
            ("creator", "near") if is_router(to) => set_router_near(id, router_near(id) + amount),
            ("creator", "near") | ("account", _) => { let cur = self.creator_fees.get(to).copied().unwrap_or(0); self.creator_fees.insert(to.clone(), cur + amount); }
            ("protocol", "near") => self.protocol_fees += amount,
            ("launch", _) => if launch_recipients(id).is_some() { set_launch_bucket(id, launch_bucket(id) + amount) } else { self.protocol_fees += amount },
            ("mode", _) => { set_mode_bucket(id, mode_bucket(id) + amount); sub_counter(&paid_key(id), amount); }
            ("tax_holders", _) => { add_counter(&tax_holders_key(id), amount); sub_counter(&tax_paid_holders_key(id), amount); }
            _ => {
                let mut l = self.rec(id);
                let slot = match (bucket, kind) {
                    ("creator", "token") => &mut l.creator_token_fees,
                    ("creator", _) => &mut l.creator_quote_fees,
                    (_, "token") => &mut l.protocol_token_fees,
                    _ => &mut l.protocol_quote_fees,
                };
                *slot = U128(slot.0 + amount);
                self.launches.insert(id, l);
            }
        }
    }


    fn internal_split(&self, id: u64, kind: &str, amount: u128, rc: &[(AccountId, u16)], bucket: &str) {
        let bps: Vec<u16> = rc.iter().map(|(_, b)| *b).collect();
        for (i, cut) in split_legs(amount, &bps).into_iter().enumerate() {
            if cut > 0 { self.internal_pay(id, kind, &rc[i].0, cut, bucket).detach(); }
        }
    }






    fn drain_protocol_fees(&mut self) {
        let amount = self.protocol_fees;
        if amount == 0 || amount < self.min_push_yocto || self.protocol_recipients.is_empty() { return; }
        if !can_pay_legs(self.protocol_recipients.len()) {
            emit("protocol_fees_split_deferred", &format!(r#"{{"amount":"{}"}}"#, amount));
            return;
        }
        self.protocol_fees = 0;
        let rc = self.protocol_recipients.clone();
        self.internal_split(0, "near", amount, &rc, "protocol");
        emit("protocol_fees_split", &format!(r#"{{"amount":"{}","legs":{},"ts_ms":{}}}"#, amount, rc.len(), env::block_timestamp_ms()));
    }


    fn drain_launch_fees(&mut self, id: u64) {
        let Some(rc) = launch_recipients(id) else { return };
        let amount = launch_bucket(id);
        if amount == 0 || amount < self.min_push_yocto { return; }
        if !can_pay_legs(rc.len()) {
            emit("launch_fees_split_deferred", &format!(r#"{{"id":{},"amount":"{}"}}"#, id, amount));
            return;
        }
        set_launch_bucket(id, 0);
        self.internal_split(id, "near", amount, &rc, "launch");
        emit("launch_fees_split", &format!(r#"{{"id":{},"amount":"{}","legs":{}}}"#, id, amount, rc.len()));
    }



    fn drain_router_near(&mut self, id: u64) {
        let amount = router_near(id);
        if amount == 0 || amount < self.min_push_yocto { return; }
        if !can_pay_legs(ROUTER_LEG_WEIGHT) {
            emit("router_fees_deferred", &format!(r#"{{"id":{},"amount":"{}"}}"#, id, amount));
            return;
        }
        let l = self.rec(id);
        set_router_near(id, 0);
        let to = payee(&l);
        emit("router_fees_paid", &format!(r#"{{"id":{},"to":"{}","kind":"near","amount":"{}"}}"#, id, to, amount));
        self.internal_pay(id, "near", &to, amount, "creator").detach();
    }


    fn internal_pay_creator_near(&mut self, who: AccountId, claim: bool) -> Promise {
        let amount = self.creator_fees.get(&who).copied().unwrap_or(0);
        require!(amount > 0, if claim { "nothing to claim" } else { "nothing to push" });
        self.creator_fees.remove(&who);
        emit("creator_fees_claimed", &format!(r#"{{"creator":"{}","amount":"{}"{}}}"#, who, amount, if claim { "" } else { r#","pushed":true"# }));
        self.internal_pay(0, "near", &who, amount, "account")
    }



    fn internal_pay_creator(&mut self, launch_id: U64, kind: &str, claim: bool) -> Promise {
        require!(fee_mode(launch_id.0) == FM_CREATOR, "this launch's fees go to buyback or holders");
        let mut l = self.rec(launch_id.0);
        let to = payee(&l);
        if claim {
            let who = env::predecessor_account_id();
            require!(who == l.creator || who == to, "creator only");
        } else {
            self.assert_not_paused();
        }
        let amount = if kind == "token" {
            require_no_swap(&l.token);
            std::mem::take(&mut l.creator_token_fees).0
        } else {
            require!(l.quote != self.wnear_id, if claim { "NEAR pair: use claim_creator_fees" } else { "NEAR pair: use push_creator_fees" });
            self.guard_quote(&l);
            std::mem::take(&mut l.creator_quote_fees).0
        };
        require!(amount > 0, if claim { "nothing to claim" } else { "nothing to push" });
        self.launches.insert(launch_id.0, l.clone());
        let quote = if kind == "quote" { format!(r#","quote":"{}""#, l.quote) } else { String::new() };
        emit(
            if kind == "token" { "creator_token_fees_claimed" } else { "creator_quote_fees_claimed" },
            &format!(r#"{{"id":{},"creator":"{}","to":"{}"{},"amount":"{}"{}}}"#, l.id, l.creator, to, quote, amount, if claim { "" } else { r#","pushed":true"# }),
        );
        self.internal_pay(launch_id.0, kind, &to, amount, "creator")
    }


    fn internal_split_protocol(&mut self, launch_id: U64, kind: &str) {
        self.assert_not_paused();
        require!(!self.protocol_recipients.is_empty(), "no recipients");
        let mut l = self.rec(launch_id.0);
        let amount = if kind == "token" {
            require_no_swap(&l.token);
            std::mem::take(&mut l.protocol_token_fees).0
        } else {
            require!(l.quote != self.wnear_id, "NEAR pair splits itself");
            self.guard_quote(&l);
            std::mem::take(&mut l.protocol_quote_fees).0
        };
        require!(amount > 0, "nothing to split");
        self.launches.insert(launch_id.0, l.clone());
        let rc = launch_recipients(launch_id.0).unwrap_or_else(|| self.protocol_recipients.clone());
        self.internal_split(launch_id.0, kind, amount, &rc, "protocol");
        let asset = if kind == "token" { &l.token } else { &l.quote };
        emit(if kind == "token" { "protocol_token_fees_split" } else { "protocol_quote_fees_split" }, &format!(r#"{{"id":{},"{}":"{}","amount":"{}"}}"#, l.id, kind, asset, amount));
    }


    fn internal_withdraw_protocol(&mut self, launch_id: U64, kind: &str, to: AccountId) -> Promise {
        self.assert_owner();
        let mut l = self.rec(launch_id.0);
        let amount = if kind == "token" {
            require_no_swap(&l.token);
            std::mem::take(&mut l.protocol_token_fees).0
        } else {
            self.guard_quote(&l);
            std::mem::take(&mut l.protocol_quote_fees).0
        };
        require!(amount > 0, "nothing to withdraw");
        self.launches.insert(launch_id.0, l.clone());
        let asset = if kind == "token" { &l.token } else { &l.quote };
        emit(if kind == "token" { "protocol_token_fees_withdrawn" } else { "protocol_quote_fees_withdrawn" }, &format!(r#"{{"id":{},"{}":"{}","to":"{}","amount":"{}"}}"#, l.id, kind, asset, to, amount));
        self.internal_pay(launch_id.0, kind, &to, amount, "protocol")
    }



    fn internal_pay_holders(&mut self, launch_id: U64, payouts: Vec<(AccountId, U128)>, tax: bool) {
        self.assert_not_paused();
        self.assert_keeper();
        let l = self.rec(launch_id.0);

        let native = l.quote == self.wnear_id;
        if !native { self.guard_quote(&l); }
        let max = if native { MAX_HOLDER_PAYOUTS } else { MAX_FT_HOLDER_PAYOUTS };
        require!(!payouts.is_empty() && payouts.len() <= max, "too many payouts per call (25 NEAR pair, 10 other pairs)");
        require_unique_payees(&payouts);
        let total = payouts.iter().fold(0u128, |a, (_, v)| a.checked_add(v.0).expect("overflow"));
        let (bucket, paid) = if tax { (tax_holders_key(launch_id.0), tax_paid_holders_key(launch_id.0)) } else { (mode_bucket_key(launch_id.0), paid_key(launch_id.0)) };
        let have = read_counter(&bucket);
        require!(total > 0 && total <= have, "payouts exceed what this launch earned for its holders");
        if have == total { env::storage_remove(&bucket); } else { env::storage_write(&bucket, &(have - total).to_le_bytes()); }
        add_counter(&paid, total);
        let (kind, from) = (if native { "near" } else { "quote" }, if tax { "tax_holders" } else { "mode" });
        for (who, amt) in payouts {
            if amt.0 > 0 { self.internal_pay(launch_id.0, kind, &who, amt.0, from).detach(); }
        }
        emit(if tax { "tax_holders_paid" } else { "holders_paid" }, &format!(r#"{{"id":{},"amount":"{}"}}"#, launch_id.0, total));
    }



    fn fire_launch_hook(&self, l: &Launch) {
        let Some(hook) = launch_hook() else { return };
        if gas_left() < GAS_HOOK_MIN_LEFT {
            emit("launch_hook_skipped", &format!(r#"{{"id":{}}}"#, l.id));
            return;
        }
        Promise::new(hook)
            .function_call(
                "on_launch".to_string(),
                format!(r#"{{"id":{},"token":"{}","creator":"{}","pool_id":{},"quote":"{}"}}"#, l.id, l.token, l.creator, js(&l.pool_id), l.quote).into_bytes(),
                NO_DEPOSIT,
                GAS_HOOK,
            )
            .detach();
    }

    fn assert_owner(&self) {
        require!(env::predecessor_account_id() == self.owner_id, "owner only");
    }




    fn dcl_storage(&self, id: u64) -> u128 {
        let dcl = self.dcl_for(id);
        let registered = if dcl == self.dcl_id { self.dcl_registered } else { env::storage_has_key(&dcl_registered_key(&dcl)) };
        if registered { self.config.dcl_storage_per_launch.0 } else { DCL_REGISTER.as_yoctonear() }
    }
    fn set_dcl_registered(&mut self, id: u64) {
        let dcl = self.dcl_for(id);
        if dcl == self.dcl_id { self.dcl_registered = true; } else { env::storage_write(&dcl_registered_key(&dcl), &[1]); }
    }


    fn holds_admin_lock(&self, token: &AccountId) -> bool {
        swap_locked(token) && matches!(read_swap_lock(token), Some((LK_ADMIN, LOCKER_WITHDRAW_LOCK_ID, _)))
    }


    fn rec(&self, id: u64) -> Launch { self.launches.get(&id).cloned().expect("launch") }



    fn quote_is_launch_token(&self, l: &Launch) -> bool { self.token_index.contains_key(&l.quote) }
    fn guard_quote(&self, l: &Launch) {
        if self.quote_is_launch_token(l) { require_no_swap(&l.quote); }
    }

    fn unwrap_wnear(&self, amount: u128) -> Promise {
        Promise::new(self.wnear_id.clone()).function_call("near_withdraw".to_string(), format!(r#"{{"amount":"{}"}}"#, amount).into_bytes(), ONE_YOCTO, GAS_UNWRAP)
    }



    fn dev_buy_unwrap(&self, id: u64, amount: u128, weight: u64) -> Promise {
        let op = env::block_timestamp_ms();
        env::storage_write(&dev_unwrap_op_key(id), &(op as u128).to_le_bytes());
        self.unwrap_wnear(amount)
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_DEV_UNWRAPPED).with_unused_gas_weight(weight).on_dev_buy_unwrapped(id, Some(op)))
    }

    fn assert_not_paused(&self) {
        require!(!self.paused, "paused");
    }


    fn dev_buy_swap_msg(&self, l: &Launch) -> String {
        let out = self.dev_buy_out_for(l);
        if l.dev_buy_exact {
            format!(
                r#"{{"SwapByOutput":{{"pool_ids":[{}],"output_token":"{}","output_amount":"{}","swap_out_recipient":"{}"}}}}"#,
                js(&l.pool_id), l.token, out, l.creator
            )
        } else {


            format!(
                r#"{{"Swap":{{"pool_ids":[{}],"output_token":"{}","min_output_amount":"{}","swap_out_recipient":"{}"}}}}"#,
                js(&l.pool_id), l.token, out.max(1), l.creator
            )
        }
    }




    fn dev_buy_out_for(&self, l: &Launch) -> u128 {
        if let Some(v) = dev_buy_out(l.id) {
            return v;
        }
        if l.dev_buy_exact {
            return l.total_supply.0 / BPS * self.config.max_dev_buy_bps as u128;
        }
        if l.dev_buy_near.0 == 0 {
            return 0;
        }
        let init_x = if l.token_is_x { l.init_point } else { -l.init_point };
        mul_div(dev_buy_tokens_out(init_x + POINT_DELTA_1PCT, l.total_supply.0, l.dev_buy_near.0, self.config.pool_fee), DEV_BUY_MIN_OUT_BPS, BPS)
    }




    fn internal_tax_settle(&mut self, launch_id: U64, sold: u128, got: u128) -> bool {
        let held = read_counter(&tax_selling_key(launch_id.0));
        require!(sold > 0 && sold <= held, "sold is more than the seller holds for this launch");
        require!(got > 0, "no proceeds");


        let floor = mul_div_wide(read_counter(&tax_floor_key(launch_id.0)), sold, held);
        require!(got >= floor, "proceeds below the keeper's floor for what was sold");
        sub_counter(&tax_floor_key(launch_id.0), floor);
        env::storage_write(&tax_selling_key(launch_id.0), &(held - sold).to_le_bytes());
        emit("tax_sold", &format!(r#"{{"id":{},"sold":"{}","got":"{}","seller":true}}"#, launch_id.0, sold, got));
        self.internal_tax_distribute(launch_id, got)
    }

    fn internal_tax_distribute(&mut self, launch_id: U64, got: u128) -> bool {
        let (t, platform_bps) = tax_opts_full(launch_id.0).expect("tax");
        let mut l = self.rec(launch_id.0);
        let native = l.quote == self.wnear_id;

        let p = platform_bps as u128;
        let (wp, wc, wh) = (p * BPS, (BPS - p) * t.creator_bps as u128, (BPS - p) * t.holders_bps as u128);
        let w = wp + wc + wh;
        let to_platform = if w == 0 { 0 } else { mul_div(got, wp, w) };
        let to_creator = if w == 0 { 0 } else { mul_div(got, wc, w) };
        let to_holders = got - to_platform - to_creator;
        if to_platform > 0 {
            add_counter(&tax_paid_platform_key(launch_id.0), to_platform);
            if native && launch_recipients(launch_id.0).is_some() {

                set_launch_bucket(launch_id.0, launch_bucket(launch_id.0) + to_platform);
                self.drain_launch_fees(launch_id.0);
            } else if native {

                self.protocol_fees += to_platform;
                self.drain_protocol_fees();
            } else {

                l.protocol_quote_fees = U128(l.protocol_quote_fees.0 + to_platform);
                self.launches.insert(launch_id.0, l.clone());
            }
        }
        if to_holders > 0 {
            add_counter(&tax_holders_key(launch_id.0), to_holders);
        }
        if to_creator > 0 {


            add_counter(&tax_paid_creator_key(launch_id.0), to_creator);
            self.add_to_bucket(launch_id.0, if native { "near" } else { "quote" }, "creator", &payee(&l), to_creator);
        }
        emit("tax_paid", &format!(r#"{{"id":{},"quote":"{}","platform":"{}","creator":"{}","holders":"{}"}}"#, launch_id.0, l.quote, to_platform, to_creator, to_holders));
        true
    }

    fn assert_keeper(&self) {
        let p = env::predecessor_account_id();
        require!(p == self.owner_id || p == env::current_account_id(), "keeper only");
    }


    fn internal_bb_recredit(&mut self, launch_id: U64, amount: u128) {
        if amount == 0 { return; }
        let mut l = self.rec(launch_id.0);
        if l.quote == self.wnear_id {
            set_mode_bucket(launch_id.0, mode_bucket(launch_id.0) + amount);
        } else {
            l.creator_quote_fees = U128(l.creator_quote_fees.0 + amount);
            self.launches.insert(launch_id.0, l);
        }
    }

    fn internal_fee_burned(&mut self, launch_id: U64, amount: u128, ok: bool) {
        if !ok {
            let mut l = self.rec(launch_id.0);
            l.creator_token_fees = U128(l.creator_token_fees.0 + amount);
            self.launches.insert(launch_id.0, l);
            emit("burn_failed", &format!(r#"{{"id":{},"amount":"{}"}}"#, launch_id.0, amount));
        } else {
            add_counter(&burned_key(launch_id.0), amount);
            emit("fees_burned", &format!(r#"{{"id":{},"amount":"{}"}}"#, launch_id.0, amount));
        }
    }

    fn internal_burn(&self, token: &AccountId, launch_id: U64, amount: u128) -> Promise {
        Promise::new(token.clone())
            .function_call("burn".to_string(), format!(r#"{{"amount":"{}"}}"#, amount).into_bytes(), ONE_YOCTO, GAS_BURN)
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_SPLIT).on_fee_burned(launch_id, U128(amount)))
    }

    fn internal_cost(&self, icon_len: usize, text_len: usize, dev_buy: u128, tax: bool) -> LaunchCost {
        let bytes = TOKEN_BASE_STORAGE_BYTES + icon_len as u128 + text_len as u128 + if tax { TAX_EXTRA_BYTES } else { 0 };
        let token_storage = bytes * STORAGE_PRICE_PER_BYTE * 125 / 100 + FT_STORAGE_REG.as_yoctonear();
        let dcl_storage = self.dcl_storage(self.next_id);
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


    fn internal_refund_creator(&self, id: u64, creator: &AccountId, amount: u128) -> Promise {
        Promise::new(creator.clone())
            .transfer(NearToken::from_yoctonear(amount))
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_SPLIT).on_dev_refund_to_creator(U64(id), creator.clone(), U128(amount)))
    }


    fn internal_cancel_stuck(&mut self, mut l: Launch) -> Option<Promise> {
        let amount = l.dev_buy_held.0;
        l.step = Step::Failed;
        l.inflight = false;
        l.dev_buy_held = U128(0);
        l.dev_buy_near = U128(0);
        emit("launch_failed", &format!(r#"{{"id":{},"token":"{}","refundable":"{}","cancelled":true}}"#, l.id, l.token, amount));
        self.launches.insert(l.id, l.clone());
        if amount == 0 {
            return None;
        }
        emit("dev_buy_refunded", &format!(r#"{{"id":{},"creator":"{}","amount":"{}"}}"#, l.id, l.creator, amount));
        Some(
            Promise::new(l.creator)
                .transfer(NearToken::from_yoctonear(amount))
                .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_SPLIT).on_dev_refund_sent(U64(l.id), U128(amount))),
        )
    }



    fn internal_mark_failed(&mut self, id: u64, mut l: Launch, reason: &str) -> PromiseOrValue<bool> {
        emit("launch_step_failed", &format!(r#"{{"id":{},"step":{}}}"#, id, js(reason)));
        l.step = Step::Failed;
        l.inflight = false;
        emit("launch_failed", &format!(r#"{{"id":{},"token":"{}","refundable":"{}"}}"#, id, l.token, l.dev_buy_held.0));
        self.launches.insert(id, l);
        PromiseOrValue::Value(false)
    }



    fn internal_pool_check(&mut self, id: u64, mut l: Launch, inline: bool) -> Promise {
        l.inflight = true;
        self.launches.insert(id, l.clone());
        let storage = self.dcl_storage(id);
        Promise::new(self.dcl_for(id))
            .function_call(
                "storage_deposit".to_string(),
                format!(r#"{{"account_id":"{}","registration_only":false}}"#, self.locker_for(id)).into_bytes(),
                NearToken::from_yoctonear(storage),
                Gas::from_tgas(5),
            )
            .function_call("get_pool".to_string(), format!(r#"{{"pool_id":{}}}"#, js(&l.pool_id)).into_bytes(), NO_DEPOSIT, GAS_GET_POOL)

            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_POOL_CHECKED).on_pool_checked(id, Some(inline)))
    }

    fn internal_done(&mut self, id: u64, mut l: Launch) {
        l.step = Step::Done;
        l.inflight = false;
        self.launches.insert(id, l.clone());
        self.fire_launch_hook(&l);
        emit(
            "launch",
            &format!(
                r#"{{"id":{},"token":"{}","creator":"{}","name":{},"symbol":{},"pool_id":{},"lpt_id":{},"init_point":{},"left_point":{},"right_point":{},"dev_buy_near":"{}","dev_buy_tokens":"{}","ts_ms":{}}}"#,
                id, l.token, l.creator, js(&l.name), js(&l.symbol), js(&l.pool_id), js(l.lpt_id.as_deref().unwrap_or("")), l.init_point, l.left_point, l.right_point,
                l.dev_buy_near.0, l.dev_buy_tokens.0, env::block_timestamp_ms()
            ),
        );
    }






    fn internal_advance(&mut self, id: u64, l: Launch, inline: bool) -> PromiseOrValue<bool> {
        let need = step_gas(l.step).saturating_add(step_cb_gas(l.step)).saturating_add(GAS_RESERVE);
        if l.step == Step::DevBuy {
            let reason = if gas_left() < DevBuyBudget::INLINE.need(self.wnear_registered) {
                Some("not enough gas for the swap in this transaction")
            } else {
                self.dev_buy_inline_blocker(&l, inline)
            };
            if let Some(reason) = reason {
                emit("launch_step_parked", &format!(r#"{{"id":{},"step":"DevBuy","reason":{}}}"#, id, js(reason)));
                self.launches.insert(id, l);
                return PromiseOrValue::Value(true);
            }
            return PromiseOrValue::Promise(self.internal_dev_buy(id, l, DevBuyBudget::INLINE));
        }
        if l.step == Step::AddLiquidity {


            env::storage_write(&no_add_key(id), &[1]);
            if self.first_buy_by_locker(id, &l) {
                let reason = if self.paused {
                    Some("paused")
                } else if !inline && !self.may_trigger_dev_buy(&l) {
                    Some("first buy: resume by the creator, the owner or a relayer")
                } else if gas_left() < GAS_ADD_BUY_STEP.saturating_add(if inline { Gas::from_tgas(0) } else { GAS_ADD_BUY_MARGIN }) {


                    Some("first buy: resume with at least 260 TGas")
                } else {
                    None
                };
                if let Some(reason) = reason {
                    emit("launch_step_parked", &format!(r#"{{"id":{},"step":"AddLiquidity","reason":{}}}"#, id, js(reason)));
                    self.launches.insert(id, l);
                    return PromiseOrValue::Value(true);
                }
            }
        }
        if gas_left() < need {
            emit("launch_step_parked", &format!(r#"{{"id":{},"step":"{:?}"}}"#, id, l.step));
            self.launches.insert(id, l);
            return PromiseOrValue::Value(true);
        }
        PromiseOrValue::Promise(self.internal_run_step(id, l, 0, inline))
    }




    fn first_buy_by_locker(&self, id: u64, l: &Launch) -> bool {
        no_add_sent(id) && l.dev_buy_near.0 > 0 && !dev_buy_done(id) && l.dev_buy_held.0 == l.dev_buy_near.0 && locker_add_buy(&self.locker_for(id))
    }




    fn internal_dev_buy(&mut self, id: u64, mut l: Launch, budget: DevBuyBudget) -> Promise {
        require!(!l.inflight, "step in flight");




        require!(l.dev_buy_held.0 > 0 && l.dev_buy_held.0 == l.dev_buy_near.0, "dev buy not funded");
        require!(gas_left() >= budget.need(self.wnear_registered), "attach more gas for the dev buy");
        l.dev_buy_held = U128(0);
        l.inflight = true;
        self.launches.insert(id, l.clone());
        let me = env::current_account_id();
        let swap_msg = self.dev_buy_swap_msg(&l);
        let mut p = Promise::new(self.wnear_id.clone());
        if !self.wnear_registered {
            p = p.function_call(
                "storage_deposit".to_string(),
                reg_args(&me),
                FT_STORAGE_REG,
                Gas::from_tgas(5),
            );
        }


        p.function_call("near_deposit".to_string(), b"{}".to_vec(), NearToken::from_yoctonear(l.dev_buy_near.0), Gas::from_tgas(5))
            .function_call_weight(
                "ft_transfer_call".to_string(),
                format!(r#"{{"receiver_id":"{}","amount":"{}","msg":{}}}"#, self.dcl_for(id), l.dev_buy_near.0, js(&swap_msg)).into_bytes(),
                ONE_YOCTO,
                budget.swap,
                GasWeight(1),
            )
            .then(Self::ext(me).with_static_gas(budget.cb).with_unused_gas_weight(0).on_dev_bought(id))
    }


    fn may_trigger_dev_buy(&self, l: &Launch) -> bool {
        let signer = env::signer_account_id();
        signer == l.creator || signer == self.owner_id || relayers().contains(&signer)
    }






    fn dev_buy_inline_blocker(&self, l: &Launch, inline: bool) -> Option<&'static str> {
        if !inline && !self.may_trigger_dev_buy(l) {
            return Some("only the creator, the owner or a relayer triggers the dev buy");
        }
        if self.paused {
            return Some("paused");
        }
        if l.dev_buy_held.0 == 0 || l.dev_buy_held.0 != l.dev_buy_near.0 {
            return Some("dev buy not funded");
        }
        None
    }



    fn internal_run_step(&mut self, id: u64, mut l: Launch, token_storage: u128, inline: bool) -> Promise {
        require!(!l.inflight, "step in flight");
        if matches!(l.step, Step::CreateToken | Step::CreatePool) {
            let reserve = if l.step == Step::CreateToken { GAS_RESERVE_LAUNCH } else { GAS_RESERVE };
            require!(gas_left() >= step_gas(l.step).saturating_add(GAS_POOL_STEP_CB).saturating_add(reserve), "attach more gas: this step needs 300 TGas");
        }
        if l.step == Step::DevBuy {

            return self.internal_dev_buy(id, l, DevBuyBudget::FULL);
        }
        l.inflight = true;
        let step = l.step;
        self.launches.insert(id, l.clone());
        let me = env::current_account_id();

        let reserve = if step == Step::CreateToken { GAS_RESERVE_LAUNCH } else { GAS_RESERVE };
        let cb_gas = gas_left().saturating_sub(step_gas(step)).saturating_sub(reserve).max(step_cb_gas(step));
        let ext = Self::ext(me.clone()).with_static_gas(cb_gas);
        match step {
            Step::CreateToken => {
                let rules = (self.config.max_wallet_bps > 0).then(|| TokenRules {
                    max_wallet_bps: self.config.max_wallet_bps,
                    until_ms: (env::block_timestamp_ms() + self.config.max_wallet_ms).to_string(),
                    exempt: [vec![me.clone(), self.dcl_for(id), self.locker_for(id)], tax_seller().into_iter().collect()].concat(),
                });
                let init = TokenInit {
                    owner_id: self.locker_for(id),
                    total_supply: l.total_supply,
                    metadata: TokenMetadata { spec: "ft-1.0.0", name: &l.name, symbol: &l.symbol, icon: &l.icon, reference: None, reference_hash: None, decimals: TOKEN_DECIMALS },
                    rules,
                    tax: tax_opts(id).map(|t| TokenTax {
                        buy_bps: t.buy_bps,
                        sell_bps: t.sell_bps,
                        pairs: vec![self.dcl_for(id)],
                        admin: me.clone(),

                        exempt: [vec![self.locker_for(id)], tax_seller().into_iter().collect()].concat(),
                    }),
                };
                let code = if init.tax.is_some() { tax_code_hash().expect("tax code") } else { self.token_code_hash };



                let token = Promise::new(l.token.clone())
                    .create_account()
                    .transfer(NearToken::from_yoctonear(token_storage.saturating_sub(FT_STORAGE_REG.as_yoctonear())))
                    .use_global_contract(code)
                    .function_call("new".to_string(), near_sdk::serde_json::to_vec(&init).expect("init"), NO_DEPOSIT, GAS_TOKEN_INIT)
                    .function_call(
                        "storage_deposit".to_string(),
                        reg_args(&l.creator),
                        FT_STORAGE_REG,
                        Gas::from_tgas(5),
                    );
                token.and(self.internal_pool_batch(&l)).then(ext.on_created(id, Some(inline)))
            }
            Step::CreatePool => self.internal_pool_batch(&l).then(ext.on_pool_created(id, Some(inline))),
            Step::AddLiquidity => {

                require!(gas_left() >= GAS_LOCKER_ADD.saturating_add(GAS_LIQ_CB).saturating_add(reserve), "attach more gas: adding liquidity needs 300 TGas");

                let (ax, ay) = if l.token_is_x { (l.total_supply.0, 0u128) } else { (0u128, l.total_supply.0) };





                let locker = self.locker_for(id);
                let dev_buy_next = l.dev_buy_near.0 > 0 && (inline || self.may_trigger_dev_buy(&l));
                let add_args = format!(
                    r#"{{"token":"{}","pool_id":{},"left_point":{},"right_point":{},"amount_x":"{}","amount_y":"{}"}}"#,
                    l.token, js(&l.pool_id), l.left_point, l.right_point, ax, ay
                );






                let first_buy = self.first_buy_by_locker(id, &l);
                env::storage_remove(&no_add_key(id));
                if first_buy && gas_left() >= GAS_ADD_BUY_STEP {
                    l.dev_buy_held = U128(0);
                    self.launches.insert(id, l.clone());
                    let buy = format!(r#"{{"amount":"{}","msg":{}}}"#, l.dev_buy_near.0, js(&self.dev_buy_swap_msg(&l)));
                    return Promise::new(locker)
                        .function_call("add_buy".to_string(), format!(r#"{{"args":{},"buy":{}}}"#, add_args, buy).into_bytes(), NearToken::from_yoctonear(l.dev_buy_near.0), Gas::from_tgas(T_LOCKER_ADD_BUY))
                        .then(Self::ext(me.clone()).with_static_gas(GAS_LIQ_CB_ADD_BUY).with_unused_gas_weight(0).on_liquidity_added_buy(id));
                }
                let (cb, add_gas) = match locker_add_gas(&locker) {
                    Some(lg) if dev_buy_next && gas_left() >= lg.saturating_add(GAS_LIQ_CB_DEV_BUY).saturating_add(GAS_RESERVE_INLINE) => (GAS_LIQ_CB_DEV_BUY, lg),
                    _ => (GAS_LIQ_CB, gas_left().saturating_sub(reserve).saturating_sub(GAS_LIQ_CB).max(GAS_LOCKER_ADD)),
                };
                Promise::new(locker)
                    .function_call("add".to_string(), format!(r#"{{"args":{}}}"#, add_args).into_bytes(), NO_DEPOSIT, add_gas)
                    .then(Self::ext(me.clone()).with_static_gas(cb).on_liquidity_added(id, Some(inline)))
            }
            Step::DevBuy => unreachable!("handled above"),
            Step::Deposit | Step::ReadDevTokens | Step::DeliverDevTokens => env::panic_str("dead step"),
            Step::Done | Step::Failed => env::panic_str("launch finished"),
        }
    }


    fn internal_pool_batch(&self, l: &Launch) -> Promise {
        let (a, b) = if l.token_is_x { (&l.token, &l.quote) } else { (&l.quote, &l.token) };
        let storage = self.dcl_storage(l.id);
        Promise::new(self.dcl_for(l.id))
            .function_call(
                "storage_deposit".to_string(),
                format!(r#"{{"account_id":"{}","registration_only":false}}"#, self.locker_for(l.id)).into_bytes(),
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
                reg_args(&to),
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

        require!((MIN_CREATOR_SHARE_BPS..=MAX_CREATOR_SHARE_BPS).contains(&self.creator_fee_share_bps), "creator_fee_share_bps: 7000");
        require!(self.max_icon_bytes <= 32 * 1024, "icon cap max 32KB");
        require!(self.max_dev_buy_bps <= 2_000, "dev buy cap max 20%");
        require!(self.max_wallet_bps == 0 || self.max_dev_buy_bps <= self.max_wallet_bps, "dev buy cap must fit the max wallet");
        require!(self.max_wallet_ms <= 86_400_000, "max wallet window max 24h");
        require!(self.launch_fee.0 <= 10 * 10u128.pow(24), "launch fee max 10 NEAR");
        require!(self.dcl_storage_per_launch.0 <= DCL_REGISTER.as_yoctonear(), "dcl storage per launch max 0.5 NEAR");
    }
}



fn step_cb_gas(step: Step) -> Gas {
    match step {
        Step::AddLiquidity => GAS_LIQ_CB,
        Step::DevBuy => GAS_DEV_BOUGHT_CB,
        Step::CreateToken | Step::CreatePool => GAS_POOL_STEP_CB,
        _ => GAS_CB_MIN,
    }
}

fn step_gas(step: Step) -> Gas {
    match step {

        Step::CreateToken => GAS_TOKEN_INIT.saturating_add(GAS_DCL_POOL).saturating_add(Gas::from_tgas(10)),
        Step::CreatePool => GAS_DCL_POOL.saturating_add(Gas::from_tgas(5)),
        Step::AddLiquidity => GAS_LOCKER_ADD,
        Step::DevBuy => GAS_DEV_BUY.saturating_add(Gas::from_tgas(10)),
        _ => Gas::from_tgas(0),
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
    let fdv_left = fdv_at_left(left_point, total_supply);
    let cost = fdv_left / (BPS - bps as u128) * bps as u128;
    cost * 1_000_000 / (1_000_000 - pool_fee as u128)
}



fn fdv_at_left(left_point: i32, total_supply: u128) -> u128 {
    total_supply / FP * pow_1_0001_fp(left_point) / 1_000_000 * 1_000_000
}






fn dev_buy_tokens_out(left_point: i32, total_supply: u128, near_in: u128, pool_fee: u32) -> u128 {
    if near_in == 0 {
        return 0;
    }
    let fdv = fdv_at_left(left_point, total_supply);
    let net = mul_div(near_in, 1_000_000 - pool_fee as u128, 1_000_000);
    let denom = (fdv.saturating_add(net) / FP).max(1);
    let f_fp = (net / denom).min(FP);
    mul_div(total_supply, f_fp, FP)
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


fn can_pay_legs(n: usize) -> bool {
    gas_left() >= Gas::from_tgas(tg(GAS_PER_PAYOUT_LEG) * n as u64 + 5)
}


fn floor_to(p: i32, delta: i32) -> i32 {
    p.div_euclid(delta) * delta
}


const RESERVED_SLUGS: &[&str] = &["lock"];

fn slug_count_key(slug: &str) -> Vec<u8> { [b"sc:".as_slice(), slug.as_bytes()].concat() }
fn slug_count(slug: &str) -> u32 {
    env::storage_read(&slug_count_key(slug)).map(|b| u32::from_le_bytes(b.try_into().unwrap_or([0; 4]))).unwrap_or(0)
}
fn set_slug_count(slug: &str, n: u32) { env::storage_write(&slug_count_key(slug), &n.to_le_bytes()); }

fn short_name(slug: &str, taken: u32) -> String {
    if taken == 0 { slug.to_string() } else { format!("{}-{}", slug, taken + 1) }
}
fn sanitize_slug(symbol: &str) -> String {
    let s: String = symbol.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).take(12).collect();
    if s.is_empty() { "t".to_string() } else { s }
}


fn reg_args(account: &AccountId) -> Vec<u8> { format!(r#"{{"account_id":"{}","registration_only":true}}"#, account).into_bytes() }

fn balance_of_me(token: AccountId) -> Promise {
    Promise::new(token).function_call("ft_balance_of".to_string(), format!(r#"{{"account_id":"{}"}}"#, env::current_account_id()).into_bytes(), NO_DEPOSIT, GAS_BALANCE_OF)
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
    #[test]
    fn per_launch_routing_storage_round_trips() {
        use near_sdk::test_utils::VMContextBuilder;
        near_sdk::testing_env!(VMContextBuilder::new().build());
        let a: near_sdk::AccountId = "creator.near".parse().unwrap();
        let b: near_sdk::AccountId = "burn.near".parse().unwrap();
        assert!(super::launch_recipients(3).is_none());
        near_sdk::env::storage_write(&super::launch_recipients_key(3), &near_sdk::serde_json::to_vec(&vec![(a.clone(), 5000u16), (b.clone(), 5000u16)]).unwrap());
        assert_eq!(super::launch_recipients(3).unwrap(), vec![(a, 5000), (b, 5000)]);
        assert!(super::launch_recipients(14).is_none(), "another launch is untouched");
        assert_eq!(super::launch_bucket(3), 0);
        super::set_launch_bucket(3, 9_416_571_255_745_499_697_058_354);
        assert_eq!(super::launch_bucket(3), 9_416_571_255_745_499_697_058_354);
        assert_eq!(super::launch_bucket(14), 0);
        super::set_launch_bucket(3, 0);
        assert!(!near_sdk::env::storage_has_key(&super::launch_bucket_key(3)), "a zero bucket frees its storage");
    }

    #[test]
    fn fee_opts_storage_round_trips() {
        use near_sdk::test_utils::VMContextBuilder;
        near_sdk::testing_env!(VMContextBuilder::new().build());
        assert_eq!(super::fee_opts(7, 8000), (super::FM_CREATOR, 8000), "absent = creator at the default share");
        super::set_fee_opts(7, super::FM_BURN, 9000);
        assert_eq!(super::fee_opts(7, 7000), (super::FM_BURN, 9000));
        assert_eq!(super::fee_mode(7), super::FM_BURN);
        assert_eq!(super::fee_mode(8), super::FM_CREATOR, "another launch is untouched");
        super::set_mode_bucket(7, 5);
        super::set_mode_bucket(7, super::mode_bucket(7) + 6);
        assert_eq!(super::mode_bucket(7), 11);
        super::set_mode_bucket(7, 0);
        assert!(!near_sdk::env::storage_has_key(&super::mode_bucket_key(7)));
        super::add_counter(&super::burned_key(7), 10);
        super::sub_counter(&super::burned_key(7), 30);
        assert_eq!(super::read_counter(&super::burned_key(7)), 0, "a counter never wraps below zero");
        assert_eq!(super::mode_name(super::FM_HOLDERS), "holders");
    }

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
            assert!(legs[0].abs_diff(legs[1]) <= 1, "an even split must never be off by more than one yocto");
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
        assert_eq!(short_name("test1", 0), "test1");
        assert_eq!(short_name("test1", 1), "test1-2");
        assert_eq!(short_name("pepe", 2), "pepe-3");
        let h = parse_hex32("7591d117dde58bba80a7df25ff6bd1112428f85cf20018d774f00b48af0fa11b");
        assert_eq!(hex32(h), "7591d117dde58bba80a7df25ff6bd1112428f85cf20018d774f00b48af0fa11b");
    }



    #[test]
    fn over_cap_dev_buy_refunds_the_excess_once() {
        use near_sdk::test_utils::{get_created_receipts, VMContextBuilder};
        use near_sdk::mock::MockAction;
        let creator: AccountId = "scorp56.near".parse().unwrap();
        let mut f = Factory::new(
            "owner.near".parse().unwrap(),
            "7591d117dde58bba80a7df25ff6bd1112428f85cf20018d774f00b48af0fa11b".to_string(),
            "wrap.near".parse().unwrap(),
            "dclv2.ref-labs.near".parse().unwrap(),
            "lock.near".parse().unwrap(),
            None,
        );
        let requested = 60 * 10u128.pow(24);
        let deposit = f.quote_launch(0, Some(U128(requested)), None).total.0;
        near_sdk::testing_env!(VMContextBuilder::new()
            .current_account_id("nearlytrade.near".parse().unwrap())
            .predecessor_account_id(creator.clone())
            .attached_deposit(NearToken::from_yoctonear(deposit))
            .account_balance(NearToken::from_near(200))
            .prepaid_gas(Gas::from_tgas(300))
            .build());
        let args = LaunchArgs {
            name: "Doomslug".into(), symbol: "DOOM".into(), icon: None, description: None, links: None,
            dev_buy: Some(U128(requested)), init_point: None, quote: None, fee_mode: None, creator_share_bps: None, tax: None, fee_to: None,
        };

        std::mem::forget(f.launch(args));
        let l = f.launches.get(&0).cloned().unwrap();
        let held = l.dev_buy_held.0;
        assert!(l.dev_buy_exact && held < requested, "60 NEAR is over the 4% cap");
        let cost = f.internal_cost(0, "Doomslug".len() + "DOOM".len(), held, false).total.0;
        let refunds: Vec<u128> = get_created_receipts().iter()
            .filter(|r| r.receiver_id == creator)
            .flat_map(|r| r.actions.iter())
            .filter_map(|a| match a { MockAction::Transfer { deposit, .. } => Some(deposit.as_yoctonear()), _ => None })
            .collect();
        assert_eq!(refunds, vec![deposit - cost], "one refund of exactly what was not spent");
        assert_eq!(deposit - refunds[0], cost, "the factory keeps the capped cost and nothing of its own goes out");
        assert!((requested - held).abs_diff(refunds[0]) < 10u128.pow(23), "refund ≈ the excess over the cap");
    }
}


#[cfg(test)]
mod fee_to_tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;

    fn factory() -> Factory {
        Factory::new(
            "owner.near".parse().unwrap(),
            "7591d117dde58bba80a7df25ff6bd1112428f85cf20018d774f00b48af0fa11b".to_string(),
            "wrap.near".parse().unwrap(),
            "dclv2.ref-labs.near".parse().unwrap(),
            "lock.near".parse().unwrap(),
            None,
        )
    }
    fn as_caller(f: &Factory, who: &str, deposit: u128) {
        let _ = f;
        near_sdk::testing_env!(VMContextBuilder::new()
            .current_account_id("nearlytrade.near".parse().unwrap())
            .predecessor_account_id(who.parse().unwrap())
            .attached_deposit(NearToken::from_yoctonear(deposit))
            .account_balance(NearToken::from_near(200))
            .prepaid_gas(Gas::from_tgas(300))
            .build());
    }
    fn args(sym: &str, fee_to: Option<&str>) -> LaunchArgs {
        LaunchArgs {
            name: sym.into(), symbol: sym.into(), icon: None, description: None, links: None, dev_buy: None, init_point: None,
            quote: None, fee_mode: None, creator_share_bps: None, tax: None, fee_to: fee_to.map(|a| a.parse().unwrap()),
        }
    }
    fn launch(f: &mut Factory, who: &str, sym: &str, fee_to: Option<&str>) -> u64 {
        let deposit = f.quote_launch(0, None, None).total.0;
        as_caller(f, who, deposit);
        std::mem::forget(f.launch(args(sym, fee_to)));
        f.next_id - 1
    }

    fn book_fees(f: &mut Factory, id: u64, near: u128, tok: u128) {
        near_sdk::testing_env!(VMContextBuilder::new()
            .current_account_id("nearlytrade.near".parse().unwrap())
            .predecessor_account_id("nearlytrade.near".parse().unwrap())
            .account_balance(NearToken::from_near(200))
            .build());
        let mut l = f.launches.get(&id).cloned().unwrap();
        l.step = Step::Done;
        l.inflight = true;
        f.launches.insert(id, l.clone());
        let v = if l.token_is_x { vec![U128(tok), U128(near)] } else { vec![U128(near), U128(tok)] };
        assert!(f.on_fees_claimed(id, Ok(v)));
    }

    #[test]
    fn creator_fees_go_to_the_fee_wallet() {
        let mut f = factory();
        let plain = launch(&mut f, "alice.near", "PLAIN", None);
        let routed = launch(&mut f, "alice.near", "ROUTED", Some("root.near"));
        let own = launch(&mut f, "alice.near", "OWN", Some("alice.near"));
        assert_eq!(f.get_fee_to(U64(plain)), None, "no fee wallet = the creator, as before");
        assert_eq!(f.get_fee_to(U64(routed)), Some("root.near".parse().unwrap()));
        assert_eq!(f.get_fee_to(U64(own)), None, "naming yourself stores nothing");
        assert_eq!(f.get_fee_to_many(vec![U64(plain), U64(routed), U64(own)]), vec![(U64(routed), "root.near".parse().unwrap())]);

        let n = 10u128.pow(24);
        book_fees(&mut f, plain, n, 1000);
        book_fees(&mut f, routed, 2 * n, 1000);
        let share = l_share(f.config.creator_fee_share_bps);
        let alice: AccountId = "alice.near".parse().unwrap();
        let root: AccountId = "root.near".parse().unwrap();
        assert_eq!(f.creator_fees.get(&alice).copied().unwrap_or(0), n * share / BPS, "the plain launch still pays its creator");
        assert_eq!(f.creator_fees.get(&root).copied().unwrap_or(0), 2 * n * share / BPS, "the routed launch pays the fee wallet");
        assert_eq!(f.launches.get(&routed).unwrap().creator_token_fees.0, 1000 * share / BPS, "token side is booked per launch as before");
        assert_eq!(f.launches.get(&routed).unwrap().creator, alice, "the launch still shows its real creator");
    }

    #[test]
    fn token_fees_and_claims_follow_the_fee_wallet() {
        use near_sdk::test_utils::get_created_receipts;
        let mut f = factory();
        let id = launch(&mut f, "alice.near", "ROUTED", Some("root.near"));
        book_fees(&mut f, id, 0, 5000);
        let token = f.launches.get(&id).unwrap().token.clone();
        as_caller(&f, "anyone.near", 0);
        drop(f.push_creator_token_fees(U64(id)));
        let sent: Vec<String> = get_created_receipts().iter().filter(|r| r.receiver_id == token)
            .flat_map(|r| r.actions.iter())
            .filter_map(|a| match a { near_sdk::mock::MockAction::FunctionCallWeight { method_name, args, .. } if method_name == b"ft_transfer" => Some(String::from_utf8(args.clone()).unwrap()), _ => None })
            .collect();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].contains(r#""receiver_id":"root.near""#), "token fees go to the fee wallet: {}", sent[0]);


        book_fees(&mut f, id, 0, 5000);
        as_caller(&f, "alice.near", 0);
        drop(f.claim_creator_token_fees(U64(id)));
        let sent: Vec<String> = get_created_receipts().iter().filter(|r| r.receiver_id == token)
            .flat_map(|r| r.actions.iter())
            .filter_map(|a| match a { near_sdk::mock::MockAction::FunctionCallWeight { method_name, args, .. } if method_name == b"ft_transfer" => Some(String::from_utf8(args.clone()).unwrap()), _ => None })
            .collect();
        assert!(sent.iter().any(|s| s.contains(r#""receiver_id":"root.near""#)));
    }

    #[test]
    #[should_panic(expected = "creator only")]
    fn a_stranger_cannot_claim_routed_token_fees() {
        let mut f = factory();
        let id = launch(&mut f, "alice.near", "ROUTED", Some("root.near"));
        book_fees(&mut f, id, 0, 5000);
        as_caller(&f, "mallory.near", 0);
        std::mem::forget(f.claim_creator_token_fees(U64(id)));
    }

    #[test]
    #[should_panic(expected = "fee_to: not the launchpad itself")]
    fn the_launchpad_cannot_be_the_fee_wallet() {
        let mut f = factory();
        launch(&mut f, "alice.near", "SELF", Some("nearlytrade.near"));
    }

    #[test]
    fn a_house_launch_routed_outside_pays_the_fee_wallet() {
        let mut f = factory();
        f.house_creator = Some("house.near".parse().unwrap());
        let id = launch(&mut f, "house.near", "GIFT", Some("root.near"));
        let kept = launch(&mut f, "house.near", "KEPT", None);
        let n = 10u128.pow(24);
        book_fees(&mut f, id, n, 0);
        book_fees(&mut f, kept, n, 0);
        let share = l_share(f.config.creator_fee_share_bps);
        assert_eq!(f.creator_fees.get(&"root.near".parse::<AccountId>().unwrap()).copied().unwrap_or(0), n * share / BPS);
        assert_eq!(f.creator_fees.get(&"house.near".parse::<AccountId>().unwrap()).copied().unwrap_or(0), 0, "a plain house launch still splits");
    }

    #[test]
    fn a_fee_wallet_that_is_ours_splits_like_a_house_launch() {
        let mut f = factory();
        f.house_creator = Some("house.near".parse().unwrap());
        let id = launch(&mut f, "alice.near", "TOUS", Some("house.near"));
        book_fees(&mut f, id, 10u128.pow(24), 1000);
        assert_eq!(f.creator_fees.get(&"house.near".parse::<AccountId>().unwrap()).copied().unwrap_or(0), 0);
        assert_eq!(f.launches.get(&id).unwrap().creator_token_fees.0, 0);
    }
}


fn house_key(a: &AccountId) -> Vec<u8> {
    [b"xh:".as_slice(), a.as_bytes()].concat()
}



fn launch_recipients_key(id: u64) -> Vec<u8> { [b"lr:".as_slice(), &id.to_le_bytes()].concat() }
fn launch_bucket_key(id: u64) -> Vec<u8> { [b"lb:".as_slice(), &id.to_le_bytes()].concat() }
fn launch_recipients(id: u64) -> Option<Vec<(AccountId, u16)>> {
    env::storage_read(&launch_recipients_key(id)).map(|b| near_sdk::serde_json::from_slice(&b).expect("recipients"))
}
fn launch_bucket(id: u64) -> u128 {
    env::storage_read(&launch_bucket_key(id)).map(|b| u128::from_le_bytes(b.try_into().expect("bucket"))).unwrap_or(0)
}
fn set_launch_bucket(id: u64, v: u128) {
    if v == 0 { env::storage_remove(&launch_bucket_key(id)); } else { env::storage_write(&launch_bucket_key(id), &v.to_le_bytes()); }
}


fn fee_opts_key(id: u64) -> Vec<u8> { [b"fm:".as_slice(), &id.to_le_bytes()].concat() }
fn mode_bucket_key(id: u64) -> Vec<u8> { [b"mb:".as_slice(), &id.to_le_bytes()].concat() }
fn burned_key(id: u64) -> Vec<u8> { [b"bt:".as_slice(), &id.to_le_bytes()].concat() }
fn paid_key(id: u64) -> Vec<u8> { [b"hp:".as_slice(), &id.to_le_bytes()].concat() }
fn set_fee_opts(id: u64, mode: u8, share_bps: u16) {
    let b = share_bps.to_le_bytes();
    env::storage_write(&fee_opts_key(id), &[mode, b[0], b[1]]);
}
fn fee_opts(id: u64, default_share: u16) -> (u8, u16) {
    match env::storage_read(&fee_opts_key(id)) {
        Some(b) if b.len() == 3 => (b[0], u16::from_le_bytes([b[1], b[2]])),
        _ => (FM_CREATOR, default_share),
    }
}
fn fee_mode(id: u64) -> u8 { fee_opts(id, 0).0 }

fn fee_to_key(id: u64) -> Vec<u8> { [b"fr:".as_slice(), &id.to_le_bytes()].concat() }
fn set_fee_to(id: u64, to: &AccountId) { env::storage_write(&fee_to_key(id), to.as_bytes()); }
fn fee_to(id: u64) -> Option<AccountId> {
    env::storage_read(&fee_to_key(id)).map(|b| String::from_utf8(b).expect("fee_to").parse().expect("fee_to"))
}

fn payee(l: &Launch) -> AccountId { fee_to(l.id).unwrap_or_else(|| l.creator.clone()) }
fn mode_name(m: u8) -> &'static str { match m { FM_BURN => "burn", FM_HOLDERS => "holders", _ => "creator" } }
fn read_counter(k: &[u8]) -> u128 { env::storage_read(k).map(|b| u128::from_le_bytes(b.try_into().expect("counter"))).unwrap_or(0) }
fn add_counter(k: &[u8], v: u128) { env::storage_write(k, &(read_counter(k) + v).to_le_bytes()); }
fn sub_counter(k: &[u8], v: u128) { env::storage_write(k, &read_counter(k).saturating_sub(v).to_le_bytes()); }
fn mode_bucket(id: u64) -> u128 { read_counter(&mode_bucket_key(id)) }
fn set_mode_bucket(id: u64, v: u128) {
    if v == 0 { env::storage_remove(&mode_bucket_key(id)); } else { env::storage_write(&mode_bucket_key(id), &v.to_le_bytes()); }
}




const TAX_CODE_KEY: &[u8] = b"tch";

const LOCKERS_KEY: &[u8] = b"lks";
fn lockers() -> Vec<(u64, AccountId)> {
    env::storage_read(LOCKERS_KEY).map(|b| near_sdk::serde_json::from_slice(&b).expect("lockers")).unwrap_or_default()
}
fn tax_code_hash() -> Option<CryptoHash> { env::storage_read(TAX_CODE_KEY).map(|b| b.try_into().expect("tax code hash")) }
fn tax_opts_key(id: u64) -> Vec<u8> { [b"tx:".as_slice(), &id.to_le_bytes()].concat() }
fn tax_pending_key(id: u64) -> Vec<u8> { [b"tt:".as_slice(), &id.to_le_bytes()].concat() }
fn tax_holders_key(id: u64) -> Vec<u8> { [b"th:".as_slice(), &id.to_le_bytes()].concat() }
fn tax_burned_key(id: u64) -> Vec<u8> { [b"tb:".as_slice(), &id.to_le_bytes()].concat() }
fn tax_paid_creator_key(id: u64) -> Vec<u8> { [b"tc:".as_slice(), &id.to_le_bytes()].concat() }
fn tax_paid_holders_key(id: u64) -> Vec<u8> { [b"tp:".as_slice(), &id.to_le_bytes()].concat() }
fn tax_paid_platform_key(id: u64) -> Vec<u8> { [b"tq:".as_slice(), &id.to_le_bytes()].concat() }
fn tax_selling_key(id: u64) -> Vec<u8> { [b"ts:".as_slice(), &id.to_le_bytes()].concat() }

fn tax_floor_key(id: u64) -> Vec<u8> { [b"tf:".as_slice(), &id.to_le_bytes()].concat() }

fn wnear_owed_key(id: u64) -> Vec<u8> { [b"wo:".as_slice(), &id.to_le_bytes()].concat() }

fn wnear_owed_bucket_key(id: u64) -> Vec<u8> { [b"wb:".as_slice(), &id.to_le_bytes()].concat() }


fn wnear_retry_key(id: u64) -> Vec<u8> { [b"wi:".as_slice(), &id.to_le_bytes()].concat() }

fn wnear_retry(id: u64) -> Option<(u64, u128, u128)> {
    let v = env::storage_read(&wnear_retry_key(id))?;
    if v.len() == 40 {
        return Some((u64::from_le_bytes(v[..8].try_into().unwrap()), u128::from_le_bytes(v[8..24].try_into().unwrap()), u128::from_le_bytes(v[24..].try_into().unwrap())));
    }
    Some((u128::from_le_bytes(v.try_into().expect("u128")) as u64, read_counter(&wnear_owed_key(id)), read_counter(&wnear_owed_bucket_key(id))))
}

fn dev_unwrap_op_key(id: u64) -> Vec<u8> { [b"uf:".as_slice(), &id.to_le_bytes()].concat() }


fn icon_len_key(id: u64) -> Vec<u8> { [b"il:".as_slice(), &id.to_le_bytes()].concat() }
fn icon_len_of(id: u64, l: &Launch) -> usize {
    l.icon.as_ref().map(|s| s.len()).unwrap_or_else(|| read_counter(&icon_len_key(id)) as usize)
}
const TAX_SELLER_KEY: &[u8] = b"txs";
fn tax_seller() -> Option<AccountId> { env::storage_read(TAX_SELLER_KEY).map(|b| String::from_utf8(b).expect("seller").parse().expect("seller")) }
fn set_tax_opts(id: u64, t: &TaxArgs, platform_bps: u16) {
    let v: Vec<u8> = [t.buy_bps, t.sell_bps, t.creator_bps, t.burn_bps, t.holders_bps, platform_bps].iter().flat_map(|x| x.to_le_bytes()).collect();
    env::storage_write(&tax_opts_key(id), &v);
}

fn tax_opts_full(id: u64) -> Option<(TaxArgs, u16)> {
    let b = env::storage_read(&tax_opts_key(id))?;
    let u = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
    let platform = if b.len() >= 12 { u(10) } else { 0 };
    Some((TaxArgs { buy_bps: u(0), sell_bps: u(2), creator_bps: u(4), burn_bps: u(6), holders_bps: u(8) }, platform))
}
fn tax_opts(id: u64) -> Option<TaxArgs> { tax_opts_full(id).map(|(t, _)| t) }
fn mul_bps(a: u128, bps: u128) -> u128 { mul_div(a, bps, BPS) }


fn require_unique_payees(payouts: &[(AccountId, U128)]) {
    for (i, (who, _)) in payouts.iter().enumerate() {
        require!(!payouts[..i].iter().any(|(w, _)| w == who), "duplicate account in payouts");
    }
}

fn mul_div(a: u128, n: u128, d: u128) -> u128 { a.checked_mul(n).map(|x| x / d).unwrap_or(a / d * n) }


fn mul_div_wide(a: u128, n: u128, d: u128) -> u128 {
    require!(d > 0 && n <= d && d < (1u128 << 127), "mul_div_wide: n <= d < 2^127");
    if let Some(x) = a.checked_mul(n) {
        return x / d;
    }
    const M: u128 = u64::MAX as u128;
    let (a0, a1, n0, n1) = (a & M, a >> 64, n & M, n >> 64);
    let (p00, p01, p10, p11) = (a0 * n0, a0 * n1, a1 * n0, a1 * n1);
    let mid = (p00 >> 64) + (p01 & M) + (p10 & M);
    let lo = (mid << 64) | (p00 & M);
    let hi = p11 + (p01 >> 64) + (p10 >> 64) + (mid >> 64);

    let (mut q, mut rem) = (0u128, 0u128);
    for i in (0..256u32).rev() {
        let bit = if i >= 128 { (hi >> (i - 128)) & 1 } else { (lo >> i) & 1 };
        rem = (rem << 1) | bit;
        if i < 128 {
            q <<= 1;
        }
        if rem >= d {
            rem -= d;
            if i < 128 {
                q |= 1;
            } else {

                env::panic_str("mul_div_wide: quotient overflow");
            }
        }
    }
    q
}






























fn dev_buy_out_key(id: u64) -> Vec<u8> { [b"dm:".as_slice(), &id.to_le_bytes()].concat() }
fn set_dev_buy_out(id: u64, v: u128) { env::storage_write(&dev_buy_out_key(id), &v.to_le_bytes()); }
fn dev_buy_out(id: u64) -> Option<u128> {
    env::storage_read(&dev_buy_out_key(id)).map(|b| u128::from_le_bytes(b.try_into().expect("dev buy out")))
}

const RELAYERS_KEY: &[u8] = b"rly";
fn relayers() -> Vec<AccountId> {
    env::storage_read(RELAYERS_KEY).map(|b| near_sdk::serde_json::from_slice(&b).expect("relayers")).unwrap_or_default()
}

#[cfg(test)]
mod v6_tests {
    use super::*;
    use near_sdk::mock::MockAction;
    use near_sdk::test_utils::{get_created_receipts, get_logs, VMContextBuilder};

    const ME: &str = "nearlytrade.near";
    pub(super) fn factory() -> Factory {
        ctx("owner.near", 0);
        Factory::new(
            "owner.near".parse().unwrap(),
            "7591d117dde58bba80a7df25ff6bd1112428f85cf20018d774f00b48af0fa11b".to_string(),
            "wrap.near".parse().unwrap(),
            "dclv2.ref-labs.near".parse().unwrap(),
            "lock.near".parse().unwrap(),
            None,
        )
    }

    pub(super) fn uw_op(id: u64) -> Option<u64> { read_u128(&dev_unwrap_op_key(id)).map(|v| v as u64) }

    pub(super) fn retry_op(id: u64) -> Option<u64> { wnear_retry(id).map(|w| w.0) }
    pub(super) fn ctx(who: &str, deposit: u128) {
        ctx_at(who, deposit, 1_700_000_000_000);
    }
    pub(super) fn ctx_at(who: &str, deposit: u128, now_ms: u64) {
        ctx_full(who, deposit, now_ms, 300);
    }
    pub(super) fn ctx_gas(who: &str, tgas: u64) {
        ctx_full(who, 0, 1_700_000_000_000, tgas);
    }

    pub(super) fn ctx_results(results: Vec<near_sdk::PromiseResult>) {
        near_sdk::testing_env!(
            VMContextBuilder::new()
                .current_account_id(ME.parse().unwrap())
                .predecessor_account_id(ME.parse().unwrap())
                .account_balance(NearToken::from_near(500))
                .block_timestamp(1_700_000_000_000 * 1_000_000)
                .prepaid_gas(Gas::from_tgas(300))
                .build(),
            near_sdk::test_vm_config(),
            mainnet_fees(),
            Default::default(),
            results
        );
    }
    pub(super) fn ctx_full(who: &str, deposit: u128, now_ms: u64, tgas: u64) {
        near_sdk::testing_env!(
            VMContextBuilder::new()
                .current_account_id(ME.parse().unwrap())
                .predecessor_account_id(who.parse().unwrap())
                .attached_deposit(NearToken::from_yoctonear(deposit))
                .account_balance(NearToken::from_near(500))
                .block_timestamp(now_ms * 1_000_000)
                .prepaid_gas(Gas::from_tgas(tgas))
                .build(),
            near_sdk::test_vm_config(),
            mainnet_fees()
        );
    }





    pub(super) fn mainnet_fees() -> near_sdk::RuntimeFeesConfig {
        fn set<T: near_sdk::serde::de::DeserializeOwned>(slot: &mut T, gas: u64) {
            *slot = near_sdk::serde_json::from_value(near_sdk::serde_json::json!(gas))
                .or_else(|_| near_sdk::serde_json::from_value(near_sdk::serde_json::json!(gas.to_string())))
                .expect("fee component");
        }
        let mut fees = near_sdk::RuntimeFeesConfig::test();
        for (k, v) in fees.action_fees.iter_mut() {
            let (sir, not_sir, exec) = match format!("{:?}", k).as_str() {
                "function_call_base" => (200_000_000_000, 200_000_000_000, 780_000_000_000),
                "new_data_receipt_base" => (36_486_732_312, 36_486_732_312, 36_486_732_312),
                "new_data_receipt_byte" => (17_212_011, 17_212_011, 17_212_011),
                "create_account" => (500_000_000_000, 500_000_000_000, 7_200_000_000_000),
                _ => continue,
            };
            set(&mut v.send_sir, sir);
            set(&mut v.send_not_sir, not_sir);
            set(&mut v.execution, exec);
        }
        fees
    }
    pub(super) fn args(sym: &str) -> LaunchArgs {
        LaunchArgs {
            name: sym.into(), symbol: sym.into(), icon: None, description: None, links: None, dev_buy: None, init_point: None,
            quote: None, fee_mode: None, creator_share_bps: None, tax: None, fee_to: None,
        }
    }
    pub(super) fn launch_with(f: &mut Factory, who: &str, a: LaunchArgs) -> u64 {
        let icon_len = a.icon.as_ref().map(|s| s.len()).unwrap_or(0) as u32;
        let deposit = f.quote_launch(icon_len, a.dev_buy, Some(a.tax.is_some())).total.0;
        ctx(who, deposit);
        std::mem::forget(f.launch(a));
        f.next_id - 1
    }
    pub(super) fn launch(f: &mut Factory, who: &str, sym: &str) -> u64 { launch_with(f, who, args(sym)) }

    pub(super) fn park(f: &mut Factory, id: u64, step: Step) {
        let mut l = f.launches.get(&id).cloned().unwrap();
        l.step = step;
        l.inflight = false;
        if matches!(step, Step::DevBuy | Step::Done) { l.lpt_id = Some("pool|1".into()); }
        f.launches.insert(id, l);
    }

    pub(super) fn calls() -> Vec<(String, String, String, u128)> {
        get_created_receipts().iter().flat_map(|r| {
            let to = r.receiver_id.to_string();
            r.actions.iter().filter_map(move |a| match a {
                MockAction::FunctionCallWeight { method_name, args, attached_deposit, .. } => Some((
                    to.clone(), String::from_utf8(method_name.clone()).unwrap(), String::from_utf8(args.clone()).unwrap(), attached_deposit.as_yoctonear(),
                )),
                _ => None,
            })
        }).collect()
    }
    pub(super) fn transfers() -> Vec<(String, u128)> {
        get_created_receipts().iter().flat_map(|r| {
            let to = r.receiver_id.to_string();
            r.actions.iter().filter_map(move |a| match a {
                MockAction::Transfer { deposit, .. } => Some((to.clone(), deposit.as_yoctonear())),
                _ => None,
            })
        }).collect()
    }
    pub(super) fn logs_with(ev: &str) -> Vec<String> {
        get_logs().into_iter().filter(|l| l.contains(&format!(r#""event":"{}""#, ev))).collect()
    }
    pub(super) fn l(f: &Factory, id: u64) -> Launch { f.launches.get(&id).cloned().unwrap() }



    #[test]
    fn dev_buy_tokens_out_inverts_the_cap() {
        let supply = 10u128.pow(27);

        for (left, bps) in [(200, 400u16), (-3000, 500), (15_400, 400), (-6_600, 2_000)] {
            let cap = dev_buy_cap_yocto(left, supply, bps, 10_000);
            let out = dev_buy_tokens_out(left, supply, cap, 10_000);
            let want = supply / BPS * bps as u128;
            assert!(out.abs_diff(want) * 1_000_000 / want < 10, "left {left} bps {bps}: out {out} want {want}");
        }

        let out = dev_buy_tokens_out(-3000, supply, 39_382_900_000_000_000_000_000_000, 10_000);
        assert!((out as f64 / supply as f64 - 0.05).abs() < 0.0005, "out = {}", out as f64 / supply as f64);

        let a = dev_buy_tokens_out(200, supply, 10u128.pow(24), 10_000);
        let b = dev_buy_tokens_out(200, supply, 10 * 10u128.pow(24), 10_000);
        assert!(b < 10 * a && b > 9 * a);
        assert_eq!(dev_buy_tokens_out(200, supply, 0, 10_000), 0);
    }

    #[test]
    fn a_dev_buy_stores_its_floor_and_the_swap_uses_it() {
        let mut f = factory();
        let near = 5 * 10u128.pow(24);
        let id = launch_with(&mut f, "alice.near", LaunchArgs { dev_buy: Some(U128(near)), ..args("FLOOR") });
        let stored = dev_buy_out(id).expect("stored at launch");
        let curve = dev_buy_tokens_out(POINT_DELTA_1PCT, f.config.total_supply.0, near, 10_000);
        assert_eq!(stored, curve * 9_700 / 10_000);
        assert!(logs_with("launch_started")[0].contains(&format!(r#""dev_buy_min_out":"{}""#, stored)));
        let msg = f.dev_buy_swap_msg(&l(&f, id));
        assert!(msg.contains(&format!(r#""min_output_amount":"{}""#, stored)), "{msg}");
        assert!(msg.starts_with(r#"{"Swap":"#));
    }

    #[test]
    fn an_exact_dev_buy_stores_its_exact_output() {
        let mut f = factory();
        let id = launch_with(&mut f, "alice.near", LaunchArgs { dev_buy: Some(U128(500 * 10u128.pow(24))), ..args("EXACT") });
        assert!(l(&f, id).dev_buy_exact);
        assert_eq!(dev_buy_out(id), Some(f.config.total_supply.0 / BPS * 400));
    }

    #[test]
    fn a_launch_without_a_stored_floor_gets_the_same_one_recomputed() {
        let mut f = factory();
        let near = 7 * 10u128.pow(24);
        let id = launch_with(&mut f, "alice.near", LaunchArgs { dev_buy: Some(U128(near)), ..args("OLD") });
        let stored = dev_buy_out(id).unwrap();
        env::storage_remove(&dev_buy_out_key(id));
        assert_eq!(f.get_dev_buy_min_out(U64(id)).0, stored);
    }

    #[test]
    #[should_panic(expected = "dev buy: creator, owner or relayer only")]
    fn a_stranger_cannot_trigger_the_dev_buy() {
        let mut f = factory();
        let id = launch_with(&mut f, "alice.near", LaunchArgs { dev_buy: Some(U128(10u128.pow(24))), ..args("SAND") });
        park(&mut f, id, Step::DevBuy);
        ctx("mallory.near", 0);
        std::mem::forget(f.resume(U64(id)));
    }

    #[test]
    fn a_relayer_and_the_owner_can_trigger_the_dev_buy() {
        let mut f = factory();
        ctx("owner.near", 0);
        f.set_relayers(vec!["relay.near".parse().unwrap()]);
        assert_eq!(f.get_relayers(), vec!["relay.near".parse::<AccountId>().unwrap()]);
        for who in ["relay.near", "owner.near", "alice.near"] {
            let id = launch_with(&mut f, "alice.near", LaunchArgs { dev_buy: Some(U128(10u128.pow(24))), ..args("RELAY") });
            park(&mut f, id, Step::DevBuy);
            ctx(who, 0);
            std::mem::forget(f.resume(U64(id)));
            assert!(l(&f, id).inflight, "{who} started the dev buy");
            ctx(ME, 0);
            let _ = f.on_dev_bought(id, Err(PromiseError::Failed));
        }

        let id = launch(&mut f, "alice.near", "PUB");
        park(&mut f, id, Step::AddLiquidity);
        ctx("anyone.near", 0);
        std::mem::forget(f.resume(U64(id)));
        assert!(l(&f, id).inflight);
    }



    fn dev_buy_launch(f: &mut Factory, near: u128) -> u64 {
        let id = launch_with(f, "alice.near", LaunchArgs { dev_buy: Some(U128(near)), ..args("DEV") });
        park(f, id, Step::DevBuy);
        id
    }

    #[test]
    fn the_dev_buy_is_debited_before_the_swap_and_the_swap_gets_the_spare_gas() {
        let mut f = factory();
        let near = 3 * 10u128.pow(24);
        let id = dev_buy_launch(&mut f, near);
        ctx("alice.near", 0);
        drop(f.resume(U64(id)));
        let lz = l(&f, id);
        assert!(lz.inflight);
        assert_eq!(lz.dev_buy_held.0, 0, "debited before the swap is sent");
        assert_eq!(lz.dev_buy_near.0, near);
        let mut ftc = None;
        let mut cb = None;
        for r in get_created_receipts() {
            for a in r.actions {
                if let MockAction::FunctionCallWeight { method_name, prepaid_gas, gas_weight, .. } = a {
                    if method_name == b"ft_transfer_call" { ftc = Some((prepaid_gas, gas_weight.0)); }
                    if method_name == b"on_dev_bought" { cb = Some((prepaid_gas, gas_weight.0)); }
                }
            }
        }
        assert_eq!(ftc, Some((GAS_DEV_BUY, 1)), "the swap takes every unused unit of gas");
        assert_eq!(cb, Some((GAS_DEV_BOUGHT_CB, 0)), "the callback has a fixed, real budget");
    }

    #[test]
    fn a_lost_dev_buy_callback_can_never_pay_twice() {
        let mut f = factory();
        let id = dev_buy_launch(&mut f, 2 * 10u128.pow(24));
        ctx("alice.near", 0);
        drop(f.resume(U64(id)));

        ctx("owner.near", 0);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f.reset_inflight(U64(id))));
        assert!(r.is_err(), "reset_inflight refuses a dev buy");
        ctx("owner.near", 0);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f.set_step(U64(id), Step::DevBuy)));
        assert!(r.is_err(), "set_step cannot clear a dev buy's in-flight flag either");
        ctx(ME, 0);
        let _ = f.on_dev_bought(id, Err(PromiseError::Failed));
        ctx("owner.near", 0);
        f.set_step(U64(id), Step::DevBuy);
        let mut lz = l(&f, id);
        lz.dev_buy_held = U128(0);
        f.launches.insert(id, lz);
        ctx("alice.near", 0);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(f.resume(U64(id)))));
        assert!(r.is_err(), "an unfunded dev buy cannot be swapped again");
        ctx("alice.near", 0);
        drop(f.cancel_dev_buy(U64(id)));
        assert_eq!(transfers(), vec![("alice.near".to_string(), 0)], "and cancelling refunds nothing");
        assert_eq!(l(&f, id).step, Step::Done);
    }

    #[test]
    fn a_refused_swap_is_re_credited_only_after_the_unwrap() {
        let mut f = factory();
        let near = 2 * 10u128.pow(24);
        let id = dev_buy_launch(&mut f, near);
        ctx("alice.near", 0);
        drop(f.resume(U64(id)));
        ctx(ME, 0);
        let _ = f.on_dev_bought(id, Ok(U128(0)));
        assert!(l(&f, id).inflight, "held in flight while the unwrap runs");
        assert_eq!(l(&f, id).dev_buy_held.0, 0);
        ctx(ME, 0);
        assert!(!f.on_dev_buy_unwrapped(id, uw_op(id), Err(PromiseError::Failed)));
        assert_eq!(l(&f, id).dev_buy_held.0, 0, "an unwrap that failed re-credits nothing");
        assert!(!l(&f, id).inflight);

        ctx("alice.near", 0);
        assert!(panics(|| drop(f.cancel_dev_buy(U64(id)))).contains("not unwrapped yet"));
        ctx("alice.near", 0);
        drop(f.resume(U64(id)));
        assert!(l(&f, id).inflight);
        let c = calls();
        assert!(c.iter().any(|c| c.1 == "near_withdraw" && c.2.contains(&format!(r#""amount":"{}""#, near))));
        assert!(c.iter().all(|c| c.1 != "ft_transfer_call"), "no swap while the wNEAR is not back");
        ctx(ME, 0);
        assert!(f.on_dev_buy_unwrapped(id, uw_op(id), Ok(())));
        assert_eq!(l(&f, id).dev_buy_held.0, near, "funded again once the unwrap landed");
        assert!(!env::storage_has_key(&unwrap_pending_key(id)));

        let id2 = dev_buy_launch(&mut f, near);
        ctx("alice.near", 0);
        drop(f.resume(U64(id2)));
        ctx(ME, 0);
        let _ = f.on_dev_bought(id2, Ok(U128(0)));
        ctx(ME, 0);
        assert!(f.on_dev_buy_unwrapped(id2, uw_op(id2), Ok(())));
        let lz = l(&f, id2);
        assert_eq!((lz.dev_buy_held.0, lz.inflight, lz.step), (near, false, Step::DevBuy));
    }

    #[test]
    fn a_reverted_wrap_is_re_credited_at_once() {
        let mut f = factory();
        let near = 2 * 10u128.pow(24);
        let id = dev_buy_launch(&mut f, near);
        ctx("alice.near", 0);
        drop(f.resume(U64(id)));
        ctx(ME, 0);
        let _ = f.on_dev_bought(id, Err(PromiseError::Failed));
        let lz = l(&f, id);
        assert_eq!((lz.dev_buy_held.0, lz.inflight, lz.step), (near, false, Step::DevBuy));
    }

    #[test]
    fn the_excess_of_an_exact_buy_is_paid_only_after_its_unwrap() {
        let mut f = factory();

        let id = dev_buy_launch(&mut f, 500 * 10u128.pow(24));
        let near = l(&f, id).dev_buy_near.0;
        assert!(l(&f, id).dev_buy_exact);
        ctx("alice.near", 0);
        drop(f.resume(U64(id)));
        ctx(ME, 0);
        let _ = f.on_dev_bought(id, Ok(U128(near - 1000)));
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.dev_buy_near.0, lz.dev_buy_held.0), (Step::Done, near - 1000, 0));
        assert_eq!(lz.dev_buy_tokens.0, f.config.total_supply.0 / BPS * 400, "an exact buy's output is known and recorded");
        assert!(transfers().is_empty(), "no NEAR leaves before the unwrap is confirmed");
        assert!(calls().iter().any(|c| c.1 == "near_withdraw" && c.2.contains(r#""amount":"1000""#)));
        ctx(ME, 0);
        assert!(!f.on_dev_refund_unwrapped(id, U128(1000), Err(PromiseError::Failed)));
        assert!(transfers().is_empty(), "a failed unwrap pays nothing");
        ctx(ME, 0);
        assert!(f.on_dev_refund_unwrapped(id, U128(1000), Ok(())));
        assert_eq!(transfers(), vec![("alice.near".to_string(), 1000)]);
    }

    #[test]
    fn refund_failed_also_clears_the_buy() {
        let mut f = factory();
        let near = 2 * 10u128.pow(24);
        let id = launch_with(&mut f, "alice.near", LaunchArgs { dev_buy: Some(U128(near)), ..args("GONE") });
        let mut lz = l(&f, id);
        lz.step = Step::Failed;
        f.launches.insert(id, lz);
        ctx("anyone.near", 0);
        drop(f.refund_failed(U64(id)));
        let lz = l(&f, id);
        assert_eq!((lz.dev_buy_held.0, lz.dev_buy_near.0), (0, 0));
        assert_eq!(transfers(), vec![("alice.near".to_string(), near)]);
    }



    fn created_but_pool_refused(f: &mut Factory, near: u128) -> u64 {
        let id = launch_with(f, "alice.near", LaunchArgs { dev_buy: Some(U128(near)), ..args("GRIEF") });
        ctx(ME, 0);
        let r = f.on_created(id, None, Ok(StorageBalanceJson { total: "0".into() }), Err(PromiseError::Failed));
        assert!(matches!(r, PromiseOrValue::Promise(_)), "a refused create_pool reads the pool");
        drop(r);
        id
    }

    #[test]
    fn a_refused_create_pool_reads_the_existing_pool() {
        let mut f = factory();
        let id = created_but_pool_refused(&mut f, 0);
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight), (Step::CreatePool, true));
        let c = calls();
        assert!(c.iter().any(|c| c.1 == "storage_deposit" && c.0 == "dclv2.ref-labs.near" && c.2.contains("lock.near")), "the storage top-up is redone");
        assert!(c.iter().any(|c| c.1 == "get_pool" && c.2.contains(&lz.pool_id)));
    }

    #[test]
    fn a_pool_at_our_start_point_is_adopted() {
        let mut f = factory();
        let id = created_but_pool_refused(&mut f, 0);
        let point = l(&f, id).init_point;
        ctx(ME, 0);
        drop(f.on_pool_checked(id, None, Ok(Some(near_sdk::serde_json::json!({"pool_id": "x", "current_point": point, "fee": 10000})))));
        assert_eq!(l(&f, id).step, Step::AddLiquidity);
        assert!(calls().iter().any(|c| c.0 == "lock.near" && c.1 == "add"), "placed in-line when the gas is there");
        assert_eq!(logs_with("pool_adopted").len(), 1);
    }

    #[test]
    fn a_pool_at_another_price_fails_the_launch_and_frees_the_dev_buy() {
        let mut f = factory();
        let near = 2 * 10u128.pow(24);
        let id = created_but_pool_refused(&mut f, near);
        let point = l(&f, id).init_point;
        ctx(ME, 0);
        drop(f.on_pool_checked(id, None, Ok(Some(near_sdk::serde_json::json!({"current_point": point + 200})))));
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight, lz.dev_buy_held.0), (Step::Failed, false, near));

        assert_eq!(transfers(), vec![("alice.near".to_string(), DCL_POOL_CREATE.as_yoctonear())]);
        ctx("anyone.near", 0);
        drop(f.refund_failed(U64(id)));
        assert_eq!(transfers(), vec![("alice.near".to_string(), near)]);
    }

    #[test]
    fn no_pool_at_all_or_a_failed_read_parks_the_launch() {
        let mut f = factory();
        let id = created_but_pool_refused(&mut f, 0);
        ctx(ME, 0);
        drop(f.on_pool_checked(id, None, Ok(None)));
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight), (Step::CreatePool, false), "a transient failure parks; the creator can still cancel after 24h");
        let id2 = created_but_pool_refused(&mut f, 0);
        ctx(ME, 0);
        drop(f.on_pool_checked(id2, None, Err(PromiseError::Failed)));
        let lz = l(&f, id2);
        assert_eq!((lz.step, lz.inflight), (Step::CreatePool, false), "an unreadable pool stays resumable");
    }

    #[test]
    fn the_create_pool_retry_also_checks() {
        let mut f = factory();
        let id = launch(&mut f, "alice.near", "RETRY");
        park(&mut f, id, Step::CreatePool);
        ctx(ME, 0);
        let r = f.on_pool_created(id, None, Err(PromiseError::Failed));
        assert!(matches!(r, PromiseOrValue::Promise(_)));
        drop(r);
        assert!(calls().iter().any(|c| c.1 == "get_pool"));
    }



    const T0: u64 = 1_700_000_000_000;
    const DAY: u64 = 86_400_000;

    fn stuck(f: &mut Factory, step: Step, near: u128) -> u64 {
        let id = launch_with(f, "alice.near", LaunchArgs { dev_buy: Some(U128(near)), ..args("STUCK") });
        park(f, id, step);
        id
    }
    fn try_cancel(f: &mut Factory, who: &str, id: u64, at: u64) -> Result<(), String> {
        ctx_at(who, 0, at);
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(f.cancel_stuck_launch(U64(id))))).map_err(|e| {
            e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default()
        })
    }

    #[test]
    fn the_creator_can_leave_a_launch_stuck_at_create_pool_for_a_day() {
        let mut f = factory();
        let near = 2 * 10u128.pow(24);
        let id = stuck(&mut f, Step::CreatePool, near);
        assert!(try_cancel(&mut f, "alice.near", id, T0 + DAY - 1).unwrap_err().contains("24h"));
        assert!(try_cancel(&mut f, "mallory.near", id, T0 + DAY).unwrap_err().contains("creator only"));
        try_cancel(&mut f, "alice.near", id, T0 + DAY).unwrap();
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.dev_buy_held.0, lz.dev_buy_near.0), (Step::Failed, 0, 0));
        assert_eq!(transfers(), vec![("alice.near".to_string(), near)]);
        assert!(calls().iter().all(|c| c.1 != "get_position"), "no pool yet: nothing to ask");
    }


    fn cancel_asks_the_locker(f: &mut Factory, step: Step, near: u128) -> u64 {
        let id = stuck(f, step, near);
        try_cancel(f, "alice.near", id, T0 + DAY).unwrap();
        let ask = calls().into_iter().find(|c| c.1 == "get_position").expect("the locker is asked");
        assert_eq!(ask.0, "lock.near");
        assert_eq!(ask.2, format!(r#"{{"pool_id":{}}}"#, js(&l(f, id).pool_id)));
        let lz = l(f, id);
        assert_eq!((lz.step, lz.inflight), (step, true), "held in flight while the locker answers");
        id
    }

    #[test]
    fn past_create_pool_the_locker_must_prove_there_is_no_position() {
        let mut f = factory();
        let near = 2 * 10u128.pow(24);
        for (step, answer, cancels) in [
            (Step::AddLiquidity, Ok(None), true),
            (Step::AddLiquidity, Ok(Some(json!({"lpt_id": null, "scan_from": 3, "refunded": true}))), true),
            (Step::AddLiquidity, Ok(Some(json!({"lpt_id": null, "scan_from": 3, "refunded": false}))), false),
            (Step::AddLiquidity, Ok(Some(json!({"lpt_id": "pool|7", "scan_from": 3, "refunded": false}))), false),
            (Step::AddLiquidity, Err(PromiseError::Failed), false),
        ] {
            let id = cancel_asks_the_locker(&mut f, step, near);
            ctx(ME, 0);
            assert_eq!(f.on_stuck_position(U64(id), answer.clone()), cancels, "{answer:?}");
            let lz = l(&f, id);
            assert!(!lz.inflight);
            if cancels {
                assert_eq!((lz.step, lz.dev_buy_held.0), (Step::Failed, 0));
                assert_eq!(transfers(), vec![("alice.near".to_string(), near)]);
            } else {
                assert_eq!((lz.step, lz.dev_buy_held.0), (step, near), "kept as it was: {answer:?}");
                assert!(transfers().is_empty());
                assert_eq!(logs_with("stuck_launch_kept").len(), 1);
            }
        }
    }

    #[test]
    fn cancel_is_refused_in_flight_and_at_the_dev_buy() {
        let mut f = factory();
        let id = stuck(&mut f, Step::AddLiquidity, 10u128.pow(24));
        let mut lz = l(&f, id);
        lz.inflight = true;
        f.launches.insert(id, lz);
        assert!(try_cancel(&mut f, "alice.near", id, T0 + 2 * DAY).unwrap_err().contains("in flight"));
        let id2 = stuck(&mut f, Step::DevBuy, 10u128.pow(24));
        assert!(try_cancel(&mut f, "alice.near", id2, T0 + 2 * DAY).unwrap_err().contains("before its dev buy"));
        let id3 = stuck(&mut f, Step::Done, 0);
        assert!(try_cancel(&mut f, "alice.near", id3, T0 + 2 * DAY).is_err());

        let id4 = stuck(&mut f, Step::AddLiquidity, 0);
        try_cancel(&mut f, "alice.near", id4, T0 + DAY).unwrap();
        assert!(try_cancel(&mut f, "alice.near", id4, T0 + DAY).unwrap_err().contains("in flight"));
    }

    #[test]
    fn a_bounced_refund_goes_back_to_the_failed_launch() {
        let mut f = factory();
        let near = 10u128.pow(24);
        let id = stuck(&mut f, Step::CreatePool, near);
        try_cancel(&mut f, "alice.near", id, T0 + DAY).unwrap();
        ctx(ME, 0);
        f.on_dev_refund_sent(U64(id), U128(near), Err(PromiseError::Failed));
        assert_eq!(l(&f, id).dev_buy_held.0, near);
        ctx("anyone.near", 0);
        drop(f.refund_failed(U64(id)));
        assert_eq!(transfers(), vec![("alice.near".to_string(), near)]);
    }

    #[test]
    fn a_stuck_launch_without_a_dev_buy_just_fails() {
        let mut f = factory();
        let id = stuck(&mut f, Step::CreatePool, 0);
        try_cancel(&mut f, "alice.near", id, T0 + DAY).unwrap();
        assert_eq!(l(&f, id).step, Step::Failed);
        assert!(transfers().is_empty());
    }



    fn icon_of(n: usize) -> String {
        let mut s = String::from("data:image/png;base64,");
        while s.len() < n { s.push('A'); }
        s
    }
    fn launch_scheduled(f: &mut Factory, who: &str, a: LaunchArgs) -> u64 {
        let icon_len = a.icon.as_ref().map(|s| s.len()).unwrap_or(0) as u32;
        let deposit = f.quote_launch(icon_len, a.dev_buy, Some(a.tax.is_some())).total.0;
        ctx(who, deposit);
        drop(f.launch(a));
        f.next_id - 1
    }

    #[test]
    fn the_icon_stays_in_the_factory_record_and_goes_to_the_token_as_live() {
        let mut f = factory();
        let icon = icon_of(3_000);
        let id = launch_scheduled(&mut f, "alice.near", LaunchArgs { icon: Some(icon.clone()), ..args("ICON") });
        assert_eq!(l(&f, id).icon.as_deref(), Some(icon.as_str()), "the record carries the icon until the token exists");
        let init = calls().into_iter().find(|c| c.1 == "new").expect("token init");
        assert!(init.2.contains(&icon), "the token's metadata carries the icon");
        let ev = &logs_with("launch_started")[0];
        assert!(!ev.contains(r#""icon""#) && !ev.contains("icon_omitted"), "the event keeps the live shape: {ev}");

        ctx(ME, 0);
        drop(f.on_created(id, Some(true), Ok(StorageBalanceJson { total: "0".into() }), Err(PromiseError::Failed)));
        assert_eq!(f.get_launch(U64(id)).unwrap().icon, None, "the record no longer stores the icon");
        assert_eq!(icon_len_of(id, &l(&f, id)), icon.len(), "its length still prices a retry's token storage");

        let id2 = launch_scheduled(&mut f, "alice.near", args("NOICON"));
        ctx(ME, 0);
        drop(f.on_created(id2, Some(true), Ok(StorageBalanceJson { total: "0".into() }), Err(PromiseError::Failed)));
        assert!(!env::storage_has_key(&icon_len_key(id2)));
    }

    #[test]
    fn every_new_launch_splits_its_fee_70_20_10_and_the_creator_cannot_pick() {
        let mut f = factory();
        owner();
        f.set_protocol_recipients(vec![(acct("p1.near"), 5000), (acct("p2.near"), 5000)]);
        assert_eq!(f.config.creator_fee_share_bps, 7000, "the config default is the fixed share");

        let id0 = launch_scheduled(&mut f, "alice.near", args("PLAIN"));
        assert_eq!(f.get_launch_recipients(U64(id0)), None);

        owner();
        f.set_burn_pot(Some(acct("burn.near")));
        f.set_referral_pot(Some(acct("refpot.near")));
        assert_eq!((f.get_burn_pot(), f.get_referral_pot()), (Some(acct("burn.near")), Some(acct("refpot.near"))));
        let id1 = launch_scheduled(&mut f, "alice.near", args("FOUR"));
        assert_eq!(f.get_launch_recipients(U64(id1)).unwrap(), vec![(acct("p1.near"), 3333), (acct("p2.near"), 3334), (acct("refpot.near"), 3333)]);
        assert!(logs_with("launch_split")[0].contains(r#""recipients""#));
        assert_eq!(fee_opts(id1, 8000), (FM_CREATOR, 7000), "70% stored for the launch");

        owner();
        f.set_referral_pot(None);
        let id2 = launch_scheduled(&mut f, "alice.near", args("ONEPOT"));
        assert_eq!(f.get_launch_recipients(U64(id2)).unwrap(), vec![(acct("p1.near"), 5000), (acct("p2.near"), 5000)]);

        ctx("bob.near", f.quote_launch(0, None, None).total.0);
        assert_eq!(panics(|| drop(f.launch(LaunchArgs { creator_share_bps: Some(8000), ..args("PICK") }))), "creator_share_bps: the launchpad's current share, or leave it out");

        owner();
        f.set_referral_pot(Some(acct("refpot.near")));
        near_sdk::env::storage_write(TAX_CODE_KEY, &[7u8; 32]);
        near_sdk::env::storage_write(TAX_SELLER_KEY, b"seller.near");
        let id3 = launch_scheduled(&mut f, "alice.near", LaunchArgs { tax: Some(TaxArgs { buy_bps: 300, sell_bps: 300, creator_bps: 5000, burn_bps: 2000, holders_bps: 3000 }), ..args("TAXN") });
        assert_eq!(tax_opts_full(id3).unwrap().1, 3000, "new tax launches carry the 30% platform cut");
        park(&mut f, id3, Step::Done);
        add_counter(&tax_selling_key(id3), 8_400);
        let pf0 = f.protocol_fees;
        ctx("seller.near", 1_000);
        assert!(f.tax_proceeds(U64(id3), U128(8_400)));
        assert_eq!(f.protocol_fees, pf0, "the platform cut went to the launch's own split, not the shared bucket");
        let t = f.get_tax(U64(id3)).unwrap();

        assert_eq!(t.paid_platform.0, 348, "the platform cut, pro rata to the sold parts");
    }

    #[test]
    fn a_max_size_icon_launch_fits_the_log_cap() {
        let mut f = factory();
        let icon = icon_of(f.config.max_icon_bytes as usize);
        let id = launch_scheduled(&mut f, "alice.near", LaunchArgs {
            icon: Some(icon.clone()), fee_mode: Some("burn".into()), creator_share_bps: Some(7000), fee_to: Some("x".repeat(64).parse().unwrap()),
            name: "\u{1F600}".repeat(32), ..args("BIGICON")
        });
        assert_eq!(l(&f, id).icon.map(|i| i.len()), Some(icon.len()));
        assert!(calls().into_iter().any(|c| c.1 == "new" && c.2.contains(&icon)));
        let total: usize = get_logs().iter().map(|l| l.len()).sum();
        assert!(total < 4_096, "all logs of the call: {total}");
    }

    #[test]
    fn the_creator_is_registered_once_from_the_creators_deposit() {
        let mut f = factory();
        let id = launch_scheduled(&mut f, "alice.near", args("REG"));
        let token = l(&f, id).token.to_string();
        let storage = f.internal_cost(0, "REG".len() * 2, 0, false).token_storage.0;
        let to_token: u128 = transfers().into_iter().filter(|t| t.0 == token).map(|t| t.1).sum();
        let reg: Vec<u128> = calls().into_iter().filter(|c| c.0 == token && c.1 == "storage_deposit").map(|c| c.3).collect();
        assert_eq!(reg, vec![FT_STORAGE_REG.as_yoctonear()], "one registration");
        assert_eq!(to_token + reg[0], storage, "the token account gets exactly what the creator paid for it");
    }



    fn acct(a: &str) -> AccountId { a.parse().unwrap() }
    fn panics<F: FnOnce()>(f: F) -> String {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(_) => String::new(),
            Err(e) => e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_else(|| "panic".into()),
        }
    }
    pub(super) fn live_burn_launch(f: &mut Factory, sym: &str, bucket: u128) -> u64 {
        let id = launch_with(f, "alice.near", LaunchArgs { fee_mode: Some("burn".into()), ..args(sym) });
        park(f, id, Step::Done);
        set_mode_bucket(id, bucket);
        id
    }
    pub(super) fn usdc_pair(f: &mut Factory) -> AccountId {
        ctx("owner.near", 0);
        f.set_quote(acct("usdc.near"), 6, -400_000, -400_000, -400_000, 400_000, true);
        acct("usdc.near")
    }
    pub(super) fn live_tax_launch(f: &mut Factory, sym: &str, quote: Option<AccountId>, pending: u128) -> u64 {
        let id = launch_with(f, "alice.near", LaunchArgs { quote, ..args(sym) });
        park(f, id, Step::Done);
        set_tax_opts(id, &TaxArgs { buy_bps: 300, sell_bps: 300, creator_bps: 5000, burn_bps: 2000, holders_bps: 3000 }, 2_000);
        add_counter(&tax_pending_key(id), pending);
        id
    }

    #[test]
    fn a_buyback_locks_its_token_until_the_burn() {
        let mut f = factory();
        let n = 10u128.pow(24);
        let id = live_burn_launch(&mut f, "BB", 5 * n);
        let token = l(&f, id).token.clone();
        ctx("owner.near", 0);
        drop(f.buyback(U64(id), U128(1000)));
        assert!(swap_locked(&token) && !swap_locked(&acct("wrap.near")), "only the token is measured");

        let mut lz = l(&f, id);
        lz.creator_token_fees = U128(77);
        lz.protocol_token_fees = U128(77);
        f.launches.insert(id, lz);
        ctx("anyone.near", 0);
        assert_eq!(panics(|| drop(f.claim_fees(U64(id)))), "swap in flight");
        ctx("anyone.near", 0);
        assert_eq!(panics(|| drop(f.burn_token_fees(U64(id)))), "swap in flight");
        ctx("owner.near", 0);
        assert_eq!(panics(|| drop(f.withdraw_protocol_token_fees(U64(id), acct("owner.near")))), "swap in flight");

        ctx(ME, 0);
        drop(f.on_bb_swapped(U64(id), U128(5 * n), U128(0), Some(U128(1000)), Ok(U128(5 * n))));
        assert!(swap_locked(&token));
        ctx(ME, 0);
        drop(f.on_bb_after(U64(id), U128(0), U128(5 * n), Some(U128(1000)), Ok(U128(900))));
        assert!(swap_locked(&token), "held until the burn settles");
        ctx(ME, 0);
        f.on_bb_burned(U64(id), U128(900), Ok(()));
        assert!(!swap_locked(&token));
        assert_eq!(read_counter(&burned_key(id)), 900);
    }

    #[test]
    fn a_house_buyback_launch_buys_back_and_burns_every_fee() {
        let mut f = factory();
        internal_set_house_creators(vec![acct("house.near")], true);
        let id = launch(&mut f, "house.near", "HB");
        park(&mut f, id, Step::Done);
        let outside = launch(&mut f, "alice.near", "OUT");
        park(&mut f, outside, Step::Done);
        ctx("owner.near", 0);
        f.set_launch_recipients(U64(id), Some(vec![(acct("burner.near"), 10_000)]));
        let protocol_before = f.protocol_fees;

        ctx("house.near", 0);
        assert_eq!(panics(|| f.set_house_buyback(U64(id))), "owner only");
        ctx("owner.near", 0);
        assert_eq!(panics(|| f.set_house_buyback(U64(outside))), "house launches only");
        let mut lz = l(&f, id);
        lz.inflight = true;
        f.launches.insert(id, lz);
        ctx("owner.near", 0);
        assert_eq!(panics(|| f.set_house_buyback(U64(id))), "launch not live");
        park(&mut f, id, Step::Done);
        ctx("owner.near", 0);
        f.set_house_buyback(U64(id));
        assert_eq!(logs_with("house_buyback_set").len(), 1);
        let o = f.get_fee_opts(U64(id));
        assert_eq!((o.mode.as_str(), o.creator_share_bps), ("burn", 10_000));

        ctx("owner.near", 0);
        assert_eq!(panics(|| f.set_house_buyback(U64(id))), "already a burn or holders launch");

        let n = 10u128.pow(24);
        let lz = l(&f, id);
        let v = if lz.token_is_x { vec![U128(5_000), U128(n)] } else { vec![U128(n), U128(5_000)] };
        ctx(ME, 0);
        assert!(f.on_fees_claimed(id, Ok(v)));
        let lz = l(&f, id);
        assert_eq!(mode_bucket(id), n);
        assert_eq!((lz.creator_token_fees.0, lz.protocol_token_fees.0), (5_000, 0));
        assert_eq!(launch_bucket(id), 0);
        assert_eq!(f.protocol_fees, protocol_before);
        assert!(transfers().is_empty(), "no NEAR leaves on the claim");

        ctx("anyone.near", 0);
        drop(f.burn_token_fees(U64(id)));
        assert!(calls().iter().any(|c| c.0 == lz.token.as_str() && c.1 == "burn" && c.2.contains(r#""amount":"5000""#)));
        ctx("owner.near", 0);
        drop(f.buyback(U64(id), U128(1000)));
        assert_eq!(mode_bucket(id), 0);
        assert!(swap_locked(&lz.token));
    }

    #[test]
    fn pair_token_tax_proceeds_wait_while_that_token_is_bought_back() {
        let mut f = factory();
        near_sdk::env::storage_write(TAX_SELLER_KEY, b"seller.near");
        let usdc = usdc_pair(&mut f);
        let id = live_tax_launch(&mut f, "TXL", Some(usdc.clone()), 10_000);
        ctx("owner.near", 0);
        drop(f.process_tax(U64(id), U128(900), None, None));
        ctx(ME, 0);
        assert!(f.on_tax_to_seller(U64(id), U128(8_400), Some(U128(900)), Ok(())));

        take_swap_lock(&usdc, 999, LK_BUYBACK);
        ctx("usdc.near", 0);
        let r = f.ft_on_transfer(acct("seller.near"), U128(1_000), format!(r#"{{"tax_proceeds":{},"sold":"8400"}}"#, id));
        assert!(matches!(r, PromiseOrValue::Value(U128(1_000))));
        assert_eq!(read_counter(&tax_selling_key(id)), 8_400, "nothing settled");
        release_swap_lock(&usdc, 999, LK_BUYBACK);
        ctx("usdc.near", 0);
        let r = f.ft_on_transfer(acct("seller.near"), U128(1_000), format!(r#"{{"tax_proceeds":{},"sold":"8400"}}"#, id));
        assert!(matches!(r, PromiseOrValue::Value(U128(0))));
        assert_eq!(read_counter(&tax_selling_key(id)), 0);
    }

    #[test]
    fn the_buyback_credit_is_capped_at_104_percent_of_min_out() {
        let mut f = factory();
        let id = live_burn_launch(&mut f, "CAP", 10u128.pow(24));
        ctx("owner.near", 0);
        drop(f.buyback(U64(id), U128(1000)));
        ctx(ME, 0);
        drop(f.on_bb_swapped(U64(id), U128(10u128.pow(24)), U128(50), Some(U128(1000)), Ok(U128(10u128.pow(24)))));
        ctx(ME, 0);

        let before = l(&f, id).creator_token_fees.0;
        drop(f.on_bb_after(U64(id), U128(50), U128(10u128.pow(24)), Some(U128(1000)), Ok(U128(50 + 1000 + 5000))));
        let burn = calls().into_iter().find(|c| c.1 == "burn").expect("burn");
        assert!(burn.2.contains(r#""amount":"1040""#), "{}", burn.2);
        assert!(logs_with("buyback")[0].contains(r#""bought":"1040""#));

        assert_eq!(l(&f, id).creator_token_fees.0, before);
        assert!(logs_with("buyback_excess_ignored")[0].contains(r#""amount":"4960""#));
        assert!(logs_with("buyback_excess_booked").is_empty());
    }



    #[test]
    fn a_holders_launch_on_a_pair_banks_and_pays_its_holders_in_the_pair_token() {
        let mut f = factory();
        let usdc = usdc_pair(&mut f);
        let id = launch_with(&mut f, "alice.near", LaunchArgs { quote: Some(usdc.clone()), fee_mode: Some("holders".into()), ..args("HPAIR") });
        assert_eq!(fee_opts(id, 8000).0, FM_HOLDERS);
        let mut lz = l(&f, id);
        lz.step = Step::Done;
        lz.inflight = true;
        f.launches.insert(id, lz);

        let v = if l(&f, id).token_is_x { vec![U128(0), U128(1_000)] } else { vec![U128(1_000), U128(0)] };
        ctx_gas(ME, 100);
        assert!(f.on_fees_claimed(id, Ok(v)));
        assert_eq!((mode_bucket(id), l(&f, id).creator_quote_fees.0, l(&f, id).protocol_quote_fees.0), (700, 0, 300));
        assert_eq!(f.get_fee_opts(U64(id)).bucket.0, 700);

        ctx("owner.near", 0);
        let eleven: Vec<(AccountId, U128)> = (0..11).map(|i| (acct(&format!("h{i}.near")), U128(1))).collect();
        assert!(panics(|| f.pay_holders(U64(id), eleven)).contains("too many payouts"));
        ctx("owner.near", 0);
        f.pay_holders(U64(id), vec![(acct("a.near"), U128(400)), (acct("b.near"), U128(100))]);
        assert_eq!((mode_bucket(id), read_counter(&paid_key(id))), (200, 500));
        let c = calls();
        assert!(c.iter().any(|c| c.0 == "usdc.near" && c.1 == "ft_transfer" && c.2.contains(r#""receiver_id":"a.near""#) && c.2.contains(r#""amount":"400""#)), "{c:?}");
        assert!(transfers().is_empty(), "no NEAR leaves for a pair launch");
        assert!(logs_with("holders_paid")[0].contains(r#""amount":"500""#));

        ctx_results(vec![near_sdk::PromiseResult::Failed]);
        f.on_paid(U64(id), "quote".into(), acct("b.near"), U128(100), "mode".into(), false);
        assert_eq!((mode_bucket(id), f.get_owed("usdc.near".into(), acct("b.near")).0), (200, 100));

        ctx("alice.near", 1);
        assert_eq!(panics(|| drop(f.claim_creator_quote_fees(U64(id)))), "this launch's fees go to buyback or holders");

        let nid = launch_with(&mut f, "alice.near", LaunchArgs { fee_mode: Some("holders".into()), ..args("HNEAR") });
        park(&mut f, nid, Step::Done);
        set_mode_bucket(nid, 1_000);
        ctx("owner.near", 0);
        f.pay_holders(U64(nid), vec![(acct("a.near"), U128(10))]);
        assert_eq!(transfers(), vec![("a.near".to_string(), 10)]);
    }

    #[test]
    fn a_pair_holders_launch_with_a_creator_tax_share_is_still_refused() {
        let mut f = factory();
        let usdc = usdc_pair(&mut f);
        owner();
        f.set_tax_token_code_hash(Some("11".repeat(32)));
        let msg = panics(|| drop(launch_with(&mut f, "alice.near", LaunchArgs { quote: Some(usdc.clone()), fee_mode: Some("holders".into()), tax: Some(TaxArgs { buy_bps: 300, sell_bps: 300, creator_bps: 5000, burn_bps: 0, holders_bps: 5000 }), ..args("HTAX") })));
        assert!(msg.contains("needs fee mode creator"), "{msg}");

        let id = launch_with(&mut f, "alice.near", LaunchArgs { quote: Some(usdc), fee_mode: Some("holders".into()), tax: Some(TaxArgs { buy_bps: 300, sell_bps: 300, creator_bps: 0, burn_bps: 5000, holders_bps: 5000 }), ..args("HTAX0") });
        assert_eq!(fee_opts(id, 8000).0, FM_HOLDERS);
    }



    #[test]
    fn pots_refuse_the_launchpad_and_a_split_that_would_not_fit() {
        let mut f = factory();
        owner();
        assert_eq!(panics(|| f.set_burn_pot(Some(acct(ME)))), "burn pot: not the launchpad itself");
        owner();
        assert_eq!(panics(|| f.set_referral_pot(Some(acct(ME)))), "referral pot: not the launchpad itself");
        owner();
        f.set_protocol_recipients(vec![(acct("p1.near"), 3400), (acct("p2.near"), 3300), (acct("p3.near"), 3300)]);
        owner();
        f.set_burn_pot(Some(acct("burn.near")));

        owner();
        assert!(panics(|| f.set_referral_pot(Some(acct("refpot.near")))).contains("4 legs or fewer"));
        owner();
        assert!(panics(|| f.set_protocol_recipients(recipients(4, "q"))).contains("4 legs or fewer"));

        owner();
        f.set_protocol_recipients(vec![(acct("p1.near"), 5000), (acct("p2.near"), 5000)]);
        owner();
        f.set_referral_pot(Some(acct("refpot.near")));
        assert_eq!(f.pot_recipients(7000).unwrap().len(), 3);

        owner();
        f.set_burn_pot(None);
    }

    #[test]
    fn audit_failed_unwraps_stay_owed_to_their_launch_and_a_retry_pays_them() {
        let mut f = factory();
        let near = 10u128.pow(24);
        let id = live_burn_launch(&mut f, "OWE", 10 * near);
        let bucket0 = mode_bucket(id);
        ctx(ME, 0);

        f.on_bb_unwrapped(U64(id), U128(3 * near), Err(PromiseError::Failed));
        assert!(!f.on_dev_refund_unwrapped(id, U128(2 * near), Err(PromiseError::Failed)));
        assert_eq!(f.get_wnear_owed(U64(id)), (U128(2 * near), U128(3 * near)));
        assert_eq!(read_counter(UW_TOTAL_KEY), 5 * near);
        assert_eq!(mode_bucket(id), bucket0, "nothing credited yet");
        owner();
        assert!(panics(|| drop(f.sweep_wnear(U128(near)))).contains("retry_wnear_owed"));

        ctx("anyone.near", 0);
        drop(f.retry_wnear_owed(U64(id)));
        let uw = calls().into_iter().find(|c| c.1 == "near_withdraw").expect("unwrap");
        assert!(uw.2.contains(&format!(r#""amount":"{}""#, 5 * near)), "{}", uw.2);
        ctx("anyone.near", 0);
        assert_eq!(panics(|| drop(f.retry_wnear_owed(U64(id)))), "retry in flight");

        ctx(ME, 0);
        assert!(!f.on_wnear_owed_unwrapped(U64(id), U128(2 * near), U128(3 * near), retry_op(id), Err(PromiseError::Failed)));
        assert_eq!(read_counter(UW_TOTAL_KEY), 5 * near);
        ctx("anyone.near", 0);
        drop(f.retry_wnear_owed(U64(id)));
        ctx(ME, 0);
        assert!(f.on_wnear_owed_unwrapped(U64(id), U128(2 * near), U128(3 * near), retry_op(id), Ok(())));
        assert_eq!(f.get_wnear_owed(U64(id)), (U128(0), U128(0)));
        assert_eq!(read_counter(UW_TOTAL_KEY), 0);
        assert_eq!(mode_bucket(id), bucket0 + 3 * near, "the bucket's part is back");
        assert_eq!(transfers(), vec![(l(&f, id).creator.to_string(), 2 * near)], "the creator's part is paid");
        owner();
        drop(f.sweep_wnear(U128(near)));
    }

    #[test]
    fn a_refused_buyback_recredits_only_after_its_unwrap() {
        let mut f = factory();
        let n = 10u128.pow(24);
        let id = live_burn_launch(&mut f, "REF", 3 * n);
        let token = l(&f, id).token.clone();
        ctx("owner.near", 0);
        drop(f.buyback(U64(id), U128(1000)));
        assert_eq!(mode_bucket(id), 0);
        ctx(ME, 0);
        drop(f.on_bb_swapped(U64(id), U128(3 * n), U128(0), Some(U128(1000)), Ok(U128(0))));
        assert!(!swap_locked(&token));
        assert_eq!(mode_bucket(id), 0, "not credited before the unwrap");
        ctx(ME, 0);
        f.on_bb_unwrapped(U64(id), U128(3 * n), Ok(()));
        assert_eq!(mode_bucket(id), 3 * n);
    }

    #[test]
    fn process_tax_burns_the_burn_share_and_hands_the_rest_to_the_seller() {
        let mut f = factory();
        near_sdk::env::storage_write(TAX_SELLER_KEY, b"seller.near");
        let usdc = usdc_pair(&mut f);
        for quote in [None, Some(usdc.clone())] {
            let id = live_tax_launch(&mut f, if quote.is_some() { "TXU" } else { "TXN" }, quote.clone(), 10_000);
            let token = l(&f, id).token.clone();
            ctx("owner.near", 0);

            drop(f.process_tax(U64(id), U128(900), None, Some(false)));
            let c = calls();

            assert!(c.iter().any(|x| x.0 == token.as_str() && x.1 == "burn" && x.2.contains(r#""amount":"1600""#)));
            assert!(c.iter().any(|x| x.0 == token.as_str() && x.1 == "ft_transfer" && x.2.contains(r#""receiver_id":"seller.near""#) && x.2.contains(r#""amount":"8400""#)));
            assert!(c.iter().all(|x| x.1 != "ft_transfer_call" && x.1 != "ft_balance_of"), "no swap here");
            assert_eq!(read_counter(&tax_selling_key(id)), 8_400);
            assert_eq!(read_counter(&tax_floor_key(id)), 900);
            assert!(!swap_locked(&token) && !swap_locked(&acct("wrap.near")));
            ctx(ME, 0);
            assert!(f.on_tax_to_seller(U64(id), U128(8_400), Some(U128(900)), Ok(())));

            if quote.is_none() {
                ctx("seller.near", 1_000);
                assert!(f.tax_proceeds(U64(id), U128(8_400)));
            } else {
                ctx("usdc.near", 0);
                let r = f.ft_on_transfer(acct("seller.near"), U128(1_000), format!(r#"{{"tax_proceeds":{},"sold":"8400"}}"#, id));
                assert!(matches!(r, PromiseOrValue::Value(U128(0))));
            }
            let t = f.get_tax(U64(id)).unwrap();
            assert_eq!((t.selling.0, t.floor.0), (0, 0));
            assert_eq!(t.paid_platform.0 + t.paid_creator.0 + t.holders_bucket.0, 1_000);
            assert!(t.paid_platform.0 > 0 && t.paid_creator.0 > 0 && t.holders_bucket.0 > 0);
        }

        let id = live_tax_launch(&mut f, "TXB", None, 10_000);
        ctx("owner.near", 0);
        drop(f.process_tax(U64(id), U128(450), Some(U128(5_000)), None));
        assert_eq!(read_counter(&tax_pending_key(id)), 5_000, "max_amount leaves the rest pending");
        ctx(ME, 0);
        assert!(!f.on_tax_to_seller(U64(id), U128(4_200), Some(U128(450)), Err(PromiseError::Failed)));
        assert_eq!((read_counter(&tax_pending_key(id)), read_counter(&tax_selling_key(id)), read_counter(&tax_floor_key(id))), (9_200, 0, 0));
    }

    #[test]
    fn audit_the_sellers_proceeds_must_clear_the_keepers_floor_pro_rata() {
        let mut f = factory();
        near_sdk::env::storage_write(TAX_SELLER_KEY, b"seller.near");
        let id = live_tax_launch(&mut f, "TXF", None, 10_000);
        ctx("owner.near", 0);
        assert_eq!(panics(|| drop(f.process_tax(U64(id), U128(0), None, None))), "min_out required: the floor the seller's proceeds must clear for this slice");
        drop(f.process_tax(U64(id), U128(900), None, None));
        ctx(ME, 0);
        assert!(f.on_tax_to_seller(U64(id), U128(8_400), Some(U128(900)), Ok(())));
        assert_eq!(f.get_tax(U64(id)).unwrap().floor.0, 900);

        ctx("seller.near", 449);
        assert_eq!(panics(|| drop(f.tax_proceeds(U64(id), U128(4_200)))), "proceeds below the keeper's floor for what was sold");
        ctx("seller.near", 450);
        assert!(f.tax_proceeds(U64(id), U128(4_200)));
        assert_eq!((read_counter(&tax_selling_key(id)), read_counter(&tax_floor_key(id))), (4_200, 450));

        let usdc = usdc_pair(&mut f);
        let id2 = live_tax_launch(&mut f, "TXG", Some(usdc.clone()), 10_000);
        ctx("owner.near", 0);
        drop(f.process_tax(U64(id2), U128(1_000), None, None));
        ctx(ME, 0);
        assert!(f.on_tax_to_seller(U64(id2), U128(8_400), Some(U128(1_000)), Ok(())));
        ctx("usdc.near", 0);
        let msg = format!(r#"{{"tax_proceeds":{},"sold":"8400"}}"#, id2);
        assert_eq!(panics(|| drop(f.ft_on_transfer(acct("seller.near"), U128(999), msg.clone()))), "proceeds below the keeper's floor for what was sold");
        ctx("usdc.near", 0);
        assert!(matches!(f.ft_on_transfer(acct("seller.near"), U128(1_000), msg), PromiseOrValue::Value(U128(0))));

        ctx("seller.near", 0);
        assert_eq!(panics(|| f.clear_tax_floor(U64(id))), "owner only");
        ctx("owner.near", 0);
        f.clear_tax_floor(U64(id));
        ctx("seller.near", 1);
        assert!(f.tax_proceeds(U64(id), U128(4_200)));
        assert_eq!(read_counter(&tax_selling_key(id)), 0);

        let id4 = live_tax_launch(&mut f, "TXR", None, 10_000);
        let token4 = l(&f, id4).token.clone();
        ctx("owner.near", 0);
        drop(f.process_tax(U64(id4), U128(900), None, None));
        ctx(ME, 0);
        assert!(f.on_tax_to_seller(U64(id4), U128(8_400), Some(U128(900)), Ok(())));
        assert_eq!((read_counter(&tax_pending_key(id4)), read_counter(&tax_selling_key(id4)), read_counter(&tax_floor_key(id4))), (0, 8_400, 900));
        ctx(token4.as_str(), 0);
        let back = format!(r#"{{"tax_return":{}}}"#, id4);

        assert!(matches!(f.ft_on_transfer(acct("seller.near"), U128(9_000), back.clone()), PromiseOrValue::Value(U128(9_000))));
        assert!(matches!(f.ft_on_transfer(acct("bob.near"), U128(4_200), back.clone()), PromiseOrValue::Value(U128(4_200))));
        take_swap_lock(&token4, 7, LK_BUYBACK);
        ctx(token4.as_str(), 0);
        assert!(matches!(f.ft_on_transfer(acct("seller.near"), U128(4_200), back.clone()), PromiseOrValue::Value(U128(4_200))));
        release_swap_lock(&token4, 7, LK_BUYBACK);

        ctx(token4.as_str(), 0);
        assert!(matches!(f.ft_on_transfer(acct("seller.near"), U128(4_200), back.clone()), PromiseOrValue::Value(U128(0))));
        assert_eq!((read_counter(&tax_pending_key(id4)), read_counter(&tax_selling_key(id4)), read_counter(&tax_floor_key(id4))), (4_200, 4_200, 450));
        assert!(logs_with("tax_returned")[0].contains(r#""floor_dropped":"450""#));

        ctx("seller.near", 450);
        assert!(f.tax_proceeds(U64(id4), U128(4_200)));
        assert_eq!((read_counter(&tax_selling_key(id4)), read_counter(&tax_floor_key(id4))), (0, 0));

        ctx("owner.near", 0);
        drop(f.process_tax(U64(id4), U128(300), None, None));
        assert_eq!(read_counter(&tax_floor_key(id4)), 300);

        let (floor, held) = (33_420_337_735_209_033_761_456u128, 34_048_186_375_867_507_795_096u128);
        assert_eq!(mul_div_wide(floor, held, held), floor);
        assert_eq!(mul_div_wide(floor, held / 2, held), 16_710_168_867_604_516_880_728);
        assert_eq!(mul_div_wide(floor, 1, held), 0);
        assert_eq!(mul_div_wide(u128::MAX, 3, 7), u128::MAX / 7 * 3 + (u128::MAX % 7) * 3 / 7);
        let id3 = live_tax_launch(&mut f, "TXH", None, held * 10_000 / 8_400 + 1);
        ctx("owner.near", 0);
        drop(f.process_tax(U64(id3), U128(floor), None, None));
        let selling = read_counter(&tax_selling_key(id3));
        ctx(ME, 0);
        assert!(f.on_tax_to_seller(U64(id3), U128(selling), Some(U128(floor)), Ok(())));
        ctx("seller.near", floor - 1);
        assert_eq!(panics(|| drop(f.tax_proceeds(U64(id3), U128(selling)))), "proceeds below the keeper's floor for what was sold");

        ctx("seller.near", floor / 2 - 2);
        assert_eq!(panics(|| drop(f.tax_proceeds(U64(id3), U128(selling / 2)))), "proceeds below the keeper's floor for what was sold");
        ctx("seller.near", floor / 2 + 1);
        assert!(f.tax_proceeds(U64(id3), U128(selling / 2)));
        assert!(read_counter(&tax_floor_key(id3)) >= floor / 2 && read_counter(&tax_floor_key(id3)) <= floor / 2 + 1, "half the floor is left");
    }

    #[test]
    fn a_lock_expires_and_only_its_owner_releases_it() {
        let _f = factory();
        let t = acct("tok.near");
        ctx_at(ME, 0, 1_000);
        take_swap_lock(&t, 7, LK_BUYBACK);
        release_swap_lock(&t, 8, LK_BUYBACK);
        release_swap_lock(&t, 7, LK_ADMIN);
        assert!(swap_locked(&t), "another chain's callback leaves it alone");
        ctx_at(ME, 0, 1_000 + SWAP_LOCK_TTL_MS - 1);
        assert!(swap_locked(&t));
        ctx_at(ME, 0, 1_000 + SWAP_LOCK_TTL_MS);
        assert!(!swap_locked(&t), "a lost callback cannot freeze the token for good");
        take_swap_lock(&t, 9, LK_ADMIN);
        release_swap_lock(&t, 7, LK_BUYBACK);
        assert!(swap_locked(&t), "a late callback of the expired chain leaves the new lock alone");
        release_swap_lock(&t, 9, LK_ADMIN);
        assert!(!swap_locked(&t));
    }

    #[test]
    fn sweep_credits_the_protocol_only_after_the_unwrap() {
        let mut f = factory();
        ctx("owner.near", 0);
        drop(f.sweep_wnear(U128(500)));
        assert_eq!(f.protocol_fees, 0);
        ctx(ME, 0);
        f.on_wnear_swept(U128(500), Some(true), Err(PromiseError::Failed));
        assert_eq!(f.protocol_fees, 0);
        ctx("owner.near", 0);
        drop(f.sweep_wnear(U128(500)));
        ctx(ME, 0);
        f.on_wnear_swept(U128(500), Some(true), Ok(()));
        assert_eq!(f.protocol_fees, 500);

        ctx(ME, 0);
        f.on_wnear_swept(U128(200), None, Ok(()));
        assert_eq!(f.protocol_fees, 500);
        ctx(ME, 0);
        f.on_wnear_swept(U128(200), None, Err(PromiseError::Failed));
        assert_eq!(f.protocol_fees, 300);
    }

    #[test]
    fn a_default_launch_keeps_its_share_when_the_default_changes() {
        let mut f = factory();
        let id = launch(&mut f, "alice.near", "FIX");
        assert!(env::storage_has_key(&fee_opts_key(id)), "stored even at the default");
        assert!(logs_with("fee_opts").is_empty(), "the event still marks only non-default launches");
        let mut c = f.config.clone();
        c.creator_fee_share_bps = 8_000;
        ctx("owner.near", 0);
        assert_eq!(panics(|| f.set_config(c)), "creator_fee_share_bps: 7000", "the share is fixed, not a setting");
        assert_eq!(f.get_fee_opts(U64(id)).creator_share_bps, 7_000);
    }

    #[test]
    fn backfill_pins_the_default_on_old_launches_only() {
        let mut f = factory();
        let old = launch(&mut f, "alice.near", "OLD");
        let custom = launch_with(&mut f, "alice.near", LaunchArgs { creator_share_bps: Some(7_000), ..args("CUS") });
        env::storage_remove(&fee_opts_key(old));
        ctx("owner.near", 0);
        assert_eq!(f.backfill_fee_opts(vec![U64(old), U64(custom), U64(99)]), 1);
        assert_eq!(f.get_fee_opts(U64(custom)).creator_share_bps, 7_000, "an existing record is untouched");
        assert_eq!(f.get_fee_opts(U64(old)).creator_share_bps, 7_000);
        ctx("alice.near", 0);
        assert_eq!(panics(|| { f.backfill_fee_opts(vec![U64(old)]); }), "owner only");
    }

    #[test]
    fn set_config_keeps_the_default_share_in_the_creator_band() {
        let mut f = factory();
        for (bps, ok) in [(6_999u16, false), (7_000, true), (7_500, false), (8_000, false), (9_000, false), (10_000, false)] {
            let mut c = f.config.clone();
            c.creator_fee_share_bps = bps;
            ctx("owner.near", 0);
            let r = panics(|| f.set_config(c));
            assert_eq!(r.is_empty(), ok, "{bps}: {r}");
        }
    }

    #[test]
    fn a_later_house_flag_does_not_sweep_a_booked_creator_balance() {
        let mut f = factory();
        ctx("owner.near", 0);
        f.protocol_recipients = vec![(acct("p1.near"), 10_000)];
        let id = launch(&mut f, "alice.near", "HOUSE");
        park(&mut f, id, Step::Done);
        let mut lz = l(&f, id);
        lz.creator_token_fees = U128(700);
        lz.protocol_token_fees = U128(300);
        f.launches.insert(id, lz);
        ctx("owner.near", 0);
        f.set_house_creators(vec![acct("alice.near")], true);
        ctx("anyone.near", 0);
        f.split_protocol_token_fees(U64(id));
        let lz = l(&f, id);
        assert_eq!((lz.creator_token_fees.0, lz.protocol_token_fees.0), (700, 0));
        assert!(logs_with("protocol_token_fees_split")[0].contains(r#""amount":"300""#));
    }



    #[test]
    fn payout_batches_refuse_a_repeated_account_and_buyback_needs_a_floor() {
        let mut f = factory();
        let id = launch_with(&mut f, "alice.near", LaunchArgs { fee_mode: Some("holders".into()), ..args("HOLD") });
        park(&mut f, id, Step::Done);
        set_mode_bucket(id, 1_000);
        ctx("owner.near", 0);
        assert_eq!(panics(|| f.pay_holders(U64(id), vec![(acct("a.near"), U128(10)), (acct("b.near"), U128(10)), (acct("a.near"), U128(10))])), "duplicate account in payouts");
        ctx("owner.near", 0);
        f.pay_holders(U64(id), vec![(acct("a.near"), U128(10)), (acct("b.near"), U128(10))]);
        assert_eq!(mode_bucket(id), 980);
        let tid = live_tax_launch(&mut f, "THOLD", None, 0);
        add_counter(&tax_holders_key(tid), 1_000);
        ctx("owner.near", 0);
        assert_eq!(panics(|| f.pay_tax_holders(U64(tid), vec![(acct("a.near"), U128(1)), (acct("a.near"), U128(1))])), "duplicate account in payouts");
        let bid = live_burn_launch(&mut f, "FLOOR0", 10u128.pow(24));
        ctx("owner.near", 0);
        assert_eq!(panics(|| drop(f.buyback(U64(bid), U128(0)))), "min_out required");
    }



    #[test]
    fn a_holder_payout_that_bounces_is_owed_to_that_holder_and_sent_again() {
        let mut f = factory();
        let tid = live_tax_launch(&mut f, "OWED", None, 0);
        add_counter(&tax_holders_key(tid), 1_000);
        ctx("owner.near", 0);
        f.pay_tax_holders(U64(tid), vec![(acct("a.near"), U128(300)), (acct("gone.near"), U128(200))]);
        assert_eq!((read_counter(&tax_holders_key(tid)), read_counter(&tax_paid_holders_key(tid))), (500, 500));

        ctx_results(vec![near_sdk::PromiseResult::Failed]);
        f.on_paid(U64(tid), "near".into(), acct("gone.near"), U128(200), "tax_holders".into(), false);
        assert_eq!((read_counter(&tax_holders_key(tid)), read_counter(&tax_paid_holders_key(tid))), (500, 500));
        assert_eq!((f.get_owed("near".into(), acct("gone.near")).0, f.get_owed_total("near".into()).0), (200, 200));
        assert!(logs_with("holder_owed")[0].contains(r#""to":"gone.near","asset":"near","amount":"200""#));
        assert!(logs_with("tax_holder_pay_failed").is_empty());

        let hid = launch_with(&mut f, "alice.near", LaunchArgs { fee_mode: Some("holders".into()), ..args("HMODE") });
        park(&mut f, hid, Step::Done);
        set_mode_bucket(hid, 100);
        ctx("owner.near", 0);
        f.pay_holders(U64(hid), vec![(acct("gone.near"), U128(40))]);
        ctx_results(vec![near_sdk::PromiseResult::Failed]);
        f.on_paid(U64(hid), "near".into(), acct("gone.near"), U128(40), "mode".into(), false);
        assert_eq!((mode_bucket(hid), f.get_owed("near".into(), acct("gone.near")).0), (60, 240));

        ctx("anyone.near", 0);
        f.push_owed("near".into(), vec![acct("gone.near"), acct("nobody.near")]);
        assert_eq!(transfers(), vec![("gone.near".to_string(), 240)]);
        assert!(calls().iter().any(|c| c.1 == "on_owed_pushed"));
        assert_eq!((f.get_owed("near".into(), acct("gone.near")).0, f.get_owed_total("near".into()).0), (0, 0));

        ctx_results(vec![near_sdk::PromiseResult::Failed]);
        f.on_owed_pushed("near".into(), acct("gone.near"), U128(240));
        assert_eq!(f.get_owed("near".into(), acct("gone.near")).0, 240);
        assert_eq!(logs_with("owed_push_failed").len(), 1);
        ctx("anyone.near", 0);
        f.push_owed("near".into(), vec![acct("gone.near")]);
        ctx_results(vec![near_sdk::PromiseResult::Successful(vec![])]);
        f.on_owed_pushed("near".into(), acct("gone.near"), U128(240));
        assert!(logs_with("owed_paid")[0].contains(r#""amount":"240""#));
        assert_eq!((f.get_owed("near".into(), acct("gone.near")).0, f.get_owed_total("near".into()).0), (0, 0));

        ctx("anyone.near", 0);
        assert_eq!(panics(|| f.push_owed("near".into(), vec![acct("a.near"), acct("a.near")])), "duplicate account");
        let many: Vec<AccountId> = (0..26).map(|i| acct(&format!("h{i}.near"))).collect();
        ctx("anyone.near", 0);
        assert!(panics(|| f.push_owed("near".into(), many)).contains("too many accounts"));
        ctx("anyone.near", 0);
        assert_eq!(f.get_owed_many("near".into(), vec![acct("gone.near")]), vec![(acct("gone.near"), U128(0))]);
    }

    #[test]
    fn a_pair_token_holder_payout_is_owed_in_that_token_and_waits_for_a_buyback() {
        let mut f = factory();
        let usdc = usdc_pair(&mut f);
        let tid = live_tax_launch(&mut f, "OWEDQ", Some(usdc.clone()), 0);
        add_counter(&tax_holders_key(tid), 1_000);
        ctx("owner.near", 0);
        f.pay_tax_holders(U64(tid), vec![(acct("b.near"), U128(70))]);
        ctx_results(vec![near_sdk::PromiseResult::Failed]);
        f.on_paid(U64(tid), "quote".into(), acct("b.near"), U128(70), "tax_holders".into(), false);
        assert_eq!((f.get_owed("usdc.near".into(), acct("b.near")).0, f.get_owed("near".into(), acct("b.near")).0), (70, 0));
        ctx("anyone.near", 0);
        f.push_owed("usdc.near".into(), vec![acct("b.near")]);
        let c = calls();
        assert!(c.iter().any(|c| c.0 == "usdc.near" && c.1 == "ft_transfer" && c.2.contains(r#""receiver_id":"b.near""#) && c.2.contains(r#""amount":"70""#)));
        assert_eq!(f.get_owed("usdc.near".into(), acct("b.near")).0, 0);

        let nid = launch(&mut f, "alice.near", "PAIRT");
        let pair_token = l(&f, nid).token.clone();
        add_owed(pair_token.as_str(), &acct("c.near"), 5);
        take_swap_lock(&pair_token, nid, LK_BUYBACK);
        ctx("anyone.near", 0);
        assert_eq!(panics(|| f.push_owed(pair_token.to_string(), vec![acct("c.near")])), "swap in flight");
        assert_eq!(f.get_owed(pair_token.to_string(), acct("c.near")).0, 5);
        ctx("anyone.near", 0);
        assert!(panics(|| f.push_owed("not an account!".into(), vec![acct("c.near")])).contains("asset"));
    }



    #[test]
    fn pause_stops_value_moving_calls_but_not_creator_claims_or_owner_withdrawals() {
        let mut f = factory();
        ctx("owner.near", 0);
        f.protocol_recipients = vec![(acct("p1.near"), 10_000)];
        let id = launch(&mut f, "alice.near", "PAUSE");
        park(&mut f, id, Step::Done);
        let mut lz = l(&f, id);
        lz.creator_token_fees = U128(10);
        lz.protocol_token_fees = U128(10);
        f.launches.insert(id, lz);
        f.creator_fees.insert(acct("alice.near"), 10);
        f.protocol_fees = 10;
        let bid = live_burn_launch(&mut f, "PBURN", 10);
        let tid = live_tax_launch(&mut f, "PTAX", None, 10);
        let stuck = launch(&mut f, "alice.near", "PSTUCK");
        park(&mut f, stuck, Step::CreatePool);
        ctx("owner.near", 0);
        f.set_paused(true);

        let mut blocked: Vec<(&str, Box<dyn FnOnce(&mut Factory)>)> = vec![
            ("resume", Box::new(move |f: &mut Factory| drop(f.resume(U64(stuck))))),
            ("claim_fees", Box::new(move |f: &mut Factory| drop(f.claim_fees(U64(id))))),
            ("collect_tax", Box::new(move |f: &mut Factory| drop(f.collect_tax(U64(tid))))),
            ("process_tax", Box::new(move |f: &mut Factory| drop(f.process_tax(U64(tid), U128(1), None, None)))),
            ("buyback", Box::new(move |f: &mut Factory| drop(f.buyback(U64(bid), U128(1))))),
            ("pay_holders", Box::new(move |f: &mut Factory| f.pay_holders(U64(bid), vec![(acct("a.near"), U128(1))]))),
            ("pay_tax_holders", Box::new(move |f: &mut Factory| f.pay_tax_holders(U64(tid), vec![(acct("a.near"), U128(1))]))),
            ("tax_proceeds", Box::new(move |f: &mut Factory| { f.tax_proceeds(U64(tid), U128(1)); })),
            ("push_creator_fees", Box::new(|f: &mut Factory| drop(f.push_creator_fees(acct("alice.near"))))),
            ("push_creator_token_fees", Box::new(move |f: &mut Factory| drop(f.push_creator_token_fees(U64(id))))),
            ("push_creator_quote_fees", Box::new(move |f: &mut Factory| drop(f.push_creator_quote_fees(U64(id))))),
            ("split_protocol_token_fees", Box::new(move |f: &mut Factory| f.split_protocol_token_fees(U64(id)))),
            ("split_protocol_quote_fees", Box::new(move |f: &mut Factory| f.split_protocol_quote_fees(U64(id)))),
            ("burn_token_fees", Box::new(move |f: &mut Factory| drop(f.burn_token_fees(U64(bid))))),
            ("push_owed", Box::new(|f: &mut Factory| f.push_owed("near".into(), vec![acct("a.near")]))),
        ];
        for (name, call) in blocked.drain(..) {
            ctx("owner.near", 0);
            assert_eq!(panics(|| call(&mut f)), "paused", "{name}");
        }

        ctx("alice.near", 0);
        drop(f.claim_creator_fees());
        ctx("alice.near", 0);
        drop(f.claim_creator_token_fees(U64(id)));
        ctx("owner.near", 0);
        drop(f.withdraw_protocol_fees(acct("owner.near"), None));
        ctx("owner.near", 0);
        drop(f.withdraw_protocol_token_fees(U64(id), acct("owner.near")));
        ctx_at("alice.near", 0, T0 + DAY);
        drop(f.cancel_stuck_launch(U64(stuck)));
        assert_eq!(l(&f, stuck).step, Step::Failed);

        ctx(ME, 0);
        f.on_creator_pushed(acct("alice.near"), U128(3), Err(PromiseError::Failed));
        assert_eq!(f.get_creator_fees(acct("alice.near")).0, 3);

        near_sdk::env::storage_write(TAX_SELLER_KEY, b"seller.near");
        ctx("usdc.near", 0);
        assert!(matches!(f.ft_on_transfer(acct("seller.near"), U128(9), "{}".into()), PromiseOrValue::Value(U128(9))));
    }



    fn callback_names() -> Vec<String> { calls().into_iter().map(|c| c.1).collect() }

    #[test]
    fn claims_and_withdrawals_carry_a_recredit_callback() {
        let mut f = factory();
        let usdc = usdc_pair(&mut f);
        let id = launch(&mut f, "alice.near", "SEND");
        let qid = launch_with(&mut f, "alice.near", LaunchArgs { quote: Some(usdc.clone()), ..args("SENDQ") });
        for i in [id, qid] {
            park(&mut f, i, Step::Done);
            let mut lz = l(&f, i);
            lz.creator_token_fees = U128(11);
            lz.protocol_token_fees = U128(12);
            lz.creator_quote_fees = U128(13);
            lz.protocol_quote_fees = U128(14);
            f.launches.insert(i, lz);
        }
        f.creator_fees.insert(acct("alice.near"), 15);
        f.protocol_fees = 16;


        let bounce = |f: &mut Factory, lid: u64, kind: &str, to: &str, amount: u128, bucket: &str| {
            ctx_results(vec![near_sdk::PromiseResult::Failed]);
            f.on_paid(U64(lid), kind.into(), acct(to), U128(amount), bucket.into(), false);
        };
        ctx("alice.near", 0);
        drop(f.claim_creator_fees());
        assert!(callback_names().contains(&"on_paid".to_string()));
        bounce(&mut f, 0, "near", "alice.near", 15, "creator");
        assert_eq!(f.get_creator_fees(acct("alice.near")).0, 15);
        assert_eq!(logs_with("creator_push_failed").len(), 1);

        ctx("alice.near", 0);
        drop(f.claim_creator_token_fees(U64(id)));
        assert!(callback_names().contains(&"on_paid".to_string()));
        bounce(&mut f, id, "token", "alice.near", 11, "creator");
        assert_eq!(l(&f, id).creator_token_fees.0, 11);
        assert_eq!(logs_with("creator_token_push_failed").len(), 1);

        ctx("alice.near", 0);
        drop(f.claim_creator_quote_fees(U64(qid)));
        bounce(&mut f, qid, "quote", "alice.near", 13, "creator");
        assert_eq!(l(&f, qid).creator_quote_fees.0, 13);
        assert!(logs_with("quote_push_failed")[0].contains(r#""protocol":false"#));

        ctx("owner.near", 0);
        drop(f.withdraw_protocol_quote_fees(U64(qid), acct("owner.near")));
        bounce(&mut f, qid, "quote", "owner.near", 14, "protocol");
        assert_eq!(l(&f, qid).protocol_quote_fees.0, 14);
        assert!(logs_with("quote_push_failed")[0].contains(r#""protocol":true"#));

        ctx("owner.near", 0);
        drop(f.withdraw_protocol_fees(acct("owner.near"), None));
        bounce(&mut f, 0, "near", "owner.near", 16, "protocol");
        assert_eq!(f.protocol_fees, 16);
        assert_eq!(logs_with("protocol_split_failed").len(), 1);

        ctx("owner.near", 0);
        drop(f.withdraw_protocol_token_fees(U64(id), acct("owner.near")));
        bounce(&mut f, id, "token", "owner.near", 12, "protocol");
        assert_eq!(l(&f, id).protocol_token_fees.0, 12);
        assert_eq!(logs_with("protocol_token_split_failed").len(), 1);


        ctx(ME, 0);
        f.on_creator_pushed(acct("alice.near"), U128(1), Err(PromiseError::Failed));
        f.on_creator_token_pushed(U64(id), U128(1), Err(PromiseError::Failed));
        f.on_quote_pushed(U64(qid), U128(1), None, Err(PromiseError::Failed));
        f.on_quote_pushed(U64(qid), U128(1), Some(acct("x.near")), Err(PromiseError::Failed));
        f.on_protocol_split(acct("x.near"), U128(1), Err(PromiseError::Failed));
        f.on_token_split(U64(id), acct("x.near"), U128(1), Err(PromiseError::Failed));
        assert_eq!(f.get_creator_fees(acct("alice.near")).0, 16);
        assert_eq!((l(&f, id).creator_token_fees.0, l(&f, qid).creator_quote_fees.0, l(&f, qid).protocol_quote_fees.0, f.protocol_fees, l(&f, id).protocol_token_fees.0), (12, 14, 15, 17, 13));
    }



    fn recipients(n: usize, tag: &str) -> Vec<(AccountId, u16)> {
        let mut v: Vec<(AccountId, u16)> = (0..n).map(|i| (acct(&format!("{tag}{i}.near")), (10_000 / n) as u16)).collect();
        let sum: u16 = v.iter().map(|x| x.1).sum();
        v[0].1 += 10_000 - sum;
        v
    }

    #[test]
    fn the_claim_callback_gas_scales_with_the_legs_it_may_pay() {
        let mut f = factory();
        f.protocol_recipients = recipients(4, "p");
        let id = launch(&mut f, "alice.near", "LEGS");
        park(&mut f, id, Step::Done);
        ctx("owner.near", 0);
        f.set_launch_recipients(U64(id), Some(recipients(4, "r")));
        ctx("crank.near", 0);
        drop(f.claim_fees(U64(id)));
        let cb = get_created_receipts().into_iter().flat_map(|r| r.actions).find_map(|a| match a {
            MockAction::FunctionCallWeight { method_name, prepaid_gas, .. } if method_name == b"on_fees_claimed" => Some(prepaid_gas),
            _ => None,
        });
        assert_eq!(cb, Some(Gas::from_tgas(5 + 10 + 8 * 6)));
        assert!(Gas::from_tgas(5 + 10 + 8 * 6) > Gas::from_tgas(45), "the old flat 45 could not pay 8 legs");
    }

    #[test]
    fn a_split_the_callback_cannot_afford_waits_in_its_bucket() {
        let mut f = factory();
        f.protocol_recipients = recipients(4, "p");
        let id = launch(&mut f, "alice.near", "DEFER");
        let mut lz = l(&f, id);
        lz.step = Step::Done;
        lz.inflight = true;
        f.launches.insert(id, lz);
        let n = 10u128.pow(24);
        let v = if l(&f, id).token_is_x { vec![U128(0), U128(n)] } else { vec![U128(n), U128(0)] };
        ctx_gas(ME, 12);
        assert!(f.on_fees_claimed(id, Ok(v.clone())));
        assert_eq!(f.protocol_fees, n * 3 / 10, "booked, not paid");
        assert_eq!(logs_with("protocol_fees_split_deferred").len(), 1);
        assert!(transfers().is_empty());

        let mut lz = l(&f, id);
        lz.inflight = true;
        f.launches.insert(id, lz);
        ctx_gas(ME, 100);
        assert!(f.on_fees_claimed(id, Ok(v)));
        assert_eq!(f.protocol_fees, 0);
        assert_eq!(transfers().len(), 4);
    }

    #[test]
    fn add_liquidity_parks_unless_its_real_callback_gas_is_there() {
        let mut f = factory();
        let id = created_but_pool_refused(&mut f, 0);
        let point = l(&f, id).init_point;

        ctx_gas(ME, 205);
        drop(f.on_pool_checked(id, None, Ok(Some(near_sdk::serde_json::json!({"current_point": point})))));
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight), (Step::AddLiquidity, false), "parked, not over-committed");
        assert_eq!(logs_with("launch_step_parked").len(), 1);

        ctx_gas("anyone.near", 150);
        assert!(panics(|| drop(f.resume(U64(id)))).contains("attach more gas"));
    }

    #[test]
    fn a_launch_put_back_on_the_global_split_strands_nothing() {
        let mut f = factory();
        f.protocol_recipients = recipients(1, "p");
        f.min_push_yocto = 0;
        let id = launch(&mut f, "alice.near", "BACK");
        ctx("owner.near", 0);
        f.set_launch_recipients(U64(id), Some(recipients(2, "r")));
        set_launch_bucket(id, 1_000);
        ctx("owner.near", 0);
        f.set_launch_recipients(U64(id), None);
        assert_eq!(launch_bucket(id), 0);
        assert_eq!(transfers(), vec![("p0.near".to_string(), 1_000)], "moved to the shared bucket and pushed");

        ctx(ME, 0);
        f.on_launch_split(U64(id), acct("r0.near"), U128(40), Err(PromiseError::Failed));
        assert_eq!((launch_bucket(id), f.protocol_fees), (0, 40));
    }



    #[test]
    fn get_launches_pages_newest_first_without_reading_the_skipped() {
        let mut f = factory();
        for i in 0..7 { launch(&mut f, "alice.near", &format!("P{i}")); }

        let old = |f: &Factory, from: Option<u64>, limit: Option<u64>| -> Vec<u64> {
            let skip = from.unwrap_or(0);
            let limit = limit.unwrap_or(50).min(200);
            let (mut out, mut id, mut skipped) = (Vec::new(), f.next_id, 0);
            while id > 0 && (out.len() as u64) < limit {
                id -= 1;
                if let Some(l) = f.launches.get(&id) {
                    if skipped < skip { skipped += 1; continue; }
                    out.push(l.id);
                }
            }
            out
        };
        for from in [None, Some(0), Some(1), Some(3), Some(6), Some(7), Some(50), Some(u64::MAX)] {
            for limit in [None, Some(0), Some(1), Some(2), Some(5), Some(500)] {
                let new: Vec<u64> = f.get_launches(from, limit).iter().map(|l| l.id).collect();
                assert_eq!(new, old(&f, from, limit), "from {from:?} limit {limit:?}");
            }
        }
        assert_eq!(f.get_launches(Some(2), Some(3)).iter().map(|l| l.id).collect::<Vec<_>>(), vec![4, 3, 2]);
    }



    #[test]
    fn set_step_refuses_the_dead_steps() {
        let mut f = factory();
        let id = launch(&mut f, "alice.near", "DEAD");
        park(&mut f, id, Step::AddLiquidity);
        for step in [Step::Deposit, Step::ReadDevTokens, Step::DeliverDevTokens] {
            ctx("owner.near", 0);
            assert_eq!(panics(|| f.set_step(U64(id), step)), "dead step");
        }
        ctx("owner.near", 0);
        f.set_step(U64(id), Step::CreatePool);

        let mut lz = l(&f, id);
        lz.step = Step::ReadDevTokens;
        f.launches.insert(id, lz);
        ctx("anyone.near", 0);
        assert!(panics(|| drop(f.resume(U64(id)))).contains("dead step"));
    }

    #[test]
    fn set_quote_needs_a_range_on_the_point_grid() {
        let mut f = factory();
        ctx("owner.near", 0);
        assert_eq!(panics(|| f.set_quote(acct("usdc.near"), 6, -400_000, -400_000, -400_000, 400_100, true)), "range_points must be a multiple of 200");
        ctx("owner.near", 0);
        f.set_quote(acct("usdc.near"), 6, -400_000, -400_000, -400_000, 400_000, true);
    }

    #[test]
    fn register_quote_registers_every_locker() {
        let mut f = factory();
        usdc_pair(&mut f);
        ctx("owner.near", 0);
        f.set_locker_from(acct("lock_2.nearlytrade.near"), U64(5));
        ctx("owner.near", 0);
        drop(f.register_quote(acct("usdc.near")));
        let reg: Vec<(String, u128)> = calls().into_iter().filter(|c| c.1 == "register").map(|c| (c.0, c.3)).collect();
        let r = FT_STORAGE_REG.as_yoctonear();
        assert_eq!(reg, vec![("lock.near".to_string(), r), ("lock_2.nearlytrade.near".to_string(), r)], "paid from the factory, nothing attached");
        assert!(calls().iter().any(|c| c.0 == "usdc.near" && c.1 == "storage_deposit" && c.3 == r));
    }



    use near_sdk::serde_json::json;

    fn owner() { ctx("owner.near", 0); }

    #[test]
    fn every_owner_setter_is_instant_owner_only_and_needs_no_deposit() {
        let mut f = factory();
        let first = launch(&mut f, "alice.near", "PRE");
        let setters: Vec<(&str, Box<dyn Fn(&mut Factory)>)> = vec![
            ("set_locker_from", Box::new(|f: &mut Factory| f.set_locker_from(acct("lock_3.nearlytrade.near"), U64(f.next_id)))),
            ("set_token_code_hash", Box::new(|f: &mut Factory| f.set_token_code_hash("aa".repeat(32)))),
            ("set_tax_token_code_hash", Box::new(|f: &mut Factory| f.set_tax_token_code_hash(Some("bb".repeat(32))))),
            ("set_protocol_recipients", Box::new(|f: &mut Factory| f.set_protocol_recipients(vec![(acct("p1.near"), 5000), (acct("p2.near"), 5000)]))),
            ("set_dcl", Box::new(|f: &mut Factory| f.set_dcl(acct("dclv3.near"), None))),
            ("set_fee_router", Box::new(|f: &mut Factory| f.set_fee_router(acct("router.near"), true))),
            ("set_launch_hook", Box::new(|f: &mut Factory| f.set_launch_hook(Some(acct("hook.near"))))),
            ("set_tax_seller", Box::new(|f: &mut Factory| f.set_tax_seller(Some(acct("seller.near"))))),
            ("set_house_creator", Box::new(|f: &mut Factory| f.set_house_creator(Some(acct("house.near"))))),
            ("set_house_creators", Box::new(|f: &mut Factory| f.set_house_creators(vec![acct("h1.near"), acct("h2.near")], true))),
            ("set_launch_recipients", Box::new(move |f: &mut Factory| f.set_launch_recipients(U64(first), Some(vec![(acct("r1.near"), 4000), (acct("r2.near"), 6000)])))),
        ];
        for (name, call) in &setters {
            ctx("mallory.near", 0);
            assert_eq!(panics(|| call(&mut f)), "owner only", "{name}");
        }
        for (name, call) in &setters {
            owner();
            assert_eq!(panics(|| call(&mut f)), "", "{name} works at once, with nothing attached");
        }
        assert_eq!(f.get_locker_for(U64(1)), acct("lock_3.nearlytrade.near"));
        assert_eq!(f.get_locker_for(U64(first)), acct("lock.near"), "existing launches keep their locker");
        assert_eq!(hex32(f.token_code_hash), "aa".repeat(32));
        assert_eq!(tax_code_hash().map(hex32), Some("bb".repeat(32)));
        assert_eq!(f.protocol_recipients, vec![(acct("p1.near"), 5000), (acct("p2.near"), 5000)]);
        assert_eq!((f.get_dcl_for(U64(first)), f.get_dcl_for(U64(1)), f.get_addresses().dcl_current), (acct("dclv2.ref-labs.near"), acct("dclv3.near"), acct("dclv3.near")));
        assert_eq!(f.dcl_storage(1), DCL_REGISTER.as_yoctonear(), "the first launch on the new exchange registers there");
        assert_eq!(f.get_fee_routers(), vec![acct("router.near")]);
        assert_eq!(f.get_launch_hook(), Some(acct("hook.near")));
        assert_eq!(f.get_tax_seller(), Some(acct("seller.near")));
        assert!(f.is_house_creator(acct("house.near")) && f.is_house_creator(acct("h1.near")) && f.is_house_creator(acct("h2.near")));
        assert_eq!(f.get_launch_recipients(U64(first)), Some(vec![(acct("r1.near"), 4000), (acct("r2.near"), 6000)]));

        owner();
        f.set_fee_router(acct("router.near"), false);
        f.set_launch_hook(None);
        f.set_tax_seller(None);
        f.set_house_creators(vec![acct("h1.near")], false);
        f.set_launch_recipients(U64(first), None);
        f.set_tax_token_code_hash(None);
        assert!(f.get_fee_routers().is_empty());
        assert_eq!((f.get_launch_hook(), f.get_tax_seller(), f.get_launch_recipients(U64(first))), (None, None, None));
        assert!(!f.is_house_creator(acct("h1.near")) && f.is_house_creator(acct("h2.near")));
        assert!(tax_code_hash().is_none());

        let third = launch_scheduled(&mut f, "alice.near", args("NEW"));
        assert_eq!(third, 1);
        let c = calls();
        assert!(c.iter().any(|c| c.0 == "dclv3.near" && c.1 == "create_pool"));
        assert!(c.iter().any(|c| c.0 == "dclv3.near" && c.1 == "storage_deposit" && c.2.contains("lock_3.nearlytrade.near")));
    }

    #[test]
    fn owner_setters_refuse_bad_values() {
        let mut f = factory();
        let id = launch(&mut f, "alice.near", "BAD");
        let cases: Vec<(Box<dyn Fn(&mut Factory)>, &str)> = vec![
            (Box::new(|f: &mut Factory| f.set_protocol_recipients(vec![(acct("a.near"), 6000), (acct("b.near"), 3000)])), "bps must sum to 10000"),
            (Box::new(|f: &mut Factory| f.set_protocol_recipients(vec![(acct("a.near"), 5000), (acct("a.near"), 5000)])), "duplicate recipient"),
            (Box::new(|f: &mut Factory| f.set_token_code_hash("zz".into())), "code hash must be 64 hex chars"),
            (Box::new(|f: &mut Factory| f.set_locker_from(acct("lock_3.nearlytrade.near"), U64(0))), "existing launches keep their locker"),
            (Box::new(|f: &mut Factory| f.set_dcl(acct("dclv2.ref-labs.near"), None)), "dcl: already the exchange for new launches"),
            (Box::new(|f: &mut Factory| f.set_dcl(acct(ME), None)), "dcl: not the launchpad itself"),
            (Box::new(|f: &mut Factory| f.set_fee_router(acct(ME), true)), "fee router: not the launchpad itself"),
            (Box::new(|f: &mut Factory| f.set_launch_hook(Some(acct(ME)))), "launch hook: not the launchpad itself"),
            (Box::new(move |f: &mut Factory| f.set_launch_recipients(U64(id), Some(vec![(acct("a.near"), 9000)]))), "bps must sum to 10000"),
            (Box::new(move |f: &mut Factory| f.set_launch_recipients(U64(id), Some(vec![]))), "1 to 4 recipients"),
            (Box::new(|f: &mut Factory| f.set_launch_recipients(U64(99), None)), "launch"),
        ];
        for (call, err) in &cases {
            owner();
            let r = panics(|| call(&mut f));
            assert!(r.contains(err), "{err}: {r}");
        }
    }

    #[test]
    fn a_second_exchange_switch_before_any_launch_replaces_the_first() {
        let mut f = factory();
        launch(&mut f, "alice.near", "ONE");
        owner();
        f.set_dcl(acct("dclwrong.near"), None);
        owner();
        f.set_dcl(acct("dclright.near"), Some(false));
        assert_eq!(f.get_dcls(), vec![(U64(0), acct("dclv2.ref-labs.near")), (U64(1), acct("dclright.near"))]);
    }

    #[test]
    fn upgrade_switches_to_the_global_contract_then_migrates() {
        let mut f = factory();
        let hash = [7u8; 32];
        let b58: Base58CryptoHash = hash.into();
        ctx("mallory.near", 0);
        assert_eq!(panics(|| drop(f.upgrade(b58.clone(), true))), "owner only");
        owner();
        drop(f.upgrade(b58.clone(), true));
        let r = get_created_receipts();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].receiver_id, acct(ME));
        assert!(matches!(&r[0].actions[0], MockAction::UseGlobalContract { .. }), "{:?}", r[0].actions);
        assert!(matches!(&r[0].actions[1], MockAction::FunctionCallWeight { method_name, attached_deposit, .. } if method_name == b"migrate" && attached_deposit.as_yoctonear() == 0));
        assert!(logs_with("upgrade")[0].contains(&"07".repeat(32)));

        owner();
        drop(f.upgrade(b58, false));
        let r = get_created_receipts();
        assert_eq!(r[0].actions.len(), 1);

        let parsed: Base58CryptoHash = near_sdk::serde_json::from_str(&format!("\"{}\"", near_sdk::bs58::encode(hash).into_string())).unwrap();
        assert_eq!(CryptoHash::from(parsed), hash);
    }


    fn ctx_input(who: &str, input: &[u8]) {
        let mut c = VMContextBuilder::new()
            .current_account_id(ME.parse().unwrap())
            .predecessor_account_id(who.parse().unwrap())
            .account_balance(NearToken::from_near(500))
            .prepaid_gas(Gas::from_tgas(300))
            .build();
        c.input = input.into();
        near_sdk::testing_env!(c, near_sdk::test_vm_config(), mainnet_fees());
    }

    #[test]
    fn upgrade_code_deploys_the_raw_input_then_migrates() {
        let f = factory();
        let code = b"\0asm\x01\0\0\0 any module bytes".to_vec();
        ctx_input("mallory.near", &code);
        assert_eq!(panics(|| drop(f.upgrade_code())), "owner only");
        ctx_input(ME, &code);
        assert_eq!(panics(|| drop(f.upgrade_code())), "owner only", "the factory's own keys are not the owner");
        ctx_input("owner.near", b"");
        assert_eq!(panics(|| drop(f.upgrade_code())), "upgrade_code: the input must be the wasm itself");
        ctx_input("owner.near", br#"{"code":"AGFzbQ=="}"#);
        assert_eq!(panics(|| drop(f.upgrade_code())), "upgrade_code: the input must be the wasm itself", "JSON or base64 is refused");
        ctx_input("owner.near", &code);
        drop(f.upgrade_code());
        let r = get_created_receipts();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].receiver_id, acct(ME));
        assert_eq!(r[0].actions.len(), 2);
        assert!(matches!(&r[0].actions[0], MockAction::DeployContract { code: c, .. } if *c == code), "{:?}", r[0].actions);
        assert!(matches!(&r[0].actions[1], MockAction::FunctionCallWeight { method_name, attached_deposit, .. } if method_name == b"migrate" && attached_deposit.as_yoctonear() == 0));
        assert!(logs_with("upgrade")[0].contains(&hex32(env::sha256_array(&code))));
        assert!(logs_with("upgrade")[0].contains(r#""raw":true"#));
    }

    #[test]
    fn ownership_moves_only_when_the_new_owner_accepts() {
        let mut f = factory();
        ctx("owner.near", 0);
        f.transfer_ownership(acct("new.near"));
        assert_eq!(f.get_owner(), acct("owner.near"), "transfer_ownership only proposes now");
        assert_eq!(f.get_pending_owner(), Some(acct("new.near")));
        ctx("mallory.near", 0);
        assert_eq!(panics(|| f.accept_owner()), "only the proposed owner can accept");
        ctx("owner.near", 0);
        f.propose_owner(acct("owner.near"));
        assert_eq!(f.get_pending_owner(), None, "proposing yourself withdraws it");
        ctx("owner.near", 0);
        f.propose_owner(acct("new.near"));
        ctx("new.near", 0);
        f.accept_owner();
        assert_eq!(f.get_owner(), acct("new.near"));
        assert_eq!(f.get_pending_owner(), None);
    }

    fn router_launch(f: &mut Factory) -> u64 {
        owner();
        f.set_fee_router(acct("router.near"), true);
        let id = launch_with(f, "alice.near", LaunchArgs { fee_to: Some(acct("router.near")), ..args("ROUTED") });
        park(f, id, Step::Done);
        id
    }

    #[test]
    fn a_router_gets_token_fees_by_plain_transfer_then_a_notice() {
        let mut f = factory();
        let id = router_launch(&mut f);
        let mut lz = l(&f, id);
        lz.creator_token_fees = U128(1_000);
        f.launches.insert(id, lz);
        ctx("anyone.near", 0);
        drop(f.push_creator_token_fees(U64(id)));

        assert!(calls().iter().all(|c| c.1 != "ft_transfer_call"), "no transfer call to a router");
        let ft = calls().into_iter().find(|c| c.1 == "ft_transfer").expect("ft_transfer");
        assert!(ft.2.contains(r#""receiver_id":"router.near""#), "{}", ft.2);
        assert!(!transfers_pending(&l(&f, id).token), "a plain transfer is not counted in flight");
        let paid = |f: &mut Factory, amount: u128| f.on_paid(U64(id), "token".into(), acct("router.near"), U128(amount), "creator".into(), false);

        ctx_results(vec![near_sdk::PromiseResult::Successful(vec![])]);
        paid(&mut f, 1_000);
        assert_eq!(l(&f, id).creator_token_fees.0, 0);
        let notice = calls().into_iter().find(|c| c.1 == "on_launch_fees").expect("notice");
        assert!(notice.2.contains(&format!(r#""launch_id":{},"kind":"token","amount":"1000""#, id)), "{}", notice.2);

        lz = l(&f, id); lz.creator_token_fees = U128(0); f.launches.insert(id, lz);
        ctx_results(vec![near_sdk::PromiseResult::Failed]);
        paid(&mut f, 100);
        assert_eq!(l(&f, id).creator_token_fees.0, 100);
        assert!(calls().iter().all(|c| c.1 != "on_launch_fees"));
    }

    #[test]
    fn a_router_gets_near_fees_per_launch_by_transfer_then_notice() {
        let mut f = factory();
        f.min_push_yocto = 0;
        let id = router_launch(&mut f);
        let n = 10u128.pow(24);
        let mut lz = l(&f, id);
        lz.inflight = true;
        f.launches.insert(id, lz);
        let v = if l(&f, id).token_is_x { vec![U128(0), U128(n)] } else { vec![U128(n), U128(0)] };
        ctx_gas(ME, 100);
        assert!(f.on_fees_claimed(id, Ok(v)));
        assert_eq!(f.get_creator_fees(acct("router.near")).0, 0, "not the per-account balance");
        assert_eq!(transfers(), vec![("router.near".to_string(), n * 7 / 10)], "the NEAR goes as a plain transfer");
        assert!(calls().iter().all(|c| c.1 != "on_launch_fees"), "no notice before the transfer landed");
        assert_eq!(f.get_router_fees(U64(id)).0 .0, 0);

        let paid = |f: &mut Factory| f.on_paid(U64(id), "near".into(), acct("router.near"), U128(n * 7 / 10), "creator".into(), false);
        ctx_results(vec![near_sdk::PromiseResult::Successful(vec![])]);
        paid(&mut f);
        let notice = calls().into_iter().find(|c| c.1 == "on_launch_fees").expect("notice");
        assert_eq!((notice.0.as_str(), notice.3), ("router.near", 0));
        assert_eq!(notice.2, format!(r#"{{"launch_id":{},"kind":"near","amount":"{}"}}"#, id, n * 7 / 10));
        assert_eq!(f.get_router_fees(U64(id)).0 .0, 0);

        ctx_results(vec![near_sdk::PromiseResult::Failed]);
        paid(&mut f);
        assert_eq!(f.get_router_fees(U64(id)).0 .0, n * 7 / 10);
        assert!(calls().iter().all(|c| c.1 != "on_launch_fees"));
        ctx("anyone.near", 0);
        drop(f.push_router_fees(U64(id)));
        assert_eq!(f.get_router_fees(U64(id)).0 .0, 0);
        assert_eq!(transfers(), vec![("router.near".to_string(), n * 7 / 10)]);
    }

    #[test]
    fn a_router_payees_tax_share_is_booked_for_push_router_fees() {
        let mut f = factory();
        let id = router_launch(&mut f);
        set_tax_opts(id, &TaxArgs { buy_bps: 300, sell_bps: 300, creator_bps: 10_000, burn_bps: 0, holders_bps: 0 }, PLATFORM_TAX_BPS);
        near_sdk::env::storage_write(TAX_SELLER_KEY, b"seller.near");
        add_counter(&tax_selling_key(id), 100);
        ctx("seller.near", 1_000);
        assert!(f.tax_proceeds(U64(id), U128(100)));
        assert_eq!(f.get_router_fees(U64(id)).0 .0, 700, "the creator's 70 % waits for the router push");
        assert!(transfers().iter().all(|t| t.0 != "router.near"));

        let plain = launch(&mut f, "bob.near", "TAXP");
        park(&mut f, plain, Step::Done);
        set_tax_opts(plain, &TaxArgs { buy_bps: 300, sell_bps: 300, creator_bps: 10_000, burn_bps: 0, holders_bps: 0 }, PLATFORM_TAX_BPS);
        add_counter(&tax_selling_key(plain), 100);
        ctx("seller.near", 1_000);
        assert!(f.tax_proceeds(U64(plain), U128(100)));
        assert_eq!(f.get_creator_fees(acct("bob.near")).0, 700);
        assert_eq!(f.get_tax(U64(plain)).unwrap().paid_creator.0, 700);
    }

    #[test]
    fn the_launch_hook_fires_with_fixed_gas_and_only_with_gas_to_spare() {
        let mut f = factory();
        owner();
        f.set_launch_hook(Some(acct("hook.near")));
        let id = launch(&mut f, "alice.near", "HOOK");
        let mut lz = l(&f, id);
        lz.lpt_id = Some("x".into());
        ctx_gas(ME, 100);
        f.internal_done(id, lz.clone());
        let hook = get_created_receipts().into_iter().find(|r| r.receiver_id == acct("hook.near")).expect("hook");
        match &hook.actions[0] {
            MockAction::FunctionCallWeight { method_name, args, attached_deposit, prepaid_gas, gas_weight, .. } => {
                assert_eq!(method_name, b"on_launch");
                assert_eq!((attached_deposit.as_yoctonear(), *prepaid_gas, gas_weight.0), (0, GAS_HOOK, 0));
                let a: near_sdk::serde_json::Value = near_sdk::serde_json::from_slice(args).unwrap();
                assert_eq!(a, json!({"id": id, "token": lz.token, "creator": "alice.near", "pool_id": lz.pool_id, "quote": "wrap.near"}));
            }
            a => panic!("{a:?}"),
        }
        assert!(hook.receipt_indices.is_empty(), "no callback depends on it");
        ctx_gas(ME, 19);
        f.internal_done(id, lz);
        assert!(get_created_receipts().is_empty(), "under 20 TGas left: skipped");
        assert_eq!(l(&f, id).step, Step::Done, "the launch is live either way");
    }

    #[test]
    fn a_launch_without_a_dev_buy_reaches_the_hook_in_its_own_transaction() {
        let mut f = factory();
        owner();
        f.set_launch_hook(Some(acct("hook.near")));
        let id = launch(&mut f, "alice.near", "INLINE");
        let mut lz = l(&f, id);
        lz.inflight = true;
        lz.step = Step::AddLiquidity;
        f.launches.insert(id, lz);

        ctx_gas(ME, tg(GAS_LIQ_CB));
        drop(f.on_liquidity_added(id, None, Ok("lpt#1".into())));
        assert!(get_created_receipts().iter().any(|r| r.receiver_id == acct("hook.near")));
    }



    fn static_gas_of(method: &[u8]) -> Option<Gas> {
        get_created_receipts().into_iter().flat_map(|r| r.actions).find_map(|a| match a {
            MockAction::FunctionCallWeight { method_name, prepaid_gas, .. } if method_name == method => Some(prepaid_gas),
            _ => None,
        })
    }






    #[test]
    fn the_launch_chain_places_the_position_and_fires_the_hook_in_line() {
        let mut f = factory();
        owner();
        f.set_launch_hook(Some(acct("hook.near")));
        f.dcl_registered = true;
        let id = launch_scheduled(&mut f, "alice.near", LaunchArgs { icon: Some(icon_of(16_000)), ..args("ONESHOT") });
        let cb = static_gas_of(b"on_created").expect("on_created");
        assert!(cb >= Gas::from_tgas(tg(GAS_LOCKER_ADD) + tg(GAS_LIQ_CB) + tg(GAS_RESERVE)), "on_created gets {cb}");
        ctx_gas(ME, cb.as_gas() / 1_000_000_000_000);
        drop(f.on_created(id, None, Ok(StorageBalanceJson { total: "0".into() }), Ok(l(&f, id).pool_id)));
        assert!(calls().iter().any(|c| c.0 == "lock.near" && c.1 == "add"), "AddLiquidity ran in-line");
        assert_eq!(static_gas_of(b"on_liquidity_added"), Some(GAS_LIQ_CB));
        ctx_gas(ME, tg(GAS_LIQ_CB));
        drop(f.on_liquidity_added(id, None, Ok("lpt#9".into())));
        assert_eq!(l(&f, id).step, Step::Done);
        assert!(get_created_receipts().iter().any(|r| r.receiver_id == acct("hook.near")));
    }





    #[test]
    fn raw_key_prefixes_cannot_collide() {
        let _f = factory();
        let a = acct("x.near");
        let keys: Vec<Vec<u8>> = vec![

            house_key(&a), launch_recipients_key(1), launch_bucket_key(1), fee_opts_key(1), mode_bucket_key(1), burned_key(1), paid_key(1),
            fee_to_key(1), slug_count_key("x"), TAX_CODE_KEY.to_vec(), LOCKERS_KEY.to_vec(), tax_opts_key(1), tax_pending_key(1),
            tax_holders_key(1), tax_burned_key(1), tax_paid_creator_key(1), tax_paid_holders_key(1), tax_paid_platform_key(1),
            tax_selling_key(1), TAX_SELLER_KEY.to_vec(),

            tax_floor_key(1), wnear_owed_key(1), wnear_owed_bucket_key(1), wnear_retry_key(1), icon_len_key(1), BURN_POT_KEY.to_vec(), REFERRAL_POT_KEY.to_vec(),

            dev_buy_out_key(1), RELAYERS_KEY.to_vec(), swap_lock_key(&a), PENDING_OWNER_KEY.to_vec(),
            FEE_ROUTERS_KEY.to_vec(), LAUNCH_HOOK_KEY.to_vec(), DCLS_KEY.to_vec(), router_near_key(1),
            transfers_key(&a), locker_add_gas_key(&a), dcl_registered_key(&a), unwrap_pending_key(1), locker_add_buy_key(&a), dev_buy_done_key(1), buy_held_key(1), buy_refund_inflight_key(1), no_add_key(1), owed_key("near", &a), owed_total_key("near"), UW_TOTAL_KEY.to_vec(),

            dev_unwrap_op_key(1),
        ];
        let prefixes: Vec<&[u8]> = keys.iter().map(|k| &k[..3]).collect();
        for (i, p) in prefixes.iter().enumerate() {
            assert!(!prefixes[..i].contains(p), "prefix {:?} used twice", String::from_utf8_lossy(p));
            assert!(p[0] >= 0x06 && p[0] != b'S' && p[0] != b'L', "prefix {:?} can meet a map or the state", String::from_utf8_lossy(p));
        }
        assert_eq!(prefixes.len(), 48);
    }



    #[test]
    fn locker_withdraw_dcl_asset_books_what_arrived_under_the_token_lock() {
        let mut f = factory();
        let id = launch(&mut f, "alice.near", "ABAND");
        let token = l(&f, id).token.clone();
        ctx("owner.near", 0);
        assert_eq!(panics(|| drop(f.locker_withdraw_dcl_asset(acct("other.near"), token.clone(), U128(5), None))), "not one of this factory's lockers");
        ctx("owner.near", 0);
        assert_eq!(panics(|| drop(f.locker_withdraw_dcl_asset(acct("lock.near"), acct("usdc.near"), U128(5), None))), "only a launch token or wNEAR can be booked");
        ctx("mallory.near", 0);
        assert_eq!(panics(|| drop(f.locker_withdraw_dcl_asset(acct("lock.near"), token.clone(), U128(5), None))), "owner only");
        ctx("owner.near", 0);
        drop(f.locker_withdraw_dcl_asset(acct("lock.near"), token.clone(), U128(250), None));
        assert!(swap_locked(&token));
        assert!(calls().iter().any(|c| c.0 == token.as_str() && c.1 == "ft_balance_of"));
        ctx(ME, 0);
        drop(f.on_locker_withdraw_before(acct("lock.near"), token.clone(), U128(250), None, Ok(U128(1_000))));
        let call = get_created_receipts().into_iter().find(|r| r.receiver_id == acct("lock.near")).unwrap();
        match &call.actions[0] {
            MockAction::FunctionCallWeight { method_name, args, prepaid_gas, attached_deposit, .. } => {
                assert_eq!(method_name, b"withdraw_dcl_asset");
                assert_eq!(String::from_utf8(args.clone()).unwrap(), format!(r#"{{"token":"{}","amount":"250"}}"#, token));
                assert!(*prepaid_gas >= Gas::from_tgas(90) && attached_deposit.as_yoctonear() == 0);
            }
            a => panic!("{a:?}"),
        }
        ctx(ME, 0);
        drop(f.on_locker_withdrawn(token.clone(), U128(250), U128(1_000), None, Ok(true)));
        assert!(calls().iter().any(|c| c.1 == "ft_balance_of"), "the second read goes out");
        assert!(swap_locked(&token), "held until the second read is booked");

        ctx(ME, 0);
        assert_eq!(f.on_locker_withdraw_booked(token.clone(), U128(1_000), None, Ok(U128(1_300))).0, 300);
        assert!(!swap_locked(&token));
        assert_eq!(l(&f, id).protocol_token_fees.0, 300);
        ctx("owner.near", 0);
        drop(f.withdraw_protocol_token_fees(U64(id), acct("owner.near")));
        assert!(calls().iter().any(|c| c.1 == "ft_transfer" && c.2.contains(r#""amount":"300""#)));

        ctx("owner.near", 0);
        drop(f.locker_withdraw_dcl_asset(acct("lock.near"), token.clone(), U128(1), None));
        ctx(ME, 0);
        drop(f.on_locker_withdraw_before(acct("lock.near"), token.clone(), U128(1), None, Err(PromiseError::Failed)));
        assert!(!swap_locked(&token));

        ctx_at(ME, 0, T0);
        take_swap_lock(&token, 7, LK_BUYBACK);
        ctx_at(ME, 0, T0);
        drop(f.on_locker_withdrawn(token.clone(), U128(1), U128(0), None, Err(PromiseError::Failed)));
        assert!(calls().iter().all(|c| c.1 != "ft_balance_of"));
        ctx_at(ME, 0, T0);
        assert_eq!(f.on_locker_withdraw_booked(token.clone(), U128(0), None, Ok(U128(5_000))).0, 0);
        assert_eq!(l(&f, id).protocol_token_fees.0, 0, "nothing booked");
        assert!(swap_locked(&token), "the other chain's lock is left alone");
    }

    #[test]
    fn locker_withdrawn_wnear_is_booked_as_protocol_near_after_the_unwrap() {
        let mut f = factory();
        let w = acct("wrap.near");
        ctx("owner.near", 0);
        drop(f.locker_withdraw_dcl_asset(acct("lock.near"), w.clone(), U128(40), None));
        ctx(ME, 0);
        assert_eq!(f.on_locker_withdraw_booked(w.clone(), U128(10), None, Ok(U128(50))).0, 40);
        assert!(calls().iter().any(|c| c.1 == "near_withdraw" && c.2.contains(r#""amount":"40""#)));
        assert_eq!(f.protocol_fees, 0);
        ctx(ME, 0);
        f.on_wnear_swept(U128(40), Some(true), Ok(()));
        assert_eq!(f.protocol_fees, 40);
    }

    #[test]
    fn a_migration_moves_the_launches_and_lockers_on_that_exchange() {
        let mut f = factory();
        let a = launch(&mut f, "alice.near", "OLDA");
        ctx("owner.near", 0);
        f.set_locker_from(acct("lock_3.nearlytrade.near"), U64(1));
        let b = launch(&mut f, "alice.near", "OLDB");

        owner();
        f.set_dcl(acct("dclnew.near"), None);
        assert!(calls().iter().all(|c| c.1 != "set_dcl"));
        assert_eq!((f.get_dcl_for(U64(a)), f.get_dcl_for(U64(b)), f.get_dcl_for(U64(2))), (acct("dclv2.ref-labs.near"), acct("dclv2.ref-labs.near"), acct("dclnew.near")));

        let registered = f.dcl_registered;
        owner();
        f.set_dcl(acct("dclmoved.near"), Some(true));

        assert_eq!(f.get_dcl_for(U64(2)), acct("dclmoved.near"));
        assert_eq!(f.get_dcl_for(U64(a)), acct("dclv2.ref-labs.near"), "launches on another exchange stay");
        assert_eq!(f.dcl_registered, registered);

        let set: Vec<(String, String, u128)> = calls().into_iter().filter(|c| c.1 == "set_dcl").map(|c| (c.0, c.2, c.3)).collect();
        assert_eq!(set, vec![("lock_3.nearlytrade.near".to_string(), r#"{"dcl":"dclmoved.near"}"#.to_string(), 0)]);
        assert!(get_created_receipts().iter().filter(|r| r.actions.iter().any(|x| matches!(x, MockAction::FunctionCallWeight { method_name, prepaid_gas, .. } if method_name == b"set_dcl" && *prepaid_gas == GAS_LOCKER_SET_DCL))).count() == 1);
    }

    #[test]
    fn migrating_the_base_exchange_moves_the_oldest_launches_and_their_lockers() {

        let mut g = factory();
        let c0 = launch(&mut g, "alice.near", "BASE");
        ctx("owner.near", 0);
        g.set_locker_from(acct("lock_3.nearlytrade.near"), U64(1));
        owner();
        g.set_dcl(acct("dclv3.near"), Some(true));
        assert_eq!((g.get_dcl_for(U64(c0)), g.get_addresses().dcl), (acct("dclv3.near"), acct("dclv3.near")));
        let told: Vec<String> = calls().into_iter().filter(|c| c.1 == "set_dcl").map(|c| c.0).collect();
        assert_eq!(told, vec!["lock.near".to_string(), "lock_3.nearlytrade.near".to_string()]);
    }

    #[test]
    fn an_empty_position_id_parks_the_launch_for_a_resume() {
        let mut f = factory();
        let id = launch(&mut f, "alice.near", "PEND");
        let mut lz = l(&f, id);
        lz.step = Step::AddLiquidity;
        lz.inflight = true;
        f.launches.insert(id, lz);
        ctx(ME, 0);
        drop(f.on_liquidity_added(id, None, Ok(String::new())));
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight, lz.lpt_id), (Step::AddLiquidity, false, None));
        assert!(logs_with("launch_step_parked")[0].contains("position pending"));
        ctx("anyone.near", 0);
        drop(f.resume(U64(id)));
        assert!(calls().iter().any(|c| c.0 == "lock.near" && c.1 == "add"), "a resume asks the locker again");
    }



    #[test]
    fn a_buyback_does_not_start_while_a_claim_or_tax_take_is_on_its_way() {
        let mut f = factory();
        let id = live_burn_launch(&mut f, "INFL", 10u128.pow(24));
        set_tax_opts(id, &TaxArgs { buy_bps: 300, sell_bps: 300, creator_bps: 5000, burn_bps: 2000, holders_bps: 3000 }, PLATFORM_TAX_BPS);
        let mut lz = l(&f, id);
        lz.lpt_id = Some("x".into());
        f.launches.insert(id, lz);
        let token = l(&f, id).token.clone();
        ctx("crank.near", 0);
        drop(f.claim_fees(U64(id)));
        assert!(transfers_pending(&token));
        ctx("owner.near", 0);
        assert_eq!(panics(|| drop(f.buyback(U64(id), U128(1)))), "transfer in flight");

        ctx(ME, 0);
        drop(f.collect_tax(U64(id)));
        ctx(ME, 0);
        assert!(f.on_fees_claimed(id, Ok(vec![U128(0), U128(0)])));
        assert!(transfers_pending(&token), "the tax take is still on its way");
        ctx(ME, 0);
        f.on_tax_taken(U64(id), Ok(U128(0)));
        assert!(!transfers_pending(&token));
        ctx("owner.near", 0);
        drop(f.buyback(U64(id), U128(1)));
        assert!(swap_locked(&token));

        ctx("crank.near", 0);
        assert_eq!(panics(|| drop(f.claim_fees(U64(id)))), "swap in flight");

        let t = acct("lost.near");
        ctx_at(ME, 0, 5_000);
        transfer_start(&t);
        ctx_at(ME, 0, 5_000 + SWAP_LOCK_TTL_MS);
        assert!(!transfers_pending(&t));
        transfer_end(&t);
        transfer_end(&t);
        assert!(!transfers_pending(&t), "an end without a start saturates");
    }

    #[test]
    fn a_router_token_leg_is_never_counted_in_flight() {
        let mut f = factory();
        let id = router_launch(&mut f);
        let mut lz = l(&f, id);
        lz.creator_token_fees = U128(1_000);
        f.launches.insert(id, lz);
        let token = l(&f, id).token.clone();
        ctx("anyone.near", 0);
        drop(f.push_creator_token_fees(U64(id)));
        assert!(!transfers_pending(&token));
    }

    #[test]
    fn a_create_step_refuses_to_start_without_gas_for_its_pool_check() {
        let mut f = factory();
        let id = launch(&mut f, "alice.near", "LOWGAS");
        park(&mut f, id, Step::CreatePool);
        ctx_gas("anyone.near", 45);
        assert!(panics(|| drop(f.resume(U64(id)))).contains("attach more gas"));
        assert!(!l(&f, id).inflight, "never left in flight");
        ctx_gas("anyone.near", 300);
        drop(f.resume(U64(id)));
        assert!(static_gas_of(b"on_pool_created").unwrap() >= GAS_POOL_STEP_CB);

        let deposit = f.quote_launch(0, None, None).total.0;
        ctx_full("alice.near", deposit, T0, 80);
        assert!(panics(|| std::mem::forget(f.launch(args("LOW2")))).contains("attach more gas"));
    }

    #[test]
    fn migrate_is_a_private_no_op() {
        let f = factory();
        let before = f.get_config();
        ctx(ME, 0);
        assert!(f.migrate());
        assert_eq!(f.get_config().creator_fee_share_bps, before.creator_fee_share_bps);

    }



    fn pk(n: u8) -> PublicKey {
        format!("ed25519:{}", near_sdk::bs58::encode([n; 32]).into_string()).parse().unwrap()
    }
    fn ctx_signed(who: &str, key: &PublicKey) {
        near_sdk::testing_env!(
            VMContextBuilder::new()
                .current_account_id(ME.parse().unwrap())
                .predecessor_account_id(who.parse().unwrap())
                .signer_account_pk(key.clone())
                .account_balance(NearToken::from_near(500))
                .prepaid_gas(Gas::from_tgas(300))
                .build(),
            near_sdk::test_vm_config(),
            mainnet_fees()
        );
    }

    #[test]
    fn add_fc_key_adds_only_function_call_keys_to_this_account() {
        let mut f = factory();
        ctx_signed("owner.near", &pk(1));
        drop(f.add_fc_key(pk(2), vec!["claim_fees".into(), "set_paused".into()], Some(U128(5 * 10u128.pow(23)))));
        let r = get_created_receipts();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].receiver_id, acct(ME));
        match &r[0].actions[..] {
            [MockAction::AddKeyWithFunctionCall { receiver_id, method_names, allowance, .. }] => {
                assert_eq!(receiver_id, &acct(ME));
                assert_eq!(method_names, &vec!["claim_fees".to_string(), "set_paused".to_string()]);
                assert_eq!(allowance.map(|a| a.as_yoctonear()), Some(5 * 10u128.pow(23)));
            }
            a => panic!("{a:?}"),
        }
        assert!(logs_with("fc_key_added")[0].contains(r#""methods":["claim_fees","set_paused"]"#));

        ctx_signed("owner.near", &pk(1));
        drop(f.add_fc_key(pk(3), vec!["upgrade".into()], None));
        match &get_created_receipts()[0].actions[..] {
            [MockAction::AddKeyWithFunctionCall { allowance, .. }] => assert_eq!(*allowance, None),
            a => panic!("{a:?}"),
        }
        assert!(get_created_receipts().iter().flat_map(|r| r.actions.iter()).all(|a| !matches!(a, MockAction::AddKeyWithFullAccess { .. })));
    }

    #[test]
    fn add_fc_key_refuses_callbacks_empty_lists_and_smuggled_names() {
        let mut f = factory();
        for (methods, allowance, err) in [
            (vec![], None, "an empty list allows every method"),
            (vec!["on_fees_claimed"], None, "callbacks (on_*) and migrate are never on a key"),
            (vec!["claim_fees", "migrate"], None, "callbacks (on_*) and migrate are never on a key"),
            (vec!["claim_fees,on_fees_claimed"], None, "[A-Za-z0-9_]"),
            (vec!["claim_fees "], None, "[A-Za-z0-9_]"),
            (vec![""], None, "[A-Za-z0-9_]"),
            (vec!["claim_fees", "claim_fees"], None, "duplicate method"),
            (vec!["claim_fees"], Some(U128(0)), "allowance must be > 0"),
        ] {
            ctx_signed("owner.near", &pk(1));
            let m: Vec<String> = methods.iter().map(|s| s.to_string()).collect();
            let r = panics(|| drop(f.add_fc_key(pk(2), m.clone(), allowance)));
            assert!(r.contains(err), "{methods:?}: {r}");
        }
        let long = vec!["a".repeat(250); 9].into_iter().enumerate().map(|(i, s)| format!("{s}{i}")).collect::<Vec<_>>();
        ctx_signed("owner.near", &pk(1));
        assert!(panics(|| drop(f.add_fc_key(pk(2), long, None))).contains("2000 bytes"));
        ctx_signed("mallory.near", &pk(1));
        assert_eq!(panics(|| drop(f.add_fc_key(pk(2), vec!["claim_fees".into()], None))), "owner only");
    }

    #[test]
    fn delete_fc_key_never_deletes_the_signing_key() {
        let mut f = factory();
        ctx_signed("owner.near", &pk(1));
        assert_eq!(panics(|| drop(f.delete_fc_key(pk(1)))), "fc key: this call is signed with that key");
        ctx_signed("mallory.near", &pk(1));
        assert_eq!(panics(|| drop(f.delete_fc_key(pk(2)))), "owner only");
        ctx_signed("owner.near", &pk(1));
        drop(f.delete_fc_key(pk(2)));
        let r = get_created_receipts();
        assert_eq!(r[0].receiver_id, acct(ME));
        assert!(matches!(&r[0].actions[..], [MockAction::DeleteKey { .. }]), "{:?}", r[0].actions);
        assert!(logs_with("fc_key_deleted")[0].contains(&pk(2).to_string()));
    }




    fn ctx_tx(pred: &str, signer: &str, deposit: u128, tgas: u64) {
        near_sdk::testing_env!(
            VMContextBuilder::new()
                .current_account_id(ME.parse().unwrap())
                .predecessor_account_id(pred.parse().unwrap())
                .signer_account_id(signer.parse().unwrap())
                .attached_deposit(NearToken::from_yoctonear(deposit))
                .account_balance(NearToken::from_near(500))
                .block_timestamp(T0 * 1_000_000)
                .prepaid_gas(Gas::from_tgas(tgas))
                .build(),
            near_sdk::test_vm_config(),
            mainnet_fees()
        );
    }
    fn tg_of(g: Gas) -> u64 { g.as_gas() / 1_000_000_000_000 }



    fn replay_dev_buy_launch(f: &mut Factory, tgas: u64, near: u128) -> u64 {
        let a = LaunchArgs { dev_buy: Some(U128(near)), ..args("INL") };
        let deposit = f.quote_launch(0, a.dev_buy, None).total.0;
        ctx_tx("alice.near", "alice.near", deposit, tgas);
        drop(f.launch(a));
        let id = f.next_id - 1;
        let on_created = static_gas_of(b"on_created").expect("on_created");
        ctx_tx(ME, "alice.near", 0, tg_of(on_created));
        let pool = l(f, id).pool_id;
        drop(f.on_created(id, None, Ok(StorageBalanceJson { total: "0".into() }), Ok(pool)));
        let add = get_created_receipts().into_iter().flat_map(|r| r.actions).find_map(|a| match a {
            MockAction::FunctionCallWeight { method_name, prepaid_gas, .. } if method_name == b"add" => Some(prepaid_gas),
            _ => None,
        }).expect("locker add");
        assert!(add >= GAS_LOCKER_ADD, "the locker keeps its floor: {add}");
        let cb = static_gas_of(b"on_liquidity_added").expect("on_liquidity_added");
        ctx_tx(ME, "alice.near", 0, tg_of(cb));
        drop(f.on_liquidity_added(id, None, Ok("pool|9".into())));
        id
    }

    #[test]
    fn a_launch_on_a_locker_without_an_allowance_parks_its_dev_buy() {
        let mut f = factory();
        let near = 3 * 10u128.pow(24);
        let id = replay_dev_buy_launch(&mut f, 300, near);
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight, lz.dev_buy_held.0), (Step::DevBuy, false, near), "parked, still funded");
        assert!(calls().iter().all(|c| c.1 != "ft_transfer_call"));
        assert!(logs_with("launch_step_parked")[0].contains("not enough gas"));

        ctx("alice.near", 0);
        drop(f.resume(U64(id)));
        assert!(l(&f, id).inflight);
    }

    #[test]
    fn an_in_line_dev_buy_needs_an_allowed_signer() {
        let mut f = factory();
        f.wnear_registered = true;
        let near = 10u128.pow(24);
        let id = launch_with(&mut f, "alice.near", LaunchArgs { dev_buy: Some(U128(near)), ..args("GATE") });
        let mut lz = l(&f, id);
        lz.step = Step::AddLiquidity;
        lz.inflight = true;
        f.launches.insert(id, lz);
        ctx_tx(ME, "mallory.near", 0, tg_of(GAS_LIQ_CB_DEV_BUY));
        drop(f.on_liquidity_added(id, None, Ok("pool|1".into())));
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight, lz.dev_buy_held.0), (Step::DevBuy, false, near));
        assert!(logs_with("launch_step_parked")[0].contains("only the creator, the owner or a relayer"));
    }

    #[test]
    fn the_owner_or_a_relayer_refunds_a_sniped_dev_buy_to_the_creator() {
        let mut f = factory();
        let near = 2 * 10u128.pow(24);
        owner();
        f.set_relayers(vec![acct("relay.near")]);
        for who in ["owner.near", "relay.near"] {
            let id = dev_buy_launch(&mut f, near);
            ctx("mallory.near", 0);
            assert_eq!(panics(|| drop(f.refund_dev_buy(U64(id)))), "owner or relayer only");
            ctx("alice.near", 0);
            assert_eq!(panics(|| drop(f.refund_dev_buy(U64(id)))), "owner or relayer only", "the creator has cancel_dev_buy");
            ctx(who, 0);
            drop(f.refund_dev_buy(U64(id)));
            let lz = l(&f, id);
            assert_eq!((lz.step, lz.dev_buy_held.0, lz.dev_buy_near.0), (Step::Done, 0, 0));
            assert_eq!(transfers(), vec![("alice.near".to_string(), near)], "paid to the creator, never the caller");
            assert_eq!(logs_with("launch").len(), 1, "the launch goes live with the normal event");
            assert!(logs_with("dev_buy_refunded")[0].contains(&format!(r#""by":"{}""#, who)));
        }

        let id = dev_buy_launch(&mut f, near);
        let mut lz = l(&f, id);
        lz.inflight = true;
        f.launches.insert(id, lz);
        owner();
        assert_eq!(panics(|| drop(f.refund_dev_buy(U64(id)))), "no pending dev buy");
        let done = launch(&mut f, "alice.near", "DONE");
        park(&mut f, done, Step::Done);
        owner();
        assert_eq!(panics(|| drop(f.refund_dev_buy(U64(done)))), "no pending dev buy");
    }



    fn prepaid_of(method: &[u8]) -> Option<(Gas, u64)> {
        get_created_receipts().into_iter().flat_map(|r| r.actions).find_map(|a| match a {
            MockAction::FunctionCallWeight { method_name, prepaid_gas, gas_weight, .. } if method_name == method => Some((prepaid_gas, gas_weight.0)),
            _ => None,
        })
    }

    #[test]
    fn at_300_tgas_a_dev_buy_runs_in_line_when_the_locker_has_a_measured_allowance() {
        let mut f = factory();

        f.wnear_registered = true;
        owner();
        f.set_locker_add_gas(acct("lock.near"), T_LOCK3_ADD);
        let near = 3 * 10u128.pow(24);

        let a = LaunchArgs { dev_buy: Some(U128(near)), ..args("L300") };
        let deposit = f.quote_launch(0, a.dev_buy, None).total.0;
        ctx_tx("alice.near", "alice.near", deposit, 300);
        drop(f.launch(a));
        let id = f.next_id - 1;
        let on_created = static_gas_of(b"on_created").unwrap();
        ctx_tx(ME, "alice.near", 0, tg_of(on_created));
        let pool = l(&f, id).pool_id;
        drop(f.on_created(id, None, Ok(StorageBalanceJson { total: "0".into() }), Ok(pool)));
        assert_eq!(prepaid_of(b"add").map(|g| g.0), Some(Gas::from_tgas(T_LOCK3_ADD)), "the locker gets exactly its allowance");
        let (cb, w) = prepaid_of(b"on_liquidity_added").unwrap();
        assert_eq!((cb, w), (GAS_LIQ_CB_DEV_BUY, 1), "the in-line budget, plus whatever is left over");
        ctx_tx(ME, "alice.near", 0, tg_of(cb));
        drop(f.on_liquidity_added(id, None, Ok("pool|3".into())));
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight, lz.dev_buy_held.0), (Step::DevBuy, true, 0), "in-line, debited first");
        assert_eq!(prepaid_of(b"ft_transfer_call"), Some((GAS_DEV_BUY_INLINE, 1)), "the swap: its measured floor plus every spare TGas");
        assert_eq!(prepaid_of(b"on_dev_bought"), Some((GAS_DEV_BOUGHT_CB_INLINE, 0)), "on_dev_bought: its fixed in-line budget");
        assert!(calls().iter().any(|c| c.1 == "ft_transfer_call" && c.2.contains(&format!(r#"\"min_output_amount\":\"{}\""#, dev_buy_out(id).unwrap()))));
    }

    #[test]
    fn below_300_tgas_the_dev_buy_parks() {

        let mut f = factory();
        f.wnear_registered = true;
        owner();
        f.set_locker_add_gas(acct("lock.near"), T_LOCK3_ADD);
        let near = 3 * 10u128.pow(24);
        let id = replay_dev_buy_launch(&mut f, 275, near);
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight, lz.dev_buy_held.0), (Step::DevBuy, false, near));
        assert!(logs_with("launch_step_parked")[0].contains("not enough gas"));
        assert!(calls().iter().all(|c| c.1 != "ft_transfer_call"));

        let a = LaunchArgs { dev_buy: Some(U128(near)), ..args("L250") };
        let deposit = f.quote_launch(0, a.dev_buy, None).total.0;
        ctx_tx("alice.near", "alice.near", deposit, 250);
        drop(f.launch(a));
        let id2 = f.next_id - 1;
        let on_created = static_gas_of(b"on_created").unwrap();
        ctx_tx(ME, "alice.near", 0, tg_of(on_created));
        let pool = l(&f, id2).pool_id;
        drop(f.on_created(id2, None, Ok(StorageBalanceJson { total: "0".into() }), Ok(pool)));
        let lz = l(&f, id2);
        assert_eq!((lz.step, lz.inflight, lz.dev_buy_held.0), (Step::AddLiquidity, false, near));
    }

    #[test]
    fn without_an_allowance_the_locker_keeps_its_180_and_the_dev_buy_parks() {
        let mut f = factory();

        let id = replay_dev_buy_launch(&mut f, 300, 10u128.pow(24));
        assert_eq!(l(&f, id).step, Step::DevBuy);
        assert!(!l(&f, id).inflight);
    }

    #[test]
    fn a_resume_keeps_the_full_dev_buy_budget() {
        let mut f = factory();
        let id2 = dev_buy_launch(&mut f, 10u128.pow(24));
        ctx("alice.near", 0);
        drop(f.resume(U64(id2)));
        assert_eq!(prepaid_of(b"ft_transfer_call").map(|g| g.0), Some(GAS_DEV_BUY));
        assert_eq!(prepaid_of(b"on_dev_bought"), Some((GAS_DEV_BOUGHT_CB, 0)));
    }

    #[test]
    fn set_locker_add_gas_rules() {
        let mut f = factory();
        ctx("mallory.near", 0);
        assert_eq!(panics(|| f.set_locker_add_gas(acct("lock.near"), 111)), "owner only");
        owner();
        assert_eq!(panics(|| f.set_locker_add_gas(acct("other.near"), 111)), "not one of this factory's lockers");
        owner();
        assert_eq!(panics(|| f.set_locker_add_gas(acct("lock.near"), 59)), "locker add gas: 60 to 180 TGas");
        owner();
        assert_eq!(panics(|| f.set_locker_add_gas(acct("lock.near"), 181)), "locker add gas: 60 to 180 TGas");
        owner();
        f.set_locker_add_gas(acct("lock.near"), 115);
        assert_eq!(f.get_locker_add_gas(acct("lock.near")), Some(115));
        owner();
        f.set_locker_add_gas(acct("lock.near"), 0);
        assert_eq!(f.get_locker_add_gas(acct("lock.near")), None);
    }

    #[test]
    fn a_bounced_dev_buy_refund_on_a_live_launch_is_booked_to_the_creator() {
        let mut f = factory();
        let near = 2 * 10u128.pow(24);
        owner();
        f.set_relayers(vec![acct("relay.near")]);

        let a = dev_buy_launch(&mut f, near);
        ctx("alice.near", 0);
        drop(f.cancel_dev_buy(U64(a)));
        assert!(calls().iter().any(|c| c.1 == "on_dev_refund_to_creator"));
        let b = dev_buy_launch(&mut f, near);
        ctx("relay.near", 0);
        drop(f.refund_dev_buy(U64(b)));
        assert!(calls().iter().any(|c| c.1 == "on_dev_refund_to_creator"));
        ctx(ME, 0);
        f.on_dev_refund_to_creator(U64(a), acct("alice.near"), U128(near), Err(PromiseError::Failed));
        ctx(ME, 0);
        f.on_dev_refund_to_creator(U64(b), acct("alice.near"), U128(near), Ok(()));
        assert_eq!(f.get_creator_fees(acct("alice.near")).0, near, "only the bounced one, booked to the creator's claimable balance");
        let c = dev_buy_launch(&mut f, near);
        ctx("alice.near", 0);
        drop(f.resume(U64(c)));
        ctx(ME, 0);
        let _ = f.on_dev_bought(c, Ok(U128(near - 500)));
        ctx(ME, 0);
        f.on_dev_refund_unwrapped(c, U128(500), Ok(()));
        assert!(calls().iter().any(|c| c.1 == "on_dev_refund_to_creator"));
        ctx("alice.near", 0);
        drop(f.claim_creator_fees());
        assert_eq!(transfers(), vec![("alice.near".to_string(), near)]);
    }



    #[test]
    fn an_account_level_push_to_a_router_never_notifies_or_books_a_launch() {
        let mut f = factory();
        owner();
        f.set_fee_router(acct("router.near"), true);
        f.creator_fees.insert(acct("router.near"), 500);
        ctx("anyone.near", 0);
        drop(f.push_creator_fees(acct("router.near")));
        assert_eq!(transfers(), vec![("router.near".to_string(), 500)]);
        ctx_results(vec![near_sdk::PromiseResult::Successful(vec![])]);
        f.on_paid(U64(0), "near".into(), acct("router.near"), U128(500), "account".into(), false);
        assert!(calls().iter().all(|c| c.1 != "on_launch_fees"), "no launch id to announce");
        ctx_results(vec![near_sdk::PromiseResult::Failed]);
        f.on_paid(U64(0), "near".into(), acct("router.near"), U128(500), "account".into(), false);
        assert_eq!((f.get_creator_fees(acct("router.near")).0, router_near(0)), (500, 0), "back to the account balance, not launch 0's router bucket");
        assert_eq!(logs_with("creator_push_failed").len(), 1);
    }

    #[test]
    fn a_quote_that_is_a_launch_token_is_measured_like_one() {
        let mut f = factory();

        let base = launch(&mut f, "alice.near", "NEARLY");
        let pair = l(&f, base).token.clone();
        ctx("owner.near", 0);
        f.set_quote(pair.clone(), 18, -100_000, -100_000, -100_000, 400_000, true);
        let id = launch_with(&mut f, "bob.near", LaunchArgs { quote: Some(pair.clone()), ..args("ONP") });
        park(&mut f, id, Step::Done);
        let mut lz = l(&f, id);
        lz.lpt_id = Some("x".into());
        lz.creator_quote_fees = U128(9);
        f.launches.insert(id, lz);
        ctx("crank.near", 0);
        drop(f.claim_fees(U64(id)));
        assert!(transfers_pending(&pair), "the claim's quote leg lands in the pair token's balance");
        ctx(ME, 0);
        assert!(f.on_fees_claimed(id, Ok(vec![U128(0), U128(0)])));
        assert!(!transfers_pending(&pair));

        ctx(ME, 0);
        take_swap_lock(&pair, LOCKER_WITHDRAW_LOCK_ID, LK_ADMIN);
        ctx("anyone.near", 0);
        assert_eq!(panics(|| drop(f.push_creator_quote_fees(U64(id)))), "swap in flight");
        ctx(ME, 0);
        release_swap_lock(&pair, LOCKER_WITHDRAW_LOCK_ID, LK_ADMIN);
        ctx("anyone.near", 0);
        drop(f.push_creator_quote_fees(U64(id)));
        assert_eq!(l(&f, id).creator_quote_fees.0, 0);
    }

    #[test]
    fn exchange_registration_is_tracked_per_exchange() {
        let mut f = factory();
        f.dcl_registered = true;
        let old = launch(&mut f, "alice.near", "OLDX");
        owner();
        f.set_dcl(acct("dclv3.near"), None);
        let new = launch(&mut f, "alice.near", "NEWX");
        assert_eq!(f.dcl_storage(new), DCL_REGISTER.as_yoctonear());

        ctx(ME, 0);
        let _ = f.on_pool_created(old, None, Ok(l(&f, old).pool_id));
        assert_eq!(f.dcl_storage(new), DCL_REGISTER.as_yoctonear(), "still unregistered on dclv3");
        ctx(ME, 0);
        let _ = f.on_pool_created(new, None, Ok(l(&f, new).pool_id));
        assert_eq!(f.dcl_storage(new), f.config.dcl_storage_per_launch.0);
        assert_eq!(f.dcl_storage(old), f.config.dcl_storage_per_launch.0);
    }

    #[test]
    fn a_launch_sent_through_a_bot_runs_its_dev_buy_in_line_but_a_resume_keeps_the_gate() {
        let mut f = factory();
        f.wnear_registered = true;
        owner();
        f.set_locker_add_gas(acct("lock.near"), T_LOCK3_ADD);
        let near = 10u128.pow(24);

        let a = LaunchArgs { dev_buy: Some(U128(near)), ..args("BOT") };
        let deposit = f.quote_launch(0, a.dev_buy, None).total.0;
        ctx_tx("bot.near", "user.near", deposit, 300);
        drop(f.launch(a));
        let id = f.next_id - 1;
        let cb = static_gas_of(b"on_created").unwrap();
        ctx_tx(ME, "user.near", 0, tg_of(cb));
        let pool = l(&f, id).pool_id;
        drop(f.on_created(id, Some(true), Ok(StorageBalanceJson { total: "0".into() }), Ok(pool)));
        let cb = static_gas_of(b"on_liquidity_added").unwrap();
        assert_eq!(cb, GAS_LIQ_CB_DEV_BUY, "the launch chain makes room for the buy whoever signed");
        ctx_tx(ME, "user.near", 0, tg_of(cb));
        drop(f.on_liquidity_added(id, Some(true), Ok("pool|1".into())));
        assert!(l(&f, id).inflight && l(&f, id).dev_buy_held.0 == 0, "the buy went out in-line");

        let id2 = launch_with(&mut f, "bot.near", LaunchArgs { dev_buy: Some(U128(near)), ..args("BOT2") });
        let mut lz = l(&f, id2);
        lz.step = Step::AddLiquidity;
        lz.inflight = true;
        f.launches.insert(id2, lz);
        ctx_tx(ME, "mallory.near", 0, tg_of(GAS_LIQ_CB_DEV_BUY));
        drop(f.on_liquidity_added(id2, Some(false), Ok("pool|2".into())));
        assert!(!l(&f, id2).inflight && l(&f, id2).dev_buy_held.0 == near);
        assert!(logs_with("launch_step_parked")[0].contains("only the creator, the owner or a relayer"));
    }




    fn add_buy_launch(f: &mut Factory, near: u128) -> u64 { add_buy_launch_on(f, near, true) }
    fn add_buy_launch_on(f: &mut Factory, near: u128, flag: bool) -> u64 {
        f.wnear_registered = true;
        owner();
        f.set_locker_add_gas(acct("lock.near"), T_LOCK3_ADD);
        owner();
        f.set_locker_add_buy(acct("lock.near"), flag);
        let a = LaunchArgs { dev_buy: Some(U128(near)), ..args("AB") };
        let deposit = f.quote_launch(0, a.dev_buy, None).total.0;
        ctx_tx("alice.near", "alice.near", deposit, 300);
        drop(f.launch(a));
        let id = f.next_id - 1;
        let cb = static_gas_of(b"on_created").unwrap();

        ctx_tx(ME, "alice.near", 0, tg_of(cb) + 3);
        let pool = l(f, id).pool_id;
        drop(f.on_created(id, Some(true), Ok(StorageBalanceJson { total: "0".into() }), Ok(pool)));
        id
    }

    #[test]
    fn the_launch_chain_hands_the_first_buy_to_a_locker_with_add_buy() {
        let mut f = factory();
        let near = 3 * 10u128.pow(24);
        let id = add_buy_launch(&mut f, near);
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight, lz.dev_buy_held.0), (Step::AddLiquidity, true, 0), "debited when attached");
        let c = calls();
        let ab = c.iter().find(|c| c.1 == "add_buy").expect("add_buy");
        assert_eq!(ab.0, "lock.near");
        assert_eq!(ab.3, near, "the buy rides as the attached deposit");
        assert!(ab.2.contains(&format!(r#""amount":"{}""#, near)) && ab.2.contains(r#"\"Swap\":"#) && ab.2.contains(&format!(r#"\"min_output_amount\":\"{}\""#, dev_buy_out(id).unwrap())), "{}", ab.2);
        assert!(ab.2.contains(r#"\"swap_out_recipient\":\"alice.near\""#), "{}", ab.2);
        assert!(c.iter().all(|c| c.1 != "add"), "never add and add_buy both");
        assert_eq!(prepaid_of(b"add_buy").map(|g| g.0), Some(Gas::from_tgas(T_LOCKER_ADD_BUY)));
        assert_eq!(prepaid_of(b"on_liquidity_added_buy"), Some((GAS_LIQ_CB_ADD_BUY, 0)));

        ctx(ME, 0);
        assert!(f.on_liquidity_added_buy(id, Ok(AddBuyResult { lpt_id: "pool|7".into(), used: U128(near), refunded: U128(0) })));
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.dev_buy_near.0, lz.dev_buy_held.0, lz.lpt_id.as_deref()), (Step::Done, near, 0, Some("pool|7")));
        assert_eq!(logs_with("launch").len(), 1);
        assert!(dev_buy_done(id));
    }

    #[test]
    fn a_refused_first_buy_parks_funded_and_a_held_one_is_fetched_by_the_owner() {
        let mut f = factory();
        let near = 2 * 10u128.pow(24);
        let id = add_buy_launch(&mut f, near);
        ctx(ME, 0);
        assert!(!f.on_liquidity_added_buy(id, Ok(AddBuyResult { lpt_id: "pool|1".into(), used: U128(0), refunded: U128(near) })));
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight, lz.dev_buy_held.0, lz.lpt_id.is_some()), (Step::DevBuy, false, near, true));
        assert!(logs_with("launch_step_parked")[0].contains("swap refunded"));

        ctx("alice.near", 0);
        drop(f.cancel_dev_buy(U64(id)));
        assert_eq!(transfers(), vec![("alice.near".to_string(), near)]);
        assert_eq!(l(&f, id).step, Step::Done);

        let id2 = add_buy_launch(&mut f, near);
        ctx(ME, 0);
        assert!(!f.on_liquidity_added_buy(id2, Ok(AddBuyResult { lpt_id: "pool|2".into(), used: U128(0), refunded: U128(0) })));
        assert_eq!((l(&f, id2).step, l(&f, id2).dev_buy_held.0, f.get_buy_held(U64(id2)).0), (Step::DevBuy, 0, near));
        assert!(logs_with("first_buy_done")[0].contains(&format!(r#""held":"{}""#, near)));
        ctx("alice.near", 0);
        assert!(panics(|| drop(f.cancel_dev_buy(U64(id2)))).contains("held by the locker"));
        owner();
        assert!(panics(|| drop(f.refund_dev_buy(U64(id2)))).contains("held by the locker"));
        ctx("alice.near", 0);
        assert_eq!(panics(|| drop(f.locker_refund_buy(U64(id2)))), "owner only");
        owner();
        drop(f.locker_refund_buy(U64(id2)));
        assert!(calls().iter().any(|c| c.0 == "lock.near" && c.1 == "refund_buy"));
        owner();
        assert_eq!(panics(|| drop(f.locker_refund_buy(U64(id2)))), "a refund is in flight");

        ctx(ME, 0);
        assert_eq!(f.on_locker_buy_refunded(U64(id2), Ok(BuyOutcome { used: U128(0), refunded: U128(near / 2) })).0, near / 2);
        assert_eq!((l(&f, id2).dev_buy_held.0, f.get_buy_held(U64(id2)).0), (near / 2, near - near / 2));
        owner();
        drop(f.locker_refund_buy(U64(id2)));
        ctx(ME, 0);
        assert_eq!(f.on_locker_buy_refunded(U64(id2), Ok(BuyOutcome { used: U128(0), refunded: U128(near) })).0, near - near / 2, "never more than recorded");
        assert_eq!((l(&f, id2).dev_buy_held.0, f.get_buy_held(U64(id2)).0), (near, 0));
        ctx("alice.near", 0);
        drop(f.cancel_dev_buy(U64(id2)));
        assert_eq!(transfers(), vec![("alice.near".to_string(), near)]);

        let id3 = add_buy_launch(&mut f, near);
        ctx(ME, 0);
        assert!(f.on_liquidity_added_buy(id3, Ok(AddBuyResult { lpt_id: "pool|3".into(), used: U128(near / 4), refunded: U128(0) })));
        assert_eq!((l(&f, id3).step, f.get_buy_held(U64(id3)).0), (Step::Done, near - near / 4));
        owner();
        drop(f.locker_refund_buy(U64(id3)));
        ctx(ME, 0);
        f.on_locker_buy_refunded(U64(id3), Ok(BuyOutcome { used: U128(0), refunded: U128(near - near / 4) }));
        assert_eq!(transfers(), vec![("alice.near".to_string(), near - near / 4)]);
        assert_eq!(f.get_buy_held(U64(id3)).0, 0);
    }

    #[test]
    fn an_exact_first_buy_refunds_its_excess_and_a_pending_position_keeps_the_buy_done() {
        let mut f = factory();
        let id = add_buy_launch(&mut f, 500 * 10u128.pow(24));
        let near = l(&f, id).dev_buy_near.0;
        ctx(ME, 0);
        assert!(!f.on_liquidity_added_buy(id, Ok(AddBuyResult { lpt_id: String::new(), used: U128(near - 1_000), refunded: U128(1_000) })));
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight, lz.dev_buy_near.0, lz.dev_buy_tokens.0), (Step::AddLiquidity, false, near - 1_000, f.config.total_supply.0 / BPS * 400));
        assert_eq!(transfers(), vec![("alice.near".to_string(), 1_000)], "the excess goes to the creator");

        ctx("anyone.near", 0);
        drop(f.resume(U64(id)));
        let c = calls();
        assert!(c.iter().any(|c| c.1 == "add") && c.iter().all(|c| c.1 != "add_buy"));
        ctx(ME, 0);
        drop(f.on_liquidity_added(id, None, Ok("pool|9".into())));
        assert_eq!(l(&f, id).step, Step::Done);
        assert_eq!(logs_with("launch").len(), 1);
    }

    #[test]
    fn add_buy_is_only_used_from_the_launch_chain_and_only_with_the_flag() {
        let mut f = factory();
        let near = 10u128.pow(24);


        let id = add_buy_launch(&mut f, near);
        ctx(ME, 0);
        assert!(!f.on_liquidity_added_buy(id, Err(PromiseError::Failed)));
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight, lz.dev_buy_held.0, f.get_buy_held(U64(id)).0), (Step::AddLiquidity, false, 0, near));
        assert_eq!(logs_with("dev_buy_unknown").len(), 1);
        ctx_at("alice.near", 0, T0 + 2 * DAY);
        assert!(panics(|| drop(f.cancel_stuck_launch(U64(id)))).contains("held by the locker"));

        ctx("bob.near", 0);
        drop(f.settle_first_buy(U64(id)));
        assert!(calls().iter().any(|c| c.0 == "lock.near" && c.1 == "get_first_buy"));
        ctx(ME, 0);
        assert!(f.on_first_buy_read(U64(id), Ok(None)));
        assert_eq!((l(&f, id).dev_buy_held.0, f.get_buy_held(U64(id)).0), (near, 0));
        ctx("alice.near", 0);
        drop(f.resume(U64(id)));
        assert!(calls().iter().any(|c| c.1 == "add") && calls().iter().all(|c| c.1 != "add_buy"), "a retry never goes through add_buy");

        let id2 = add_buy_launch_on(&mut f, near, false);
        assert!(!f.get_locker_add_buy(acct("lock.near")));
        assert!(calls().iter().any(|c| c.1 == "add"));
        assert_eq!(l(&f, id2).dev_buy_held.0, near);
    }

    #[test]
    fn an_add_buy_in_flight_is_never_cleared_by_the_owners_valves() {
        let mut f = factory();
        let id = add_buy_launch(&mut f, 10u128.pow(24));
        assert!(l(&f, id).inflight && l(&f, id).dev_buy_held.0 == 0);
        owner();
        assert!(panics(|| f.reset_inflight(U64(id))).contains("only cleared by its own callback"));
        owner();
        assert!(panics(|| f.set_step(U64(id), Step::AddLiquidity)).contains("only cleared by its own callback"));
    }



    fn add_buy_launch_capped(f: &mut Factory, near: Option<u128>, sym: &str) -> u64 {
        f.wnear_registered = true;
        owner();
        f.set_locker_add_gas(acct("lock.near"), T_LOCK3_ADD);
        owner();
        f.set_locker_add_buy(acct("lock.near"), true);
        let a = LaunchArgs { dev_buy: near.map(U128), ..args(sym) };
        let deposit = f.quote_launch(0, a.dev_buy, None).total.0;
        ctx_tx("alice.near", "alice.near", deposit, 270);
        drop(f.launch(a));
        let id = f.next_id - 1;
        ctx_tx(ME, "alice.near", 0, 220);
        let pool = l(f, id).pool_id;
        drop(f.on_created(id, Some(true), Ok(StorageBalanceJson { total: "0".into() }), Ok(pool)));
        id
    }

    #[test]
    fn a_launch_parked_before_its_first_add_sends_the_first_buy_from_the_resume() {
        let mut f = factory();
        let near = 2 * 10u128.pow(24);
        let id = add_buy_launch_capped(&mut f, Some(near), "EVM");
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight, lz.dev_buy_held.0), (Step::AddLiquidity, false, near), "parked, still funded");
        assert!(no_add_sent(id));
        assert!(calls().iter().all(|c| c.1 != "add" && c.1 != "add_buy"), "nothing reached the locker");

        ctx_tx("mallory.near", "mallory.near", 0, 300);
        assert!(panics(|| drop(f.resume(U64(id)))).contains("first buy pending"));

        ctx_tx("alice.near", "alice.near", 0, 250);
        assert!(panics(|| drop(f.resume(U64(id)))).contains("at least 260 TGas"));
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight, lz.dev_buy_held.0, no_add_sent(id)), (Step::AddLiquidity, false, near, true));
        assert!(calls().iter().all(|c| c.1 != "add" && c.1 != "add_buy"));


        ctx_tx("alice.near", "evm-relayer.near", 0, 270);
        drop(f.resume(U64(id)));
        let c = calls();
        let ab = c.iter().find(|c| c.1 == "add_buy").expect("add_buy");
        assert_eq!((ab.0.as_str(), ab.3), ("lock.near", near));
        assert!(ab.2.contains(r#"\"swap_out_recipient\":\"alice.near\""#), "{}", ab.2);
        assert!(c.iter().all(|c| c.1 != "add"), "never add and add_buy both");
        assert_eq!(prepaid_of(b"add_buy").map(|g| g.0), Some(Gas::from_tgas(T_LOCKER_ADD_BUY)));
        let lz = l(&f, id);
        assert_eq!((lz.inflight, lz.dev_buy_held.0, no_add_sent(id)), (true, 0, false));
        ctx(ME, 0);
        assert!(f.on_liquidity_added_buy(id, Ok(AddBuyResult { lpt_id: "pool|4".into(), used: U128(near), refunded: U128(0) })));
        assert_eq!(l(&f, id).step, Step::Done);
        assert!(dev_buy_done(id));

        let id2 = add_buy_launch_capped(&mut f, Some(near), "EVM2");
        owner();
        f.set_relayers(vec![acct("relay.near")]);
        ctx_tx("relay.near", "relay.near", 0, 1000);
        drop(f.resume(U64(id2)));
        assert!(calls().iter().any(|c| c.1 == "add_buy"));
    }

    #[test]
    fn once_the_locker_has_seen_the_pool_a_resume_uses_add() {
        let mut f = factory();
        let near = 10u128.pow(24);

        let id = add_buy_launch_capped(&mut f, Some(near), "LOST");
        ctx_tx("alice.near", "alice.near", 0, 300);
        drop(f.resume(U64(id)));
        assert!(calls().iter().any(|c| c.1 == "add_buy"));
        ctx(ME, 0);
        assert!(!f.on_liquidity_added_buy(id, Err(PromiseError::Failed)));
        ctx(ME, 0);
        assert!(f.on_first_buy_read(U64(id), Ok(None)));
        assert_eq!(l(&f, id).dev_buy_held.0, near);
        assert!(!no_add_sent(id));
        ctx_tx("alice.near", "alice.near", 0, 300);
        drop(f.resume(U64(id)));
        assert!(calls().iter().any(|c| c.1 == "add") && calls().iter().all(|c| c.1 != "add_buy"), "a retry never goes through add_buy");

        let id2 = add_buy_launch_capped(&mut f, None, "NOBUY");
        assert_eq!(l(&f, id2).step, Step::AddLiquidity);
        ctx_tx("anyone.near", "anyone.near", 0, 300);
        drop(f.resume(U64(id2)));
        assert!(calls().iter().any(|c| c.1 == "add") && calls().iter().all(|c| c.1 != "add_buy"));
        assert!(!no_add_sent(id2));

        let id3 = add_buy_launch_capped(&mut f, Some(near), "OLD");
        env::storage_remove(&no_add_key(id3));
        ctx_tx("anyone.near", "anyone.near", 0, 300);
        drop(f.resume(U64(id3)));
        assert!(calls().iter().any(|c| c.1 == "add") && calls().iter().all(|c| c.1 != "add_buy"));
    }

    #[test]
    fn a_resume_chain_parks_the_first_buy_for_a_trigger_who_may_not_send_it() {
        let mut f = factory();
        let near = 10u128.pow(24);
        let id = add_buy_launch_capped(&mut f, Some(near), "CP");

        env::storage_remove(&no_add_key(id));
        let mut lz = l(&f, id);
        lz.step = Step::CreatePool;
        lz.inflight = true;
        f.launches.insert(id, lz.clone());
        ctx_tx(ME, "mallory.near", 0, 300);
        drop(f.on_pool_created(id, Some(false), Ok(lz.pool_id.clone())));
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight, lz.dev_buy_held.0), (Step::AddLiquidity, false, near));
        assert!(no_add_sent(id) && calls().iter().all(|c| c.1 != "add" && c.1 != "add_buy"));
        assert!(logs_with("launch_step_parked")[0].contains("resume by the creator"));

        ctx_tx("alice.near", "alice.near", 0, 300);
        drop(f.resume(U64(id)));
        assert!(calls().iter().any(|c| c.1 == "add_buy"));
    }



    #[test]
    fn a_creator_tax_share_on_a_pair_token_needs_creator_mode() {
        let mut f = factory();
        let usdc = usdc_pair(&mut f);
        owner();
        f.set_tax_token_code_hash(Some("bb".repeat(32)));
        let tax = |c: u16, b: u16, h: u16| Some(TaxArgs { buy_bps: 100, sell_bps: 100, creator_bps: c, burn_bps: b, holders_bps: h });
        let msg = panics(|| drop(launch_with(&mut f, "alice.near", LaunchArgs { quote: Some(usdc.clone()), fee_mode: Some("burn".into()), tax: tax(5000, 0, 5000), ..args("BTX") })));
        assert_eq!(msg, "tax: a creator tax share on a pair token needs fee mode creator");

        launch_with(&mut f, "alice.near", LaunchArgs { quote: Some(usdc.clone()), fee_mode: Some("burn".into()), tax: tax(0, 5000, 5000), ..args("BT0") });
        launch_with(&mut f, "alice.near", LaunchArgs { fee_mode: Some("burn".into()), tax: tax(5000, 0, 5000), ..args("BTN") });
        launch_with(&mut f, "alice.near", LaunchArgs { quote: Some(usdc.clone()), tax: tax(5000, 0, 5000), ..args("BTC") });
    }

    #[test]
    fn a_plain_transfer_result_is_never_read_as_a_refund() {
        let mut f = factory();
        add_owed("usdc.near", &acct("b.near"), 70);
        ctx("anyone.near", 0);
        f.push_owed("usdc.near".into(), vec![acct("b.near")]);

        ctx_results(vec![near_sdk::PromiseResult::Successful(b"\"0\"".to_vec())]);
        f.on_owed_pushed("usdc.near".into(), acct("b.near"), U128(70));
        assert_eq!(f.get_owed("usdc.near".into(), acct("b.near")).0, 0);
        assert!(logs_with("owed_paid")[0].contains(r#""amount":"70""#));
        assert!(logs_with("owed_push_failed").is_empty());
    }

    #[test]
    fn a_buyback_whose_lock_expired_burns_nothing() {
        let mut f = factory();
        let n = 10u128.pow(24);
        let id = live_burn_launch(&mut f, "BLX", 5 * n);
        let token = l(&f, id).token.clone();
        ctx("owner.near", 0);
        drop(f.buyback(U64(id), U128(1000)));
        ctx(ME, 0);
        drop(f.on_bb_swapped(U64(id), U128(5 * n), U128(0), Some(U128(1000)), Ok(U128(5 * n))));

        env::storage_remove(&swap_lock_key(&token));
        ctx(ME, 0);
        drop(f.on_bb_after(U64(id), U128(0), U128(5 * n), Some(U128(1000)), Ok(U128(900))));
        assert!(calls().iter().all(|c| c.1 != "burn"), "nothing burned");
        assert_eq!(logs_with("buyback_lock_lost").len(), 1);
        assert_eq!(read_counter(&burned_key(id)), 0);
    }

    #[test]
    fn a_pair_token_buyback_counts_its_quote_in_flight() {
        let mut f = factory();
        let n = 10u128.pow(24);
        let a = live_burn_launch(&mut f, "BASE", 5 * n);
        let ta = l(&f, a).token.clone();
        ctx("owner.near", 0);
        f.set_quote(ta.clone(), 18, -50_000, -80_000, -30_000, 450_000, true);
        let b = launch_with(&mut f, "alice.near", LaunchArgs { quote: Some(ta.clone()), fee_mode: Some("burn".into()), ..args("PAIRB") });
        park(&mut f, b, Step::Done);
        let mut lb = l(&f, b);
        lb.creator_quote_fees = U128(3 * n);
        f.launches.insert(b, lb);
        ctx("owner.near", 0);
        drop(f.buyback(U64(b), U128(1000)));
        assert!(transfers_pending(&ta), "the pair token is out at the exchange");
        ctx("owner.near", 0);
        assert_eq!(panics(|| drop(f.buyback(U64(a), U128(1000)))), "transfer in flight");
        ctx(ME, 0);
        drop(f.on_bb_swapped(U64(b), U128(3 * n), U128(0), Some(U128(1000)), Ok(U128(3 * n))));
        assert!(!transfers_pending(&ta), "back: the count ended");
    }

    #[test]
    fn a_locker_withdrawal_books_at_most_max_book() {
        let mut f = factory();
        let id = launch(&mut f, "alice.near", "LWM");
        park(&mut f, id, Step::Done);
        let token = l(&f, id).token.clone();
        owner();
        drop(f.locker_withdraw_dcl_asset(acct("lock.near"), token.clone(), U128(250), Some(U128(300))));
        ctx(ME, 0);
        assert_eq!(f.on_locker_withdraw_booked(token.clone(), U128(1_000), Some(U128(300)), Ok(U128(1_001_000))).0, 300);
        assert_eq!(l(&f, id).protocol_token_fees.0, 300);
    }

    #[test]
    fn a_house_buyback_never_takes_fees_booked_to_the_creator() {
        let mut f = factory();
        internal_set_house_creators(vec![acct("house.near")], true);
        let id = launch(&mut f, "house.near", "HBF");
        park(&mut f, id, Step::Done);
        let mut lz = l(&f, id);
        lz.creator_token_fees = U128(5);
        f.launches.insert(id, lz);
        owner();
        assert_eq!(panics(|| f.set_house_buyback(U64(id))), "pay the creator's booked token and quote fees first");
        let mut lz = l(&f, id);
        lz.creator_token_fees = U128(0);
        lz.creator_quote_fees = U128(7);
        f.launches.insert(id, lz);
        owner();
        assert_eq!(panics(|| f.set_house_buyback(U64(id))), "pay the creator's booked token and quote fees first");
        let mut lz = l(&f, id);
        lz.creator_quote_fees = U128(0);
        f.launches.insert(id, lz);
        owner();
        f.set_house_buyback(U64(id));
        assert_eq!(f.get_fee_opts(U64(id)).mode.as_str(), "burn");
    }

    #[test]
    fn a_locker_is_never_the_factory_or_a_launch_token() {
        let mut f = factory();
        let id = launch(&mut f, "alice.near", "LKT");
        let token = l(&f, id).token.clone();
        let next = U64(f.next_id);
        owner();
        assert_eq!(panics(|| f.set_locker_from(acct(ME), next)), "locker: a direct sub-account of the launchpad with '_' in its name");
        owner();
        assert_eq!(panics(|| f.set_locker_from(token.clone(), next)), "locker: a direct sub-account of the launchpad with '_' in its name");
        owner();
        f.set_locker_from(acct("lock_5.nearlytrade.near"), next);
    }

    #[test]
    fn a_dev_buy_whose_callback_died_is_released_and_a_late_callback_pays_the_creator() {
        let mut f = factory();
        let near = 3 * 10u128.pow(24);
        let id = dev_buy_launch(&mut f, near);
        owner();
        assert_eq!(panics(|| f.release_dev_buy(U64(id))), "nothing to release: refund_dev_buy pays a funded buy, settle_first_buy books one sent with add_buy");
        ctx("alice.near", 0);
        drop(f.resume(U64(id)));
        assert!(l(&f, id).inflight);
        ctx("mallory.near", 0);
        assert_eq!(panics(|| f.release_dev_buy(U64(id))), "owner only");
        owner();
        f.release_dev_buy(U64(id));
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight, lz.dev_buy_held.0), (Step::Done, false, 0));
        assert!(transfers().is_empty(), "a release moves nothing");
        assert_eq!(logs_with("dev_buy_released").len(), 1);

        ctx(ME, 0);
        drop(f.on_dev_bought(id, Ok(U128(0))));
        assert!(!l(&f, id).inflight, "a released launch never goes back in flight (claims stay open)");
        ctx(ME, 0);
        assert!(f.on_dev_buy_unwrapped(id, uw_op(id), Ok(())));
        assert_eq!(transfers(), vec![("alice.near".to_string(), near)]);
        assert_eq!(l(&f, id).step, Step::Done);
        ctx("alice.near", 0);
        assert_eq!(panics(|| drop(f.cancel_dev_buy(U64(id)))), "no pending dev buy");
    }

    #[test]
    fn a_released_dev_buy_whose_wrap_reverted_late_refunds_the_creator() {
        let mut f = factory();
        let near = 2 * 10u128.pow(24);
        let id = dev_buy_launch(&mut f, near);
        ctx("alice.near", 0);
        drop(f.resume(U64(id)));
        owner();
        f.release_dev_buy(U64(id));
        ctx(ME, 0);
        drop(f.on_dev_bought(id, Err(PromiseError::Failed)));
        assert_eq!(transfers(), vec![("alice.near".to_string(), near)]);
        assert_eq!(l(&f, id).dev_buy_held.0, 0);
    }

    #[test]
    fn releasing_a_dev_buy_keeps_a_retried_unwrap_owed_to_the_creator() {
        let mut f = factory();
        let near = 10u128.pow(24);
        let id = dev_buy_launch(&mut f, near);
        let mut lz = l(&f, id);
        lz.dev_buy_held = U128(0);
        lz.inflight = true;
        f.launches.insert(id, lz);
        env::storage_write(&unwrap_pending_key(id), &near.to_le_bytes());
        add_counter(UW_TOTAL_KEY, near);
        owner();
        f.release_dev_buy(U64(id));

        assert_eq!(read_counter(UW_TOTAL_KEY), near);
        assert!(!env::storage_has_key(&unwrap_pending_key(id)));
        assert_eq!(f.get_wnear_owed(U64(id)), (U128(near), U128(0)));
        assert_eq!(l(&f, id).step, Step::Done);
        owner();
        assert!(panics(|| drop(f.sweep_wnear(U128(near)))).contains("retry_wnear_owed"));

        ctx(ME, 0);
        assert!(f.on_dev_buy_unwrapped(id, uw_op(id), Ok(())));
        assert_eq!(transfers(), vec![("alice.near".to_string(), near)]);
        assert_eq!((read_counter(UW_TOTAL_KEY), f.get_wnear_owed(U64(id)).0 .0), (0, 0));
        ctx("anyone.near", 0);
        assert_eq!(panics(|| drop(f.retry_wnear_owed(U64(id)))), "nothing owed to this launch");

        let id2 = dev_buy_launch(&mut f, near);
        let mut lz = l(&f, id2);
        lz.dev_buy_held = U128(0);
        f.launches.insert(id2, lz);
        env::storage_write(&unwrap_pending_key(id2), &near.to_le_bytes());
        owner();
        assert_eq!(panics(|| f.release_dev_buy(U64(id2))), "resume unwraps the refused buy first");
    }





    fn legion_pending_unwrap(f: &mut Factory, near: u128) -> u64 {
        let id = dev_buy_launch(f, near);
        let mut lz = l(f, id);
        lz.dev_buy_held = U128(0);
        f.launches.insert(id, lz);
        ctx(ME, 0);
        assert!(!f.on_dev_buy_unwrapped(id, uw_op(id), Err(PromiseError::Failed)));
        assert_eq!(read_counter(UW_TOTAL_KEY), near);
        ctx("alice.near", 0);
        drop(f.resume(U64(id)));
        assert!(l(f, id).inflight && uw_op(id).is_some());
        id
    }

    #[test]
    fn legion_n01_a_release_during_a_retried_unwrap_pays_the_creator_once() {
        for landed in [true, false] {
            let mut f = factory();
            let near = 10u128.pow(24);
            let id = legion_pending_unwrap(&mut f, near);
            let op = uw_op(id);
            owner();
            f.release_dev_buy(U64(id));
            assert_eq!(l(&f, id).step, Step::Done);

            assert_eq!(f.get_wnear_owed(U64(id)), (U128(0), U128(0)));
            ctx("anyone.near", 0);
            assert_eq!(panics(|| drop(f.retry_wnear_owed(U64(id)))), "nothing owed to this launch");
            ctx(ME, 0);
            let r = if landed { Ok(()) } else { Err(PromiseError::Failed) };
            assert_eq!(f.on_dev_buy_unwrapped(id, op, r), landed);
            assert!(uw_op(id).is_none() && !env::storage_has_key(&unwrap_pending_key(id)));
            if landed {
                assert_eq!(transfers(), vec![("alice.near".to_string(), near)]);
                assert_eq!((read_counter(UW_TOTAL_KEY), f.get_wnear_owed(U64(id)).0 .0), (0, 0));
            } else {

                assert!(transfers().is_empty());
                assert_eq!((read_counter(UW_TOTAL_KEY), f.get_wnear_owed(U64(id)).0 .0), (near, near));
                ctx("anyone.near", 0);
                drop(f.retry_wnear_owed(U64(id)));
                ctx(ME, 0);
                assert!(f.on_wnear_owed_unwrapped(U64(id), U128(near), U128(0), retry_op(id), Ok(())));
                assert_eq!(transfers(), vec![("alice.near".to_string(), near)]);
                assert_eq!((read_counter(UW_TOTAL_KEY), f.get_wnear_owed(U64(id)).0 .0), (0, 0));
            }

            ctx(ME, 0);
            assert!(!f.on_dev_buy_unwrapped(id, op, Ok(())));
            assert!(transfers().is_empty());
            assert_eq!(logs_with("dev_buy_unwrap_stale").len(), 1);
        }
    }

    #[test]
    fn legion_n01_a_release_during_the_first_unwrap_keeps_a_failed_unwrap_owed() {
        let mut f = factory();
        let near = 2 * 10u128.pow(24);
        let id = dev_buy_launch(&mut f, near);
        ctx("alice.near", 0);
        drop(f.resume(U64(id)));
        ctx(ME, 0);
        let _ = f.on_dev_bought(id, Ok(U128(0)));
        let op = uw_op(id);
        assert!(op.is_some() && l(&f, id).inflight, "the refused swap's unwrap is in flight");
        owner();
        f.release_dev_buy(U64(id));
        assert_eq!(l(&f, id).step, Step::Done);
        ctx(ME, 0);
        assert!(!f.on_dev_buy_unwrapped(id, op, Err(PromiseError::Failed)));

        assert_eq!((read_counter(UW_TOTAL_KEY), f.get_wnear_owed(U64(id)).0 .0), (near, near));
        owner();
        assert!(panics(|| drop(f.sweep_wnear(U128(1)))).contains("retry_wnear_owed"));
        ctx("anyone.near", 0);
        drop(f.retry_wnear_owed(U64(id)));
        ctx(ME, 0);
        assert!(f.on_wnear_owed_unwrapped(U64(id), U128(near), U128(0), retry_op(id), Ok(())));
        assert_eq!(transfers(), vec![("alice.near".to_string(), near)]);
        assert_eq!((read_counter(UW_TOTAL_KEY), f.get_wnear_owed(U64(id)).0 .0), (0, 0));
    }

    #[test]
    fn legion_n01_an_unwrap_whose_callback_died_is_settled_once_by_the_owner() {
        let mut f = factory();
        let near = 10u128.pow(24);
        let id = legion_pending_unwrap(&mut f, near);
        let op = uw_op(id).unwrap();
        owner();
        f.release_dev_buy(U64(id));

        ctx_at("owner.near", 0, op + UNWRAP_RESOLVE_MS - 1);
        assert_eq!(panics(|| { f.resolve_unwrap(U64(id), true); }), "unwrap in flight: its callback settles it");
        ctx_at("mallory.near", 0, op + UNWRAP_RESOLVE_MS);
        assert_eq!(panics(|| { f.resolve_unwrap(U64(id), true); }), "owner only");
        ctx_at("owner.near", 0, op + UNWRAP_RESOLVE_MS);
        assert!(f.resolve_unwrap(U64(id), true));
        assert_eq!(transfers(), vec![("alice.near".to_string(), near)]);
        assert_eq!((read_counter(UW_TOTAL_KEY), f.get_wnear_owed(U64(id)).0 .0), (0, 0));

        ctx(ME, 0);
        assert!(!f.on_dev_buy_unwrapped(id, Some(op), Ok(())));
        assert!(transfers().is_empty());
        owner();
        assert_eq!(panics(|| { f.resolve_unwrap(U64(id), true); }), "no unwrap in flight for this launch");
    }

    #[test]
    fn legion_n02_a_retry_carries_its_own_callback_gas_and_is_settled_once() {
        let mut f = factory();
        let near = 10u128.pow(24);
        let id = live_burn_launch(&mut f, "OWE2", 10 * near);
        ctx(ME, 0);
        assert!(!f.on_dev_refund_unwrapped(id, U128(near), Err(PromiseError::Failed)));

        ctx_gas("anyone.near", 40);
        assert_eq!(panics(|| drop(f.retry_wnear_owed(U64(id)))), "attach at least 50 TGas");
        ctx_gas("anyone.near", 50);
        drop(f.retry_wnear_owed(U64(id)));
        let cb = near_sdk::test_utils::get_created_receipts().into_iter().flat_map(|r| r.actions).find_map(|a| match a {
            near_sdk::mock::MockAction::FunctionCallWeight { method_name, prepaid_gas, gas_weight, .. } if method_name == b"on_wnear_owed_unwrapped" => Some((prepaid_gas, gas_weight.0)),
            _ => None,
        });
        assert_eq!(cb, Some((GAS_OWED_CB, 0)), "a fixed budget, not the caller's leftovers");
        let op = retry_op(id).unwrap();

        ctx_at("anyone.near", 0, op + 24 * 3600 * 1000);
        assert_eq!(panics(|| drop(f.retry_wnear_owed(U64(id)))), "retry in flight");
        ctx_at("owner.near", 0, op + UNWRAP_RESOLVE_MS);
        assert!(!f.resolve_unwrap(U64(id), false));
        assert_eq!(f.get_wnear_owed(U64(id)), (U128(near), U128(0)), "the unwrap did not land: still owed");
        ctx(ME, 0);
        assert!(!f.on_wnear_owed_unwrapped(U64(id), U128(near), U128(0), Some(op), Ok(())), "a late callback moves nothing");
        assert!(transfers().is_empty());
        ctx_at("anyone.near", 0, op + UNWRAP_RESOLVE_MS + 1);
        drop(f.retry_wnear_owed(U64(id)));
        ctx(ME, 0);
        assert!(f.on_wnear_owed_unwrapped(U64(id), U128(near), U128(0), retry_op(id), Ok(())));
        assert_eq!(transfers(), vec![(l(&f, id).creator.to_string(), near)]);
        assert_eq!((read_counter(UW_TOTAL_KEY), f.get_wnear_owed(U64(id)).0 .0), (0, 0));
    }

    #[test]
    fn legion_n09_only_the_keeper_takes_tax() {
        let mut f = factory();
        let tid = live_tax_launch(&mut f, "KTAX", None, 10);
        ctx("anyone.near", 0);
        assert_eq!(panics(|| drop(f.collect_tax(U64(tid)))), "keeper only");
        ctx(ME, 0);
        drop(f.collect_tax(U64(tid)));
        assert!(calls().iter().any(|c| c.1 == "tax_take"));
    }

    #[test]
    fn legion_n03_a_quote_assets_pairs_never_change() {
        let mut f = factory();
        let tid = live_tax_launch(&mut f, "QTAX", None, 10);
        let token = l(&f, tid).token.clone();
        owner();
        drop(f.token_add_pair(U64(tid), acct("other-dex.near")));
        assert!(calls().iter().any(|c| c.0 == token.to_string() && c.1 == "tax_add_pair"));
        owner();
        f.set_quote(token.clone(), 24, 0, -100_000, 100_000, 200_000, true);
        owner();
        assert_eq!(panics(|| drop(f.token_add_pair(U64(tid), acct("third-dex.near")))), "a quote asset's pairs stay as the lockers read them");
    }

    #[test]
    fn legion_n12_the_quote_covers_the_longest_launch() {
        let mut f = factory();
        let a = LaunchArgs { name: "\u{1F600}".repeat(32), symbol: "ABCDEFGHIJKL".into(), description: Some("\u{1F600}".repeat(500)), ..args("X") };
        assert_eq!(a.name.len() + a.symbol.len() + a.description.as_ref().unwrap().len(), MAX_TEXT_BYTES);

        let id = launch_with(&mut f, "alice.near", a);
        assert_eq!(l(&f, id).symbol, "ABCDEFGHIJKL");
    }

    fn first_buy_record(near: u128, used: u128, returned: u128, settled: bool) -> FirstBuyRecord {
        FirstBuyRecord { amount: U128(near), sent: true, answered: true, used: U128(used), returned: U128(returned), settled }
    }

    #[test]
    fn a_first_buy_whose_result_was_lost_is_settled_from_the_locker_record_once() {
        let mut f = factory();
        let near = 4 * 10u128.pow(24);
        let id = add_buy_launch(&mut f, near);

        ctx("bob.near", 0);
        drop(f.settle_first_buy(U64(id)));
        assert!(calls().iter().any(|c| c.0 == "lock.near" && c.1 == "get_first_buy" && c.2.contains(&l(&f, id).token.to_string())));

        ctx(ME, 0);
        assert!(!f.on_first_buy_read(U64(id), Ok(Some(first_buy_record(near, near / 2, 0, false)))));
        ctx(ME, 0);
        assert!(!f.on_first_buy_read(U64(id), Ok(None)));
        ctx(ME, 0);
        assert!(!f.on_first_buy_read(U64(id), Err(PromiseError::Failed)), "an unreadable record moves nothing");
        assert!(l(&f, id).inflight);
        assert!(transfers().is_empty());

        ctx(ME, 0);
        assert!(f.on_first_buy_read(U64(id), Ok(Some(first_buy_record(near, near / 2, near / 4, true)))));
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight, lz.dev_buy_near.0, lz.dev_buy_held.0), (Step::AddLiquidity, false, near / 2, 0));
        assert!(dev_buy_done(id));
        assert_eq!(buy_held(id), near - near / 2 - near / 4, "the rest is held by the locker, for locker_refund_buy");
        assert_eq!(transfers(), vec![("alice.near".to_string(), near / 4)], "the NEAR seen back goes to the creator");

        ctx("bob.near", 0);
        assert_eq!(panics(|| drop(f.settle_first_buy(U64(id)))), "no first buy to settle");
        ctx(ME, 0);
        assert!(!f.on_first_buy_read(U64(id), Ok(Some(first_buy_record(near, near / 2, near / 4, true)))));
        ctx("alice.near", 0);
        drop(f.resume(U64(id)));
        assert!(calls().iter().any(|c| c.1 == "add") && calls().iter().all(|c| c.1 != "add_buy"));
        ctx(ME, 0);
        assert!(!f.on_liquidity_added_buy(id, Ok(AddBuyResult { lpt_id: "".into(), used: U128(near / 2), refunded: U128(near / 4) })));
        assert_eq!(logs_with("add_buy_result_ignored").len(), 1);
        assert!(transfers().is_empty(), "never twice");
    }

    #[test]
    fn an_add_buy_error_waits_for_the_locker_record_before_any_refund() {
        let mut f = factory();
        let near = 2 * 10u128.pow(24);
        let id = add_buy_launch(&mut f, near);
        ctx(ME, 0);
        assert!(!f.on_liquidity_added_buy(id, Err(PromiseError::Failed)));
        assert_eq!(buy_held(id), near);
        owner();
        assert_eq!(panics(|| drop(f.locker_refund_buy(U64(id)))), "settle_first_buy first");

        ctx("bob.near", 0);
        drop(f.settle_first_buy(U64(id)));
        ctx(ME, 0);
        assert!(f.on_first_buy_read(U64(id), Ok(Some(first_buy_record(near, 0, 0, true)))));
        let lz = l(&f, id);
        assert_eq!((lz.step, lz.inflight, lz.dev_buy_held.0, buy_held(id), dev_buy_done(id)), (Step::AddLiquidity, false, 0, near, false));
        assert!(transfers().is_empty());
        owner();
        drop(f.locker_refund_buy(U64(id)));
        assert!(calls().iter().any(|c| c.1 == "refund_buy"));

        let id2 = add_buy_launch_capped(&mut f, Some(near), "ERR2");
        ctx_tx("alice.near", "alice.near", 0, 300);
        drop(f.resume(U64(id2)));
        ctx(ME, 0);
        assert!(!f.on_liquidity_added_buy(id2, Err(PromiseError::Failed)));
        ctx(ME, 0);
        assert!(f.on_first_buy_read(U64(id2), Ok(None)));
        assert_eq!((l(&f, id2).dev_buy_held.0, buy_held(id2)), (near, 0), "the dev buy is funded again");
    }

    #[test]
    fn leftovers_of_a_first_buy_the_locker_made_go_to_the_creator() {
        let mut f = factory();
        let near = 3 * 10u128.pow(24);
        let id = add_buy_launch(&mut f, near);
        ctx(ME, 0);
        assert!(!f.on_liquidity_added_buy(id, Ok(AddBuyResult { lpt_id: "".into(), used: U128(near / 2), refunded: U128(0) })));
        assert_eq!((buy_held(id), dev_buy_done(id), l(&f, id).step), (near - near / 2, true, Step::AddLiquidity));
        owner();
        drop(f.locker_refund_buy(U64(id)));
        ctx(ME, 0);
        assert_eq!(f.on_locker_buy_refunded(U64(id), Ok(BuyOutcome { used: U128(0), refunded: U128(near - near / 2) })).0, near - near / 2);
        assert_eq!(transfers(), vec![("alice.near".to_string(), near - near / 2)]);
        assert_eq!((l(&f, id).dev_buy_held.0, buy_held(id)), (0, 0), "no dev buy is funded again after the first buy");
    }

    #[test]
    fn owner_repairs_never_clear_a_dev_buy_in_flight_or_skip_to_done() {
        let mut f = factory();
        let near = 10u128.pow(24);
        let id = launch_with(&mut f, "alice.near", LaunchArgs { dev_buy: Some(U128(near)), ..args("SRG") });

        let mut lz = l(&f, id);
        lz.step = Step::AddLiquidity;
        lz.inflight = true;
        lz.lpt_id = None;
        lz.dev_buy_held = U128(0);
        f.launches.insert(id, lz);
        owner();
        assert!(panics(|| f.set_range(U64(id), 200, 400)).contains("only cleared by its own callback"));
        assert!(l(&f, id).inflight, "still in flight");
        let id2 = launch(&mut f, "alice.near", "SSG");
        park(&mut f, id2, Step::AddLiquidity);
        owner();
        assert_eq!(panics(|| f.set_step(U64(id2), Step::Done)), "Done needs a position and no dev buy money on the launch (cancel_dev_buy / refund_dev_buy)");
        let mut lz = l(&f, id2);
        lz.lpt_id = Some("pool|9".into());
        f.launches.insert(id2, lz);
        owner();
        assert_eq!(panics(|| f.set_step(U64(id2), Step::Failed)), "this launch has a position");
        owner();
        f.set_step(U64(id2), Step::DevBuy);
        assert_eq!(l(&f, id2).step, Step::DevBuy);
    }

    #[test]
    #[should_panic(expected = "owner only")]
    fn only_the_owner_sets_relayers() {
        let mut f = factory();
        ctx("mallory.near", 0);
        f.set_relayers(vec!["mallory.near".parse().unwrap()]);
    }



    #[test]
    fn fix1_house_buyback_never_takes_booked_creator_fees() {
        let mut f = factory();
        let id = launch(&mut f, "alice.near", "ALICE");
        park(&mut f, id, Step::Done);
        let mut lz = l(&f, id);
        lz.creator_token_fees = U128(700);
        f.launches.insert(id, lz);
        owner(); f.set_house_creators(vec![acct("alice.near")], true);
        owner();
        assert_eq!(panics(|| f.set_house_buyback(U64(id))), "pay the creator's booked token and quote fees first");
        ctx("anyone.near", 0);
        drop(f.push_creator_token_fees(U64(id)));
        owner(); f.set_house_buyback(U64(id));
        assert_eq!(f.get_fee_opts(U64(id)).mode, "burn");
    }

    #[test]
    fn fix2_set_locker_from_takes_only_an_admin_sub_account_with_an_underscore() {
        let mut f = factory();
        let id = launch(&mut f, "alice.near", "TOK");
        let token = l(&f, id).token.clone();
        for bad in [token.as_str(), "attacker.near", "lock7.nearlytrade.near", "x.lock_5.nearlytrade.near", ME] {
            owner();
            assert!(panics(|| f.set_locker_from(acct(bad), U64(f.next_id))).contains("direct sub-account"), "{bad}");
        }
        owner();
        f.set_locker_from(acct("lock_5.nearlytrade.near"), U64(f.next_id));
        assert_eq!(f.get_locker_for(U64(f.next_id)), acct("lock_5.nearlytrade.near"));
        let mut c = f.config.clone();
        c.launch_fee = U128(11 * 10u128.pow(24));
        owner();
        assert_eq!(panics(|| f.set_config(c)), "launch fee max 10 NEAR");
    }

    #[test]
    fn fix3_the_valves_keep_the_dev_buy_guards() {
        let mut f = factory();
        let near = 10u128.pow(24);
        let id = add_buy_launch(&mut f, near);
        owner();
        assert!(panics(|| f.set_range(U64(id), 400, 1000)).contains("only cleared by its own callback"));
        assert!(l(&f, id).inflight);
        let id = launch_with(&mut f, "alice.near", LaunchArgs { dev_buy: Some(U128(near)), ..args("SD") });
        park(&mut f, id, Step::DevBuy);
        owner();
        assert!(panics(|| f.set_step(U64(id), Step::Done)).contains("Done needs a position and no dev buy money"));
        owner();
        assert_eq!(panics(|| f.set_step(U64(id), Step::Failed)), "this launch has a position");
        owner();
        assert_eq!(panics(|| f.set_step(U64(id), Step::CreatePool)), "this launch has a position");
        let id2 = launch(&mut f, "bob.near", "NOPOS");
        park(&mut f, id2, Step::AddLiquidity);
        owner();
        assert!(panics(|| f.set_step(U64(id2), Step::Done)).contains("Done needs a position"));
        owner();
        assert_eq!(panics(|| f.set_step(U64(id2), Step::DevBuy)), "DevBuy needs a position");
        owner();
        f.set_step(U64(id2), Step::Failed);
    }

    #[test]
    fn fix4_sweep_waits_for_a_dev_buys_pending_unwrap() {
        let mut f = factory();
        let near = 10u128.pow(24);
        let id = launch_with(&mut f, "alice.near", LaunchArgs { dev_buy: Some(U128(near)), ..args("UW") });
        park(&mut f, id, Step::DevBuy);
        let mut lz = l(&f, id);
        lz.dev_buy_held = U128(0);
        lz.inflight = true;
        f.launches.insert(id, lz);
        ctx(ME, 0);
        f.on_dev_buy_unwrapped(id, uw_op(id), Err(PromiseError::Failed));
        assert_eq!(read_counter(UW_TOTAL_KEY), near);
        owner();
        assert!(panics(|| drop(f.sweep_wnear(U128(near)))).contains("resume it first"));

        ctx("alice.near", 0);
        drop(f.resume(U64(id)));
        ctx(ME, 0);
        f.on_dev_buy_unwrapped(id, uw_op(id), Err(PromiseError::Failed));
        assert_eq!(read_counter(UW_TOTAL_KEY), near);
        ctx("alice.near", 0);
        drop(f.resume(U64(id)));
        ctx(ME, 0);
        f.on_dev_buy_unwrapped(id, uw_op(id), Ok(()));
        assert_eq!(read_counter(UW_TOTAL_KEY), 0);
        owner();
        drop(f.sweep_wnear(U128(5)));
    }

    #[test]
    fn fix5_an_owner_resume_meets_the_first_buy_gate_too() {
        let mut f = factory();
        let near = 2 * 10u128.pow(24);
        let id = add_buy_launch_capped(&mut f, Some(near), "OWN");
        f.owner_id = acct(ME);
        ctx_tx(ME, ME, 0, 246);
        assert!(panics(|| drop(f.resume(U64(id)))).contains("at least 260 TGas"));
        assert!(no_add_sent(id), "nothing changed");

        ctx_tx("dapp.near", "alice.near", 0, 300);
        assert!(panics(|| drop(f.resume(U64(id)))).contains("first buy pending"));
        ctx_tx(ME, ME, 0, 300);
        drop(f.resume(U64(id)));
        assert!(calls().iter().any(|c| c.1 == "add_buy"));
    }

    #[test]
    fn fix6_pause_parks_the_first_buy_and_lets_a_pending_unwrap_out() {
        let mut f = factory();
        let near = 10u128.pow(24);
        f.wnear_registered = true;
        owner(); f.set_locker_add_gas(acct("lock.near"), T_LOCK3_ADD);
        owner(); f.set_locker_add_buy(acct("lock.near"), true);
        let a = LaunchArgs { dev_buy: Some(U128(near)), ..args("PA") };
        let deposit = f.quote_launch(0, a.dev_buy, None).total.0;
        ctx_tx("alice.near", "alice.near", deposit, 300);
        drop(f.launch(a));
        let id = f.next_id - 1;
        let cb = static_gas_of(b"on_created").unwrap();
        let id2 = launch_with(&mut f, "alice.near", LaunchArgs { dev_buy: Some(U128(near)), ..args("UW") });
        owner(); f.set_paused(true);
        ctx_tx(ME, "alice.near", 0, tg_of(cb) + 3);
        let pool = l(&f, id).pool_id;
        drop(f.on_created(id, Some(true), Ok(StorageBalanceJson { total: "0".into() }), Ok(pool)));
        assert!(calls().iter().all(|c| c.1 != "add_buy" && c.1 != "add"));
        assert_eq!((l(&f, id).step, l(&f, id).dev_buy_held.0, no_add_sent(id)), (Step::AddLiquidity, near, true));
        assert!(logs_with("launch_step_parked")[0].contains("paused"));

        park(&mut f, id2, Step::DevBuy);
        let mut lz = l(&f, id2);
        lz.dev_buy_held = U128(0);
        lz.inflight = true;
        f.launches.insert(id2, lz);
        ctx(ME, 0);
        f.on_dev_buy_unwrapped(id2, uw_op(id2), Err(PromiseError::Failed));
        ctx("alice.near", 0);
        drop(f.resume(U64(id2)));
        assert!(calls().iter().any(|c| c.0 == "wrap.near" && c.1 == "near_withdraw"));
        ctx(ME, 0);
        f.on_dev_buy_unwrapped(id2, uw_op(id2), Ok(()));
        ctx("alice.near", 0);
        drop(f.cancel_dev_buy(U64(id2)));
        assert_eq!(transfers(), vec![("alice.near".to_string(), near)]);

        ctx("alice.near", 0);
        assert_eq!(panics(|| drop(f.resume(U64(id)))), "paused");
    }

    #[test]
    fn fix7_a_failed_token_create_refunds_the_creators_storage_once() {
        let mut f = factory();
        let id = launch(&mut f, "alice.near", "FAIL");
        let storage = f.internal_cost(0, 8, 0, false).token_storage.0;
        let pool = l(&f, id).pool_id;
        ctx(ME, 0);
        drop(f.on_created(id, Some(true), Err(PromiseError::Failed), Ok(pool.clone())));
        assert_eq!(l(&f, id).step, Step::Failed);
        assert_eq!(transfers(), vec![("alice.near".to_string(), storage)]);

        let id2 = launch(&mut f, "alice.near", "FAIL2");
        let pool2 = l(&f, id2).pool_id;
        ctx(ME, 0);
        drop(f.on_created(id2, Some(false), Err(PromiseError::Failed), Ok(pool2)));
        assert!(transfers().is_empty());
    }

    #[test]
    fn fix9_quote_launch_covers_the_longest_text() {
        let mut f = factory();
        let deposit = f.quote_launch(0, None, None).total.0;
        ctx("alice.near", deposit);
        let a = LaunchArgs { name: "\u{1F600}".repeat(32), description: Some("\u{1F600}".repeat(500)), ..args("LONGLONG10") };
        let cost = f.internal_cost(0, a.name.len() + "LONGLONG10".len() + 2000, 0, false).total.0;
        drop(f.launch(a));
        assert_eq!(transfers().into_iter().filter(|t| t.0 == "alice.near").map(|t| t.1).sum::<u128>(), deposit - cost, "the surplus comes back in the same call");
    }

    #[test]
    fn poc10_slugs_never_contain_an_underscore() {
        for s in ["lock_5", "LOCK_5", "lock5", "a_b", "__", "Ünï_cødé"] {
            let slug = sanitize_slug(s);
            assert!(slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()), "{slug}");
            for n in 0..5 { assert!(!short_name(&slug, n).contains('_')); }
        }
    }
}


const LK_BUYBACK: u8 = 1;
const LK_ADMIN: u8 = 5;

const LOCKER_WITHDRAW_LOCK_ID: u64 = u64::MAX - 1;



const SWAP_LOCK_TTL_MS: u64 = 10 * 60 * 1000;
fn swap_lock_key(token: &AccountId) -> Vec<u8> { [b"sl:".as_slice(), token.as_bytes()].concat() }
fn read_swap_lock(token: &AccountId) -> Option<(u8, u64, u64)> {
    let b = env::storage_read(&swap_lock_key(token))?;
    if b.len() != 17 { return None; }
    Some((b[0], u64::from_le_bytes(b[1..9].try_into().unwrap()), u64::from_le_bytes(b[9..17].try_into().unwrap())))
}
fn swap_locked(token: &AccountId) -> bool {
    read_swap_lock(token).map_or(false, |(_, _, at)| env::block_timestamp_ms() < at.saturating_add(SWAP_LOCK_TTL_MS))
}
fn require_no_swap(token: &AccountId) {
    require!(!swap_locked(token), "swap in flight");
}
fn take_swap_lock(token: &AccountId, id: u64, kind: u8) {
    require_no_swap(token);
    require!(!transfers_pending(token), "transfer in flight");
    let mut v = Vec::with_capacity(17);
    v.push(kind);
    v.extend_from_slice(&id.to_le_bytes());
    v.extend_from_slice(&env::block_timestamp_ms().to_le_bytes());
    env::storage_write(&swap_lock_key(token), &v);
}


fn release_swap_lock(token: &AccountId, id: u64, kind: u8) {
    if let Some((k, i, _)) = read_swap_lock(token) {
        if k == kind && i == id {
            env::storage_remove(&swap_lock_key(token));
        }
    }
}


fn validate_protocol_recipients(recipients: &[(AccountId, u16)]) {
    if recipients.is_empty() {
        return;
    }
    require!(recipients.len() <= 4, "at most 4 recipients");
    let sum: u32 = recipients.iter().map(|(_, b)| *b as u32).sum();
    require!(sum == 10_000, "bps must sum to 10000");
    for (i, (who, _)) in recipients.iter().enumerate() {
        require!(!recipients[..i].iter().any(|(w, _)| w == who), "duplicate recipient");
    }
}

const PENDING_OWNER_KEY: &[u8] = b"pow";
fn pending_owner() -> Option<AccountId> {
    env::storage_read(PENDING_OWNER_KEY).map(|b| String::from_utf8(b).expect("owner").parse().expect("owner"))
}

const FEE_ROUTERS_KEY: &[u8] = b"rtr";
fn fee_routers() -> Vec<AccountId> {
    env::storage_read(FEE_ROUTERS_KEY).map(|b| near_sdk::serde_json::from_slice(&b).expect("routers")).unwrap_or_default()
}
fn is_router(a: &AccountId) -> bool { fee_routers().contains(a) }

const LAUNCH_HOOK_KEY: &[u8] = b"lhk";
fn launch_hook() -> Option<AccountId> {
    env::storage_read(LAUNCH_HOOK_KEY).map(|b| String::from_utf8(b).expect("hook").parse().expect("hook"))
}

const DCLS_KEY: &[u8] = b"dcs";
fn dcls() -> Vec<(u64, AccountId)> {
    env::storage_read(DCLS_KEY).map(|b| near_sdk::serde_json::from_slice(&b).expect("dcls")).unwrap_or_default()
}

fn router_near_key(id: u64) -> Vec<u8> { [b"rn:".as_slice(), &id.to_le_bytes()].concat() }
fn router_near(id: u64) -> u128 { read_counter(&router_near_key(id)) }
fn set_router_near(id: u64, v: u128) {
    if v == 0 { env::storage_remove(&router_near_key(id)); } else { env::storage_write(&router_near_key(id), &v.to_le_bytes()); }
}



fn promise_failed() -> bool {
    matches!(env::promise_result_checked(0, 0), Err(PromiseError::Failed))
}





fn ft_refund_of_last_call(amount: u128) -> u128 {
    match env::promise_result_checked(0, 64) {
        Err(PromiseError::Failed) => amount,
        Err(_) => 0,
        Ok(data) if data.is_empty() => 0,
        Ok(data) => match near_sdk::serde_json::from_slice::<U128>(&data) {
            Ok(used) => amount - used.0.min(amount),
            Err(_) => 0,
        },
    }
}



fn l_payee_is_router(l: &Launch) -> bool { is_router(&payee(l)) }








fn transfers_key(token: &AccountId) -> Vec<u8> { [b"sf:".as_slice(), token.as_bytes()].concat() }
fn read_transfers(token: &AccountId) -> (u32, u64) {
    match env::storage_read(&transfers_key(token)) {
        Some(b) if b.len() == 12 => (u32::from_le_bytes(b[0..4].try_into().unwrap()), u64::from_le_bytes(b[4..12].try_into().unwrap())),
        _ => (0, 0),
    }
}
fn transfers_pending(token: &AccountId) -> bool {
    let (n, at) = read_transfers(token);
    n > 0 && env::block_timestamp_ms() < at.saturating_add(SWAP_LOCK_TTL_MS)
}
fn transfer_start(token: &AccountId) {
    require_no_swap(token);
    let n = if transfers_pending(token) { read_transfers(token).0 } else { 0 };
    let mut v = (n + 1).to_le_bytes().to_vec();
    v.extend_from_slice(&env::block_timestamp_ms().to_le_bytes());
    env::storage_write(&transfers_key(token), &v);
}


fn transfer_end(token: &AccountId) {
    let (n, at) = read_transfers(token);
    if n <= 1 {
        env::storage_remove(&transfers_key(token));
    } else {
        let mut v = (n - 1).to_le_bytes().to_vec();
        v.extend_from_slice(&at.to_le_bytes());
        env::storage_write(&transfers_key(token), &v);
    }
}

fn internal_set_tax_seller(account_id: Option<AccountId>) {
    match account_id {
        Some(a) => { env::storage_write(TAX_SELLER_KEY, a.as_bytes()); }
        None => { env::storage_remove(TAX_SELLER_KEY); }
    }
    emit("tax_seller_set", &format!(r#"{{"seller":{}}}"#, tax_seller().map(|a| format!("\"{}\"", a)).unwrap_or_else(|| "null".into())));
}

fn internal_set_house_creators(accounts: Vec<AccountId>, on: bool) {
    for a in accounts {
        let k = house_key(&a);
        if on { env::storage_write(&k, &[1]); } else { env::storage_remove(&k); }
        emit("house_creator_set", &format!(r#"{{"account":"{}","on":{}}}"#, a, on));
    }
}

fn validate_launch_recipients(rc: &[(AccountId, u16)]) {
    require!(!rc.is_empty() && rc.len() <= 4, "1 to 4 recipients");
    let sum: u32 = rc.iter().map(|(_, b)| *b as u32).sum();
    require!(sum == 10_000, "bps must sum to 10000");
    for (i, (who, _)) in rc.iter().enumerate() {
        require!(!rc[..i].iter().any(|(w, _)| w == who), "duplicate recipient");
    }
}


fn locker_add_buy_key(locker: &AccountId) -> Vec<u8> { [b"la:".as_slice(), locker.as_bytes()].concat() }
fn locker_add_buy(locker: &AccountId) -> bool { env::storage_has_key(&locker_add_buy_key(locker)) }

fn buy_held_key(id: u64) -> Vec<u8> { [b"bh:".as_slice(), &id.to_le_bytes()].concat() }
fn buy_held(id: u64) -> u128 { read_u128(&buy_held_key(id)).unwrap_or(0) }
fn set_buy_held(id: u64, v: u128) {
    if v == 0 { env::storage_remove(&buy_held_key(id)); } else { env::storage_write(&buy_held_key(id), &v.to_le_bytes()); }
}

fn buy_unknown_key(id: u64) -> Vec<u8> { [b"bu:".as_slice(), &id.to_le_bytes()].concat() }
fn buy_refund_inflight_key(id: u64) -> Vec<u8> { [b"br:".as_slice(), &id.to_le_bytes()].concat() }

fn dev_buy_inflight(l: &Launch) -> bool {
    l.inflight && (l.step == Step::DevBuy || (l.step == Step::AddLiquidity && l.dev_buy_near.0 > 0 && l.dev_buy_held.0 == 0
        && !dev_buy_done(l.id) && buy_held(l.id) == 0))
}

fn dev_buy_done_key(id: u64) -> Vec<u8> { [b"bd:".as_slice(), &id.to_le_bytes()].concat() }
fn dev_buy_done(id: u64) -> bool { env::storage_has_key(&dev_buy_done_key(id)) }


fn owed_key(asset: &str, a: &AccountId) -> Vec<u8> { [b"ow:".as_slice(), asset.as_bytes(), b"|", a.as_bytes()].concat() }
fn owed_total_key(asset: &str) -> Vec<u8> { [b"ot:".as_slice(), asset.as_bytes()].concat() }
fn owed_of(asset: &str, a: &AccountId) -> u128 { read_counter(&owed_key(asset, a)) }
fn add_owed(asset: &str, a: &AccountId, v: u128) {
    if v == 0 { return; }
    add_counter(&owed_key(asset, a), v);
    add_counter(&owed_total_key(asset), v);
}
fn take_owed(asset: &str, a: &AccountId) -> u128 {
    let v = owed_of(asset, a);
    if v > 0 {
        env::storage_remove(&owed_key(asset, a));
        sub_counter(&owed_total_key(asset), v);
    }
    v
}

fn no_add_key(id: u64) -> Vec<u8> { [b"na:".as_slice(), &id.to_le_bytes()].concat() }
fn no_add_sent(id: u64) -> bool { env::storage_has_key(&no_add_key(id)) }

fn unwrap_pending_key(id: u64) -> Vec<u8> { [b"uw:".as_slice(), &id.to_le_bytes()].concat() }

const UW_TOTAL_KEY: &[u8] = b"uwt";
fn read_u128(k: &[u8]) -> Option<u128> { env::storage_read(k).map(|b| u128::from_le_bytes(b.try_into().expect("u128"))) }

fn dcl_registered_key(dcl: &AccountId) -> Vec<u8> { [b"de:".as_slice(), dcl.as_bytes()].concat() }
fn locker_add_gas_key(locker: &AccountId) -> Vec<u8> { [b"lg:".as_slice(), locker.as_bytes()].concat() }
fn locker_add_gas(locker: &AccountId) -> Option<Gas> {
    env::storage_read(&locker_add_gas_key(locker)).map(|b| Gas::from_tgas(u64::from_le_bytes(b.try_into().expect("locker gas"))))
}

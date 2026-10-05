




















#![no_std]
#![allow(clippy::missing_safety_doc, static_mut_refs)]
extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};
use near_sys as sys;




struct Bump;
static HEAP: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Bump {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut p = HEAP.load(Ordering::Relaxed);
        if p == 0 {
            p = core::arch::wasm32::memory_size(0) * 65536;
        }
        let start = (p + layout.align() - 1) & !(layout.align() - 1);
        let end = start + layout.size();
        let have = core::arch::wasm32::memory_size(0) * 65536;
        if end > have {
            let pages = (end - have + 65535) / 65536;
            if core::arch::wasm32::memory_grow(0, pages) == usize::MAX {
                return core::ptr::null_mut();
            }
        }
        HEAP.store(end, Ordering::Relaxed);
        start as *mut u8
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}
#[global_allocator]
static A: Bump = Bump;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { sys::panic() }
}




const REG: u64 = 0;
const TGAS: u64 = 1_000_000_000_000;
const GAS_RESOLVE: u64 = 5 * TGAS;
const GAS_RESERVE: u64 = 15 * TGAS;

const REG_COST: u128 = 1_250_000_000_000_000_000_000;

const MAX_TAX_BPS: u128 = 400;

const BYTE_COST: u128 = 10_000_000_000_000_000_000;

const AUTO_REG_MARGIN: u128 = REG_COST;

fn paid_record_cost(a: &[u8]) -> u128 {
    (40 + 1 + a.len() as u128 + 16) * BYTE_COST
}


fn unbacked() -> u128 {
    sread(b"u").map_or(0, |v| {
        let mut b = [0u8; 16];
        b.copy_from_slice(&v[..16]);
        u128::from_le_bytes(b)
    })
}
fn set_unbacked(n: u128) {
    swrite(b"u", &n.to_le_bytes());
}

fn reg(r: u64) -> Vec<u8> {
    unsafe {
        let len = sys::register_len(r);
        if len == u64::MAX {
            return Vec::new();
        }
        let mut v = vec![0u8; len as usize];
        sys::read_register(r, v.as_mut_ptr() as u64);
        v
    }
}
fn input() -> Vec<u8> {
    unsafe { sys::input(REG) };
    reg(REG)
}
fn predecessor() -> Vec<u8> {
    unsafe { sys::predecessor_account_id(REG) };
    reg(REG)
}
fn current() -> Vec<u8> {
    unsafe { sys::current_account_id(REG) };
    reg(REG)
}
fn attached() -> u128 {
    let mut b = [0u8; 16];
    unsafe { sys::attached_deposit(b.as_mut_ptr() as u64) };
    u128::from_le_bytes(b)
}
fn account_balance() -> u128 {
    let mut b = [0u8; 16];
    unsafe { sys::account_balance(b.as_mut_ptr() as u64) };
    u128::from_le_bytes(b)
}
fn sread(key: &[u8]) -> Option<Vec<u8>> {
    unsafe {
        if sys::storage_read(key.len() as u64, key.as_ptr() as u64, REG) == 1 {
            Some(reg(REG))
        } else {
            None
        }
    }
}
fn swrite(key: &[u8], val: &[u8]) {
    unsafe { sys::storage_write(key.len() as u64, key.as_ptr() as u64, val.len() as u64, val.as_ptr() as u64, REG) };
}
fn shas(key: &[u8]) -> bool {
    unsafe { sys::storage_has_key(key.len() as u64, key.as_ptr() as u64) == 1 }
}
fn sremove(key: &[u8]) {
    unsafe { sys::storage_remove(key.len() as u64, key.as_ptr() as u64, REG) };
}
fn ret(v: &[u8]) {
    unsafe { sys::value_return(v.len() as u64, v.as_ptr() as u64) }
}
fn log(v: &[u8]) {
    unsafe { sys::log_utf8(v.len() as u64, v.as_ptr() as u64) }
}
fn die(msg: &str) -> ! {
    unsafe { sys::panic_utf8(msg.len() as u64, msg.as_ptr() as u64) }
}
fn block_ms() -> u64 {
    unsafe { sys::block_timestamp() / 1_000_000 }
}
fn transfer_near(to: &[u8], amount: u128) {
    let a = amount.to_le_bytes();
    unsafe {
        let p = sys::promise_batch_create(to.len() as u64, to.as_ptr() as u64);
        sys::promise_batch_action_transfer(p, a.as_ptr() as u64);
    }
}
fn one_yocto() {
    if attached() != 1 {
        die("Requires attached deposit of exactly 1 yoctoNEAR");
    }
}




fn ws(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && matches!(b[i], b' ' | b'\n' | b'\r' | b'\t') {
        i += 1;
    }
    i
}

fn str_end(b: &[u8], i: usize) -> usize {
    let mut j = i + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 2,
            b'"' => return j + 1,
            _ => j += 1,
        }
    }
    die("bad json string")
}
fn value_end(b: &[u8], i: usize) -> usize {
    if i >= b.len() {
        die("bad json");
    }
    match b[i] {
        b'"' => str_end(b, i),
        b'{' | b'[' => {
            let mut depth = 0i32;
            let mut j = i;
            while j < b.len() {
                match b[j] {
                    b'"' => j = str_end(b, j),
                    b'{' | b'[' => {
                        depth += 1;
                        j += 1
                    }
                    b'}' | b']' => {
                        depth -= 1;
                        j += 1;
                        if depth == 0 {
                            return j;
                        }
                    }
                    _ => j += 1,
                }
            }
            die("bad json nesting")
        }
        _ => {
            let mut j = i;
            while j < b.len() && !matches!(b[j], b',' | b'}' | b']' | b' ' | b'\n' | b'\r' | b'\t') {
                j += 1;
            }
            if j == i {
                die("bad json");
            }
            j
        }
    }
}




#[inline(always)]
fn find_all<'a, const N: usize>(b: &'a [u8], keys: [&str; N]) -> [Option<&'a [u8]>; N] {
    let mut found = [None; N];
    find_into(b, &keys, &mut found);
    found
}

fn find_into<'a>(b: &'a [u8], keys: &[&str], found: &mut [Option<&'a [u8]>]) {
    let mut i = ws(b, 0);
    if i >= b.len() || b[i] != b'{' {
        die("expected json object");
    }
    i = ws(b, i + 1);
    if i < b.len() && b[i] != b'}' {
        loop {
            if i >= b.len() || b[i] != b'"' {
                die("bad json key");
            }
            let ke = str_end(b, i);
            let k = &b[i + 1..ke - 1];
            if k.contains(&b'\\') {
                die("bad json key");
            }
            i = ws(b, ke);
            if i >= b.len() || b[i] != b':' {
                die("bad json");
            }
            i = ws(b, i + 1);
            let ve = value_end(b, i);
            for (n, key) in keys.iter().enumerate() {
                if k == key.as_bytes() {
                    if found[n].is_some() {
                        die("duplicate json key");
                    }
                    found[n] = Some(&b[i..ve]);
                }
            }
            i = ws(b, ve);
            if i < b.len() && b[i] == b',' {
                i = ws(b, i + 1);
            } else {
                break;
            }
        }
    }
    if i >= b.len() || b[i] != b'}' || ws(b, i + 1) != b.len() {
        die("bad json");
    }
}

fn find<'a>(b: &'a [u8], key: &str) -> Option<&'a [u8]> {
    let [v] = find_all(b, [key]);
    v
}

fn find_str<'a>(b: &'a [u8], key: &str) -> Option<&'a [u8]> {
    str_of(find(b, key))
}

fn str_of(v: Option<&[u8]>) -> Option<&[u8]> {
    let v = v?;
    if v == b"null" {
        return None;
    }
    if v.len() < 2 || v[0] != b'"' {
        die("expected json string");
    }
    Some(&v[1..v.len() - 1])
}
fn parse_u128(d: &[u8]) -> u128 {
    if d.is_empty() || d.len() > 39 {
        die("bad amount");
    }
    let mut n: u128 = 0;
    for &c in d {
        if !c.is_ascii_digit() {
            die("bad amount");
        }
        n = n.checked_mul(10).and_then(|n| n.checked_add((c - b'0') as u128)).unwrap_or_else(|| die("amount overflow"));
    }
    n
}

fn check_account(a: &[u8]) {
    let mut sep = true;
    let ok = a.len() >= 2 && a.len() <= 64 && a.iter().all(|&c| {
        let s = matches!(c, b'.' | b'_' | b'-');
        let good = if s { !sep } else { c.is_ascii_lowercase() || c.is_ascii_digit() };
        sep = s;
        good
    });
    if !ok || sep {
        die("invalid account id");
    }
}



fn check_json_str(s: &[u8]) {
    let hex = |i: usize| -> u32 {
        if i + 4 > s.len() {
            die("bad json string");
        }
        s[i..i + 4].iter().fold(0u32, |n, &c| {
            n * 16
                + match c {
                    b'0'..=b'9' => c - b'0',
                    b'a'..=b'f' => c - b'a' + 10,
                    b'A'..=b'F' => c - b'A' + 10,
                    _ => die("bad json string"),
                } as u32
        })
    };
    let mut i = 0;
    while i < s.len() {
        match s[i] {
            0..=0x1f => die("bad json string"),
            b'\\' => match s.get(i + 1) {
                Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => i += 2,
                Some(b'u') => {
                    let u = hex(i + 2);
                    i += 6;
                    if (0xdc00..0xe000).contains(&u) {
                        die("bad json string");
                    }
                    if (0xd800..0xdc00).contains(&u) {
                        if s.get(i) != Some(&b'\\') || s.get(i + 1) != Some(&b'u') || !(0xdc00..0xe000).contains(&hex(i + 2)) {
                            die("bad json string");
                        }
                        i += 6;
                    }
                }
                _ => die("bad json string"),
            },
            c @ 0x80.. => {

                let (n, lo, hi) = match c {
                    0xc2..=0xdf => (1, 0x80, 0xbf),
                    0xe0 => (2, 0xa0, 0xbf),
                    0xe1..=0xec | 0xee..=0xef => (2, 0x80, 0xbf),
                    0xed => (2, 0x80, 0x9f),
                    0xf0 => (3, 0x90, 0xbf),
                    0xf1..=0xf3 => (3, 0x80, 0xbf),
                    0xf4 => (3, 0x80, 0x8f),
                    _ => die("bad json string"),
                };
                for k in 1..=n {
                    let b = s.get(i + k).copied().unwrap_or(0);
                    let (l, h) = if k == 1 { (lo, hi) } else { (0x80, 0xbf) };
                    if b < l || b > h {
                        die("bad json string");
                    }
                }
                i += n + 1;
            }
            _ => i += 1,
        }
    }
}

const MAX_MEMO: usize = 1024;
fn check_memo(m: Option<&[u8]>) {
    if let Some(m) = m {
        if m.len() > MAX_MEMO {
            die("memo too long");
        }
        check_json_str(m);
    }
}




struct Buf(Vec<u8>);
impl Buf {
    fn new() -> Self {
        Buf(Vec::with_capacity(256))
    }
    fn s(&mut self, s: &str) -> &mut Self {
        self.0.extend_from_slice(s.as_bytes());
        self
    }
    fn b(&mut self, b: &[u8]) -> &mut Self {
        self.0.extend_from_slice(b);
        self
    }
    fn n(&mut self, mut n: u128) -> &mut Self {
        let mut tmp = [0u8; 40];
        let mut i = 40;
        loop {
            i -= 1;
            tmp[i] = b'0' + (n % 10) as u8;
            n /= 10;
            if n == 0 {
                break;
            }
        }
        self.0.extend_from_slice(&tmp[i..]);
        self
    }

    fn qn(&mut self, n: u128) -> &mut Self {
        self.s("\"").n(n).s("\"")
    }
}




fn bal_key(a: &[u8]) -> Vec<u8> {
    let mut k = Vec::with_capacity(a.len() + 1);
    k.push(b'b');
    k.extend_from_slice(a);
    k
}
fn balance(a: &[u8]) -> Option<u128> {
    sread(&bal_key(a)).map(|v| {
        let mut b = [0u8; 16];
        b.copy_from_slice(&v[..16]);
        u128::from_le_bytes(b)
    })
}

fn auto_flag(k: &[u8]) -> bool {
    sread(k).map(|v| v.len() > 16 && v[16] == 1).unwrap_or(false)
}
fn is_auto(a: &[u8]) -> bool {
    auto_flag(&bal_key(a))
}

fn set_balance(a: &[u8], n: u128) {
    let k = bal_key(a);
    if auto_flag(&k) {
        let mut v = [0u8; 17];
        v[..16].copy_from_slice(&n.to_le_bytes());
        v[16] = 1;
        swrite(&k, &v);
    } else {
        swrite(&k, &n.to_le_bytes());
    }
}
fn supply() -> u128 {
    let v = sread(b"s").unwrap_or_else(|| die("not initialized"));
    let mut b = [0u8; 16];
    b.copy_from_slice(&v[..16]);
    u128::from_le_bytes(b)
}
fn set_supply(n: u128) {
    swrite(b"s", &n.to_le_bytes());
}

fn in_flight() -> u128 {
    sread(b"p").map(|v| {
        let mut b = [0u8; 16];
        b.copy_from_slice(&v[..16]);
        u128::from_le_bytes(b)
    })
    .unwrap_or(0)
}

fn set_in_flight(n: u128) {
    if n == 0 {
        sremove(b"p");
    } else {
        swrite(b"p", &n.to_le_bytes());
    }
}

fn ev_transfer(from: &[u8], to: &[u8], amount: u128, memo: Option<&[u8]>) {
    let mut o = Buf::new();
    o.s(r#"EVENT_JSON:{"standard":"nep141","version":"1.0.0","event":"ft_transfer","data":[{"old_owner_id":""#)
        .b(from)
        .s(r#"","new_owner_id":""#)
        .b(to)
        .s(r#"","amount":""#)
        .n(amount)
        .s("\"");
    if let Some(m) = memo {
        o.s(r#","memo":""#).b(m).s("\"");
    }
    o.s("}]}");
    log(&o.0);
}
fn ev_supply(event: &str, who: &[u8], amount: u128, memo: &str) {
    let mut o = Buf::new();
    o.s(r#"EVENT_JSON:{"standard":"nep141","version":"1.0.0","event":""#)
        .s(event)
        .s(r#"","data":[{"owner_id":""#)
        .b(who)
        .s(r#"","amount":""#)
        .n(amount)
        .s("\"");
    if !memo.is_empty() {
        o.s(r#","memo":""#).s(memo).s("\"");
    }
    o.s("}]}");
    log(&o.0);
}





fn check_max_wallet(from: &[u8], to: &[u8], new_balance: u128) {
    let Some(rules) = sread(b"r") else { return };
    let until = find_str(&rules, "until_ms").map(parse_u128).unwrap_or(0) as u64;
    if block_ms() >= until {
        return;
    }
    let bps = find(&rules, "max_wallet_bps").map(parse_u128).unwrap_or(0);
    if bps == 0 {
        return;
    }
    if let Some(ex) = find(&rules, "exempt") {

        let mut needle = Vec::with_capacity(to.len() + 2);
        needle.push(b'"');
        needle.extend_from_slice(to);
        needle.push(b'"');
        if ex.windows(needle.len()).any(|w| w == needle.as_slice()) {
            return;
        }
    }
    let cap = supply() / 10_000 * bps;

    if new_balance > cap && !is_pair(from) && !is_pair(to) {
        die("Launch window: a wallet may not hold more than the max wallet share yet");
    }
}


fn list_has(list: Option<&[u8]>, id: &[u8]) -> bool {
    let Some(l) = list else { return false };
    let mut needle = Vec::with_capacity(id.len() + 2);
    needle.push(b'"');
    needle.extend_from_slice(id);
    needle.push(b'"');
    l.windows(needle.len()).any(|w| w == needle.as_slice())
}


fn is_pair(a: &[u8]) -> bool {
    sread(b"t").map(|t| list_has(find(&t, "pairs"), a)).unwrap_or(false)
}


fn tax_for(from: &[u8], to: &[u8], amount: u128) -> u128 {
    let Some(t) = sread(b"t") else { return 0 };
    let [pairs, admin, ex, buy, sell] = find_all(&t, ["pairs", "admin", "exempt", "buy_bps", "sell_bps"]);
    let admin = str_of(admin).unwrap_or(b"");
    let me = current();
    let exempt = |a: &[u8]| a == admin || a == me.as_slice() || list_has(ex, a);
    let bps = if list_has(pairs, from) && !exempt(to) {
        buy.map(parse_u128).unwrap_or(0)
    } else if list_has(pairs, to) && !exempt(from) {
        sell.map(parse_u128).unwrap_or(0)
    } else {
        0
    };
    if bps == 0 {
        return 0;
    }
    amount.checked_mul(bps.min(MAX_TAX_BPS)).map(|x| x / 10_000).unwrap_or_else(|| amount / 10_000 * bps.min(MAX_TAX_BPS))
}



fn mul_div(a: u128, b: u128, c: u128) -> u128 {
    match a.checked_mul(b) {
        Some(x) => x / c,
        None => {
            let s = (256 - a.leading_zeros() - b.leading_zeros()).saturating_sub(128);
            (a * (b >> s)).checked_div(c >> s).unwrap_or(0)
        }
    }
}






fn auto_register(to: &[u8]) -> bool {

    let bytes = unsafe { sys::storage_usage() } as u128 + 40 + 1 + to.len() as u128 + 17;
    if account_balance() < bytes * BYTE_COST + AUTO_REG_MARGIN + unbacked() {
        return false;
    }
    let mut v = [0u8; 17];
    v[16] = 1;
    swrite(&bal_key(to), &v);
    let mut o = Buf::new();
    o.s("auto-registered ").b(to).s(" for a swap payout (storage paid by the token)");
    log(&o.0);
    true
}



fn internal_transfer(from: &[u8], to: &[u8], amount: u128, memo: Option<&[u8]>) -> (u128, u128) {
    check_account(to);
    if from == to {
        die("Sender and receiver should be different");
    }


    if to == current().as_slice() {
        die("The token account does not take transfers");
    }
    if amount == 0 {
        die("The amount should be a positive number");
    }
    let fb = balance(from).unwrap_or_else(|| die("The sender is not registered"));
    if fb < amount {
        die("The account doesn't have enough balance");
    }
    let tb = match balance(to) {
        Some(b) => b,
        None if is_pair(from) && auto_register(to) => 0,
        None => die("The receiver is not registered"),
    };
    let tax = tax_for(from, to, amount);
    let net = amount - tax;
    let nb = tb.checked_add(net).unwrap_or_else(|| die("Balance overflow"));
    check_max_wallet(from, to, nb);
    set_balance(from, fb - amount);
    set_balance(to, nb);
    ev_transfer(from, to, net, memo);
    if tax > 0 {
        let me = current();
        set_balance(&me, balance(&me).unwrap_or(0) + tax);
        ev_transfer(from, &me, tax, Some(b"tax"));
    }
    (net, tax)
}

fn storage_balance_json(registered: bool) -> Vec<u8> {
    if !registered {
        return b"null".to_vec();
    }
    let mut o = Buf::new();
    o.s(r#"{"total":""#).n(REG_COST).s(r#"","available":"0"}"#);
    o.0
}





#[no_mangle]
pub extern "C" fn new() {
    if shas(b"s") {
        die("Already initialized");
    }
    let inp = input();

    let [owner, total, meta, rules, tax] = find_all(&inp, ["owner_id", "total_supply", "metadata", "rules", "tax"]);
    let owner = str_of(owner).unwrap_or_else(|| die("owner_id"));
    check_account(owner);
    let total = parse_u128(str_of(total).unwrap_or_else(|| die("total_supply")));
    if total == 0 {
        die("total_supply must be > 0");
    }
    let meta = meta.unwrap_or_else(|| die("metadata"));
    if meta.first() != Some(&b'{') {
        die("metadata must be an object");
    }
    let [spec, name, symbol, decimals] = find_all(meta, ["spec", "name", "symbol", "decimals"]);
    if str_of(spec) != Some(b"ft-1.0.0") {
        die("metadata.spec must be ft-1.0.0");
    }
    if str_of(name).map(|n| n.is_empty()).unwrap_or(true) || str_of(symbol).map(|s| s.is_empty()).unwrap_or(true) {
        die("metadata.name/symbol required");
    }
    if decimals.is_none() {
        die("metadata.decimals required");
    }
    swrite(b"m", meta);
    if let Some(rules) = rules {
        if rules != b"null" {
            if rules.first() != Some(&b'{') {
                die("rules must be an object");
            }


            let [until, bps, _] = find_all(rules, ["until_ms", "max_wallet_bps", "exempt"]);
            str_of(until).map(parse_u128);
            bps.map(parse_u128);
            swrite(b"r", rules);
        }
    }
    if let Some(tax) = tax {
        if tax != b"null" {
            if tax.first() != Some(&b'{') {
                die("tax must be an object");
            }
            let [buy, sell, admin, pairs, _] = find_all(tax, ["buy_bps", "sell_bps", "admin", "pairs", "exempt"]);
            for v in [buy, sell] {
                if v.map(parse_u128).unwrap_or(0) > MAX_TAX_BPS {
                    die("tax: max 400 bps a side");
                }
            }
            let admin = str_of(admin).unwrap_or_else(|| die("tax.admin"));
            check_account(admin);
            if pairs.map(|p| p.first() != Some(&b'[')).unwrap_or(true) {
                die("tax.pairs must be an array");
            }
            swrite(b"t", tax);

            set_balance(&current(), 0);
            if balance(admin).is_none() {
                set_balance(admin, 0);
            }
        }
    }
    set_supply(total);
    set_balance(owner, total);
    ev_supply("ft_mint", owner, total, "fixed supply");
}


#[no_mangle]
pub extern "C" fn get_rules() {
    ret(&sread(b"r").unwrap_or_else(|| b"null".to_vec()));
}

#[no_mangle]
pub extern "C" fn ft_transfer() {
    one_yocto();
    let inp = input();
    let [to, amount, memo] = find_all(&inp, ["receiver_id", "amount", "memo"]);
    let to = str_of(to).unwrap_or_else(|| die("receiver_id"));
    let amount = parse_u128(str_of(amount).unwrap_or_else(|| die("amount")));
    let memo = str_of(memo);
    check_memo(memo);
    internal_transfer(&predecessor(), to, amount, memo);
}



#[no_mangle]
pub extern "C" fn get_tax() {
    let mut o = Buf::new();
    let pending = balance(&current()).unwrap_or(0).saturating_sub(in_flight());
    o.s(r#"{"tax":"#).b(&sread(b"t").unwrap_or_else(|| b"null".to_vec())).s(r#","pending":"#).qn(pending).s("}");
    ret(&o.0);
}




#[no_mangle]
pub extern "C" fn tax_take() {
    let t = sread(b"t").unwrap_or_else(|| die("no tax"));
    let admin = find_str(&t, "admin").unwrap_or_else(|| die("tax.admin"));
    let pred = predecessor();
    if pred.as_slice() != admin {
        die("admin only");
    }
    let me = current();
    let vault = balance(&me).unwrap_or(0);
    let amount = vault.saturating_sub(in_flight());
    if amount > 0 {
        set_balance(&me, vault - amount);
        set_balance(&pred, balance(&pred).unwrap_or(0) + amount);
        ev_transfer(&me, &pred, amount, Some(b"tax"));
    }
    let mut o = Buf::new();
    o.qn(amount);
    ret(&o.0);
}




#[no_mangle]
pub extern "C" fn tax_add_pair() {
    let t = sread(b"t").unwrap_or_else(|| die("no tax"));
    let [admin, pairs] = find_all(&t, ["admin", "pairs"]);
    let admin = str_of(admin).unwrap_or_else(|| die("tax.admin"));
    let pred = predecessor();
    if pred.as_slice() != admin {
        die("admin only");
    }
    let pairs = pairs.unwrap_or_else(|| die("tax.pairs"));
    if pairs.first() != Some(&b'[') {
        die("tax.pairs must be an array");
    }
    let inp = input();
    let pair = find_str(&inp, "pair").unwrap_or_else(|| die("pair"));
    check_account(pair);
    if list_has(Some(pairs), pair) {
        die("pair already listed");
    }

    let end = pairs.as_ptr() as usize - t.as_ptr() as usize + pairs.len();
    let empty = pairs[1..pairs.len() - 1].iter().all(|&c| matches!(c, b' ' | b'\n' | b'\r' | b'\t'));
    let mut o = Buf::new();
    o.b(&t[..end - 1]).s(if empty { "\"" } else { ",\"" }).b(pair).s("\"").b(&t[end - 1..]);
    swrite(b"t", &o.0);
    let mut l = Buf::new();
    l.s("tax: pair added ").b(pair);
    log(&l.0);
}

#[no_mangle]
pub extern "C" fn ft_transfer_call() {
    one_yocto();
    let inp = input();
    let [to, amount, memo, msg] = find_all(&inp, ["receiver_id", "amount", "memo", "msg"]);
    let to = str_of(to).unwrap_or_else(|| die("receiver_id"));
    let amount = parse_u128(str_of(amount).unwrap_or_else(|| die("amount")));
    let memo = str_of(memo);
    let msg = str_of(msg).unwrap_or_else(|| die("msg"));
    check_memo(memo);
    check_json_str(msg);
    let from = predecessor();



    let (amount, tax) = internal_transfer(&from, to, amount, memo);
    if tax > 0 {
        set_in_flight(in_flight() + tax);
    }

    let used = unsafe { sys::used_gas() };
    let prepaid = unsafe { sys::prepaid_gas() };
    let gas = prepaid.saturating_sub(used).saturating_sub(GAS_RESERVE).saturating_sub(GAS_RESOLVE);
    if gas < 5 * TGAS {
        die("More gas is required");
    }
    let mut a1 = Buf::new();
    a1.s(r#"{"sender_id":""#).b(&from).s(r#"","amount":""#).n(amount).s(r#"","msg":""#).b(msg).s("\"}");
    let mut a2 = Buf::new();
    a2.s(r#"{"sender_id":""#).b(&from).s(r#"","receiver_id":""#).b(to).s(r#"","amount":""#).n(amount).s(r#"","tax":""#).n(tax).s("\"}");
    let zero = 0u128.to_le_bytes();
    let me = current();
    unsafe {
        let p = sys::promise_batch_create(to.len() as u64, to.as_ptr() as u64);
        let m1 = b"ft_on_transfer";
        sys::promise_batch_action_function_call(p, m1.len() as u64, m1.as_ptr() as u64, a1.0.len() as u64, a1.0.as_ptr() as u64, zero.as_ptr() as u64, gas);
        let m2 = b"ft_resolve_transfer";
        let cb = sys::promise_then(p, me.len() as u64, me.as_ptr() as u64, m2.len() as u64, m2.as_ptr() as u64, a2.0.len() as u64, a2.0.as_ptr() as u64, zero.as_ptr() as u64, GAS_RESOLVE);
        sys::promise_return(cb);
    }
}




fn unused_of_receiver() -> Option<u128> {
    unsafe {
        if sys::promise_result(0, REG) != 1 || sys::register_len(REG) > 64 {
            return None;
        }
    }
    let v = reg(REG);
    let mut a = ws(&v, 0);
    let mut z = v.len();
    while z > a && matches!(v[z - 1], b' ' | b'\n' | b'\r' | b'\t') {
        z -= 1;
    }
    if z < a + 2 || v[a] != b'"' || v[z - 1] != b'"' {
        return None;
    }
    a += 1;
    z -= 1;
    if a < z && v[a] == b'+' {
        a += 1;
    }
    if a == z {
        return None;
    }
    let mut n: u128 = 0;
    for &c in &v[a..z] {
        if !c.is_ascii_digit() {
            return None;
        }
        n = n.checked_mul(10)?.checked_add((c - b'0') as u128)?;
    }
    Some(n)
}

#[no_mangle]
pub extern "C" fn ft_resolve_transfer() {
    if predecessor() != current() {
        die("Method ft_resolve_transfer is private");
    }
    let inp = input();
    let sender = find_str(&inp, "sender_id").unwrap_or_else(|| die("sender_id"));
    let receiver = find_str(&inp, "receiver_id").unwrap_or_else(|| die("receiver_id"));
    let amount = parse_u128(find_str(&inp, "amount").unwrap_or_else(|| die("amount")));



    let unused = unused_of_receiver().map_or(amount, |n| n.min(amount));
    let tax = find_str(&inp, "tax").map(parse_u128).unwrap_or(0);
    let me = current();


    let held = in_flight().saturating_sub(tax);


    let mut returned = 0u128;
    let mut back = 0u128;
    if unused > 0 {
        let rb = balance(receiver).unwrap_or(0);

        let free = if receiver == me.as_slice() { rb.saturating_sub(held) } else { rb };
        let refund = unused.min(free);
        if refund > 0 {
            set_balance(receiver, rb - refund);
            match balance(sender) {
                Some(sb) => {
                    set_balance(sender, sb + refund);
                    ev_transfer(receiver, sender, refund, Some(b"refund"));
                    returned = refund;
                }
                None => {
                    set_supply(supply() - refund);
                    ev_supply("ft_burn", receiver, refund, "refund to unregistered sender");
                }
            }
        }


        if tax > 0 && refund > 0 {
            let due = if refund == amount { tax } else { mul_div(tax, refund, amount) };
            let vault = balance(&me).unwrap_or(0);
            let give = due.min(vault.saturating_sub(held));
            if give > 0 {
                if let Some(sb) = balance(sender) {
                    set_balance(&me, vault - give);
                    set_balance(sender, sb + give);
                    ev_transfer(&me, sender, give, Some(b"tax refund"));
                    back = give;
                }
            }
        }
    }

    if tax > 0 {
        set_in_flight(held);
    }


    let mut o = Buf::new();
    o.qn(amount + tax - returned - back);
    ret(&o.0);
}

#[no_mangle]
pub extern "C" fn ft_total_supply() {
    let mut o = Buf::new();
    o.qn(supply());
    ret(&o.0);
}

#[no_mangle]
pub extern "C" fn ft_balance_of() {
    let inp = input();
    let a = find_str(&inp, "account_id").unwrap_or_else(|| die("account_id"));
    let mut o = Buf::new();
    o.qn(balance(a).unwrap_or(0));
    ret(&o.0);
}

#[no_mangle]
pub extern "C" fn ft_metadata() {
    ret(&sread(b"m").unwrap_or_else(|| die("not initialized")));
}

#[no_mangle]
pub extern "C" fn storage_deposit() {
    let inp = input();
    let pred = predecessor();
    let account: Vec<u8> = match find_str(&inp, "account_id") {
        Some(a) => a.to_vec(),
        None => pred.clone(),
    };
    check_account(&account);
    let dep = attached();
    if balance(&account).is_some() {
        if dep > 0 {
            transfer_near(&pred, dep);
        }
    } else {
        if dep < REG_COST {
            die("The attached deposit is less than the minimum storage balance");
        }
        set_balance(&account, 0);
        set_unbacked(unbacked() + REG_COST.saturating_sub(paid_record_cost(&account)));
        if dep > REG_COST {
            transfer_near(&pred, dep - REG_COST);
        }
    }
    ret(&storage_balance_json(true));
}

#[no_mangle]
pub extern "C" fn storage_balance_bounds() {
    let mut o = Buf::new();
    o.s(r#"{"min":""#).n(REG_COST).s(r#"","max":""#).n(REG_COST).s("\"}");
    ret(&o.0);
}

#[no_mangle]
pub extern "C" fn storage_balance_of() {
    let inp = input();
    let a = find_str(&inp, "account_id").unwrap_or_else(|| die("account_id"));
    ret(&storage_balance_json(balance(a).is_some()));
}



#[no_mangle]
pub extern "C" fn storage_withdraw() {
    one_yocto();
    let inp = input();
    if balance(&predecessor()).is_none() {
        die("The account is not registered");
    }
    if find_str(&inp, "amount").map(parse_u128).unwrap_or(0) > 0 {
        die("The amount is greater than the available storage balance");
    }
    ret(&storage_balance_json(true));
}


#[no_mangle]
pub extern "C" fn storage_unregister() {
    one_yocto();
    let inp = input();
    let force = find(&inp, "force") == Some(b"true");
    let who = predecessor();
    match balance(&who) {
        None => ret(b"false"),
        Some(b) => {
            if b > 0 {
                if !force {
                    die("Can't unregister the account with the positive balance without force");
                }
                set_supply(supply() - b);
                ev_supply("ft_burn", &who, b, "unregister");
            }

            let auto = is_auto(&who);
            if !auto {
                set_unbacked(unbacked().saturating_sub(REG_COST.saturating_sub(paid_record_cost(&who))));
            }
            sremove(&bal_key(&who));
            transfer_near(&who, if auto { 1 } else { REG_COST + 1 });
            ret(b"true");
        }
    }
}


#[no_mangle]
pub extern "C" fn burn() {
    one_yocto();
    let inp = input();
    let amount = parse_u128(find_str(&inp, "amount").unwrap_or_else(|| die("amount")));
    let who = predecessor();
    let b = balance(&who).unwrap_or_else(|| die("not registered"));
    if amount == 0 || amount > b {
        die("bad amount");
    }
    set_balance(&who, b - amount);
    set_supply(supply() - amount);
    ev_supply("ft_burn", &who, amount, "");
}









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

fn reg(r: u64) -> Vec<u8> {
    unsafe {
        let len = sys::register_len(r);
        if len == u64::MAX {
            return Vec::new();
        }
        let mut v = vec![0u8; len as usize];
        sys::read_register(r, v.as_ptr() as u64);
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
    let b = [0u8; 16];
    unsafe { sys::attached_deposit(b.as_ptr() as u64) };
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
            j
        }
    }
}

fn find<'a>(b: &'a [u8], key: &str) -> Option<&'a [u8]> {
    let mut i = ws(b, 0);
    if i >= b.len() || b[i] != b'{' {
        die("expected json object");
    }
    i = ws(b, i + 1);
    while i < b.len() && b[i] != b'}' {
        if b[i] != b'"' {
            die("bad json key");
        }
        let ke = str_end(b, i);
        let k = &b[i + 1..ke - 1];
        i = ws(b, ke);
        if i >= b.len() || b[i] != b':' {
            die("bad json");
        }
        i = ws(b, i + 1);
        let ve = value_end(b, i);
        if k == key.as_bytes() {
            return Some(&b[i..ve]);
        }
        i = ws(b, ve);
        if i < b.len() && b[i] == b',' {
            i = ws(b, i + 1);
        }
    }
    None
}

fn find_str<'a>(b: &'a [u8], key: &str) -> Option<&'a [u8]> {
    let v = find(b, key)?;
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
    if a.len() < 2 || a.len() > 64 || !a.iter().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'.' | b'_' | b'-')) {
        die("invalid account id");
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
fn set_balance(a: &[u8], n: u128) {
    swrite(&bal_key(a), &n.to_le_bytes());
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


fn check_max_wallet(to: &[u8], new_balance: u128) {
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
    if new_balance > cap {
        die("Launch window: a wallet may not hold more than the max wallet share yet");
    }
}

fn internal_transfer(from: &[u8], to: &[u8], amount: u128, memo: Option<&[u8]>) {
    check_account(to);
    if from == to {
        die("Sender and receiver should be different");
    }
    if amount == 0 {
        die("The amount should be a positive number");
    }
    let fb = balance(from).unwrap_or_else(|| die("The sender is not registered"));
    if fb < amount {
        die("The account doesn't have enough balance");
    }
    let tb = balance(to).unwrap_or_else(|| die("The receiver is not registered"));
    let nb = tb.checked_add(amount).unwrap_or_else(|| die("Balance overflow"));
    check_max_wallet(to, nb);
    set_balance(from, fb - amount);
    set_balance(to, nb);
    ev_transfer(from, to, amount, memo);
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
    let owner = find_str(&inp, "owner_id").unwrap_or_else(|| die("owner_id"));
    check_account(owner);
    let total = parse_u128(find_str(&inp, "total_supply").unwrap_or_else(|| die("total_supply")));
    if total == 0 {
        die("total_supply must be > 0");
    }
    let meta = find(&inp, "metadata").unwrap_or_else(|| die("metadata"));
    if meta.first() != Some(&b'{') {
        die("metadata must be an object");
    }
    if find_str(meta, "spec") != Some(b"ft-1.0.0") {
        die("metadata.spec must be ft-1.0.0");
    }
    if find_str(meta, "name").map(|n| n.is_empty()).unwrap_or(true) || find_str(meta, "symbol").map(|s| s.is_empty()).unwrap_or(true) {
        die("metadata.name/symbol required");
    }
    if find(meta, "decimals").is_none() {
        die("metadata.decimals required");
    }
    swrite(b"m", meta);
    if let Some(rules) = find(&inp, "rules") {
        if rules != b"null" {
            if rules.first() != Some(&b'{') {
                die("rules must be an object");
            }
            swrite(b"r", rules);
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
    let to = find_str(&inp, "receiver_id").unwrap_or_else(|| die("receiver_id"));
    let amount = parse_u128(find_str(&inp, "amount").unwrap_or_else(|| die("amount")));
    let memo = find_str(&inp, "memo");
    internal_transfer(&predecessor(), to, amount, memo);
}

#[no_mangle]
pub extern "C" fn ft_transfer_call() {
    one_yocto();
    let inp = input();
    let to = find_str(&inp, "receiver_id").unwrap_or_else(|| die("receiver_id"));
    let amount = parse_u128(find_str(&inp, "amount").unwrap_or_else(|| die("amount")));
    let memo = find_str(&inp, "memo");
    let msg = find_str(&inp, "msg").unwrap_or_else(|| die("msg"));
    let from = predecessor();
    internal_transfer(&from, to, amount, memo);

    let used = unsafe { sys::used_gas() };
    let prepaid = unsafe { sys::prepaid_gas() };
    let gas = prepaid.saturating_sub(used).saturating_sub(GAS_RESERVE).saturating_sub(GAS_RESOLVE);
    if gas < 5 * TGAS {
        die("More gas is required");
    }
    let mut a1 = Buf::new();
    a1.s(r#"{"sender_id":""#).b(&from).s(r#"","amount":""#).n(amount).s(r#"","msg":""#).b(msg).s("\"}");
    let mut a2 = Buf::new();
    a2.s(r#"{"sender_id":""#).b(&from).s(r#"","receiver_id":""#).b(to).s(r#"","amount":""#).n(amount).s("\"}");
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

#[no_mangle]
pub extern "C" fn ft_resolve_transfer() {
    if predecessor() != current() {
        die("Method ft_resolve_transfer is private");
    }
    let inp = input();
    let sender = find_str(&inp, "sender_id").unwrap_or_else(|| die("sender_id"));
    let receiver = find_str(&inp, "receiver_id").unwrap_or_else(|| die("receiver_id"));
    let amount = parse_u128(find_str(&inp, "amount").unwrap_or_else(|| die("amount")));

    let unused = unsafe {
        match sys::promise_result(0, REG) {
            1 => {
                let v = reg(REG);
                if v.len() >= 2 && v[0] == b'"' && v[v.len() - 1] == b'"' && v[1..v.len() - 1].iter().all(|c| c.is_ascii_digit()) && v.len() <= 41 {
                    parse_u128(&v[1..v.len() - 1]).min(amount)
                } else {
                    amount
                }
            }
            _ => amount,
        }
    };
    let mut refund = 0u128;
    if unused > 0 {
        let rb = balance(receiver).unwrap_or(0);
        refund = unused.min(rb);
        if refund > 0 {
            set_balance(receiver, rb - refund);
            match balance(sender) {
                Some(sb) => {
                    set_balance(sender, sb + refund);
                    ev_transfer(receiver, sender, refund, Some(b"refund"));
                }
                None => {
                    set_supply(supply() - refund);
                    ev_supply("ft_burn", receiver, refund, "refund to unregistered sender");
                }
            }
        }
    }
    let mut o = Buf::new();
    o.qn(amount - refund);
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
            sremove(&bal_key(&who));
            transfer_near(&who, REG_COST + 1);
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

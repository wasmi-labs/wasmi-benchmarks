#![cfg_attr(target_arch = "wasm32", no_std)]

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn handle_panic(_panic_info: &::core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static GLOBAL: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[no_mangle]
pub extern "C" fn run(input: i64) -> i64 {
    if !(0..=10).contains(&input) {
        panic!("invalid input: {input}")
    }
    let password = b"some random password";
    let salt = b"some random salt";
    let m_cost = (input as u32) * 1024;
    let t_cost = argon2::Params::DEFAULT_T_COST;
    let p_cost = argon2::Params::DEFAULT_P_COST;

    let params = argon2::Params::new(m_cost, t_cost, p_cost, None).unwrap();
    let argon = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let mut hash = [0u8; 32];
    argon.hash_password_into(password, salt, &mut hash).unwrap();
    i64::from_be_bytes(
        <[u8; 8]>::try_from(&hash[..8]).expect("array and slice have the same length"),
    )
}

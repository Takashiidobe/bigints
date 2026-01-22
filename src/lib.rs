#![feature(bigint_helper_methods)]

mod i128;
mod i256;
mod i64;
mod u128;
mod u256;
mod u64;

#[cfg(test)]
mod tests;

pub use i64::Int64;
pub use i128::Int128;
pub use i256::Int256;
pub use u64::Uint64;
pub use u128::Uint128;
pub use u256::{Uint256, optimal_u256_mul};

// ============================================================================
// Test functions for codegen comparison
// ============================================================================

pub fn custom256_add(a: Uint256, b: Uint256) -> Uint256 {
    a + b
}

pub fn custom256_sub(a: Uint256, b: Uint256) -> Uint256 {
    a - b
}

pub fn custom256_mul(a: Uint256, b: Uint256) -> Uint256 {
    a * b
}

/// Test: 256÷64 division (fast path)
#[inline(never)]
pub fn custom256_div_u64(a: Uint256, d: u64) -> Uint256 {
    a.div_by_u64(d)
}

#[inline(never)]
pub fn custom256_div(a: Uint256, b: Uint256) -> Uint256 {
    a / b
}

#[inline(never)]
pub fn ethnum_mul(a: ethnum::U256, b: ethnum::U256) -> ethnum::U256 {
    a * b
}

#[inline(never)]
pub fn ethnum_div(a: ethnum::U256, b: ethnum::U256) -> ethnum::U256 {
    a / b
}

pub fn native_add(a: u128, b: u128) -> u128 {
    a.wrapping_add(b)
}

pub fn native_sub(a: u128, b: u128) -> u128 {
    a.wrapping_sub(b)
}

pub fn native_mul(a: u128, b: u128) -> u128 {
    a.wrapping_mul(b)
}

pub fn native_div(a: u128, b: u128) -> u128 {
    a.wrapping_div(b)
}

pub fn native_rem(a: u128, b: u128) -> u128 {
    a.wrapping_rem(b)
}

pub fn native_eq(a: u128, b: u128) -> bool {
    a == b
}

pub fn native_lt(a: u128, b: u128) -> bool {
    a < b
}

pub fn native_le(a: u128, b: u128) -> bool {
    a <= b
}

pub fn native_gt(a: u128, b: u128) -> bool {
    a > b
}

pub fn native_ge(a: u128, b: u128) -> bool {
    a >= b
}

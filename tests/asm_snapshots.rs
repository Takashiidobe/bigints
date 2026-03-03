use regex::Regex;
use std::process::Command;

fn cargo_asm(symbol: &str, target: &str) -> String {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "asm",
        "--lib",
        symbol,
        "--simplify",
        "--release",
        "--target",
        target,
    ]);
    let output = cmd.output().expect("failed to run cargo asm");
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("cargo asm failed for {symbol}: {stderr}");
    }
    let asm = String::from_utf8_lossy(&output.stdout).into_owned();
    normalize_asm(&asm)
}

/// Normalize assembly output for stable snapshots.
/// Replaces variable labels like .LBB68_5 with .LBB_5 (strips the function-specific prefix).
fn normalize_asm(asm: &str) -> String {
    // Normalize .LBB<N>_<M> labels - strip the N which varies by function order
    // Keep the M suffix which is stable within a function
    let lbb_re = Regex::new(r"\.LBB\d+_(\d+)").unwrap();
    let result = lbb_re.replace_all(asm, ".LBB_$1");

    // Normalize .Lanon.<hash>.<N> labels - strip the hash, keep the suffix
    let anon_re = Regex::new(r"\.Lanon\.[a-f0-9]+\.(\d+)").unwrap();
    let result = anon_re.replace_all(&result, ".Lanon.$1");

    let ltmp_re = Regex::new(r"\.Ltmp\d+").unwrap();
    let result = ltmp_re.replace_all(&result, ".Ltmp");

    result.into_owned()
}

macro_rules! asm_snapshot {
    ($name:ident, $symbol:literal, $target:literal) => {
        #[test]
        fn $name() {
            let asm = cargo_asm($symbol, $target);
            insta::with_settings!({ snapshot_suffix => $target }, {
                insta::assert_snapshot!(stringify!($name), asm);
            });
        }
    };
}

/// Strip the first line (function name) from asm output, returning just the instruction body.
fn asm_body(asm: &str) -> &str {
    asm.find('\n').map(|i| &asm[i + 1..]).unwrap_or(asm)
}

/// Normalize asm body for comparison: strip function name and normalize .Lanon.N
/// references (anonymous data indices differ per function position).
fn asm_body_normalized(asm: &str) -> String {
    let body = asm_body(asm);
    let anon_re = Regex::new(r"\.Lanon\.\d+").unwrap();
    anon_re.replace_all(body, ".Lanon").into_owned()
}

/// Assert that a custom bigint operation produces identical asm to the native u128 equivalent.
macro_rules! asm_matches_native {
    ($name:ident, $custom_symbol:literal, $native_symbol:literal, $target:literal) => {
        #[test]
        fn $name() {
            let custom = cargo_asm($custom_symbol, $target);
            let native = cargo_asm($native_symbol, $target);
            let custom_norm = asm_body_normalized(&custom);
            let native_norm = asm_body_normalized(&native);
            assert_eq!(
                custom_norm,
                native_norm,
                "\n=== custom ===\n{custom}\n=== native ===\n{native}"
            );
        }
    };
}

asm_snapshot!(
    asm_i64_add,
    "<bigints::i64::Int64 as core::ops::arith::Add>::add",
    "i686-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i64_sub,
    "<bigints::i64::Int64 as core::ops::arith::Sub>::sub",
    "i686-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i64_mul,
    "<bigints::i64::Int64 as core::ops::arith::Mul>::mul",
    "i686-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i64_div,
    "<bigints::i64::Int64 as core::ops::arith::Div>::div",
    "i686-unknown-linux-gnu"
);

asm_snapshot!(
    asm_i128_add,
    "<bigints::i128::Int128 as core::ops::arith::Add>::add",
    "aarch64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i128_sub,
    "<bigints::i128::Int128 as core::ops::arith::Sub>::sub",
    "aarch64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i128_mul,
    "<bigints::i128::Int128 as core::ops::arith::Mul>::mul",
    "aarch64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i128_div,
    "<bigints::i128::Int128 as core::ops::arith::Div>::div",
    "aarch64-unknown-linux-gnu"
);

asm_snapshot!(
    asm_i256_add,
    "<bigints::i256::Int256 as core::ops::arith::Add>::add",
    "aarch64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i256_sub,
    "<bigints::i256::Int256 as core::ops::arith::Sub>::sub",
    "aarch64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i256_mul,
    "<bigints::i256::Int256 as core::ops::arith::Mul>::mul",
    "aarch64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i256_div,
    "<bigints::i256::Int256 as core::ops::arith::Div>::div",
    "aarch64-unknown-linux-gnu"
);

asm_snapshot!(
    asm_u64_add,
    "<bigints::u64::Uint64 as core::ops::arith::Add>::add",
    "i686-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u64_sub,
    "<bigints::u64::Uint64 as core::ops::arith::Sub>::sub",
    "i686-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u64_mul,
    "<bigints::u64::Uint64 as core::ops::arith::Mul>::mul",
    "i686-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u64_div,
    "<bigints::u64::Uint64 as core::ops::arith::Div>::div",
    "i686-unknown-linux-gnu"
);

asm_snapshot!(
    asm_u128_add,
    "<bigints::u128::Uint128 as core::ops::arith::Add>::add",
    "aarch64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u128_sub,
    "<bigints::u128::Uint128 as core::ops::arith::Sub>::sub",
    "aarch64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u128_mul,
    "<bigints::u128::Uint128 as core::ops::arith::Mul>::mul",
    "aarch64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u128_div,
    "<bigints::u128::Uint128 as core::ops::arith::Div>::div",
    "aarch64-unknown-linux-gnu"
);

asm_snapshot!(
    asm_u256_add,
    "<bigints::u256::Uint256 as core::ops::arith::Add>::add",
    "aarch64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u256_sub,
    "<bigints::u256::Uint256 as core::ops::arith::Sub>::sub",
    "aarch64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u256_mul,
    "<bigints::u256::Uint256 as core::ops::arith::Mul>::mul",
    "aarch64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u256_div,
    "<bigints::u256::Uint256 as core::ops::arith::Div>::div",
    "aarch64-unknown-linux-gnu"
);

asm_snapshot!(
    asm_i64_add_x86_64,
    "<bigints::i64::Int64 as core::ops::arith::Add>::add",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i64_sub_x86_64,
    "<bigints::i64::Int64 as core::ops::arith::Sub>::sub",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i64_mul_x86_64,
    "<bigints::i64::Int64 as core::ops::arith::Mul>::mul",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i64_div_x86_64,
    "<bigints::i64::Int64 as core::ops::arith::Div>::div",
    "x86_64-unknown-linux-gnu"
);

asm_snapshot!(
    asm_i128_add_x86_64,
    "<bigints::i128::Int128 as core::ops::arith::Add>::add",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i128_sub_x86_64,
    "<bigints::i128::Int128 as core::ops::arith::Sub>::sub",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i128_mul_x86_64,
    "<bigints::i128::Int128 as core::ops::arith::Mul>::mul",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i128_div_x86_64,
    "<bigints::i128::Int128 as core::ops::arith::Div>::div",
    "x86_64-unknown-linux-gnu"
);

asm_snapshot!(
    asm_i256_add_x86_64,
    "<bigints::i256::Int256 as core::ops::arith::Add>::add",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i256_sub_x86_64,
    "<bigints::i256::Int256 as core::ops::arith::Sub>::sub",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i256_mul_x86_64,
    "<bigints::i256::Int256 as core::ops::arith::Mul>::mul",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i256_div_x86_64,
    "<bigints::i256::Int256 as core::ops::arith::Div>::div",
    "x86_64-unknown-linux-gnu"
);

asm_snapshot!(
    asm_u64_add_x86_64,
    "<bigints::u64::Uint64 as core::ops::arith::Add>::add",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u64_sub_x86_64,
    "<bigints::u64::Uint64 as core::ops::arith::Sub>::sub",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u64_mul_x86_64,
    "<bigints::u64::Uint64 as core::ops::arith::Mul>::mul",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u64_div_x86_64,
    "<bigints::u64::Uint64 as core::ops::arith::Div>::div",
    "x86_64-unknown-linux-gnu"
);

asm_snapshot!(
    asm_u128_add_x86_64,
    "<bigints::u128::Uint128 as core::ops::arith::Add>::add",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u128_sub_x86_64,
    "<bigints::u128::Uint128 as core::ops::arith::Sub>::sub",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u128_mul_x86_64,
    "<bigints::u128::Uint128 as core::ops::arith::Mul>::mul",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u128_div_x86_64,
    "<bigints::u128::Uint128 as core::ops::arith::Div>::div",
    "x86_64-unknown-linux-gnu"
);

asm_snapshot!(
    asm_u256_add_x86_64,
    "<bigints::u256::Uint256 as core::ops::arith::Add>::add",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u256_sub_x86_64,
    "<bigints::u256::Uint256 as core::ops::arith::Sub>::sub",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u256_mul_x86_64,
    "<bigints::u256::Uint256 as core::ops::arith::Mul>::mul",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u256_div_x86_64,
    "<bigints::u256::Uint256 as core::ops::arith::Div>::div",
    "x86_64-unknown-linux-gnu"
);

asm_snapshot!(
    asm_i64_add_armv7,
    "<bigints::i64::Int64 as core::ops::arith::Add>::add",
    "armv7-unknown-linux-gnueabihf"
);
asm_snapshot!(
    asm_i64_sub_armv7,
    "<bigints::i64::Int64 as core::ops::arith::Sub>::sub",
    "armv7-unknown-linux-gnueabihf"
);
asm_snapshot!(
    asm_i64_mul_armv7,
    "<bigints::i64::Int64 as core::ops::arith::Mul>::mul",
    "armv7-unknown-linux-gnueabihf"
);
asm_snapshot!(
    asm_i64_div_armv7,
    "<bigints::i64::Int64 as core::ops::arith::Div>::div",
    "armv7-unknown-linux-gnueabihf"
);

asm_snapshot!(
    asm_u64_add_armv7,
    "<bigints::u64::Uint64 as core::ops::arith::Add>::add",
    "armv7-unknown-linux-gnueabihf"
);
asm_snapshot!(
    asm_u64_sub_armv7,
    "<bigints::u64::Uint64 as core::ops::arith::Sub>::sub",
    "armv7-unknown-linux-gnueabihf"
);
asm_snapshot!(
    asm_u64_mul_armv7,
    "<bigints::u64::Uint64 as core::ops::arith::Mul>::mul",
    "armv7-unknown-linux-gnueabihf"
);
asm_snapshot!(
    asm_u64_div_armv7,
    "<bigints::u64::Uint64 as core::ops::arith::Div>::div",
    "armv7-unknown-linux-gnueabihf"
);

asm_snapshot!(
    asm_i128_add_riscv,
    "<bigints::i128::Int128 as core::ops::arith::Add>::add",
    "riscv64gc-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i128_sub_riscv,
    "<bigints::i128::Int128 as core::ops::arith::Sub>::sub",
    "riscv64gc-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i128_mul_riscv,
    "<bigints::i128::Int128 as core::ops::arith::Mul>::mul",
    "riscv64gc-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i128_div_riscv,
    "<bigints::i128::Int128 as core::ops::arith::Div>::div",
    "riscv64gc-unknown-linux-gnu"
);

asm_snapshot!(
    asm_i256_add_riscv,
    "<bigints::i256::Int256 as core::ops::arith::Add>::add",
    "riscv64gc-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i256_sub_riscv,
    "<bigints::i256::Int256 as core::ops::arith::Sub>::sub",
    "riscv64gc-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i256_mul_riscv,
    "<bigints::i256::Int256 as core::ops::arith::Mul>::mul",
    "riscv64gc-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i256_div_riscv,
    "<bigints::i256::Int256 as core::ops::arith::Div>::div",
    "riscv64gc-unknown-linux-gnu"
);

asm_snapshot!(
    asm_u128_add_riscv,
    "<bigints::u128::Uint128 as core::ops::arith::Add>::add",
    "riscv64gc-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u128_sub_riscv,
    "<bigints::u128::Uint128 as core::ops::arith::Sub>::sub",
    "riscv64gc-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u128_mul_riscv,
    "<bigints::u128::Uint128 as core::ops::arith::Mul>::mul",
    "riscv64gc-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u128_div_riscv,
    "<bigints::u128::Uint128 as core::ops::arith::Div>::div",
    "riscv64gc-unknown-linux-gnu"
);

asm_snapshot!(
    asm_native_add_riscv,
    "bigints::native_add",
    "riscv64gc-unknown-linux-gnu"
);
asm_snapshot!(
    asm_native_sub_riscv,
    "bigints::native_sub",
    "riscv64gc-unknown-linux-gnu"
);
asm_snapshot!(
    asm_native_mul_riscv,
    "bigints::native_mul",
    "riscv64gc-unknown-linux-gnu"
);
asm_snapshot!(
    asm_native_div_riscv,
    "bigints::native_div",
    "riscv64gc-unknown-linux-gnu"
);

asm_snapshot!(
    asm_u256_add_riscv,
    "<bigints::u256::Uint256 as core::ops::arith::Add>::add",
    "riscv64gc-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u256_sub_riscv,
    "<bigints::u256::Uint256 as core::ops::arith::Sub>::sub",
    "riscv64gc-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u256_mul_riscv,
    "<bigints::u256::Uint256 as core::ops::arith::Mul>::mul",
    "riscv64gc-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u256_div_riscv,
    "<bigints::u256::Uint256 as core::ops::arith::Div>::div",
    "riscv64gc-unknown-linux-gnu"
);

asm_snapshot!(
    asm_i128_add_s390x,
    "<bigints::i128::Int128 as core::ops::arith::Add>::add",
    "s390x-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i128_sub_s390x,
    "<bigints::i128::Int128 as core::ops::arith::Sub>::sub",
    "s390x-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i128_mul_s390x,
    "<bigints::i128::Int128 as core::ops::arith::Mul>::mul",
    "s390x-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i128_div_s390x,
    "<bigints::i128::Int128 as core::ops::arith::Div>::div",
    "s390x-unknown-linux-gnu"
);

asm_snapshot!(
    asm_i256_add_s390x,
    "<bigints::i256::Int256 as core::ops::arith::Add>::add",
    "s390x-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i256_sub_s390x,
    "<bigints::i256::Int256 as core::ops::arith::Sub>::sub",
    "s390x-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i256_mul_s390x,
    "<bigints::i256::Int256 as core::ops::arith::Mul>::mul",
    "s390x-unknown-linux-gnu"
);
asm_snapshot!(
    asm_i256_div_s390x,
    "<bigints::i256::Int256 as core::ops::arith::Div>::div",
    "s390x-unknown-linux-gnu"
);

asm_snapshot!(
    asm_u128_add_s390x,
    "<bigints::u128::Uint128 as core::ops::arith::Add>::add",
    "s390x-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u128_sub_s390x,
    "<bigints::u128::Uint128 as core::ops::arith::Sub>::sub",
    "s390x-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u128_mul_s390x,
    "<bigints::u128::Uint128 as core::ops::arith::Mul>::mul",
    "s390x-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u128_div_s390x,
    "<bigints::u128::Uint128 as core::ops::arith::Div>::div",
    "s390x-unknown-linux-gnu"
);

asm_snapshot!(
    asm_u256_add_s390x,
    "<bigints::u256::Uint256 as core::ops::arith::Add>::add",
    "s390x-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u256_sub_s390x,
    "<bigints::u256::Uint256 as core::ops::arith::Sub>::sub",
    "s390x-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u256_mul_s390x,
    "<bigints::u256::Uint256 as core::ops::arith::Mul>::mul",
    "s390x-unknown-linux-gnu"
);
asm_snapshot!(
    asm_u256_div_s390x,
    "<bigints::u256::Uint256 as core::ops::arith::Div>::div",
    "s390x-unknown-linux-gnu"
);

// ============================================================================
// Verify custom u128 operations produce identical asm to native u128
// ============================================================================

// x86_64
asm_snapshot!(
    asm_native_add_x86_64,
    "bigints::native_add",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_native_sub_x86_64,
    "bigints::native_sub",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_native_mul_x86_64,
    "bigints::native_mul",
    "x86_64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_native_div_x86_64,
    "bigints::native_div",
    "x86_64-unknown-linux-gnu"
);
asm_matches_native!(
    asm_u128_add_matches_native_x86_64,
    "<bigints::u128::Uint128 as core::ops::arith::Add>::add",
    "bigints::native_add",
    "x86_64-unknown-linux-gnu"
);
// sub: matches native (same LLVM usub.with.overflow fusion bug doesn't affect x86_64,
//   but codegen is identical)
asm_matches_native!(
    asm_u128_sub_matches_native_x86_64,
    "<bigints::u128::Uint128 as core::ops::arith::Sub>::sub",
    "bigints::native_sub",
    "x86_64-unknown-linux-gnu"
);
// mul: excluded — _mulx_u64 intrinsic pre-commits instruction selection before regalloc,
//   causing different (but equivalent) register choices vs native mul i128 decomposition.
//   Same instruction count and mix (1 mulx + 2 imul + 2 add).
asm_matches_native!(
    asm_u128_div_matches_native_x86_64,
    "<bigints::u128::Uint128 as core::ops::arith::Div>::div",
    "bigints::native_div",
    "x86_64-unknown-linux-gnu"
);

// aarch64
asm_snapshot!(
    asm_native_add_aarch64,
    "bigints::native_add",
    "aarch64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_native_sub_aarch64,
    "bigints::native_sub",
    "aarch64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_native_mul_aarch64,
    "bigints::native_mul",
    "aarch64-unknown-linux-gnu"
);
asm_snapshot!(
    asm_native_div_aarch64,
    "bigints::native_div",
    "aarch64-unknown-linux-gnu"
);
asm_matches_native!(
    asm_u128_add_matches_native_aarch64,
    "<bigints::u128::Uint128 as core::ops::arith::Add>::add",
    "bigints::native_add",
    "aarch64-unknown-linux-gnu"
);
// sub: excluded — LLVM aarch64 bug (does not fuse chained usub.with.overflow into subs/sbc)
// mul: excluded — aarch64 has umulh for 64×64→128 but we only use _mulx_u64 on x86_64,
//   falling back to 32-bit schoolbook on aarch64. TODO: add aarch64 umulh path.
asm_matches_native!(
    asm_u128_div_matches_native_aarch64,
    "<bigints::u128::Uint128 as core::ops::arith::Div>::div",
    "bigints::native_div",
    "aarch64-unknown-linux-gnu"
);

use std::process::Command;
use regex::Regex;

fn cargo_asm(symbol: &str) -> String {
    let output = Command::new("cargo")
        .args([
            "asm",
            "--lib",
            symbol,
            "--release",
        ])
        .output()
        .expect("failed to run cargo asm");
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

    result.into_owned()
}

macro_rules! asm_snapshot {
    ($name:ident, $symbol:literal) => {
        #[test]
        fn $name() {
            let asm = cargo_asm($symbol);
            insta::assert_snapshot!(stringify!($name), asm);
        }
    };
}

asm_snapshot!(
    asm_i64_add,
    "<bigints::i64::Int64 as core::ops::arith::Add>::add"
);
asm_snapshot!(
    asm_i64_sub,
    "<bigints::i64::Int64 as core::ops::arith::Sub>::sub"
);
asm_snapshot!(
    asm_i64_mul,
    "<bigints::i64::Int64 as core::ops::arith::Mul>::mul"
);
asm_snapshot!(
    asm_i64_div,
    "<bigints::i64::Int64 as core::ops::arith::Div>::div"
);

asm_snapshot!(
    asm_i128_add,
    "<bigints::i128::Int128 as core::ops::arith::Add>::add"
);
asm_snapshot!(
    asm_i128_sub,
    "<bigints::i128::Int128 as core::ops::arith::Sub>::sub"
);
asm_snapshot!(
    asm_i128_mul,
    "<bigints::i128::Int128 as core::ops::arith::Mul>::mul"
);
asm_snapshot!(
    asm_i128_div,
    "<bigints::i128::Int128 as core::ops::arith::Div>::div"
);

asm_snapshot!(
    asm_i256_add,
    "<bigints::i256::Int256 as core::ops::arith::Add>::add"
);
asm_snapshot!(
    asm_i256_sub,
    "<bigints::i256::Int256 as core::ops::arith::Sub>::sub"
);
asm_snapshot!(
    asm_i256_mul,
    "<bigints::i256::Int256 as core::ops::arith::Mul>::mul"
);
asm_snapshot!(
    asm_i256_div,
    "<bigints::i256::Int256 as core::ops::arith::Div>::div"
);

asm_snapshot!(
    asm_u64_add,
    "<bigints::u64::Uint64 as core::ops::arith::Add>::add"
);
asm_snapshot!(
    asm_u64_sub,
    "<bigints::u64::Uint64 as core::ops::arith::Sub>::sub"
);
asm_snapshot!(
    asm_u64_mul,
    "<bigints::u64::Uint64 as core::ops::arith::Mul>::mul"
);
asm_snapshot!(
    asm_u64_div,
    "<bigints::u64::Uint64 as core::ops::arith::Div>::div"
);

asm_snapshot!(
    asm_u128_add,
    "<bigints::u128::Uint128 as core::ops::arith::Add>::add"
);
asm_snapshot!(
    asm_u128_sub,
    "<bigints::u128::Uint128 as core::ops::arith::Sub>::sub"
);
asm_snapshot!(
    asm_u128_mul,
    "<bigints::u128::Uint128 as core::ops::arith::Mul>::mul"
);
asm_snapshot!(
    asm_u128_div,
    "<bigints::u128::Uint128 as core::ops::arith::Div>::div"
);

asm_snapshot!(
    asm_u256_add,
    "<bigints::u256::Uint256 as core::ops::arith::Add>::add"
);
asm_snapshot!(
    asm_u256_sub,
    "<bigints::u256::Uint256 as core::ops::arith::Sub>::sub"
);
asm_snapshot!(
    asm_u256_mul,
    "<bigints::u256::Uint256 as core::ops::arith::Mul>::mul"
);
asm_snapshot!(
    asm_u256_div,
    "<bigints::u256::Uint256 as core::ops::arith::Div>::div"
);

use std::cmp::Ordering;

/// 128-bit unsigned integer stored as two 64-bit limbs.
///
/// Field order matches native u128 ABI layout for optimal codegen:
/// - Little-endian (x86_64): { l, h } - LSB at lower address
/// - Big-endian (PowerPC64-BE, s390x): { h, l } - MSB at lower address
///
/// # x86_64 Little-Endian ABI Example
///
/// With Uint128 { l, h } (low first, matching native u128 layout):
///   - Arguments arrive as: rdi=a.l, rsi=a.h, rdx=b.l, rcx=b.h
///   - mulx requires rdx as implicit operand, and we need b.l there
///   - rdx already contains b.l, so no register shuffling needed
///
/// Generated assembly (6 instructions, identical to native u128):
/// ```asm
///   mulx r8, rax, rdi    ; (r8:rax) = b.l * a.l
///   imul rcx, rdi        ; rcx = b.h * a.l
///   imul rdx, rsi        ; rdx = b.l * a.h
///   add  rdx, rcx
///   add  rdx, r8
///   ret
/// ```
///
/// With Uint128 { h, l } (high first, WRONG order for little-endian):
/// Arguments arrive as: rdi=a.h, rsi=a.l, rdx=b.h, rcx=b.l
/// rdx contains b.h, but mulx needs b.l
/// Requires 2 extra mov instructions to shuffle registers:
/// ```asm
///   mov rax, rdx         ; save b.h
///   mov rdx, rcx         ; move b.l into rdx for mulx
///   mulx r8, rdx, rsi    ; now we can multiply
///   ...
/// ```
#[derive(Debug, Clone, Copy)]
#[cfg(target_endian = "little")]
pub struct Uint128 {
    pub l: u64, // bits 0-63 (lower address)
    pub h: u64, // bits 64-127 (higher address)
}

#[derive(Debug, Clone, Copy)]
#[cfg(target_endian = "big")]
pub struct Uint128 {
    pub h: u64, // bits 64-127 (lower address)
    pub l: u64, // bits 0-63 (higher address)
}

impl Uint128 {
    #[cfg(not(target_arch = "x86_64"))]
    fn mul_u64_full(a: u64, b: u64) -> (u64, u64) {
        let a0 = a as u32 as u64;
        let a1 = (a >> 32) as u32 as u64;
        let b0 = b as u32 as u64;
        let b1 = (b >> 32) as u32 as u64;

        let p0 = a0 * b0;
        let p1 = a0 * b1;
        let p2 = a1 * b0;
        let p3 = a1 * b1;

        let (middle, carry_mid) = p1.overflowing_add(p2);
        let (low, carry_low) = p0.overflowing_add(middle << 32);
        let mut high = p3
            .wrapping_add(middle >> 32)
            .wrapping_add((carry_mid as u64) << 32);
        if carry_low {
            high = high.wrapping_add(1);
        }

        (high, low)
    }
}

impl std::ops::Add for Uint128 {
    type Output = Self;

    #[inline(never)]
    fn add(self, rhs: Self) -> Self::Output {
        let (l, carry) = self.l.overflowing_add(rhs.l);
        let h = self.h.wrapping_add(rhs.h).wrapping_add(carry as u64);

        Self { h, l }
    }
}

impl std::ops::Sub for Uint128 {
    type Output = Self;

    #[inline(never)]
    fn sub(self, rhs: Self) -> Self::Output {
        let (l, borrow) = self.l.overflowing_sub(rhs.l);
        let h = self.h.wrapping_sub(rhs.h).wrapping_sub(borrow as u64);

        Self { h, l }
    }
}

impl std::ops::Mul for Uint128 {
    type Output = Self;

    /// 128-bit multiplication, keeping only the low 128 bits of the 256-bit result.
    #[inline(never)]
    ///
    /// # Algorithm
    ///
    /// Schoolbook multiplication of two 128-bit numbers as pairs of 64-bit limbs:
    ///
    /// ```text
    ///              self.h : self.l
    ///            × rhs.h  : rhs.l
    ///       ─────────────────────
    ///                      self.l × rhs.l  →  (p0_hi : p0_lo)  [128 bits]
    ///             self.l × rhs.h           →  (  __ : t1_lo)   [only low 64 matters]
    ///             self.h × rhs.l           →  (  __ : t2_lo)   [only low 64 matters]
    ///    self.h × rhs.h                    →  [discarded, overflows 128 bits]
    ///       ─────────────────────
    ///       result.h = p0_hi + t1_lo + t2_lo
    ///       result.l = p0_lo
    /// ```
    ///
    /// Only `self.l × rhs.l` needs a full 128-bit result. The cross terms (`l×h`, `h×l`)
    /// only contribute their low 64 bits to the result's high limb. The `h×h` term
    /// would only affect bits 128+ which we discard.
    ///
    /// # x86_64: `mulx` intrinsic
    ///
    /// Uses `_mulx_u64` (BMI2) for the 64×64→128 multiplication:
    /// - Single instruction: `mulx r64, r64, r/m64`
    /// - Returns full 128-bit product in two registers
    /// - Requires `target-feature=+bmi2` for optimal codegen (see `.cargo/config.toml`)
    ///
    /// # Fallback: 32-bit schoolbook
    ///
    /// On non-x86_64, `mul_u64_full` decomposes each 64-bit operand into two 32-bit
    /// halves and uses four 32×32→64 multiplications:
    ///
    /// ```text
    ///        a1 : a0      (a = a1<<32 | a0)
    ///      × b1 : b0      (b = b1<<32 | b0)
    ///     ──────────
    ///             p0 = a0 × b0    [bits 0-63]
    ///        p1 = a0 × b1         [bits 32-95]
    ///        p2 = a1 × b0         [bits 32-95]
    ///   p3 = a1 × b1              [bits 64-127]
    /// ```
    ///
    /// These partial products are then combined with proper carry propagation.
    fn mul(self, rhs: Self) -> Self::Output {
        #[cfg(target_arch = "x86_64")]
        let (p0_hi, p0_lo) = {
            let mut hi = 0u64;
            let lo = unsafe { core::arch::x86_64::_mulx_u64(self.l, rhs.l, &mut hi) };
            (hi, lo)
        };

        #[cfg(not(target_arch = "x86_64"))]
        let (p0_hi, p0_lo) = Self::mul_u64_full(self.l, rhs.l);

        let t1_lo = self.l.wrapping_mul(rhs.h);
        let t2_lo = self.h.wrapping_mul(rhs.l);
        let h = p0_hi.wrapping_add(t1_lo).wrapping_add(t2_lo);
        Self { h, l: p0_lo }
    }
}

impl std::ops::Div for Uint128 {
    type Output = Self;

    /// Division that mirrors native u128 behavior - delegates to __udivti3.
    ///
    /// # Why we delegate to u128 division
    ///
    /// Unlike multiplication (which has the `mulx` intrinsic), there's no Rust intrinsic
    /// for 128÷64 hardware division. The `div` instruction exists but is only accessible
    /// via inline assembly. Without it, any "optimized" implementation we write still
    /// ends up calling `__udivti3` (the compiler-builtins runtime function) for the
    /// hard cases, but with extra branching overhead on top.
    ///
    /// # What __udivti3 does under the hood
    ///
    /// 1. If divisor == 0: panics via `panic_const_div_by_zero`
    /// 2. If divisor fits in 64 bits (d.h == 0):
    ///    - Uses hardware `div` instruction for 128÷64
    ///    - Two divisions: (0:n.h) / d.l → q_hi, then (r:n.l) / d.l → q_lo
    /// 3. If divisor is full 128-bit (d.h != 0):
    ///    - Quotient must be < 2^64 (since d > 2^64 and n < 2^128)
    ///    - Normalize: shift divisor left until MSB is set
    ///    - Estimate: q ≈ (n_hi:n_mid) / d_hi using hardware div
    ///    - Correct: if q * d > n, decrement q (at most 2 iterations)
    ///
    /// See: https://github.com/rust-lang/compiler-builtins/blob/master/src/int/specialized_div_rem/mod.rs
    ///
    /// # Pitfall: Don't add an explicit zero check
    ///
    /// Adding `if rhs.is_zero() { panic!("...") }` before the division generates
    /// *worse* code. The explicit `panic!()` macro emits a `panic_fmt` call with a
    /// custom message, creating a separate code path that LLVM cannot fuse:
    ///
    /// ```asm
    /// ; BAD: Two separate panic paths, unfused branches
    ///     or rax, rdx
    ///     je .LBB_panic_fmt      ; our explicit check
    ///     je .LBB_panic_const    ; dead code from u128's check!
    ///     call __udivti3
    /// ```
    ///
    /// By letting u128 handle the zero check, it uses `panic_const_div_by_zero`
    /// (a diverging `-> !` function), allowing LLVM to fuse the branches:
    ///
    /// ```asm
    /// ; GOOD: Single panic path, matches native u128
    ///     push rax
    ///     mov rax, rdx
    ///     or rax, rcx
    ///     je .LBB_panic
    ///     call __udivti3
    ///     pop rcx
    ///     ret
    /// .LBB_panic:
    ///     call panic_const_div_by_zero
    ///     ud2
    /// ```
    fn div(self, rhs: Self) -> Self::Output {
        let n = (self.h as u128) << 64 | self.l as u128;
        let d = (rhs.h as u128) << 64 | rhs.l as u128;
        let q = n / d;
        Self {
            l: q as u64,
            h: (q >> 64) as u64,
        }
    }
}

impl std::ops::Rem for Uint128 {
    type Output = Self;

    /// Remainder that mirrors native u128 behavior - delegates to __umodti3.
    ///
    /// Same considerations as division: no explicit zero check needed,
    /// let u128 handle it for optimal codegen.
    fn rem(self, rhs: Self) -> Self::Output {
        let n = (self.h as u128) << 64 | self.l as u128;
        let d = (rhs.h as u128) << 64 | rhs.l as u128;
        let r = n % d;
        Self {
            l: r as u64,
            h: (r >> 64) as u64,
        }
    }
}

impl PartialEq for Uint128 {
    fn eq(&self, other: &Self) -> bool {
        self.h == other.h && self.l == other.l
    }
}

impl Eq for Uint128 {}

impl PartialOrd for Uint128 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Uint128 {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.h.cmp(&other.h) {
            Ordering::Equal => self.l.cmp(&other.l),
            other => other,
        }
    }
}

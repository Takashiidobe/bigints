use std::cmp::Ordering;

/// 256-bit unsigned integer stored as four 64-bit limbs.
///
/// Field order matches native memory layout for optimal codegen:
/// - Little-endian: { l0, l1, l2, l3 } - LSB at lower address
/// - Big-endian: { l3, l2, l1, l0 } - MSB at lower address
///
/// This ensures efficient load/store and correct behavior when
/// reinterpreting memory as native integer arrays.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[cfg(target_endian = "little")]
pub struct Uint256 {
    pub l0: u64, // bits 0-63 (lowest address)
    pub l1: u64, // bits 64-127
    pub l2: u64, // bits 128-191
    pub l3: u64, // bits 192-255 (highest address)
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[cfg(target_endian = "big")]
pub struct Uint256 {
    pub l3: u64, // bits 192-255 (lowest address)
    pub l2: u64, // bits 128-191
    pub l1: u64, // bits 64-127
    pub l0: u64, // bits 0-63 (highest address)
}

impl Uint256 {
    pub const ZERO: Self = Self {
        l0: 0,
        l1: 0,
        l2: 0,
        l3: 0,
    };

    pub fn is_zero(&self) -> bool {
        self.l0 == 0 && self.l1 == 0 && self.l2 == 0 && self.l3 == 0
    }
}

impl std::ops::Add for Uint256 {
    type Output = Self;

    /// 256-bit addition with carry chain.
    /// this is much better generated code than previously (uint) pre 1.79
    #[inline(never)]
    fn add(self, rhs: Self) -> Self::Output {
        let (l0, c0) = self.l0.overflowing_add(rhs.l0);

        let (l1, c1) = self.l1.carrying_add(rhs.l1, c0);
        let (l2, c2) = self.l2.carrying_add(rhs.l2, c1);
        let (l3, _) = self.l3.carrying_add(rhs.l3, c2);
        Self { l0, l1, l2, l3 }
    }
}

impl std::ops::Sub for Uint256 {
    type Output = Self;

    /// 256-bit subtraction with borrow chain.
    #[inline(never)]
    fn sub(self, rhs: Self) -> Self::Output {
        let (l0, b0) = self.l0.overflowing_sub(rhs.l0);
        let (l1, b1) = self.l1.borrowing_sub(rhs.l1, b0);
        let (l2, b2) = self.l2.borrowing_sub(rhs.l2, b1);
        let (l3, _) = self.l3.borrowing_sub(rhs.l3, b2);
        Self { l0, l1, l2, l3 }
    }
}

impl std::ops::Mul for Uint256 {
    type Output = Self;

    /// 256-bit multiplication (schoolbook), keeping only the low 256 bits.
    #[inline(never)]
    fn mul(self, rhs: Self) -> Self::Output {
        #[cfg(target_arch = "x86_64")]
        {
            Self::mul_adx(self, rhs)
        }

        #[cfg(not(target_arch = "x86_64"))]
        self.mul_portable(rhs)
    }
}

impl Uint256 {
    /// Portable multiplication fallback using u128.
    ///
    /// Tracks overflow when column sums exceed u128 to ensure correct carry
    /// propagation for all input values.
    #[cfg(not(target_arch = "x86_64"))]
    fn mul_portable(self, rhs: Self) -> Self {
        let (a0, a1, a2, a3) = (self.l0, self.l1, self.l2, self.l3);
        let (b0, b1, b2, b3) = (rhs.l0, rhs.l1, rhs.l2, rhs.l3);

        // Column 0: single product, no overflow possible
        let p00 = (a0 as u128) * (b0 as u128);
        let r0 = p00 as u64;
        let mut carry = p00 >> 64;

        // Column 1: two products + carry
        let p01 = (a0 as u128) * (b1 as u128);
        let p10 = (a1 as u128) * (b0 as u128);
        let (sum, o1) = carry.overflowing_add(p01);
        let (col1, o2) = sum.overflowing_add(p10);
        let r1 = col1 as u64;
        carry = (col1 >> 64) + ((((o1 as u64) + (o2 as u64)) as u128) << 64);

        // Column 2: three products + carry
        let p02 = (a0 as u128) * (b2 as u128);
        let p11 = (a1 as u128) * (b1 as u128);
        let p20 = (a2 as u128) * (b0 as u128);
        let (sum, o1) = carry.overflowing_add(p02);
        let (sum, o2) = sum.overflowing_add(p11);
        let (col2, o3) = sum.overflowing_add(p20);
        let r2 = col2 as u64;
        carry = (col2 >> 64) + ((((o1 as u64) + (o2 as u64) + (o3 as u64)) as u128) << 64);

        // Column 3: only need low 64 bits, overflow discarded
        let r3 = (carry as u64)
            .wrapping_add(a0.wrapping_mul(b3))
            .wrapping_add(a1.wrapping_mul(b2))
            .wrapping_add(a2.wrapping_mul(b1))
            .wrapping_add(a3.wrapping_mul(b0));

        Self { l0: r0, l1: r1, l2: r2, l3: r3 }
    }

    /// x86_64 optimized multiplication using u128 intermediates.
    /// LLVM tends to generate good code for u128 arithmetic on x86_64.
    ///
    /// Tracks overflow when column sums exceed u128 to ensure correct carry
    /// propagation for all input values.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    fn mul_adx(self, rhs: Self) -> Self {
        let (a0, a1, a2, a3) = (self.l0, self.l1, self.l2, self.l3);
        let (b0, b1, b2, b3) = (rhs.l0, rhs.l1, rhs.l2, rhs.l3);

        // Column 0: single product, no overflow possible
        let p00 = (a0 as u128) * (b0 as u128);
        let r0 = p00 as u64;
        let mut carry = p00 >> 64;

        // Column 1: two products + carry
        // Track overflow when sum exceeds u128
        let p01 = (a0 as u128) * (b1 as u128);
        let p10 = (a1 as u128) * (b0 as u128);
        let (sum, o1) = carry.overflowing_add(p01);
        let (col1, o2) = sum.overflowing_add(p10);
        let r1 = col1 as u64;
        // Carry includes overflow: each overflow adds 2^128, which is 2^64 in the next column's scale
        carry = (col1 >> 64) + ((((o1 as u64) + (o2 as u64)) as u128) << 64);

        // Column 2: three products + carry
        let p02 = (a0 as u128) * (b2 as u128);
        let p11 = (a1 as u128) * (b1 as u128);
        let p20 = (a2 as u128) * (b0 as u128);
        let (sum, o1) = carry.overflowing_add(p02);
        let (sum, o2) = sum.overflowing_add(p11);
        let (col2, o3) = sum.overflowing_add(p20);
        let r2 = col2 as u64;
        carry = (col2 >> 64) + ((((o1 as u64) + (o2 as u64) + (o3 as u64)) as u128) << 64);

        // Column 3: only need low 64 bits, overflow discarded
        let r3 = (carry as u64)
            .wrapping_add(a0.wrapping_mul(b3))
            .wrapping_add(a1.wrapping_mul(b2))
            .wrapping_add(a2.wrapping_mul(b1))
            .wrapping_add(a3.wrapping_mul(b0));

        Self { l0: r0, l1: r1, l2: r2, l3: r3 }
    }
}

impl PartialEq for Uint256 {
    fn eq(&self, other: &Self) -> bool {
        self.l0 == other.l0 && self.l1 == other.l1 && self.l2 == other.l2 && self.l3 == other.l3
    }
}

impl Eq for Uint256 {}

impl PartialOrd for Uint256 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Uint256 {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.l3.cmp(&other.l3) {
            Ordering::Equal => match self.l2.cmp(&other.l2) {
                Ordering::Equal => match self.l1.cmp(&other.l1) {
                    Ordering::Equal => self.l0.cmp(&other.l0),
                    other => other,
                },
                other => other,
            },
            other => other,
        }
    }
}

impl std::ops::Div for Uint256 {
    type Output = Self;

    /// 256-bit division using Knuth's Algorithm D.
    ///
    /// # Algorithm Overview
    ///
    /// 1. **Fast path (divisor fits in u64)**: Use hardware 64-bit division
    ///    to compute quotient one limb at a time.
    ///
    /// 2. **Fast path (divisor fits in u128)**: Use hardware 128÷64 division
    ///    via __udivti3 for the 256÷128 case.
    ///
    /// 3. **General case (Knuth Algorithm D)**: Normalize divisor, estimate
    ///    quotient digits using top limbs, correct estimates.
    fn div(self, rhs: Self) -> Self::Output {
        // Dispatch based on divisor size for optimal codegen
        if rhs.l3 == 0 && rhs.l2 == 0 {
            if rhs.l1 == 0 {
                // Divisor fits in u64 - use simple long division
                self.div_by_u64(rhs.l0)
            } else {
                // Divisor fits in u128
                let d = (rhs.l1 as u128) << 64 | rhs.l0 as u128;
                self.div_by_u128(d)
            }
        } else {
            // Full 256-bit divisor - use Knuth Algorithm D
            self.div_knuth(rhs)
        }
    }
}

impl Uint256 {
    /// Division by u64 using hardware div instruction.
    /// Computes quotient by processing limbs from most to least significant.
    #[inline]
    pub fn div_by_u64(self, d: u64) -> Self {
        // r starts at 0, then accumulates remainder as we go
        let (q3, r) = div_u128_by_u64(self.l3 as u128, d);
        let (q2, r) = div_u128_by_u64((r as u128) << 64 | self.l2 as u128, d);
        let (q1, r) = div_u128_by_u64((r as u128) << 64 | self.l1 as u128, d);
        let (q0, _) = div_u128_by_u64((r as u128) << 64 | self.l0 as u128, d);

        Self { l0: q0, l1: q1, l2: q2, l3: q3 }
    }

    /// Division by u128 - quotient fits in 128 bits when divisor > 2^64.
    #[inline]
    fn div_by_u128(self, d: u128) -> Self {
        // Combine high and low halves for the divisions
        let n_hi = (self.l3 as u128) << 64 | self.l2 as u128;
        let n_lo = (self.l1 as u128) << 64 | self.l0 as u128;

        // First: divide high 128 bits
        let (q_hi, r_hi) = (n_hi / d, n_hi % d);

        // Second: divide (remainder : low 128 bits) by d
        // This requires 256÷128 which we approximate with Knuth-style estimation
        let q_lo = div_u256_by_u128(r_hi, n_lo, d);

        Self {
            l0: q_lo as u64,
            l1: (q_lo >> 64) as u64,
            l2: q_hi as u64,
            l3: (q_hi >> 64) as u64,
        }
    }

    /// Knuth Algorithm D for full 256÷256 division.
    /// Used when divisor has bits in l2 or l3.
    #[inline]
    fn div_knuth(self, d: Self) -> Self {
        // When divisor uses high limbs, quotient is small
        // If d.l3 != 0, quotient fits in ~64 bits
        // If d.l2 != 0, quotient fits in ~128 bits

        if self < d {
            return Self::ZERO;
        }

        if d.l3 != 0 {
            // Quotient fits in 64 bits - single iteration
            self.div_large_divisor_64bit_quotient(d)
        } else {
            // d.l2 != 0, quotient fits in 128 bits - two iterations max
            self.div_large_divisor_128bit_quotient(d)
        }
    }

    /// 256÷256 where divisor uses l3; quotient fits in 64 bits.
    #[inline]
    fn div_large_divisor_64bit_quotient(self, d: Self) -> Self {
        debug_assert!(d.l3 != 0);

        let shift = d.l3.leading_zeros();
        let d_norm = d.shl_u32(shift);
        let mut rem = self.shl_wide(shift);

        let r_hi = rem[4];
        let r_lo = rem[3];
        let d_hi = d_norm.l3;

        let mut qhat = if r_hi >= d_hi {
            u64::MAX
        } else {
            let numer = (r_hi as u128) << 64 | r_lo as u128;
            (numer / d_hi as u128) as u64
        };

        loop {
            let rhat = ((r_hi as u128) << 64 | r_lo as u128)
                .wrapping_sub((qhat as u128) * (d_hi as u128));

            if rhat > u64::MAX as u128 {
                break;
            }

            let left = (qhat as u128) * (d_norm.l2 as u128);
            let right = (rhat << 64) | rem[2] as u128;

            if left <= right {
                break;
            }
            qhat -= 1;
        }

        let borrow = sub_mul_limbs(&mut rem, 0, &d_norm, qhat);
        if borrow {
            qhat -= 1;
            add_back_limbs(&mut rem, 0, &d_norm);
        }

        Self { l0: qhat, l1: 0, l2: 0, l3: 0 }
    }

    /// 256÷192 where divisor uses l2; quotient fits in 128 bits.
    #[inline]
    fn div_large_divisor_128bit_quotient(self, d: Self) -> Self {
        debug_assert!(d.l2 != 0 && d.l3 == 0);

        let shift = d.l2.leading_zeros();
        let d_norm = d.shl_u32(shift);
        let mut rem = self.shl_wide(shift);

        let d_hi = d_norm.l2;
        let mut q_hi = 0u64;
        let mut q_lo = 0u64;

        for j in (0..=1).rev() {
            let r_hi = rem[j + 3];
            let r_lo = rem[j + 2];

            let mut qhat = if r_hi >= d_hi {
                u64::MAX
            } else {
                let numer = (r_hi as u128) << 64 | r_lo as u128;
                (numer / d_hi as u128) as u64
            };

            loop {
                let rhat = ((r_hi as u128) << 64 | r_lo as u128)
                    .wrapping_sub((qhat as u128) * (d_hi as u128));

                if rhat > u64::MAX as u128 {
                    break;
                }

                let left = (qhat as u128) * (d_norm.l1 as u128);
                let right = (rhat << 64) | rem[j + 1] as u128;

                if left <= right {
                    break;
                }
                qhat -= 1;
            }

            let borrow = sub_mul_limbs_3(&mut rem, j, &d_norm, qhat);
            if borrow {
                qhat -= 1;
                add_back_limbs_3(&mut rem, j, &d_norm);
            }

            if j == 1 {
                q_hi = qhat;
            } else {
                q_lo = qhat;
            }
        }

        Self { l0: q_lo, l1: q_hi, l2: 0, l3: 0 }
    }

    /// Count leading zeros
    #[inline]
    pub fn leading_zeros(&self) -> u32 {
        if self.l3 != 0 {
            self.l3.leading_zeros()
        } else if self.l2 != 0 {
            64 + self.l2.leading_zeros()
        } else if self.l1 != 0 {
            128 + self.l1.leading_zeros()
        } else {
            192 + self.l0.leading_zeros()
        }
    }

    /// Shift left, returning 448 bits (7 limbs) to capture overflow.
    /// The extra limbs capture overflow from the shift and are needed for Knuth division
    /// to safely access indices during quotient digit estimation.
    #[inline]
    fn shl_wide(&self, n: u32) -> [u64; 7] {
        if n == 0 {
            return [self.l0, self.l1, self.l2, self.l3, 0, 0, 0];
        }

        let mut result = [0u64; 7];
        let limbs = [self.l0, self.l1, self.l2, self.l3];

        let full_limbs = (n / 64) as usize;
        let bits = n % 64;

        if bits == 0 {
            for i in full_limbs..7 {
                if i - full_limbs < 4 {
                    result[i] = limbs[i - full_limbs];
                }
            }
        } else {
            for i in full_limbs..7 {
                if i - full_limbs < 4 {
                    result[i] = limbs[i - full_limbs] << bits;
                }
                if i > full_limbs && i - full_limbs - 1 < 4 {
                    result[i] |= limbs[i - full_limbs - 1] >> (64 - bits);
                }
            }
        }

        result
    }

    /// Shift left by n bits (n < 256)
    #[inline]
    fn shl_u32(&self, n: u32) -> Self {
        if n == 0 {
            return *self;
        }
        if n >= 256 {
            return Self::ZERO;
        }

        let full_limbs = (n / 64) as usize;
        let bits = n % 64;

        let mut result = [0u64; 4];
        let limbs = [self.l0, self.l1, self.l2, self.l3];

        if bits == 0 {
            for i in full_limbs..4 {
                result[i] = limbs[i - full_limbs];
            }
        } else {
            for i in full_limbs..4 {
                result[i] = limbs[i - full_limbs] << bits;
                if i > full_limbs {
                    result[i] |= limbs[i - full_limbs - 1] >> (64 - bits);
                }
            }
        }

        Self {
            l0: result[0],
            l1: result[1],
            l2: result[2],
            l3: result[3],
        }
    }
}

// ============================================================================
// Division helper functions
// ============================================================================

/// Divide 128-bit by 64-bit, returning (quotient, remainder).
/// Uses hardware `div` instruction directly for optimal codegen.
///
/// # Safety
/// Caller must ensure n_hi < d to avoid division overflow.
#[inline]
#[cfg(target_arch = "x86_64")]
fn div_u128_by_u64(n: u128, d: u64) -> (u64, u64) {
    let n_lo = n as u64;
    let n_hi = (n >> 64) as u64;
    let q: u64;
    let r: u64;
    unsafe {
        std::arch::asm!(
            "div {d}",
            d = in(reg) d,
            inout("rax") n_lo => q,
            inout("rdx") n_hi => r,
            options(pure, nomem, nostack),
        );
    }
    (q, r)
}

#[inline]
#[cfg(not(target_arch = "x86_64"))]
fn div_u128_by_u64(n: u128, d: u64) -> (u64, u64) {
    let q = n / d as u128;
    let r = n % d as u128;
    (q as u64, r as u64)
}

/// Divide 256-bit (hi:lo) by 128-bit divisor.
/// Assumes hi < d (so quotient fits in 128 bits).
#[inline]
fn div_u256_by_u128(hi: u128, lo: u128, d: u128) -> u128 {
    if hi == 0 {
        return lo / d;
    }

    // Knuth-style: normalize and estimate
    let shift = d.leading_zeros();
    let d_norm = d << shift;
    let d_hi = (d_norm >> 64) as u64;

    // Shift numerator
    let n2 = (hi << shift) | (lo >> (128 - shift));
    let n1 = lo << shift;

    // Estimate high 64 bits of quotient
    let n_hi = (n2 >> 64) as u64;

    let mut qhat = if n_hi >= d_hi {
        u64::MAX
    } else {
        ((n2) / d_hi as u128) as u64
    };

    // Refine estimate
    let d_lo = d_norm as u64;
    loop {
        let rhat = n2.wrapping_sub((qhat as u128) * (d_hi as u128));
        if rhat > u64::MAX as u128 {
            break;
        }
        let left = (qhat as u128) * (d_lo as u128);
        let right = (rhat << 64) | (n1 >> 64);
        if left <= right {
            break;
        }
        qhat -= 1;
    }

    let q_hi = qhat as u128;

    // Compute remainder for low quotient digit
    let rem = ((n2 << 64) | (n1 >> 64)).wrapping_sub(q_hi.wrapping_mul(d_norm));

    // Estimate low 64 bits of quotient
    let n_hi2 = (rem >> 64) as u64;

    let mut qhat2 = if n_hi2 >= d_hi {
        u64::MAX
    } else {
        (rem / d_hi as u128) as u64
    };

    // Refine
    loop {
        let rhat = rem.wrapping_sub((qhat2 as u128) * (d_hi as u128));
        if rhat > u64::MAX as u128 {
            break;
        }
        let left = (qhat2 as u128) * (d_lo as u128);
        let right = (rhat << 64) | (n1 as u64) as u128;
        if left <= right {
            break;
        }
        qhat2 -= 1;
    }

    (q_hi << 64) | qhat2 as u128
}

/// Subtract qhat * d from rem[j..j+5], returning true if borrow occurred.
#[inline]
fn sub_mul_limbs(rem: &mut [u64; 7], j: usize, d: &Uint256, qhat: u64) -> bool {
    let d_limbs = [d.l0, d.l1, d.l2, d.l3];
    let mut borrow: u128 = 0;

    for i in 0..4 {
        if j + i < 7 {
            let prod = (qhat as u128) * (d_limbs[i] as u128) + borrow;
            let (diff, b) = rem[j + i].overflowing_sub(prod as u64);
            rem[j + i] = diff;
            borrow = (prod >> 64) + b as u128;
        }
    }

    if j + 4 < 7 {
        let (diff, b) = rem[j + 4].overflowing_sub(borrow as u64);
        rem[j + 4] = diff;
        return b;
    }

    borrow != 0
}

/// Subtract qhat * d (3 limbs) from rem[j..j+4], returning true if borrow occurred.
#[inline]
fn sub_mul_limbs_3(rem: &mut [u64; 7], j: usize, d: &Uint256, qhat: u64) -> bool {
    let d_limbs = [d.l0, d.l1, d.l2];
    let mut borrow: u128 = 0;

    for i in 0..3 {
        if j + i < 7 {
            let prod = (qhat as u128) * (d_limbs[i] as u128) + borrow;
            let (diff, b) = rem[j + i].overflowing_sub(prod as u64);
            rem[j + i] = diff;
            borrow = (prod >> 64) + b as u128;
        }
    }

    if j + 3 < 7 {
        let (diff, b) = rem[j + 3].overflowing_sub(borrow as u64);
        rem[j + 3] = diff;
        return b;
    }

    borrow != 0
}

/// Add d back to rem[j..j+5] after over-subtraction.
#[inline]
fn add_back_limbs(rem: &mut [u64; 7], j: usize, d: &Uint256) {
    let d_limbs = [d.l0, d.l1, d.l2, d.l3];
    let mut carry = false;

    for i in 0..4 {
        if j + i < 7 {
            let (sum, c1) = rem[j + i].overflowing_add(d_limbs[i]);
            let (sum, c2) = sum.overflowing_add(carry as u64);
            rem[j + i] = sum;
            carry = c1 || c2;
        }
    }

    if j + 4 < 7 {
        rem[j + 4] = rem[j + 4].wrapping_add(carry as u64);
    }
}

/// Add d (3 limbs) back to rem[j..j+4] after over-subtraction.
#[inline]
fn add_back_limbs_3(rem: &mut [u64; 7], j: usize, d: &Uint256) {
    let d_limbs = [d.l0, d.l1, d.l2];
    let mut carry = false;

    for i in 0..3 {
        if j + i < 7 {
            let (sum, c1) = rem[j + i].overflowing_add(d_limbs[i]);
            let (sum, c2) = sum.overflowing_add(carry as u64);
            rem[j + i] = sum;
            carry = c1 || c2;
        }
    }

    if j + 3 < 7 {
        rem[j + 3] = rem[j + 3].wrapping_add(carry as u64);
    }
}

// ============================================================================
// Optimal inline assembly implementations
// ============================================================================

/// Optimal u256 multiplication using inline assembly with BMI2.
/// Carefully scheduled to minimize register pressure and spills.
#[inline(never)]
#[cfg(target_arch = "x86_64")]
pub fn optimal_u256_mul(a: &Uint256, b: &Uint256) -> Uint256 {
    let mut r0: u64;
    let mut r1: u64;
    let mut r2: u64;
    let mut r3: u64;

    unsafe {
        std::arch::asm!(
            // Load b[0] into rdx for first mulx operations
            "mov rdx, [{b}]",

            // Clear t4 for later use as carry accumulator
            "xor {t4:e}, {t4:e}",

            // Column 0: a0*b0
            "mulx {t1}, {r0}, [{a}]",           // t1:r0 = a0*b0

            // Column 1: a0*b1 + a1*b0
            "mulx {t2}, {t0}, [{a} + 8]",       // t2:t0 = a1*b0
            "add {t1}, {t0}",                   // t1 += lo(a1*b0)
            "adc {t2}, 0",                      // t2 += carry

            "mov rdx, [{b} + 8]",               // rdx = b1
            "mulx {t3}, {t0}, [{a}]",           // t3:t0 = a0*b1
            "add {t1}, {t0}",                   // t1 += lo(a0*b1), t1 is now r1
            "adc {t2}, {t3}",                   // t2 += hi(a0*b1) + carry
            "adc {t4}, 0",                      // t4 = overflow carry

            "mov {r1}, {t1}",                   // save r1

            // Column 2: a0*b2 + a1*b1 + a2*b0
            "mulx {t1}, {t0}, [{a} + 8]",       // t1:t0 = a1*b1
            "add {t2}, {t0}",                   // t2 += lo(a1*b1)
            "adc {t4}, {t1}",                   // t4 += hi(a1*b1) + carry

            "mov rdx, [{b}]",                   // rdx = b0
            "mulx {t1}, {t0}, [{a} + 16]",      // t1:t0 = a2*b0
            "add {t2}, {t0}",                   // t2 += lo(a2*b0)
            "adc {t4}, {t1}",                   // t4 += hi(a2*b0) + carry

            "mov rdx, [{b} + 16]",              // rdx = b2
            "mulx {t3}, {t0}, [{a}]",           // t3:t0 = a0*b2
            "add {t2}, {t0}",                   // t2 += lo(a0*b2), t2 is now r2
            "adc {t4}, {t3}",                   // t4 += hi(a0*b2) + carry
            "setc {t3:l}",                      // save final carry

            "mov {r2}, {t2}",                   // save r2

            // Column 3: a0*b3 + a1*b2 + a2*b1 + a3*b0 + carries (only need low 64 bits)
            "mov rdx, [{b} + 24]",
            "imul rdx, [{a}]",                  // a0*b3
            "add {t4}, rdx",

            "mov rdx, [{b} + 16]",
            "imul rdx, [{a} + 8]",              // a1*b2
            "add {t4}, rdx",

            "mov rdx, [{b} + 8]",
            "imul rdx, [{a} + 16]",             // a2*b1
            "add {t4}, rdx",

            "mov rdx, [{b}]",
            "imul rdx, [{a} + 24]",             // a3*b0
            "add {t4}, rdx",

            "movzx {t3:e}, {t3:l}",             // zero-extend carry
            "add {t4}, {t3}",                   // add final carry
            "mov {r3}, {t4}",

            a = in(reg) a as *const Uint256 as *const u64,
            b = in(reg) b as *const Uint256 as *const u64,
            r0 = out(reg) r0,
            r1 = out(reg) r1,
            r2 = out(reg) r2,
            r3 = out(reg) r3,
            t0 = out(reg) _,
            t1 = out(reg) _,
            t2 = out(reg) _,
            t3 = out(reg) _,
            t4 = out(reg) _,
            out("rdx") _,
            options(nostack, pure, readonly),
        );
    }

    Uint256 { l0: r0, l1: r1, l2: r2, l3: r3 }
}

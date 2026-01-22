//! 256-bit signed integer implemented as four 64-bit limbs.
//!
//! Uses two's complement representation. Addition, subtraction, and wrapping
//! multiplication are bitwise identical to unsigned operations.

use std::cmp::Ordering;
use crate::u256::Uint256;

/// 256-bit signed integer stored as four 64-bit limbs.
///
/// Uses two's complement representation. The high limb's MSB is the sign bit.
/// Field order matches native memory layout for optimal codegen.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[cfg(target_endian = "little")]
pub struct Int256 {
    pub l0: u64, // bits 0-63 (lowest address)
    pub l1: u64, // bits 64-127
    pub l2: u64, // bits 128-191
    pub l3: u64, // bits 192-255, MSB is sign bit (highest address)
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[cfg(target_endian = "big")]
pub struct Int256 {
    pub l3: u64, // bits 192-255, MSB is sign bit (lowest address)
    pub l2: u64, // bits 128-191
    pub l1: u64, // bits 64-127
    pub l0: u64, // bits 0-63 (highest address)
}

impl Int256 {
    pub const ZERO: Self = Self { l0: 0, l1: 0, l2: 0, l3: 0 };
    pub const ONE: Self = Self { l0: 1, l1: 0, l2: 0, l3: 0 };
    pub const NEG_ONE: Self = Self {
        l0: u64::MAX,
        l1: u64::MAX,
        l2: u64::MAX,
        l3: u64::MAX,
    };
    pub const MIN: Self = Self {
        l0: 0,
        l1: 0,
        l2: 0,
        l3: 0x8000_0000_0000_0000,
    };
    pub const MAX: Self = Self {
        l0: u64::MAX,
        l1: u64::MAX,
        l2: u64::MAX,
        l3: 0x7FFF_FFFF_FFFF_FFFF,
    };

    #[inline]
    pub const fn new(l0: u64, l1: u64, l2: u64, l3: u64) -> Self {
        Self { l0, l1, l2, l3 }
    }

    /// Create from i128, sign-extending to 256 bits.
    #[inline]
    pub const fn from_i128(v: i128) -> Self {
        let sign_ext = if v < 0 { u64::MAX } else { 0 };
        Self {
            l0: v as u64,
            l1: (v >> 64) as u64,
            l2: sign_ext,
            l3: sign_ext,
        }
    }

    /// Convert to i128, truncating high bits.
    #[inline]
    pub const fn to_i128(self) -> i128 {
        (self.l1 as i128) << 64 | self.l0 as i128
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        self.l0 == 0 && self.l1 == 0 && self.l2 == 0 && self.l3 == 0
    }

    #[inline]
    pub fn is_negative(&self) -> bool {
        (self.l3 as i64) < 0
    }

    #[inline]
    pub fn is_positive(&self) -> bool {
        !self.is_negative() && !self.is_zero()
    }

    #[inline]
    pub fn signum(&self) -> Self {
        if self.is_zero() {
            Self::ZERO
        } else if self.is_negative() {
            Self::NEG_ONE
        } else {
            Self::ONE
        }
    }

    /// Absolute value. Note: MIN.abs() overflows (returns MIN).
    #[inline]
    pub fn abs(&self) -> Self {
        if self.is_negative() {
            Self::ZERO - *self
        } else {
            *self
        }
    }

    /// Wrapping absolute value.
    #[inline]
    pub fn wrapping_abs(&self) -> Self {
        self.abs()
    }

    /// Checked absolute value. Returns None for MIN.
    #[inline]
    pub fn checked_abs(&self) -> Option<Self> {
        if *self == Self::MIN {
            None
        } else {
            Some(self.abs())
        }
    }

    /// Convert to unsigned, interpreting bits directly.
    #[inline]
    pub fn to_uint256(&self) -> Uint256 {
        Uint256 {
            l0: self.l0,
            l1: self.l1,
            l2: self.l2,
            l3: self.l3,
        }
    }

    /// Create from unsigned, interpreting bits directly.
    #[inline]
    pub fn from_uint256(u: Uint256) -> Self {
        Self {
            l0: u.l0,
            l1: u.l1,
            l2: u.l2,
            l3: u.l3,
        }
    }

    /// Count leading zeros (not counting sign, just the bits).
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
}

// ============================================================================
// Addition (identical to unsigned - two's complement)
// ============================================================================

impl std::ops::Add for Int256 {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let (l0, c0) = self.l0.overflowing_add(rhs.l0);
        let (l1, c1) = self.l1.carrying_add(rhs.l1, c0);
        let (l2, c2) = self.l2.carrying_add(rhs.l2, c1);
        let (l3, _) = self.l3.carrying_add(rhs.l3, c2);
        Self { l0, l1, l2, l3 }
    }
}

// ============================================================================
// Subtraction (identical to unsigned - two's complement)
// ============================================================================

impl std::ops::Sub for Int256 {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        let (l0, b0) = self.l0.overflowing_sub(rhs.l0);
        let (l1, b1) = self.l1.borrowing_sub(rhs.l1, b0);
        let (l2, b2) = self.l2.borrowing_sub(rhs.l2, b1);
        let (l3, _) = self.l3.borrowing_sub(rhs.l3, b2);
        Self { l0, l1, l2, l3 }
    }
}

// ============================================================================
// Multiplication (wrapping - identical to unsigned for low bits)
// ============================================================================

impl std::ops::Mul for Int256 {
    type Output = Self;

    /// Wrapping multiplication. Low 256 bits are identical for signed/unsigned.
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        // Delegate to unsigned multiplication - low bits are identical
        let result = self.to_uint256() * rhs.to_uint256();
        Self::from_uint256(result)
    }
}

// ============================================================================
// Negation
// ============================================================================

impl std::ops::Neg for Int256 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Self::ZERO - self
    }
}

// ============================================================================
// Division (requires sign handling)
// ============================================================================

impl std::ops::Div for Int256 {
    type Output = Self;

    /// Signed division with truncation toward zero.
    ///
    /// Strategy: Convert to unsigned magnitudes, divide, fix sign.
    /// This avoids implementing a separate signed division algorithm.
    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        if rhs.is_zero() {
            panic!("attempt to divide by zero");
        }

        // Handle signs
        let self_neg = self.is_negative();
        let rhs_neg = rhs.is_negative();
        let result_neg = self_neg ^ rhs_neg;

        // Get absolute values as unsigned
        let a = if self_neg {
            (Self::ZERO - self).to_uint256()
        } else {
            self.to_uint256()
        };

        let b = if rhs_neg {
            (Self::ZERO - rhs).to_uint256()
        } else {
            rhs.to_uint256()
        };

        // Unsigned division
        let q = a / b;

        // Fix sign of result
        let result = Self::from_uint256(q);
        if result_neg {
            Self::ZERO - result
        } else {
            result
        }
    }
}

impl std::ops::Rem for Int256 {
    type Output = Self;

    /// Signed remainder. Result has same sign as dividend.
    ///
    /// Uses the identity: a % b = a - (a / b) * b
    #[inline]
    fn rem(self, rhs: Self) -> Self::Output {
        if rhs.is_zero() {
            panic!("attempt to calculate remainder with a divisor of zero");
        }

        // Handle signs
        let self_neg = self.is_negative();
        let rhs_neg = rhs.is_negative();

        // Get absolute values as unsigned
        let a = if self_neg {
            (Self::ZERO - self).to_uint256()
        } else {
            self.to_uint256()
        };

        let b = if rhs_neg {
            (Self::ZERO - rhs).to_uint256()
        } else {
            rhs.to_uint256()
        };

        // Unsigned division to get quotient
        let q = a / b;

        // remainder = |a| - q * |b|
        let r = a - q * b;

        // Result has same sign as dividend
        let result = Self::from_uint256(r);
        if self_neg && !result.is_zero() {
            Self::ZERO - result
        } else {
            result
        }
    }
}

// ============================================================================
// Comparison (high limb interpreted as signed)
// ============================================================================

impl PartialEq for Int256 {
    fn eq(&self, other: &Self) -> bool {
        self.l0 == other.l0 && self.l1 == other.l1 && self.l2 == other.l2 && self.l3 == other.l3
    }
}

impl Eq for Int256 {}

impl PartialOrd for Int256 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Int256 {
    /// Signed comparison: interpret highest limb as signed, rest as unsigned.
    fn cmp(&self, other: &Self) -> Ordering {
        // First compare l3 as signed
        match (self.l3 as i64).cmp(&(other.l3 as i64)) {
            Ordering::Equal => {
                // Then compare remaining limbs as unsigned
                match self.l2.cmp(&other.l2) {
                    Ordering::Equal => match self.l1.cmp(&other.l1) {
                        Ordering::Equal => self.l0.cmp(&other.l0),
                        other => other,
                    },
                    other => other,
                }
            }
            other => other,
        }
    }
}

// ============================================================================
// Bitwise operations
// ============================================================================

impl std::ops::Not for Int256 {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self {
            l0: !self.l0,
            l1: !self.l1,
            l2: !self.l2,
            l3: !self.l3,
        }
    }
}

impl std::ops::BitAnd for Int256 {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self {
            l0: self.l0 & rhs.l0,
            l1: self.l1 & rhs.l1,
            l2: self.l2 & rhs.l2,
            l3: self.l3 & rhs.l3,
        }
    }
}

impl std::ops::BitOr for Int256 {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            l0: self.l0 | rhs.l0,
            l1: self.l1 | rhs.l1,
            l2: self.l2 | rhs.l2,
            l3: self.l3 | rhs.l3,
        }
    }
}

impl std::ops::BitXor for Int256 {
    type Output = Self;

    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self {
            l0: self.l0 ^ rhs.l0,
            l1: self.l1 ^ rhs.l1,
            l2: self.l2 ^ rhs.l2,
            l3: self.l3 ^ rhs.l3,
        }
    }
}

// ============================================================================
// Shifts (arithmetic right shift for signed)
// ============================================================================

impl std::ops::Shl<u32> for Int256 {
    type Output = Self;

    #[inline]
    fn shl(self, n: u32) -> Self::Output {
        if n >= 256 {
            return Self::ZERO;
        }
        if n == 0 {
            return self;
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

impl std::ops::Shr<u32> for Int256 {
    type Output = Self;

    /// Arithmetic right shift: fills with sign bit.
    #[inline]
    fn shr(self, n: u32) -> Self::Output {
        if n >= 256 {
            return if self.is_negative() { Self::NEG_ONE } else { Self::ZERO };
        }
        if n == 0 {
            return self;
        }

        let full_limbs = (n / 64) as usize;
        let bits = n % 64;
        let sign_fill = if self.is_negative() { u64::MAX } else { 0 };

        let mut result = [sign_fill; 4];
        let limbs = [self.l0, self.l1, self.l2, self.l3];

        if bits == 0 {
            for i in 0..(4 - full_limbs) {
                result[i] = limbs[i + full_limbs];
            }
        } else {
            for i in 0..(4 - full_limbs) {
                result[i] = limbs[i + full_limbs] >> bits;
                if i + full_limbs + 1 < 4 {
                    result[i] |= limbs[i + full_limbs + 1] << (64 - bits);
                } else {
                    // Fill from sign for the highest partial shift
                    result[i] |= sign_fill << (64 - bits);
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

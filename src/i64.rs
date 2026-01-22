//! 64-bit signed integer implemented as two 32-bit limbs.
//!
//! Uses two's complement representation. Addition, subtraction, and wrapping
//! multiplication are bitwise identical to unsigned operations.

use std::cmp::Ordering;

/// 64-bit signed integer stored as two 32-bit limbs.
///
/// Uses two's complement representation. The high limb's MSB is the sign bit.
/// Field order matches native ABI layout for optimal codegen.
#[derive(Debug, Clone, Copy)]
#[cfg(target_endian = "little")]
pub struct Int64 {
    pub l: u32, // bits 0-31 (lower address)
    pub h: u32, // bits 32-63, MSB is sign bit (higher address)
}

#[derive(Debug, Clone, Copy)]
#[cfg(target_endian = "big")]
pub struct Int64 {
    pub h: u32, // bits 32-63, MSB is sign bit (lower address)
    pub l: u32, // bits 0-31 (higher address)
}

impl Int64 {
    pub const ZERO: Self = Self { l: 0, h: 0 };
    pub const ONE: Self = Self { l: 1, h: 0 };
    pub const NEG_ONE: Self = Self {
        l: u32::MAX,
        h: u32::MAX,
    };
    pub const MIN: Self = Self {
        l: 0,
        h: 0x8000_0000,
    };
    pub const MAX: Self = Self {
        l: u32::MAX,
        h: 0x7FFF_FFFF,
    };

    pub const fn new(l: u32, h: u32) -> Self {
        Self { l, h }
    }

    pub const fn from_i64(v: i64) -> Self {
        Self {
            l: v as u32,
            h: (v >> 32) as u32,
        }
    }

    pub const fn to_i64(self) -> i64 {
        (self.h as i64) << 32 | self.l as i64
    }

    pub fn is_zero(&self) -> bool {
        self.l == 0 && self.h == 0
    }

    pub fn is_negative(&self) -> bool {
        (self.h as i32) < 0
    }

    pub fn is_positive(&self) -> bool {
        !self.is_negative() && !self.is_zero()
    }

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
    pub fn abs(&self) -> Self {
        if self.is_negative() {
            Self::ZERO - *self
        } else {
            *self
        }
    }

    /// Wrapping absolute value.
    pub fn wrapping_abs(&self) -> Self {
        self.abs()
    }

    /// Checked absolute value. Returns None for MIN.
    pub fn checked_abs(&self) -> Option<Self> {
        if *self == Self::MIN {
            None
        } else {
            Some(self.abs())
        }
    }
}

// ============================================================================
// Addition (identical to unsigned - two's complement)
// ============================================================================

impl std::ops::Add for Int64 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let (l, carry) = self.l.overflowing_add(rhs.l);
        let h = self.h.wrapping_add(rhs.h).wrapping_add(carry as u32);
        Self { l, h }
    }
}

// ============================================================================
// Subtraction (identical to unsigned - two's complement)
// ============================================================================

impl std::ops::Sub for Int64 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let (l, borrow) = self.l.overflowing_sub(rhs.l);
        let h = self.h.wrapping_sub(rhs.h).wrapping_sub(borrow as u32);
        Self { l, h }
    }
}

// ============================================================================
// Multiplication (wrapping - identical to unsigned for low bits)
// ============================================================================

impl std::ops::Mul for Int64 {
    type Output = Self;

    /// Wrapping multiplication. Low 64 bits are identical for signed/unsigned.
    fn mul(self, rhs: Self) -> Self::Output {
        let p0 = (self.l as u64) * (rhs.l as u64);
        let p0_lo = p0 as u32;
        let p0_hi = (p0 >> 32) as u32;

        let t1 = self.l.wrapping_mul(rhs.h);
        let t2 = self.h.wrapping_mul(rhs.l);

        let h = p0_hi.wrapping_add(t1).wrapping_add(t2);

        Self { l: p0_lo, h }
    }
}

// ============================================================================
// Negation
// ============================================================================

impl std::ops::Neg for Int64 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::ZERO - self
    }
}

// ============================================================================
// Division (requires sign handling)
// ============================================================================

impl std::ops::Div for Int64 {
    type Output = Self;

    /// Signed division with truncation toward zero.
    fn div(self, rhs: Self) -> Self::Output {
        Self::from_i64(self.to_i64() / rhs.to_i64())
    }
}

impl std::ops::Rem for Int64 {
    type Output = Self;

    /// Signed remainder. Result has same sign as dividend.
    fn rem(self, rhs: Self) -> Self::Output {
        Self::from_i64(self.to_i64() % rhs.to_i64())
    }
}

// ============================================================================
// Comparison (high limb interpreted as signed)
// ============================================================================

impl PartialEq for Int64 {
    fn eq(&self, other: &Self) -> bool {
        self.h == other.h && self.l == other.l
    }
}

impl Eq for Int64 {}

impl PartialOrd for Int64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Int64 {
    /// Signed comparison: interpret high limb as signed.
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.h as i32).cmp(&(other.h as i32)) {
            Ordering::Equal => self.l.cmp(&other.l),
            other => other,
        }
    }
}

// ============================================================================
// Bitwise operations
// ============================================================================

impl std::ops::Not for Int64 {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self {
            l: !self.l,
            h: !self.h,
        }
    }
}

impl std::ops::BitAnd for Int64 {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self {
            l: self.l & rhs.l,
            h: self.h & rhs.h,
        }
    }
}

impl std::ops::BitOr for Int64 {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            l: self.l | rhs.l,
            h: self.h | rhs.h,
        }
    }
}

impl std::ops::BitXor for Int64 {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self {
            l: self.l ^ rhs.l,
            h: self.h ^ rhs.h,
        }
    }
}

// ============================================================================
// Shifts (arithmetic right shift for signed)
// ============================================================================

impl std::ops::Shl<u32> for Int64 {
    type Output = Self;

    fn shl(self, n: u32) -> Self::Output {
        if n >= 64 {
            Self::ZERO
        } else if n >= 32 {
            Self {
                l: 0,
                h: self.l << (n - 32),
            }
        } else if n == 0 {
            self
        } else {
            Self {
                l: self.l << n,
                h: (self.h << n) | (self.l >> (32 - n)),
            }
        }
    }
}

impl std::ops::Shr<u32> for Int64 {
    type Output = Self;

    /// Arithmetic right shift: fills with sign bit.
    fn shr(self, n: u32) -> Self::Output {
        if n >= 64 {
            if self.is_negative() {
                Self::NEG_ONE
            } else {
                Self::ZERO
            }
        } else if n >= 32 {
            // Arithmetic shift of high limb
            let h_signed = self.h as i32;
            let new_l = (h_signed >> (n - 32)) as u32;
            let new_h = (h_signed >> 31) as u32; // all sign bits
            Self { l: new_l, h: new_h }
        } else if n == 0 {
            self
        } else {
            let h_signed = self.h as i32;
            Self {
                l: (self.l >> n) | (self.h << (32 - n)),
                h: (h_signed >> n) as u32,
            }
        }
    }
}

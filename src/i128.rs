//! 128-bit signed integer implemented as two 64-bit limbs.
//!
//! Uses two's complement representation. Addition, subtraction, and wrapping
//! multiplication are bitwise identical to unsigned operations.

use std::cmp::Ordering;

/// 128-bit signed integer stored as two 64-bit limbs.
///
/// Uses two's complement representation. The high limb's MSB is the sign bit.
/// Field order matches native i128 ABI layout for optimal codegen.
#[derive(Debug, Clone, Copy)]
#[cfg(target_endian = "little")]
pub struct Int128 {
    pub l: u64, // bits 0-63 (lower address)
    pub h: u64, // bits 64-127, MSB is sign bit (higher address)
}

#[derive(Debug, Clone, Copy)]
#[cfg(target_endian = "big")]
pub struct Int128 {
    pub h: u64, // bits 64-127, MSB is sign bit (lower address)
    pub l: u64, // bits 0-63 (higher address)
}

impl Int128 {
    pub const ZERO: Self = Self { l: 0, h: 0 };
    pub const ONE: Self = Self { l: 1, h: 0 };
    pub const NEG_ONE: Self = Self { l: u64::MAX, h: u64::MAX };
    pub const MIN: Self = Self { l: 0, h: 0x8000_0000_0000_0000 };
    pub const MAX: Self = Self { l: u64::MAX, h: 0x7FFF_FFFF_FFFF_FFFF };

    #[inline]
    pub const fn new(l: u64, h: u64) -> Self {
        Self { l, h }
    }

    #[inline]
    pub const fn from_i128(v: i128) -> Self {
        Self {
            l: v as u64,
            h: (v >> 64) as u64,
        }
    }

    #[inline]
    pub const fn to_i128(self) -> i128 {
        (self.h as i128) << 64 | self.l as i128
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        self.l == 0 && self.h == 0
    }

    #[inline]
    pub fn is_negative(&self) -> bool {
        (self.h as i64) < 0
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

    /// Helper for 64x64->128 multiplication (portable fallback).
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

// ============================================================================
// Addition (identical to unsigned - two's complement)
// ============================================================================

impl std::ops::Add for Int128 {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let (l, carry) = self.l.overflowing_add(rhs.l);
        let h = self.h.wrapping_add(rhs.h).wrapping_add(carry as u64);
        Self { l, h }
    }
}

// ============================================================================
// Subtraction (identical to unsigned - two's complement)
// ============================================================================

impl std::ops::Sub for Int128 {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        let (l, borrow) = self.l.overflowing_sub(rhs.l);
        let h = self.h.wrapping_sub(rhs.h).wrapping_sub(borrow as u64);
        Self { l, h }
    }
}

// ============================================================================
// Multiplication (wrapping - identical to unsigned for low bits)
// ============================================================================

impl std::ops::Mul for Int128 {
    type Output = Self;

    /// Wrapping multiplication. Low 128 bits are identical for signed/unsigned.
    #[inline]
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
        Self { l: p0_lo, h }
    }
}

// ============================================================================
// Negation
// ============================================================================

impl std::ops::Neg for Int128 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Self::ZERO - self
    }
}

// ============================================================================
// Division (requires sign handling)
// ============================================================================

impl std::ops::Div for Int128 {
    type Output = Self;

    /// Signed division with truncation toward zero.
    /// Delegates to native i128 for optimal codegen (__divti3).
    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        Self::from_i128(self.to_i128() / rhs.to_i128())
    }
}

impl std::ops::Rem for Int128 {
    type Output = Self;

    /// Signed remainder. Result has same sign as dividend.
    /// Delegates to native i128 for optimal codegen (__modti3).
    #[inline]
    fn rem(self, rhs: Self) -> Self::Output {
        Self::from_i128(self.to_i128() % rhs.to_i128())
    }
}

// ============================================================================
// Comparison (high limb interpreted as signed)
// ============================================================================

impl PartialEq for Int128 {
    fn eq(&self, other: &Self) -> bool {
        self.h == other.h && self.l == other.l
    }
}

impl Eq for Int128 {}

impl PartialOrd for Int128 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Int128 {
    /// Signed comparison: interpret high limb as signed.
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.h as i64).cmp(&(other.h as i64)) {
            Ordering::Equal => self.l.cmp(&other.l),
            other => other,
        }
    }
}

// ============================================================================
// Bitwise operations
// ============================================================================

impl std::ops::Not for Int128 {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self { l: !self.l, h: !self.h }
    }
}

impl std::ops::BitAnd for Int128 {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self { l: self.l & rhs.l, h: self.h & rhs.h }
    }
}

impl std::ops::BitOr for Int128 {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self { l: self.l | rhs.l, h: self.h | rhs.h }
    }
}

impl std::ops::BitXor for Int128 {
    type Output = Self;

    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self { l: self.l ^ rhs.l, h: self.h ^ rhs.h }
    }
}

// ============================================================================
// Shifts (arithmetic right shift for signed)
// ============================================================================

impl std::ops::Shl<u32> for Int128 {
    type Output = Self;

    #[inline]
    fn shl(self, n: u32) -> Self::Output {
        if n >= 128 {
            Self::ZERO
        } else if n >= 64 {
            Self { l: 0, h: self.l << (n - 64) }
        } else if n == 0 {
            self
        } else {
            Self {
                l: self.l << n,
                h: (self.h << n) | (self.l >> (64 - n)),
            }
        }
    }
}

impl std::ops::Shr<u32> for Int128 {
    type Output = Self;

    /// Arithmetic right shift: fills with sign bit.
    #[inline]
    fn shr(self, n: u32) -> Self::Output {
        if n >= 128 {
            if self.is_negative() { Self::NEG_ONE } else { Self::ZERO }
        } else if n >= 64 {
            let h_signed = self.h as i64;
            let new_l = (h_signed >> (n - 64)) as u64;
            let new_h = (h_signed >> 63) as u64;
            Self { l: new_l, h: new_h }
        } else if n == 0 {
            self
        } else {
            let h_signed = self.h as i64;
            Self {
                l: (self.l >> n) | (self.h << (64 - n)),
                h: (h_signed >> n) as u64,
            }
        }
    }
}

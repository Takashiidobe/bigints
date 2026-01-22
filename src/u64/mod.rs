//! 64-bit unsigned integer implemented as two 32-bit limbs.
//!
//! LLVM generates optimal architecture-specific instructions from portable Rust:
//! - x86-32: `mul` (32×32→64), `div` (64÷32→32,32), `adc`/`sbb`
//! - ARM32: `umull`, `umlal`, `adds`/`adc`, `subs`/`sbc`
//!
//! No inline assembly needed - LLVM recognizes the patterns.

use std::cmp::Ordering;

/// 64-bit unsigned integer stored as two 32-bit limbs.
///
/// Field order matches native ABI layout for optimal codegen:
/// - Little-endian: { l, h } - LSB at lower address
/// - Big-endian: { h, l } - MSB at lower address
#[derive(Debug, Clone, Copy)]
#[cfg(target_endian = "little")]
pub struct Uint64 {
    pub l: u32, // bits 0-31 (lower address)
    pub h: u32, // bits 32-63 (higher address)
}

#[derive(Debug, Clone, Copy)]
#[cfg(target_endian = "big")]
pub struct Uint64 {
    pub h: u32, // bits 32-63 (lower address)
    pub l: u32, // bits 0-31 (higher address)
}

impl Uint64 {
    pub const ZERO: Self = Self { l: 0, h: 0 };
    pub const MAX: Self = Self {
        l: u32::MAX,
        h: u32::MAX,
    };

    pub const fn new(l: u32, h: u32) -> Self {
        Self { l, h }
    }

    pub const fn from_u64(v: u64) -> Self {
        Self {
            l: v as u32,
            h: (v >> 32) as u32,
        }
    }

    pub const fn to_u64(self) -> u64 {
        (self.h as u64) << 32 | self.l as u64
    }

    pub fn is_zero(&self) -> bool {
        self.l == 0 && self.h == 0
    }

    pub fn leading_zeros(&self) -> u32 {
        if self.h != 0 {
            self.h.leading_zeros()
        } else {
            32 + self.l.leading_zeros()
        }
    }
}

// ============================================================================
// Addition
// ============================================================================

impl std::ops::Add for Uint64 {
    type Output = Self;

    /// 64-bit addition with carry propagation.
    ///
    /// LLVM generates:
    /// - x86-32: `add`/`adc`
    /// - ARM32: `adds`/`adc`
    #[inline(never)]
    fn add(self, rhs: Self) -> Self::Output {
        let (l, carry) = self.l.overflowing_add(rhs.l);
        let h = self.h.wrapping_add(rhs.h).wrapping_add(carry as u32);
        Self { l, h }
    }
}

// ============================================================================
// Subtraction
// ============================================================================

impl std::ops::Sub for Uint64 {
    type Output = Self;

    /// 64-bit subtraction with borrow propagation.
    ///
    /// LLVM generates:
    /// - x86-32: `sub`/`sbb`
    /// - ARM32: `subs`/`sbc`
    #[inline(never)]
    fn sub(self, rhs: Self) -> Self::Output {
        let (l, borrow) = self.l.overflowing_sub(rhs.l);
        let h = self.h.wrapping_sub(rhs.h).wrapping_sub(borrow as u32);
        Self { l, h }
    }
}

// ============================================================================
// Multiplication
// ============================================================================

impl std::ops::Mul for Uint64 {
    type Output = Self;

    /// 64-bit multiplication, keeping low 64 bits.
    ///
    /// LLVM generates:
    /// - x86-32: `mul` for 32×32→64, then cross-product additions
    /// - ARM32: `umull` for 32×32→64, `mla` for multiply-accumulate
    ///
    /// Algorithm:
    /// ```text
    ///        a.h : a.l
    ///      × b.h : b.l
    ///     ──────────────
    ///              a.l × b.l  →  (p0_hi : p0_lo)
    ///       a.l × b.h         →  (  __  : t1)     [only low 32 matters]
    ///       a.h × b.l         →  (  __  : t2)     [only low 32 matters]
    ///     ──────────────
    ///     result.h = p0_hi + t1 + t2
    ///     result.l = p0_lo
    /// ```
    #[inline(never)]
    fn mul(self, rhs: Self) -> Self::Output {
        // This cast pattern tells LLVM we want 32×32→64
        // x86-32: emits `mul`
        // ARM32: emits `umull`
        let p0 = (self.l as u64) * (rhs.l as u64);
        let p0_lo = p0 as u32;
        let p0_hi = (p0 >> 32) as u32;

        // Cross terms - wrapping_mul tells LLVM we only need low 32 bits
        // ARM32: emits `mla` (multiply-accumulate) when combined with add
        let t1 = self.l.wrapping_mul(rhs.h);
        let t2 = self.h.wrapping_mul(rhs.l);

        let h = p0_hi.wrapping_add(t1).wrapping_add(t2);

        Self { l: p0_lo, h }
    }
}

// ============================================================================
// Division
// ============================================================================

impl std::ops::Div for Uint64 {
    type Output = Self;

    /// 64-bit division.
    ///
    /// For 64÷32 on x86-32, LLVM generates:
    /// ```asm
    /// div ecx        ; EDX:EAX / ECX → EAX, EDX
    /// ```
    ///
    /// ARM32 has no hardware divide on most cores, so LLVM calls
    /// `__aeabi_uldivmod` (software division).
    fn div(self, rhs: Self) -> Self::Output {
        // LLVM handles the optimal instruction selection
        Self::from_u64(self.to_u64() / rhs.to_u64())
    }
}

impl std::ops::Rem for Uint64 {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        Self::from_u64(self.to_u64() % rhs.to_u64())
    }
}

// ============================================================================
// Widening operations
// ============================================================================

impl Uint64 {
    /// Division by u32 - fast path for small divisors.
    ///
    /// On x86-32, LLVM generates two `div` instructions.
    /// On ARM32, calls software division.
    pub fn div_by_u32(self, d: u32) -> Self {
        Self::from_u64(self.to_u64() / d as u64)
    }

    /// Division by u32 with remainder.
    pub fn divrem_by_u32(self, d: u32) -> (Self, u32) {
        let n = self.to_u64();
        let q = n / d as u64;
        let r = (n % d as u64) as u32;
        (Self::from_u64(q), r)
    }

    /// Full 64×64→128 multiplication.
    ///
    /// LLVM generates:
    /// - x86-32: Four `mul` instructions with `adc` chain
    /// - ARM32: `umull` + `umlal` (multiply-accumulate long)
    ///
    /// The `umlal` instruction does: Rd_hi:Rd_lo += Rm × Rs
    /// This is perfect for accumulating partial products.
    pub fn widening_mul(self, rhs: Self) -> (Self, Self) {
        let a0 = self.l as u64;
        let a1 = self.h as u64;
        let b0 = rhs.l as u64;
        let b1 = rhs.h as u64;

        // Four partial products - LLVM recognizes this pattern
        // ARM32: uses umull for first, umlal for accumulates
        let p00 = a0 * b0; // bits 0-63
        let p01 = a0 * b1; // bits 32-95
        let p10 = a1 * b0; // bits 32-95
        let p11 = a1 * b1; // bits 64-127

        // Combine with carries
        let r0 = p00 as u32;
        let carry = p00 >> 32;

        let mid = carry + (p01 as u32 as u64) + (p10 as u32 as u64);
        let r1 = mid as u32;
        let carry = mid >> 32;

        let mid = carry + (p01 >> 32) + (p10 >> 32) + (p11 as u32 as u64);
        let r2 = mid as u32;
        let carry = mid >> 32;

        let r3 = (carry + (p11 >> 32)) as u32;

        (
            Self { l: r2, h: r3 }, // high
            Self { l: r0, h: r1 }, // low
        )
    }
}

// ============================================================================
// Comparison traits
// ============================================================================

impl PartialEq for Uint64 {
    fn eq(&self, other: &Self) -> bool {
        self.h == other.h && self.l == other.l
    }
}

impl Eq for Uint64 {}

impl PartialOrd for Uint64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Uint64 {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.h.cmp(&other.h) {
            Ordering::Equal => self.l.cmp(&other.l),
            other => other,
        }
    }
}

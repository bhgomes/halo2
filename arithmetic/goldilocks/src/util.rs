use std::arch::asm;
use std::hint::unreachable_unchecked;

#[inline(always)]
pub fn assume(p: bool) {
    debug_assert!(p);
    if !p {
        unsafe {
            unreachable_unchecked();
        }
    }
}

/// Try to force Rust to emit a branch. Example:
///     if x > 2 {
///         y = foo();
///         branch_hint();
///     } else {
///         y = bar();
///     }
/// This function has no semantics. It is a hint only.
#[inline(always)]
pub fn branch_hint() {
    unsafe {
        asm!("", options(nomem, nostack, preserves_flags));
    }
}

/// Fast addition modulo ORDER for x86-64.
/// This function is marked unsafe for the following reasons:
///   - It is only correct if x + y < 2**64 + ORDER = 0x1ffffffff00000001.
///   - It is only faster in some circumstances. In particular, on x86 it overwrites both inputs in
///     the registers, so its use is not recommended when either input will be used again.
#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub(crate) unsafe fn add_no_canonicalize_trashing_input(x: u64, y: u64) -> u64 {
    let res_wrapped: u64;
    let adjustment: u64;
    asm!(
        "add {0}, {1}",
        // Trick. The carry flag is set iff the addition overflowed.
        // sbb x, y does x := x - y - CF. In our case, x and y are both {1:e}, so it simply does
        // {1:e} := 0xffffffff on overflow and {1:e} := 0 otherwise. {1:e} is the low 32 bits of
        // {1}; the high 32-bits are zeroed on write. In the end, we end up with 0xffffffff in {1}
        // on overflow; this happens be EPSILON.
        // Note that the CPU does not realize that the result of sbb x, x does not actually depend
        // on x. We must write the result to a register that we know to be ready. We have a
        // dependency on {1} anyway, so let's use it.
        "sbb {1:e}, {1:e}",
        inlateout(reg) x => res_wrapped,
        inlateout(reg) y => adjustment,
        options(pure, nomem, nostack),
    );
    assume(x != 0 || (res_wrapped == y && adjustment == 0));
    assume(y != 0 || (res_wrapped == x && adjustment == 0));
    // Add EPSILON == subtract ORDER.
    // Cannot overflow unless the assumption if x + y < 2**64 + ORDER is incorrect.
    res_wrapped + adjustment
}

#[inline(always)]
#[cfg(not(target_arch = "x86_64"))]
pub(crate) unsafe fn add_no_canonicalize_trashing_input(x: u64, y: u64) -> u64 {
    let (res_wrapped, carry) = x.overflowing_add(y);
    // Below cannot overflow unless the assumption if x + y < 2**64 + ORDER is incorrect.
    res_wrapped + EPSILON * (carry as u64)
}

#[inline]
pub(crate) fn split(x: u128) -> (u64, u64) {
    (x as u64, (x >> 64) as u64)
}

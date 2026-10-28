//! Binary64 enclosures. Nonfinite/undefined operations remain invalid intervals;
//! callers must turn them into numerical Undetermined, never a safety certificate.

#[derive(Debug, Clone, Copy)]
pub(crate) struct Iv {
    pub lo: f64,
    pub hi: f64,
}

impl Iv {
    pub const ZERO: Self = Self { lo: 0.0, hi: 0.0 };
    pub const ONE: Self = Self { lo: 1.0, hi: 1.0 };
    pub const INVALID: Self = Self {
        lo: f64::NEG_INFINITY,
        hi: f64::INFINITY,
    };

    pub fn new(lo: f64, hi: f64) -> Self {
        if lo.is_finite() && hi.is_finite() && lo <= hi {
            Self { lo, hi }
        } else {
            Self::INVALID
        }
    }

    pub fn point(x: f64) -> Self {
        Self::new(x, x)
    }
    pub fn valid(self) -> bool {
        self.lo.is_finite() && self.hi.is_finite() && self.lo <= self.hi
    }
    pub fn exact_zero(self) -> bool {
        self.lo == 0.0 && self.hi == 0.0
    }
    pub fn neg(self) -> Self {
        Self::new(-self.hi, -self.lo)
    }
    pub fn add(self, b: Self) -> Self {
        if !self.valid() || !b.valid() {
            return Self::INVALID;
        }
        if self.exact_zero() {
            return b;
        }
        if b.exact_zero() {
            return self;
        }
        Self::new((self.lo + b.lo).next_down(), (self.hi + b.hi).next_up())
    }
    pub fn sub(self, b: Self) -> Self {
        self.add(b.neg())
    }
    pub fn mul(self, b: Self) -> Self {
        if !self.valid() || !b.valid() {
            return Self::INVALID;
        }
        if self.exact_zero() || b.exact_zero() {
            return Self::ZERO;
        }
        if self.lo == 1.0 && self.hi == 1.0 {
            return b;
        }
        if b.lo == 1.0 && b.hi == 1.0 {
            return self;
        }
        let p = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        if p.iter().any(|x| !x.is_finite()) {
            return Self::INVALID;
        }
        Self::new(
            p.iter().copied().fold(f64::INFINITY, f64::min).next_down(),
            p.iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
                .next_up(),
        )
    }
    pub fn div(self, b: Self) -> Self {
        if !self.valid() || !b.valid() || b.lo <= 0.0 && b.hi >= 0.0 {
            return Self::INVALID;
        }
        if b.lo == 1.0 && b.hi == 1.0 {
            return self;
        }
        self.mul(Self::new((1.0 / b.hi).next_down(), (1.0 / b.lo).next_up()))
    }
    pub fn square(self) -> Self {
        if !self.valid() {
            return Self::INVALID;
        }
        if self.exact_zero() {
            return Self::ZERO;
        }
        let hi = (self.lo.abs().max(self.hi.abs()).powi(2)).next_up();
        let lo = if self.lo <= 0.0 && self.hi >= 0.0 {
            0.0
        } else {
            self.lo
                .abs()
                .min(self.hi.abs())
                .powi(2)
                .next_down()
                .max(0.0)
        };
        Self::new(lo, hi)
    }
    /// The caller knows the true operand is nonnegative (e.g. a squared norm).
    pub fn sqrt_nonnegative(self) -> Self {
        if !self.valid() || self.hi < 0.0 {
            return Self::INVALID;
        }
        Self::new(
            if self.lo <= 0.0 {
                0.0
            } else {
                self.lo.sqrt().next_down()
            },
            self.hi.sqrt().next_up(),
        )
    }
    pub fn pow(self, n: usize) -> Self {
        (0..n).fold(Self::ONE, |v, _| v.mul(self))
    }
    pub fn midpoint(self) -> f64 {
        self.lo.midpoint(self.hi)
    }
    pub fn abs_upper(self) -> f64 {
        self.lo.abs().max(self.hi.abs())
    }
    pub fn width(self) -> f64 {
        (self.hi - self.lo).next_up()
    }

    pub fn sin(self) -> Self {
        self.trig(false)
    }
    pub fn cos(self) -> Self {
        self.trig(true)
    }
    fn trig(self, cosine: bool) -> Self {
        if !self.valid() {
            return Self::INVALID;
        }
        if self.exact_zero() {
            return if cosine { Self::ONE } else { Self::ZERO };
        }
        // The frozen exact Fraction/Machin reference is the acceptance source
        // for this pi bracket. k may be any exactly represented integer;
        // periodicity does not require trusting the host's trig implementation.
        let pi = Self::new(
            std::f64::consts::PI.next_down(),
            std::f64::consts::PI.next_up(),
        );
        if self.width() >= 2.0 * pi.lo || self.midpoint().abs() > (1_u64 << 50) as f64 {
            return Self::new(-1.0, 1.0);
        }
        let k = (self.midpoint() / std::f64::consts::TAU).round();
        let reduced = self.sub(pi.mul(Self::point(2.0 * k)));
        if !reduced.valid() {
            return Self::INVALID;
        }
        if reduced.abs_upper() > 4.0 {
            return Self::new(-1.0, 1.0);
        }
        let m = reduced.midpoint();
        let radius = Self::point(m)
            .sub(Self::point(reduced.lo))
            .abs_upper()
            .max(Self::point(reduced.hi).sub(Self::point(m)).abs_upper())
            .next_up();
        let shift = usize::from(!cosine);
        let factorial = |n: usize| (1..=n).fold(Self::ONE, |v, i| v.mul(Self::point(i as f64)));
        let mut result = Self::ZERO;
        for k in 0..18 {
            let n = 2 * k + shift;
            let term = Self::point(m).pow(n).div(factorial(n));
            result = if k % 2 == 0 {
                result.add(term)
            } else {
                result.sub(term)
            };
        }
        let n = 35 + shift;
        let remainder = Self::point(m.abs()).pow(n).div(factorial(n)).hi;
        let error = Self::point(remainder).add(Self::point(radius)).hi;
        let lo = result.sub(Self::point(error)).lo.max(-1.0);
        let hi = result.add(Self::point(error)).hi.min(1.0);
        Self::new(lo, hi)
    }
}

impl std::ops::Add for Iv {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Iv::add(self, b)
    }
}
impl std::ops::Sub for Iv {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Iv::sub(self, b)
    }
}
impl std::ops::Mul for Iv {
    type Output = Self;
    fn mul(self, b: Self) -> Self {
        Iv::mul(self, b)
    }
}
impl std::ops::Div for Iv {
    type Output = Self;
    fn div(self, b: Self) -> Self {
        Iv::div(self, b)
    }
}
impl std::ops::Neg for Iv {
    type Output = Self;
    fn neg(self) -> Self {
        Iv::neg(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_encloses_cancellation_and_does_not_certify_overflow() {
        let v = Iv::point(1e16).add(Iv::ONE).sub(Iv::point(1e16));
        assert!(v.lo <= 1.0 && v.hi >= 1.0);
        assert!(!Iv::point(f64::MAX).mul(Iv::point(2.0)).valid());
        assert!(!Iv::ONE.div(Iv::new(-1.0, 1.0)).valid());
        assert!(Iv::ZERO.mul(Iv::point(3.0)).exact_zero());
    }

    #[test]
    fn trig_includes_known_values_and_complete_interval_ranges() {
        let pi = Iv::new(
            std::f64::consts::PI.next_down(),
            std::f64::consts::PI.next_up(),
        );
        let s = pi.div(Iv::point(6.0)).sin();
        let c = pi.div(Iv::point(3.0)).cos();
        assert!(s.lo < 0.5 && s.hi > 0.5);
        assert!(c.lo < 0.5 && c.hi > 0.5);
        assert!(s.width() < 1e-11 && c.width() < 1e-11);
        let broad = Iv::new(-4.0, 4.0).sin();
        assert_eq!((broad.lo, broad.hi), (-1.0, 1.0));
        assert_eq!((Iv::ZERO.cos().lo, Iv::ZERO.cos().hi), (1.0, 1.0));
    }
}

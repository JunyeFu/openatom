//! Exact cost arithmetic; one rounded IEEE output for a line, subtotal or unit cost.
//! Limbs grow with the selected price denominators, so accumulating unlike
//! fractions does not round intermediate charges or impose a new cost limit.
use sf_domain::{DecimalAmount, FiniteNumber, Quantity};
use std::cmp::Ordering;

#[derive(Clone, PartialEq, Eq)]
struct Wide(Vec<u64>);
impl Wide {
    fn normalized(mut limbs: Vec<u64>) -> Self {
        while limbs.last() == Some(&0) {
            limbs.pop();
        }
        Self(limbs)
    }
    fn integer(n: u64) -> Self {
        Self::normalized(vec![n])
    }
    fn mul(self, n: u64) -> Self {
        let mut result = Vec::with_capacity(self.0.len() + 1);
        let mut carry = 0u128;
        for limb in self.0 {
            let product = limb as u128 * n as u128 + carry;
            result.push(product as u64);
            carry = product >> 64;
        }
        result.push(carry as u64);
        Self::normalized(result)
    }
    fn product(&self, other: &Self) -> Self {
        let mut result = vec![0; self.0.len() + other.0.len()];
        for (i, &a) in self.0.iter().enumerate() {
            let mut carry = 0u128;
            for (j, &b) in other.0.iter().enumerate() {
                let value = a as u128 * b as u128 + result[i + j] as u128 + carry;
                result[i + j] = value as u64;
                carry = value >> 64;
            }
            if !other.0.is_empty() {
                result[i + other.0.len()] = carry as u64;
            }
        }
        Self::normalized(result)
    }
    fn add(&self, other: &Self) -> Self {
        let mut result = Vec::with_capacity(self.0.len().max(other.0.len()) + 1);
        let mut carry = 0u128;
        for i in 0..self.0.len().max(other.0.len()) {
            let value = self.0.get(i).copied().unwrap_or(0) as u128
                + other.0.get(i).copied().unwrap_or(0) as u128
                + carry;
            result.push(value as u64);
            carry = value >> 64;
        }
        result.push(carry as u64);
        Self::normalized(result)
    }
    fn bits(&self) -> i32 {
        self.0.last().map_or(0, |n| {
            (self.0.len() - 1) as i32 * 64 + 64 - n.leading_zeros() as i32
        })
    }
    fn shift(&self, n: u32) -> Self {
        let words = (n / 64) as usize;
        let bits = n % 64;
        let mut result = vec![0; self.0.len() + words + 1];
        for (i, &limb) in self.0.iter().enumerate() {
            result[i + words] |= limb << bits;
            if bits > 0 {
                result[i + words + 1] |= limb >> (64 - bits);
            }
        }
        Self::normalized(result)
    }
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }
    fn sub(&self, other: &Self) -> Self {
        let mut result = vec![0; self.0.len()];
        let mut borrow = false;
        for (i, value) in result.iter_mut().enumerate() {
            let (part, a) = self.0[i].overflowing_sub(other.0.get(i).copied().unwrap_or(0));
            let (part, b) = part.overflowing_sub(u64::from(borrow));
            *value = part;
            borrow = a || b;
        }
        Self::normalized(result)
    }
}

fn fraction(rate: &DecimalAmount, quantity: &Quantity, denominator: &Quantity) -> (Wide, Wide) {
    (
        Wide::integer(rate.coefficient.value())
            .mul(quantity.value.coefficient.value())
            .mul(10u64.pow(denominator.value.decimal_places.value() as u32)),
        Wide::integer(denominator.value.coefficient.value())
            .mul(10u64.pow(rate.decimal_places.value() as u32))
            .mul(10u64.pow(quantity.value.decimal_places.value() as u32)),
    )
}

fn rounded(numerator: Wide, divisor: Wide) -> FiniteNumber {
    if numerator.bits() == 0 {
        return FiniteNumber::try_from(0.0).unwrap();
    }
    // Legal 53-bit decimal operands, addressable selected-cost lists and
    // positive accepted quantities keep these output ratios normal and finite.
    let mut exponent = numerator.bits() - divisor.bits();
    let below = if exponent >= 0 {
        numerator.cmp(&divisor.shift(exponent as u32)) == Ordering::Less
    } else {
        numerator.shift((-exponent) as u32).cmp(&divisor) == Ordering::Less
    };
    if below {
        exponent -= 1;
    }
    let shift = 52 - exponent;
    let (mut remainder, divisor) = if shift >= 0 {
        (numerator.shift(shift as u32), divisor)
    } else {
        (numerator, divisor.shift((-shift) as u32))
    };
    let mut mantissa = 0u64;
    for bit in (0..=52).rev() {
        let candidate = divisor.shift(bit);
        if remainder.cmp(&candidate) != Ordering::Less {
            remainder = remainder.sub(&candidate);
            mantissa |= 1u64 << bit;
        }
    }
    let rounding = remainder.shift(1).cmp(&divisor);
    if rounding == Ordering::Greater || rounding == Ordering::Equal && mantissa & 1 == 1 {
        mantissa += 1;
    }
    if mantissa == 1u64 << 53 {
        mantissa >>= 1;
        exponent += 1;
    }
    let bits = ((exponent + 1023) as u64) << 52 | (mantissa - (1u64 << 52));
    FiniteNumber::try_from(f64::from_bits(bits)).unwrap()
}

pub(crate) fn charge(
    rate: &DecimalAmount,
    quantity: &Quantity,
    denominator: &Quantity,
) -> Option<FiniteNumber> {
    if denominator.value.coefficient.value() == 0 {
        return None;
    }
    let (numerator, divisor) = fraction(rate, quantity, denominator);
    Some(rounded(numerator, divisor))
}

pub(crate) fn sum_charges<'a>(
    terms: impl Iterator<Item = (&'a DecimalAmount, &'a Quantity, &'a Quantity)>,
    accepted: Option<&Quantity>,
) -> (FiniteNumber, Option<FiniteNumber>) {
    let (mut numerator, mut divisor) = (Wide::integer(0), Wide::integer(1));
    // Caller supplies only already-established known lines with positive price
    // denominators; no rounded line amount is fed back into this arithmetic.
    for (rate, quantity, denominator) in terms {
        let (n, d) = fraction(rate, quantity, denominator);
        numerator = numerator.product(&d).add(&n.product(&divisor));
        divisor = divisor.product(&d);
    }
    let unit_cost = accepted
        .filter(|q| q.value.coefficient.value() > 0)
        .map(|q| {
            rounded(
                numerator
                    .clone()
                    .mul(10u64.pow(q.value.decimal_places.value() as u32)),
                divisor.clone().mul(q.value.coefficient.value()),
            )
        });
    (rounded(numerator, divisor), unit_cost)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decimal_cost_matches_independent_rational_rounding_through_contract_extremes() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/evaluation/decimal-cost-expectations.json"
        ))
        .unwrap();
        for c in fixture["cases"].as_array().unwrap() {
            let rate = serde_json::from_value(c["rate"].clone()).unwrap();
            let quantity = serde_json::from_value(c["quantity"].clone()).unwrap();
            let denominator = serde_json::from_value(c["denominator"].clone()).unwrap();
            assert_eq!(
                charge(&rate, &quantity, &denominator)
                    .unwrap()
                    .value()
                    .to_bits(),
                c["expected_f64_bits"].as_u64().unwrap(),
                "{c}"
            );
        }
    }
}

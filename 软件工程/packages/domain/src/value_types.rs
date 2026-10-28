//! Promoted unchanged from the frozen foundation contract.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const MAX_SAFE_UINT: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, TS)]
#[serde(try_from = "u64", into = "u64")]
#[schemars(!try_from, !into)]
#[ts(type = "number")]
pub struct SafeUint(#[schemars(range(min = 0, max = 9007199254740991u64))] u64);
impl TryFrom<u64> for SafeUint {
    type Error = String;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value <= MAX_SAFE_UINT {
            Ok(Self(value))
        } else {
            Err("outside JavaScript safe integer range".into())
        }
    }
}
impl From<SafeUint> for u64 {
    fn from(value: SafeUint) -> Self {
        value.0
    }
}
impl SafeUint {
    pub fn value(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(try_from = "f64", into = "f64")]
#[ts(type = "number")]
pub struct FiniteNumber(f64);
impl TryFrom<f64> for FiniteNumber {
    type Error = String;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err("finite value required".into())
        }
    }
}
impl From<FiniteNumber> for f64 {
    fn from(value: FiniteNumber) -> Self {
        value.0
    }
}
impl FiniteNumber {
    pub fn value(&self) -> f64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(try_from = "String", into = "String")]
#[schemars(!try_from, !into)]
pub struct RelativeFilePath(
    #[schemars(pattern(
        r"^(?!\.{1,2}(?:/|$))[^/\\:\u0000]+(?:/(?!\.{1,2}(?:/|$))[^/\\:\u0000]+)*$"
    ))]
    String,
);
impl TryFrom<String> for RelativeFilePath {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty()
            || value.contains(['\\', ':', '\0'])
            || value
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            Err("package-relative forward-slash file path required".into())
        } else {
            Ok(Self(value))
        }
    }
}
impl From<RelativeFilePath> for String {
    fn from(value: RelativeFilePath) -> Self {
        value.0
    }
}
impl RelativeFilePath {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(transparent)]
pub struct Revision(pub SafeUint);
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(transparent)]
pub struct SimTimeMs(pub SafeUint);
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(transparent)]
pub struct DurationMs(pub SafeUint);

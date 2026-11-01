//! Promoted unchanged from the frozen foundation contract.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

macro_rules! string_id {
    ($name:ident) => {
        #[derive(
            Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, TS,
        )]
        #[serde(try_from = "String", into = "String")]
        #[schemars(!try_from, !into)]
        pub struct $name(#[schemars(length(min = 1), pattern(r"^[^\u0000]+$"))] String);
        impl TryFrom<String> for $name {
            type Error = String;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.is_empty() || value.contains('\0') {
                    Err("nonempty ID without NUL required".into())
                } else {
                    Ok(Self(value))
                }
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

string_id!(ProjectId);
string_id!(ObjectId);
string_id!(ScenarioId);
string_id!(RunId);
string_id!(TaskId);
string_id!(ModuleId);
string_id!(AssetId);
string_id!(ArtifactId);
string_id!(RoleId);
string_id!(PortId);
string_id!(PhaseId);
string_id!(UnitId);
string_id!(ExecutionSessionId);
string_id!(ExecutionEpochId);
string_id!(DispatchCommandId);
string_id!(CloudProjectId);
string_id!(CloudDatasetId);
string_id!(DeviceBootId);
string_id!(ObservationProbeId);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(try_from = "String", into = "String")]
#[schemars(!try_from, !into)]
pub struct VersionId(
    #[schemars(pattern(r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*))*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$"))]
     String,
);
impl TryFrom<String> for VersionId {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        fn identifiers(s: &str, prerelease: bool) -> bool {
            s.split('.').all(|p| {
                !p.is_empty()
                    && p.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
                    && (!prerelease
                        || !p.bytes().all(|c| c.is_ascii_digit())
                        || p.len() == 1
                        || !p.starts_with('0'))
            })
        }
        let build: Vec<&str> = value.split('+').collect();
        if build.len() > 2 || (build.len() == 2 && !identifiers(build[1], false)) {
            return Err("invalid SemVer build identifiers".into());
        }
        let (core, prerelease) = match build[0].split_once('-') {
            Some((a, b)) => (a, Some(b)),
            None => (build[0], None),
        };
        let numbers: Vec<&str> = core.split('.').collect();
        if numbers.len() != 3
            || !numbers.iter().all(|n| {
                !n.is_empty()
                    && n.bytes().all(|c| c.is_ascii_digit())
                    && (n.len() == 1 || !n.starts_with('0'))
            })
            || prerelease.is_some_and(|s| !identifiers(s, true))
        {
            Err("exact SemVer required".into())
        } else {
            Ok(Self(value))
        }
    }
}
impl From<VersionId> for String {
    fn from(value: VersionId) -> Self {
        value.0
    }
}

string_id!(SubjectRevisionId);

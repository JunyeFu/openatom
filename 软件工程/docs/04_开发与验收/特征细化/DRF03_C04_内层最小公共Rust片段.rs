// Selected design delta, for the unique canonical writer (review window 1).
// Existing HallDesign field:
// #[ts(optional = nullable)]
// pub height_m: Option<FiniteNumber>,
// Existing TemplateSubset gains:
// pub locations: Vec<StockLocation>,
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case", deny_unknown_fields)]
pub enum MaterialEndpoint {
    Port(PortRef),
    Location(ObjectId),
}
// Existing MaterialConnection changes only these carrier types:
// pub source: MaterialEndpoint,
// pub destination: MaterialEndpoint,
// destination_store remains; enforce endpoint/location-binding consistency.
// Unknown hall height is preserved draft data, never a site-check Pass.

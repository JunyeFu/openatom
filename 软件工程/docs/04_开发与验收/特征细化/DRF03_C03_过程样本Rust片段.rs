// Selected design fragment for window 1; not a compiled canonical product type.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PackagingPanelPose {
    pub panel_id: ObjectId,
    pub pose: Pose,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PackagingProcessSample {
    pub step_id: ObjectId,
    pub progress: FiniteNumber,
    pub panel_poses: Vec<PackagingPanelPose>,
}
// In the existing PackagingResult, replace only the field's element carrier:
// pub process_poses: Vec<PackagingProcessSample>,
// Resolve step/panel IDs using parent PackagingResult.job_ref; validate u and Pose
// at the actual public boundary. Samples do not certify continuous collision safety.

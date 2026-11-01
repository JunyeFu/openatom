//! Private, actual output facts; no new public RunResult/event format.
use sf_domain::*;

/// Tuple order: actual time, Job id, original execution ref, resolved port id,
/// exact physical item type, frozen quantity, transferred (false = produced),
/// actual physical item id. Existing DTOs keep this internally serializable
/// without adding a simulation serialization dependency or public wire type.
pub type PortOutputRecord = (
    SimTimeMs,
    ObjectId,
    JobExecutionRef,
    PortId,
    SubjectRevisionRef,
    Quantity,
    bool,
    ObjectId,
);

//! Run the unchanged owned input unit modules without compiling unrelated fixture-dependent modules.
pub use sf_domain::*;
#[path = "../../../packages/domain/src/material_inputs.rs"]
mod material_inputs;
pub use material_inputs::check_material_inputs;
#[path = "../../../packages/domain/src/business_inputs.rs"]
mod business_inputs;
pub use business_inputs::{check_business_inputs, check_business_inputs_with_definitions};

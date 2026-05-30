//! Physical plan: concrete executable operators lowered from the logical plan.

use crate::logical_plan::LogicalPlan;
use chronos_common::Result;

#[derive(Debug, Default)]
pub struct PhysicalPlan {
    pub stages: Vec<String>,
}

pub fn lower(logical: LogicalPlan) -> Result<PhysicalPlan> {
    Ok(PhysicalPlan {
        stages: logical.ops.iter().map(|op| format!("{op:?}")).collect(),
    })
}

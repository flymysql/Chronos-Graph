//! Query layer: from text to executed retrieval.
//!
//! Pipeline: `lexer -> parser -> ast -> logical_plan -> optimizer ->
//! physical_plan -> executor`. The language is an openCypher subset extended
//! with temporal (`AS OF ... TIME`) and semantic (`SIMILAR`, `TRAVERSE
//! SEMANTIC`, `CONTEXT`) operators.

pub mod ast;
pub mod executor;
pub mod lexer;
pub mod logical_plan;
pub mod optimizer;
pub mod parser;
pub mod physical_plan;

pub use ast::Query;
pub use executor::context::ContextBlock;

/// A query compiled down to an executable physical plan.
pub struct CompiledQuery {
    pub plan: physical_plan::PhysicalPlan,
}

/// Compile query text into an executable plan. Each stage is currently a stub.
pub fn compile(src: &str) -> chronos_common::Result<CompiledQuery> {
    let tokens = lexer::lex(src)?;
    let ast = parser::parse(tokens)?;
    let logical = logical_plan::build(&ast)?;
    let optimized = optimizer::optimize(logical)?;
    let plan = physical_plan::lower(optimized)?;
    Ok(CompiledQuery { plan })
}

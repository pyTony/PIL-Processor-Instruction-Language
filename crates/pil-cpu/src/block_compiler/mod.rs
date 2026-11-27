//! Program-independent block planning and run-time blocks (see the pil
//! crate's src/block_compiler/mod.rs and docs/development/BLOCK_COMPILER.md).
//!
//! - `sites`: `Site`, `InstructionProfile` and `BlockPlan`.
//! - `codegen`: Rust rendering of a PIL program's body and of a planned block.
//! - `guards`: the entry checks a compiled block needs.
//! - `engine`: run-time blocks over the precompiled instruction forms.
#![allow(dead_code)]
pub mod sites;
pub mod codegen;
pub mod guards;
pub mod engine;
pub mod pil_rust;
mod compare_branch;

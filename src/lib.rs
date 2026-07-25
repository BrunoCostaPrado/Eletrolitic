// ponytail: pedantic allows — match_same_arms/too_many_lines intentional in parser/codegen,
// casts intentional in source map VLQ encoding and number display, doc nits not worth churn.
#![allow(
  clippy::match_same_arms,
  clippy::too_many_lines,
  clippy::cast_possible_wrap,
  clippy::cast_possible_truncation,
  clippy::cast_sign_loss,
  clippy::missing_errors_doc,
  clippy::missing_panics_doc,
  clippy::unnecessary_box_returns,
  clippy::needless_pass_by_value,
  clippy::unused_self,
  clippy::float_cmp
)]

pub mod ast;
pub mod codegen;
pub mod compiler;
pub mod config;
pub mod decl_emit;
pub mod diagnostic;
pub mod lexer;
pub mod parser;
pub mod source_map;
pub mod token;
pub mod tree_shake;
pub mod type_checker;

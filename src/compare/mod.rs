mod diff;
pub mod display;

pub use diff::{compare_results, CompareResult, Regression};
pub use display::print_comparison;

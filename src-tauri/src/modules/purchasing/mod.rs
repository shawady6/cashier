//! المشتريات: الموردون، فواتير الشراء (مسودة ← استلام)، الدفعات، المرتجعات.

pub mod model;
pub mod purchases;
pub mod returns;
pub mod suppliers;

#[cfg(test)]
mod tests;

pub use model::*;

//! المنتجات: الكتالوج، الباركود، المتغيرات، الوحدات، البحث، الصور.

pub mod images;
pub mod model;
pub mod query;
pub mod repo;
mod validate;

#[cfg(test)]
mod tests;

pub use model::*;
pub use query::{create_unit, find_by_barcode, get, list, list_units};
pub use repo::{save, set_archived};

//! Application services: orchestrate system readers into UI-ready data.
//!
//! The UI never calls `system/` directly (ARCHITECTURE.md §1).

pub mod app_data;

pub use app_data::AppData;

//! Application services: orchestrate system readers into UI-ready data.
//!
//! The UI never calls `system/` directly (ARCHITECTURE.md §1).

pub mod app_data;
pub mod group_service;
pub mod user_service;

pub use app_data::AppData;

//! System integration layer: command execution, account reading,
//! privilege checks, audit logging. No UI code here.

pub mod account_reader;
pub mod acl;
pub mod audit;
pub mod command;
pub mod filesystem;
pub mod permission;
pub mod privilege;
pub mod updater;

//! System integration layer: command execution, account reading,
//! privilege checks, audit logging. No UI code here.

pub mod account_reader;
pub mod audit;
pub mod command;
pub mod privilege;

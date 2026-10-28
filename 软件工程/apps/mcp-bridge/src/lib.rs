//! MCP consumes the one desktop Host through authenticated local IPC.
pub mod catalog;
#[cfg(windows)]
pub mod client;
#[path = "../../../integrations/agent/runtime/contract.rs"]
pub mod contract;
pub mod decode;
#[path = "../../../integrations/agent/runtime/framing.rs"]
pub mod framing;
#[cfg(windows)]
#[path = "../../../integrations/agent/runtime/managed_io.rs"]
pub mod managed_io;
#[cfg(windows)]
#[path = "../../../integrations/agent/runtime/pipe_os.rs"]
pub mod pipe_os;
pub mod run_identity;
pub mod execution_identity;

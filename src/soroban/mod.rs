//! Soroban JSON-RPC client used by the donation submission flow.
//!
//! Exposes both the concrete [`SorobanRpcClient`] and the
//! [`SorobanRpc`](crate::soroban::rpc_client::SorobanRpc) trait that the HTTP
//! handlers depend on for testability.

pub mod rpc_client;

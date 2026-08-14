// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Core modules for the `smth` CLI.

pub mod cmd;
pub mod config;

mod app;
mod model;
mod path;
mod terminal;

pub use crate::app::App;
pub use crate::app::Context;
pub use crate::model::Model;
pub use crate::model::agent::AgentState;
pub use crate::model::agent::STATE_OPTION as AGENT_STATE_OPTION;
pub use crate::model::serialize::SerializedSession;
pub use crate::model::session::Session;

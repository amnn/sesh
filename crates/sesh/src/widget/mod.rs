// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Small reusable widgets for the terminal UI.

mod block;
mod loading;

pub(crate) use crate::widget::block::Block;
pub(crate) use crate::widget::loading::Loading;
pub(crate) use crate::widget::loading::LoadingState;

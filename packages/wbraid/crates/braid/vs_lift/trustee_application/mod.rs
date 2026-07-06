// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! Trustee actor implementation.

/// Ascent-based protocol logic used by the trustee application actor.
pub(crate) mod ascent_logic;
/// State handlers and dispatch wrappers for trustee application states.
pub(crate) mod handlers;
/// Top-level trustee application actor and public API.
pub mod top_level_actor;

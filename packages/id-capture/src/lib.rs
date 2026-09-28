// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Frame analysis for the guided identity capture, compiled to WebAssembly for the voter's
//! browser.
//!
//! [`DocumentAnalyzer`] checks that an identity card fills the on-screen guide, is well exposed,
//! free of glare, sharp and still. [`FaceAnalyzer`] runs the `YuNet` face detector and checks
//! that a single, frontal, well lit face fills the oval and holds still. The checks only guide
//! the voter; the identity provider performs the actual verification.

mod document;
mod error;
mod face;
mod frame;
mod geometry;
mod stability;
#[cfg(test)]
mod synthetic;
mod wasm;
mod yunet;

pub use document::{DocumentAnalyzer, DocumentFrame, DocumentStatus};
pub use error::CaptureError;
pub use face::{FaceAnalyzer, FaceFrame, FaceStatus, Oval};
pub use geometry::{Point, Rect};
pub use wasm::{JsDocumentAnalyzer, JsFaceAnalyzer};

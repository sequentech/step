// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Errors reported by the analyzers.

use thiserror::Error;

/// Why a frame or a model could not be analysed.
#[derive(Debug, Error)]
pub enum CaptureError {
    /// The frame has no pixels.
    #[error("the frame is empty")]
    EmptyFrame,
    /// The pixel buffer does not hold `width * height` RGBA pixels.
    #[error("the frame buffer has {actual} bytes, expected {expected}")]
    FrameSizeMismatch {
        /// Bytes needed for the declared dimensions.
        expected: usize,
        /// Bytes received.
        actual: usize,
    },
    /// The declared dimensions overflow the address space.
    #[error("the frame dimensions are too large")]
    FrameTooLarge,
    /// A guide or oval parameter is not a finite, positive geometry inside the frame.
    #[error("invalid {0}")]
    InvalidGeometry(&'static str),
    /// The face detection model could not be loaded or run.
    #[error("face detection model error: {0}")]
    Model(String),
}

impl From<tract_onnx::prelude::TractError> for CaptureError {
    fn from(error: tract_onnx::prelude::TractError) -> Self {
        Self::Model(format!("{error:#}"))
    }
}

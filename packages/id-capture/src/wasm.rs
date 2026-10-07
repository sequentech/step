// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! JavaScript bindings.

use serde::Serialize;
use wasm_bindgen::prelude::{wasm_bindgen, JsError, JsValue};

use crate::document::{check_still, DocumentAnalyzer};
use crate::face::{FaceAnalyzer, Oval};
use crate::geometry::Rect;

#[wasm_bindgen(typescript_custom_section)]
const TYPESCRIPT_TYPES: &str = r#"
export type Point = [number, number];
export type DocumentStatus =
  | "NO_DOCUMENT" | "TOO_FAR" | "TOO_CLOSE" | "NOT_ALIGNED" | "TILTED"
  | "TOO_DARK" | "TOO_BRIGHT" | "GLARE" | "BLURRY" | "HOLD_STILL" | "READY";
export interface DocumentFrame {
  status: DocumentStatus;
  corners: Point[] | null;
  fill: number;
  sharpness: number;
  glare: number;
  brightness: number;
  stability: number;
}
export interface StillCheck {
  status: DocumentStatus;
  corners: Point[] | null;
  cardWidth: number;
  blur: number;
}
export type FaceStatus =
  | "NO_FACE" | "MULTIPLE_FACES" | "TOO_FAR" | "TOO_CLOSE" | "OFF_CENTER"
  | "TURN_TO_CAMERA" | "TOO_DARK" | "TOO_BRIGHT" | "BLURRY" | "HOLD_STILL" | "READY";
export interface FaceFrame {
  status: FaceStatus;
  box: {x: number; y: number; width: number; height: number} | null;
  landmarks: Point[];
  yaw: number;
  roll: number;
  brightness: number;
  sharpness: number;
  stability: number;
}
"#;

/// Converts an analysis result into a plain JavaScript object, with `null` for absent values.
fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_missing_as_null(true);
    value
        .serialize(&serializer)
        .map_err(|error| JsError::new(&error.to_string()))
}

/// Document analyzer exposed to JavaScript as `DocumentAnalyzer`.
#[wasm_bindgen(js_name = DocumentAnalyzer)]
pub struct JsDocumentAnalyzer {
    /// Wrapped analyzer.
    inner: DocumentAnalyzer,
}

#[wasm_bindgen(js_class = DocumentAnalyzer)]
impl JsDocumentAnalyzer {
    /// Analyzer with no history.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: DocumentAnalyzer::new(),
        }
    }

    /// Analyses a downscaled RGBA frame against the guide rectangle.
    ///
    /// # Errors
    ///
    /// Throws when the buffer does not hold `width * height` RGBA pixels or the guide is invalid.
    #[wasm_bindgen(unchecked_return_type = "DocumentFrame")]
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the JavaScript API, which passes the frame and geometry as numbers"
    )]
    pub fn analyze(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Uint8Array | Uint8ClampedArray")] rgba: &[u8],
        width: u32,
        height: u32,
        #[wasm_bindgen(js_name = guideX)] guide_x: f32,
        #[wasm_bindgen(js_name = guideY)] guide_y: f32,
        #[wasm_bindgen(js_name = guideWidth)] guide_width: f32,
        #[wasm_bindgen(js_name = guideHeight)] guide_height: f32,
    ) -> Result<JsValue, JsError> {
        let guide = Rect::new(guide_x, guide_y, guide_width, guide_height, "guide")?;
        to_js(&self.inner.analyze(rgba, width, height, guide)?)
    }

    /// Checks the full resolution RGBA still that is uploaded, against the guide rectangle in its
    /// pixel coordinates. The stillness history is left alone.
    ///
    /// # Errors
    ///
    /// Throws when the buffer does not hold `width * height` RGBA pixels or the guide is invalid.
    #[wasm_bindgen(js_name = checkStill, unchecked_return_type = "StillCheck")]
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the JavaScript API, which passes the frame and geometry as numbers"
    )]
    pub fn check_still(
        &self,
        #[wasm_bindgen(unchecked_param_type = "Uint8Array | Uint8ClampedArray")] rgba: &[u8],
        width: u32,
        height: u32,
        #[wasm_bindgen(js_name = guideX)] guide_x: f32,
        #[wasm_bindgen(js_name = guideY)] guide_y: f32,
        #[wasm_bindgen(js_name = guideWidth)] guide_width: f32,
        #[wasm_bindgen(js_name = guideHeight)] guide_height: f32,
    ) -> Result<JsValue, JsError> {
        let guide = Rect::new(guide_x, guide_y, guide_width, guide_height, "guide")?;
        to_js(&check_still(rgba, width, height, guide)?)
    }

    /// Forgets the stillness history.
    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

impl Default for JsDocumentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Face analyzer exposed to JavaScript as `FaceAnalyzer`.
#[wasm_bindgen(js_name = FaceAnalyzer)]
pub struct JsFaceAnalyzer {
    /// Wrapped analyzer.
    inner: FaceAnalyzer,
}

#[wasm_bindgen(js_class = FaceAnalyzer)]
impl JsFaceAnalyzer {
    /// Loads the `YuNet` ONNX model.
    ///
    /// # Errors
    ///
    /// Throws when the bytes are not a `YuNet` model the runtime can execute.
    #[wasm_bindgen(constructor)]
    pub fn new(model: &[u8]) -> Result<Self, JsError> {
        Ok(Self {
            inner: FaceAnalyzer::new(model)?,
        })
    }

    /// Analyses an RGBA frame against the oval the face must fill.
    ///
    /// # Errors
    ///
    /// Throws when the buffer does not hold `width * height` RGBA pixels, the oval is invalid or
    /// inference fails.
    #[wasm_bindgen(unchecked_return_type = "FaceFrame")]
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the JavaScript API, which passes the frame and geometry as numbers"
    )]
    pub fn analyze(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Uint8Array | Uint8ClampedArray")] rgba: &[u8],
        width: u32,
        height: u32,
        #[wasm_bindgen(js_name = ovalCenterX)] oval_center_x: f32,
        #[wasm_bindgen(js_name = ovalCenterY)] oval_center_y: f32,
        #[wasm_bindgen(js_name = ovalRadiusX)] oval_radius_x: f32,
        #[wasm_bindgen(js_name = ovalRadiusY)] oval_radius_y: f32,
    ) -> Result<JsValue, JsError> {
        let oval = Oval::new(oval_center_x, oval_center_y, oval_radius_x, oval_radius_y)?;
        to_js(&self.inner.analyze(rgba, width, height, oval)?)
    }

    /// Forgets the stillness history.
    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# id-capture

Frame analysis for the guided identity capture (Scanovate `embedded` mode),
compiled to WebAssembly and run on every camera frame in the voter's browser.
The checks only guide the voter and decide when to take the photo; the server
never trusts them: the Scanovate services hosted on premise perform the actual
verification.

## What it checks

`DocumentAnalyzer` works on a downscaled RGBA frame (about 480 px on the long
side) and the on-screen guide rectangle:

- finds the card edges: for each guide side, a restricted Hough transform over
  near-axis-aligned lines (up to ±12°) in a band around it, keeping long
  edges of consistent polarity, and intersects the four lines into corners;
- `NO_DOCUMENT`, `TOO_FAR` / `TOO_CLOSE` (card area below half or above 1.3
  times the guide area), `NOT_ALIGNED` (missing side, a corner within 2% of
  the frame border, or the card centre more than 15% of the guide diagonal
  from the guide centre), `TILTED` (opposite sides differing by more than 15%:
  the card is seen at an angle and its text distorted), `TOO_DARK` /
  `TOO_BRIGHT` (mean luma in the guide), `GLARE`
  (largest connected blob of near-white, low-saturation pixels on the card),
  `BLURRY` (variance of the Laplacian over the luma variance on the card),
  then `HOLD_STILL` until the corners stay put for 9 frames and `READY`.

The guide only shows where to hold the document. The whole camera frame is
uploaded and the OCR service finds the document in it, so the geometry checks
are loose: they keep the whole document in the frame, large enough and flat to
the camera, while the glare and sharpness checks protect what the OCR reads.

Those checks run on small frames, where blur and a low camera resolution go
unnoticed, so `checkStill` checks the full resolution still before it is
uploaded. It finds the card on the still downscaled like the live frames and
runs the same checks, since the card may have moved since the last of them.
Then the card must be at least 480 pixels wide in the still (`TOO_FAR`), or
85% of the guide width when the camera can't give it that many, and its blur
effect ([Crété-Roffet et al., 2007][blur-effect]: the share of the edge
contrast left by a further box blur) at a 480 pixel card width at most 0.45
(`BLURRY`). Measured at a fixed card width, blur is judged against the size of
the text whatever the camera resolution, and the blur effect, unlike the
Laplacian variance, doesn't depend on how much detail the card has. A failed
still is not uploaded: the page shows its problem and takes another.

`FaceAnalyzer` runs the OpenCV Zoo [YuNet][yunet] face detector
(`face_detection_yunet_2023mar`, MIT) with [tract][tract] on the frame scaled to
224 px on the long side, and checks against the on-screen oval:
`NO_FACE`, `MULTIPLE_FACES`, `TOO_FAR` / `TOO_CLOSE` (face box height over oval
height), `OFF_CENTER`, `TURN_TO_CAMERA` (yaw and roll estimated from the five
landmarks), `TOO_DARK` / `TOO_BRIGHT`, `BLURRY`, then `HOLD_STILL` until the
face box stays put for 8 frames and `READY`. Frames of about 640x480 are
expected; the sharpness thresholds depend on the face size in pixels.

The statuses are listed in priority order: the first failing check wins.
All thresholds are named constants at the top of `src/document.rs` and
`src/face.rs`.

## JavaScript API

```ts
import init, {DocumentAnalyzer, FaceAnalyzer} from "./capture-wasm/index.js"

await init({module_or_path: wasmUrl})
const doc = new DocumentAnalyzer()
doc.analyze(rgba, width, height, guideX, guideY, guideWidth, guideHeight) // DocumentFrame
doc.checkStill(rgba, width, height, guideX, guideY, guideWidth, guideHeight) // StillCheck
const face = new FaceAnalyzer(modelBytes) // throws on an invalid model
face.analyze(rgba, width, height, ovalCenterX, ovalCenterY, ovalRadiusX, ovalRadiusY) // FaceFrame
```

`reset()` forgets the stillness history and `free()` releases the analyzer.

## Rebuilding

From the repository root, inside `devenv shell`:

```bash
packages/id-capture/build-wasm.sh
```

It builds the crate for `wasm32-unknown-unknown` (with `simd128`, which
browsers need anyway for the SIMD code tract ships), runs `wasm-bindgen`
0.2.128 and, when available, `wasm-opt` (`nix shell nixpkgs#binaryen`), and
writes `index.js`, `index.d.ts`, `index_bg.wasm` and `face_detection_yunet.onnx`
to `packages/keycloak-ui/src/login/scanovate/capture-wasm/`.

Tests run natively:

```bash
cd packages && CARGO_TARGET_DIR=$PWD/id-capture/rust-local-target cargo test -p id-capture
```

[blur-effect]: https://hal.science/hal-00232709
[yunet]: https://github.com/opencv/opencv_zoo/tree/main/models/face_detection_yunet
[tract]: https://github.com/sonos/tract

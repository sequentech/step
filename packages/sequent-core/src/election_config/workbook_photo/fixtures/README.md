# Synthetic SQL photo fixtures

These files contain four flat color blocks (RGB 223/49/40, 45/175/91,
44/71/226, and 220/186/31), not photographs or election data. The source
rasters were generated with Python's standard library, then encoded with
FFmpeg's SVT-AV1 and MJPEG codecs and libwebp's `cwebp`/`img2webp` tools.

- `blocks.avif`: 64 × 64 lossy 4:2:0, 8-bit; `blocks10.avif`: 10-bit.
- `max-side.avif`: 1024 × 768; `blocks-wide.avif` and `blocks-wide.webp`:
  1040 × 760 landscape stills.
- `blocks.webp` and `blocks-lossless.webp`: 64 × 64 lossy and lossless WebP.
- `too-wide.avif` and `too-wide.webp`: valid 1282 × 64 stills, beyond the side limit.
- `over-area.webp`: 1200 × 1000; both sides fit, but the pixel total exceeds the cap.
- `blocks-animated.avif` and `blocks-animated.webp`: actual two-frame files.
- `blocks.jpeg`: a 64 × 64 JPEG for the mixed-source and Excel regressions.
- `blocks-hdr.avif`: synthetic 10-bit PQ/BT.2020 signaling; the SQL importer refuses it.
- `blocks-alpha.webp`: a blue block with half opaque and half translucent pixels
  using compressed WebP alpha; `blocks-entropy.webp` is a 128 × 128 patterned
  image that exercises the multi-group lossless decoder.

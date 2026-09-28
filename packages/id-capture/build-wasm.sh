#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Builds the id-capture WebAssembly package into the Keycloak login theme.
# Run inside the devenv shell, which provides cargo and wasm-bindgen.

set -euo pipefail

WASM_BINDGEN_VERSION="0.2.128"
MODEL="face_detection_yunet_2023mar.onnx"
PUBLISHED_MODEL="face_detection_yunet.onnx"

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
workspace_dir="$(dirname "${crate_dir}")"
target_dir="${crate_dir}/rust-local-target"
out_dir="${workspace_dir}/keycloak-ui/src/login/scanovate/capture-wasm"

installed="$(wasm-bindgen --version | awk '{print $2}')"
if [[ "${installed}" != "${WASM_BINDGEN_VERSION}" ]]; then
  echo "wasm-bindgen ${WASM_BINDGEN_VERSION} is required, found ${installed}" >&2
  exit 1
fi

# simd128 is required anyway: rustfft, pulled in by tract, always ships SIMD code for wasm32.
(
  cd "${workspace_dir}"
  RUSTFLAGS="-C target-feature=+simd128" CARGO_TARGET_DIR="${target_dir}" cargo build \
    --release \
    --target wasm32-unknown-unknown \
    --package id-capture \
    --config 'profile.release.opt-level="s"' \
    --config 'profile.release.lto=true' \
    --config 'profile.release.codegen-units=1' \
    --config 'profile.release.strip=true'
)

mkdir -p "${out_dir}"
rm -f "${out_dir}"/index.js "${out_dir}"/index.d.ts "${out_dir}"/index_bg.wasm "${out_dir}"/index_bg.wasm.d.ts
wasm-bindgen \
  --target web \
  --out-name index \
  --typescript \
  --out-dir "${out_dir}" \
  "${target_dir}/wasm32-unknown-unknown/release/id_capture.wasm"

if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -O3 \
    --enable-simd \
    --enable-bulk-memory \
    --enable-nontrapping-float-to-int \
    --enable-sign-ext \
    --enable-reference-types \
    --enable-multivalue \
    --enable-mutable-globals \
    "${out_dir}/index_bg.wasm" \
    -o "${out_dir}/index_bg.wasm"
else
  echo "wasm-opt not found, skipping the size optimisation (nix shell nixpkgs#binaryen)" >&2
fi

cp "${crate_dir}/models/${MODEL}" "${out_dir}/${PUBLISHED_MODEL}"
echo "Wrote ${out_dir}"

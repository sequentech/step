// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! Splitting byte arrays into fixed-size chunks, one per group element.
//!
//! Encoding data into group elements is a two-step affair: chunk the input, then
//! embed each chunk in a point. The embedding is curve-specific, but the
//! chunking is not, so it lives here and is shared by the backends.
//!
//! `CHUNK` is a parameter because it is a property of the curve — how many bytes
//! one element can carry. Both current backends happen to carry 30.

/// Splits a `BYTES`-long array into `ELEMENTS` chunks of `CHUNK` bytes, and back.
///
/// The relationship `ELEMENTS == BYTES.div_ceil(CHUNK)` is asserted at compile
/// time, so a mismatched instantiation is a build error rather than a panic.
pub(crate) struct Codec<const CHUNK: usize, const BYTES: usize, const ELEMENTS: usize> {}

impl<const CHUNK: usize, const BYTES: usize, const ELEMENTS: usize> Codec<CHUNK, BYTES, ELEMENTS> {
    /// Compile-time validation of the chunking parameters.
    const CHECK: () = {
        assert!(CHUNK > 0);
        assert!(ELEMENTS > 0);
        assert!(BYTES > 0);
        assert!(ELEMENTS == BYTES.div_ceil(CHUNK));
        let max = ELEMENTS.checked_mul(CHUNK);
        assert!(max.is_some());
    };

    /// Split an array into chunks, filling each sequentially up to `CHUNK`.
    ///
    /// A trailing partial chunk is zero-padded; [`join`](Self::join) discards
    /// that padding, so the round trip is exact.
    pub(crate) fn split(input: &[u8; BYTES]) -> [[u8; CHUNK]; ELEMENTS] {
        #[allow(path_statements)]
        Self::CHECK;

        let mut result = [[0u8; CHUNK]; ELEMENTS];
        let mut input_pos = 0;

        for chunk in &mut result {
            let remaining = BYTES.checked_sub(input_pos).expect("input_pos <= BYTES");
            let to_copy = remaining.min(CHUNK);
            let upper = input_pos
                .checked_add(to_copy)
                .expect("ELEMENTS * CHUNK <= usize::MAX");
            chunk[0..to_copy].copy_from_slice(&input[input_pos..upper]);
            input_pos = input_pos
                .checked_add(to_copy)
                .expect("ELEMENTS * CHUNK <= usize::MAX");
        }

        result
    }

    /// Join chunks back into the original byte array, taking data sequentially
    /// from each and dropping any padding in the final chunk.
    pub(crate) fn join(chunks: &[[u8; CHUNK]; ELEMENTS]) -> [u8; BYTES] {
        #[allow(path_statements)]
        Self::CHECK;

        let mut result = [0u8; BYTES];
        let mut output_pos = 0;

        for chunk in chunks {
            let remaining = BYTES.checked_sub(output_pos).expect("output_pos <= BYTES");
            let to_copy = remaining.min(CHUNK);
            let upper = output_pos
                .checked_add(to_copy)
                .expect("ELEMENTS * CHUNK <= usize::MAX");
            result[output_pos..upper].copy_from_slice(&chunk[0..to_copy]);
            output_pos = output_pos
                .checked_add(to_copy)
                .expect("ELEMENTS * CHUNK <= usize::MAX");
        }

        result
    }
}

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
    /// The chunks are written into `out`, a buffer the caller owns, so that a
    /// caller holding a secret can keep them in a buffer that is overwritten
    /// when dropped. A trailing partial chunk is zero-padded;
    /// [`join_into`](Self::join_into) discards that padding, so the round trip
    /// is exact.
    pub(crate) fn split_into(input: &[u8; BYTES], out: &mut [[u8; CHUNK]; ELEMENTS]) {
        #[allow(path_statements)]
        Self::CHECK;

        let mut input_pos = 0;

        for chunk in out {
            chunk.fill(0);
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
    }

    /// Join chunks back into the original byte array, taking data sequentially
    /// from each and dropping any padding in the final chunk.
    ///
    /// The bytes are written into `out`, a buffer the caller owns, for the same
    /// reason as in [`split_into`](Self::split_into).
    pub(crate) fn join_into(chunks: &[[u8; CHUNK]; ELEMENTS], out: &mut [u8; BYTES]) {
        #[allow(path_statements)]
        Self::CHECK;

        let mut output_pos = 0;

        for chunk in chunks {
            let remaining = BYTES.checked_sub(output_pos).expect("output_pos <= BYTES");
            let to_copy = remaining.min(CHUNK);
            let upper = output_pos
                .checked_add(to_copy)
                .expect("ELEMENTS * CHUNK <= usize::MAX");
            out[output_pos..upper].copy_from_slice(&chunk[0..to_copy]);
            output_pos = output_pos
                .checked_add(to_copy)
                .expect("ELEMENTS * CHUNK <= usize::MAX");
        }
    }
}

/// Tests of the chunking.
#[cfg(test)]
mod tests {
    use super::Codec;

    /// `split_into` writes every byte of its output, padding included, and
    /// `join_into` restores the input.
    #[test]
    fn test_split_into_join_into_round_trip() {
        let input = [7u8; 31];
        let mut chunks = [[0xaau8; 30]; 2];
        Codec::<30, 31, 2>::split_into(&input, &mut chunks);
        assert_eq!(chunks[0], [7u8; 30]);
        assert_eq!(chunks[1][0], 7);
        assert!(chunks[1][1..].iter().all(|&b| b == 0));

        let mut output = [0u8; 31];
        Codec::<30, 31, 2>::join_into(&chunks, &mut output);
        assert_eq!(output, input);
    }
}

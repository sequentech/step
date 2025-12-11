// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
extern crate cfg_if;

pub mod protocol;
pub mod util;

/// Macro to dispatch threshold trustee operations based on runtime values
/// to compile-time const generic parameters
#[macro_export]
macro_rules! dispatch_threshold_trustees {
    ($threshold:expr, $trustees:expr, $body:expr) => {
        match ($threshold, $trustees) {
            (2, 2) => { const T: usize = 2; const P: usize = 2; $body }
            (2, 3) => { const T: usize = 2; const P: usize = 3; $body }
            (2, 4) => { const T: usize = 2; const P: usize = 4; $body }
            (2, 5) => { const T: usize = 2; const P: usize = 5; $body }
            (2, 6) => { const T: usize = 2; const P: usize = 6; $body }
            (3, 3) => { const T: usize = 3; const P: usize = 3; $body }
            (3, 4) => { const T: usize = 3; const P: usize = 4; $body }
            (3, 5) => { const T: usize = 3; const P: usize = 5; $body }
            (3, 6) => { const T: usize = 3; const P: usize = 6; $body }
            (4, 4) => { const T: usize = 4; const P: usize = 4; $body }
            (4, 5) => { const T: usize = 4; const P: usize = 5; $body }
            (4, 6) => { const T: usize = 4; const P: usize = 6; $body }
            (5, 5) => { const T: usize = 5; const P: usize = 5; $body }
            (5, 6) => { const T: usize = 5; const P: usize = 6; $body }
            (6, 6) => { const T: usize = 6; const P: usize = 6; $body }
            _ => panic!("Unsupported threshold={} trustees={} combination", $threshold, $trustees)
        }
    };
}

/// Macro to dispatch ciphertext width operations based on runtime values
/// to compile-time const generic parameters
#[macro_export]
macro_rules! dispatch_ciphertext_width {
    ($width:expr, $body:expr) => {
        match $width {
            1 => { const W: usize = 1; $body }
            2 => { const W: usize = 2; $body }
            3 => { const W: usize = 3; $body }
            4 => { const W: usize = 4; $body }
            _ => panic!("Unsupported ciphertext_width={}", $width)
        }
    };
}

// Platform-specific modules
#[cfg(feature = "native")]
pub mod native;
#[cfg(feature = "wasm")]
pub mod wasm;

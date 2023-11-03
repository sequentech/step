#[cfg(any(feature = "openssl_core", feature="openssl_full"))]
pub mod openssl;
#[cfg(not(any(feature = "openssl_core", feature="openssl_full")))]
pub mod rustcrypto;

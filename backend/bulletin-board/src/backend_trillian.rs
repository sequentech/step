// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use core::slice;
use sha2::{Digest, Sha256};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_uchar};
use thiserror::Error;
use tracing::{debug, error, instrument};

static RFC6962_LEAF_HASH_PREFIX: u8 = 0;

/// Hash a leaf entry in the Trillian Log. Equivalent to HashLeaf in
/// https://github.com/transparency-dev/merkle/blob/3492aa43de727559b194156789b87a31c3366697/rfc6962/rfc6962.go#L49
pub fn hash_leaf(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(&[RFC6962_LEAF_HASH_PREFIX]);
    hasher.update(bytes);
    hasher.finalize().to_vec()
}

/// Enumerates all possible errors returned by this Go Trillian Log Rust wrapper
#[derive(Error, Debug)]
pub enum TrillianError {
    /// Error when calling CString::new indicating that an interion nul byte
    /// was found
    #[error(transparent)]
    NulError(#[from] std::ffi::NulError),

    /// Error when calling CStr::to_str indicating that the string was not
    /// valid UTF-8
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    /// Error when the requested Origin is empty
    #[error("Origin cannot be empty")]
    OriginEmpty,

    /// Error parsing the private key to instantiate the signer
    #[error("Error `{0}` parsing the private key to instantiate the signer")]
    SignerInstantiationError(String),

    /// Error when creating the log
    #[error("Error `{0}` when creating the log")]
    LogCreationError(String),

    /// Error when signing a checkpoint
    #[error("Error `{0}` when signing a checkpoint")]
    CheckpointSigningError(String),

    /// Error when reading a checkpoint
    #[error(
        "Error `{0}` when reading the log checkpoint from the storage dir"
    )]
    ReadLogCheckpointError(String),

    /// Error reading the public to instantiate the verifier
    #[error("Error `{0}` reading the public to instantiate the verifier")]
    VerifierInstantiationError(String),

    /// Error when parsing a checkpoint
    #[error("Error `{0}` when parsing a checkpoint")]
    CheckpointParsingError(String),

    /// Error when loading a storage instance
    #[error("Error `{0}` when loading a storage instance")]
    StorageLoadError(String),

    /// Error integrating new entries
    #[error("Error `{0}` integrating new entries")]
    IntegrationError(String),

    /// Entry is duplicated
    #[error("Entry is duplicated")]
    DuplicatedEntryError(String),

    /// Error `{0}` when sequencing entry
    #[error("Error `{0}` when sequencing entry")]
    EntrySequencingError(String),

    /// Unexpected Error: this might be a known error but one that is not
    /// expected to be possible that it's returned in a specifc result
    #[error("Unexpected Error")]
    UnexpectedError,

    /// Unknown Error
    #[error("Unknown Error")]
    UnknownError,

    /// Error serializing an entry
    #[error("Error serializing an entry")]
    EntrySerializationError,
}

extern "C" {
    fn CGenerateKeys(name: *const c_char) -> *const GenerateKeysResultC;

    fn CIntegrate(
        // Root directory to store log dat
        storageDirC: *const c_char,
        // Set when creating a new log to initialise the structure
        initialise: bool,
        // Public key
        pubKeyC: *const c_char,
        // Private key
        privKeyC: *const c_char,
        // Log origin string to use in produced checkpoint
        originC: *const c_char,
    ) -> *const IntegrateResultC;

    fn CSequence(
        // Root directory to store log dat
        storageDirC: *const c_char,
        // List of entries to sequence
        entriesC: *const EntryC,
        // Number of entries
        numEntries: usize,
        // Public key
        pubKeyC: *const c_char,
        // Log origin string to use in produced checkpoint
        originC: *const c_char,
    ) -> *const SequenceResultC;

    fn CReadCheckpoint(
        // Root directory to store log dat
        storageDirC: *const c_char,
        // Public key
        pubKeyC: *const c_char,
        // Log origin string
        originC: *const c_char,
    ) -> *const IntegrateResultC;
}

#[derive(Debug)]
#[repr(C)]
pub struct GenerateKeysResultC {
    pub public_key: *const c_char,
    pub private_key: *const c_char,
    pub error_code: *const c_char,
    pub error_description: *const c_char,
}

#[derive(Debug)]
pub struct GenerateKeysResult {
    pub public_key: String,
    pub private_key: String,
}

#[derive(Debug)]
#[repr(C)]
pub struct IntegrateResultC {
    pub checkpoint_origin: *const c_char,
    pub checkpoint_size: u64,
    pub checkpoint_hash: *const c_char,
    pub error_code: *const c_char,
    pub error_description: *const c_char,
}

#[derive(Debug)]
pub struct IntegrateResult {
    pub checkpoint_origin: String,
    pub checkpoint_size: u64,
    pub checkpoint_hash: String,
}

#[derive(Debug)]
#[repr(C)]
pub struct EntryC {
    pub temp_file_path: *const c_char,
    pub hash: [c_uchar; 32],
}

/// Frees an owned string pointer
fn free_string(ptr: *const c_char) {
    if ptr.is_null() {
        error!("free_string errored: got NULL ptr");
        ::std::process::abort();
    }
    drop(unsafe { CString::from_raw(ptr as *mut _) });
}

impl Drop for EntryC {
    fn drop(&mut self) {
        free_string(self.temp_file_path);
    }
}
pub struct Entry {
    pub hash: [u8; 32],
    pub temp_file_path: String,
}

#[derive(Debug)]
#[repr(C)]
pub struct SequenceResultC {
    pub entries_sequence_ids: *const u64,
    pub error_code: *const c_char,
    pub error_description: *const c_char,
}

pub struct SequenceResult {
    pub entries_sequence_ids: Vec<u64>,
}

#[derive(Debug)]
#[repr(C)]
pub struct ReadCheckpointResultC {
    pub checkpoint_origin: *const c_char,
    pub checkpoint_size: u64,
    pub checkpoint_hash: *const c_char,
    pub error_code: *const c_char,
    pub error_description: *const c_char,
}

#[derive(Debug)]
pub struct ReadCheckpointResult {
    pub checkpoint_origin: String,
    pub checkpoint_size: u64,
    pub checkpoint_hash: String,
}

#[instrument]
pub fn generate_keys(name: &str) -> Result<GenerateKeysResult, String> {
    let c_name = CString::new(name).map_err(|x| x.to_string())?;
    debug!("calling GenerateKeys(): before");
    let c_result = unsafe { CGenerateKeys(c_name.as_ptr()) };
    debug!("calling GenerateKeys(): after");

    // check for errors
    let has_error = unsafe { !(*c_result).error_code.is_null() };
    if has_error {
        let error = unsafe { CStr::from_ptr((*c_result).error_code) }
            .to_str()
            .map_err(|x| x.to_string())?;
        if error.len() > 0 {
            error!(error, "calling GeneratedKeys(): returned error");
            return Err(String::from(error));
        }
    }

    // if no errors, convert to a result
    let result = unsafe {
        GenerateKeysResult {
            public_key: CStr::from_ptr((*c_result).public_key)
                .to_str()
                .map_err(|x| x.to_string())?
                .to_string(),
            private_key: CStr::from_ptr((*c_result).private_key)
                .to_str()
                .map_err(|x| x.to_string())?
                .to_string(),
        }
    };
    return Ok(result);
}

#[instrument]
pub fn integrate(
    // Root directory to store log dat
    storage_dir: &str,
    // Set when creating a new log to initialise the structure
    initialise: bool,
    // Public key
    pub_key: &str,
    // Private key
    priv_key: &str,
    // Log origin string to use in produced checkpoint
    origin: &str,
) -> Result<IntegrateResult, TrillianError> {
    let c_storage_dir = CString::new(storage_dir)?;
    let c_pub_key = CString::new(pub_key)?;
    let c_priv_key = CString::new(priv_key)?;
    let c_origin = CString::new(origin)?;

    debug!("calling Integrate(): before");
    let c_result = unsafe {
        CIntegrate(
            c_storage_dir.as_ptr(),
            initialise,
            c_pub_key.as_ptr(),
            c_priv_key.as_ptr(),
            c_origin.as_ptr(),
        )
    };
    debug!("calling Integrate(): after");

    // check for errors
    let has_error = unsafe { !(*c_result).error_code.is_null() };
    if has_error {
        let error_code =
            unsafe { CStr::from_ptr((*c_result).error_code) }.to_str()?;
        let error_description: String =
            unsafe { CStr::from_ptr((*c_result).error_description) }
                .to_str()?
                .into();
        if error_code.len() > 0 {
            error!(error_code, "calling Integrate(): returned error");
            return Err(match error_code {
                "OriginEmpty" => TrillianError::OriginEmpty,
                "SignerInstantiationError" => {
                    TrillianError::SignerInstantiationError(error_description)
                }
                "LogCreationError" => {
                    TrillianError::LogCreationError(error_description)
                }
                "SigningError" => {
                    TrillianError::CheckpointSigningError(error_description)
                }
                "ReadLogCheckpointError" => {
                    TrillianError::ReadLogCheckpointError(error_description)
                }
                "VerifierInstantiationError" => {
                    TrillianError::VerifierInstantiationError(error_description)
                }
                "CheckpointParsingError" => {
                    TrillianError::CheckpointParsingError(error_description)
                }
                "StorageLoadError" => {
                    TrillianError::StorageLoadError(error_description)
                }
                "IntegrationError" => {
                    TrillianError::IntegrationError(error_description)
                }
                _ => TrillianError::UnexpectedError,
            });
        }
    }

    let checkpoint_hash_str =
        unsafe { CStr::from_ptr((*c_result).checkpoint_hash) }.to_str()?;

    let checkpoint_origin_str =
        unsafe { CStr::from_ptr((*c_result).checkpoint_origin) }.to_str()?;

    // Everything was ok
    return Ok(IntegrateResult {
        checkpoint_origin: String::from(checkpoint_origin_str),
        checkpoint_size: unsafe { (*c_result).checkpoint_size },
        checkpoint_hash: String::from(checkpoint_hash_str),
    });
}

// Following the suggestion in
// https://users.rust-lang.org/t/preparing-an-array-of-structs-for-ffi/33411/2
fn vec_to_cffi_array<T>(input: Vec<T>) -> (*mut T, usize) {
    let boxed_slice: Box<[T]> = input.into_boxed_slice();
    let length = boxed_slice.len();
    let fat_ptr: *mut [T] = Box::into_raw(boxed_slice);
    let slim_ptr: *mut T = fat_ptr as _;
    return (slim_ptr, length);
}

// Inspired in https://stackoverflow.com/questions/34622127/how-to-convert-a-const-pointer-into-a-vec-to-correctly-drop-it
fn cffi_array_into_vec<T>(data: *const T, len: usize) -> Vec<T> {
    unsafe { Vec::from_raw_parts(data as *mut T, len, len) }
}

fn free_entries(ptr: *mut EntryC, len: usize) {
    if ptr.is_null() {
        eprintln!("free_entries() errored: got NULL ptr!");
        ::std::process::abort();
    }
    let entries = unsafe { slice::from_raw_parts_mut(ptr, len) };
    drop(unsafe { Box::from_raw(entries) });
}

/// Sequence an ordered list of entries.
/// Regarding the logging with the `#instrument` decorator, note we skip the
/// entries since it could be huge.
#[instrument(skip(entries))]
pub fn sequence(
    // Root directory to store log dat
    storage_dir: &str,
    // List of entries to sequence
    entries: Vec<Entry>,
    // Public key
    pub_key: &str,
    // Log origin string to use in produced checkpoint
    origin: &str,
) -> Result<SequenceResult, TrillianError> {
    let c_storage_dir = CString::new(storage_dir)?;
    let c_pub_key = CString::new(pub_key)?;
    let c_origin = CString::new(origin)?;
    let (c_entries, c_entries_length) = vec_to_cffi_array(
        entries
            .into_iter()
            .map(|entry| {
                let c_temp_file_path = CString::new(entry.temp_file_path)?;
                let c_entry = EntryC {
                    temp_file_path: c_temp_file_path.into_raw(),
                    hash: entry.hash,
                };
                Ok(c_entry)
            })
            .collect::<Result<Vec<EntryC>, TrillianError>>()?,
    );

    debug!("calling Sequence(): before");
    let c_result = unsafe {
        CSequence(
            c_storage_dir.as_ptr(),
            c_entries,
            c_entries_length,
            c_pub_key.as_ptr(),
            c_origin.as_ptr(),
        )
    };
    debug!("calling Sequence(): after");

    // free the array
    free_entries(c_entries, c_entries_length);

    // check for errors
    let has_error = unsafe { !(*c_result).error_code.is_null() };
    if has_error {
        let error_code =
            unsafe { CStr::from_ptr((*c_result).error_code) }.to_str()?;
        let error_description: String =
            unsafe { CStr::from_ptr((*c_result).error_description) }
                .to_str()?
                .into();
        if error_code.len() > 0 {
            error!(error_code, "calling Sequence(): returned error");
            return Err(match error_code {
                "ReadLogCheckpointError" => {
                    TrillianError::ReadLogCheckpointError(error_description)
                }
                "VerifierInstantiationError" => {
                    TrillianError::VerifierInstantiationError(error_description)
                }
                "CheckpointParsingError" => {
                    TrillianError::CheckpointParsingError(error_description)
                }
                "DuplicatedEntryError" => {
                    TrillianError::DuplicatedEntryError(error_description)
                }
                "EntrySequencingError" => {
                    TrillianError::EntrySequencingError(error_description)
                }
                _ => TrillianError::UnexpectedError,
            });
        }
    }

    return Ok(SequenceResult {
        entries_sequence_ids: cffi_array_into_vec(
            unsafe { (*c_result).entries_sequence_ids },
            c_entries_length,
        ),
    });
}

#[instrument]
pub fn read_checkpoint(
    // Root directory to store log dat
    storage_dir: &str,
    // Public key
    pub_key: &str,
    // Log origin string
    origin: &str,
) -> Result<ReadCheckpointResult, TrillianError> {
    let c_storage_dir = CString::new(storage_dir)?;
    let c_pub_key = CString::new(pub_key)?;
    let c_origin = CString::new(origin)?;

    debug!("calling ReadCheckpoint(): before");
    let c_result = unsafe {
        CReadCheckpoint(
            c_storage_dir.as_ptr(),
            c_pub_key.as_ptr(),
            c_origin.as_ptr(),
        )
    };
    debug!("calling ReadCheckpoint(): after");

    // check for errors
    let has_error = unsafe { !(*c_result).error_code.is_null() };
    if has_error {
        let error_code =
            unsafe { CStr::from_ptr((*c_result).error_code) }.to_str()?;
        let error_description: String =
            unsafe { CStr::from_ptr((*c_result).error_description) }
                .to_str()?
                .into();
        if error_code.len() > 0 {
            error!(error_code, "calling ReadCheckpoint(): returned error");
            return Err(match error_code {
                "ReadLogCheckpointError" => {
                    TrillianError::ReadLogCheckpointError(error_description)
                }
                "VerifierInstantiationError" => {
                    TrillianError::VerifierInstantiationError(error_description)
                }
                "CheckpointParsingError" => {
                    TrillianError::CheckpointParsingError(error_description)
                }
                _ => TrillianError::UnknownError,
            });
        }
    }

    let checkpoint_hash_str =
        unsafe { CStr::from_ptr((*c_result).checkpoint_hash) }.to_str()?;

    let checkpoint_origin_str =
        unsafe { CStr::from_ptr((*c_result).checkpoint_origin) }.to_str()?;

    // Everything was ok
    return Ok(ReadCheckpointResult {
        checkpoint_origin: String::from(checkpoint_origin_str),
        checkpoint_size: unsafe { (*c_result).checkpoint_size },
        checkpoint_hash: String::from(checkpoint_hash_str),
    });
}

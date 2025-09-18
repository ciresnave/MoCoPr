//! JSON parsing utilities, with support for multiple backends.
//!
//! This module provides a consistent interface for JSON parsing, allowing
//! the underlying implementation to be swapped out at compile time using
//! feature flags.
//!
//! By default, it uses `serde_json`, but can be configured to use `simd-json`
//! for improved performance by enabling the `simd-json-performance` feature.

use serde::de::DeserializeOwned;
use crate::{Error, Result};

/// Parse a JSON string from a string slice.
///
/// # Arguments
///
/// * `s` - The string slice to parse
///
/// # Returns
///
/// A `Result` containing the parsed value or an error
#[cfg(feature = "simd-json-performance")]
pub fn from_str<T>(s: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    // Unsafe is required here because simd-json expects a mutable string.
    // This is safe because from_str does not actually mutate the string.
    let mut s_mut = s.to_string();
    unsafe {
        simd_json::from_str(&mut s_mut).map_err(|e| Error::Json(e.to_string()))
    }
}

/// Parse a JSON string from a byte slice.
///
/// # Arguments
///
/// * `s` - The byte slice to parse
///
/// # Returns
///
/// A `Result` containing the parsed value or an error
#[cfg(feature = "simd-json-performance")]
pub fn from_slice<T>(s: &[u8]) -> Result<T>
where
    T: DeserializeOwned,
{
    let mut bytes = s.to_vec();
    simd_json::from_slice(&mut bytes).map_err(|e| Error::Json(e.to_string()))
}

/// Parse a JSON string from a string slice.
///
/// # Arguments
///
/// * `s` - The string slice to parse
///
/// # Returns
///
/// A `Result` containing the parsed value or an error
#[cfg(not(feature = "simd-json-performance"))]
pub fn from_str<T>(s: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_str(s).map_err(|e| Error::Json(e.to_string()))
}

/// Parse a JSON string from a byte slice.
///
/// # Arguments
///
/// * `s` - The byte slice to parse
///
/// # Returns
///
/// A `Result` containing the parsed value or an error
#[cfg(not(feature = "simd-json-performance"))]
pub fn from_slice<T>(s: &[u8]) -> Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_slice(s).map_err(|e| Error::Json(e.to_string()))
}

/// Serialize a value to a JSON string.
///
/// This function uses `serde_json` for serialization, as `simd-json`
/// does not provide a serialization API.
///
/// # Arguments
///
/// * `value` - The value to serialize
///
/// # Returns
///
/// A `Result` containing the JSON string or an error
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: serde::Serialize,
{
    serde_json::to_string(value).map_err(|e| Error::Json(e.to_string()))
}

/// Serialize a value to a JSON byte vector.
///
/// # Arguments
///
/// * `value` - The value to serialize
///
/// # Returns
///
/// A `Result` containing the JSON byte vector or an error
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>>
where
    T: serde::Serialize,
{
    serde_json::to_vec(value).map_err(|e| Error::Json(e.to_string()))
}

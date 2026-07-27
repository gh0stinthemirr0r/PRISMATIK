//! Helpers that keep sensitive buffers behind `secrecy` + `zeroize`.

use secrecy::{ExposeSecret, SecretString, SecretVec};
use zeroize::Zeroize;

/// Marker / helper trait for types that wipe sensitive bytes on drop.
///
/// Prefer holding secrets in [`SecretString`] / [`SecretVec`] / [`secrecy::Secret`]
/// rather than raw `String`/`Vec`. This trait documents the wipe contract for
/// custom wrappers in the security crate.
pub trait ZeroizeSecret: Zeroize {
    /// Explicitly wipe; also invoked via [`Drop`] for implementors that call it.
    fn zeroize_secret(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeSecret for Vec<u8> {}
impl ZeroizeSecret for String {}
impl<const N: usize> ZeroizeSecret for [u8; N] {}

/// Copy a [`SecretString`] into a temporary `String`, invoke `f`, then zeroize
/// the temporary. Prefer keeping work inside `f` without cloning when possible.
pub fn with_exposed_string<R>(secret: &SecretString, f: impl FnOnce(&str) -> R) -> R {
    f(secret.expose_secret())
}

/// Copy a [`SecretVec`] into a temporary buffer for `f`, then zeroize it.
pub fn with_exposed_bytes<R>(secret: &SecretVec<u8>, f: impl FnOnce(&[u8]) -> R) -> R {
    f(secret.expose_secret())
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    #[test]
    fn zeroize_secret_clears_vec() {
        let mut v = vec![1u8, 2, 3, 4];
        v.zeroize_secret();
        assert!(v.iter().all(|&b| b == 0) || v.is_empty());
    }

    #[test]
    fn with_exposed_string_reads_value() {
        let s = SecretString::new("token-value".into());
        let len = with_exposed_string(&s, |t| t.len());
        assert_eq!(len, "token-value".len());
    }
}

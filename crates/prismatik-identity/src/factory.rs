//! Entropy-backed asset id factory.
//!
//! Spec: `DOCS/waves/Wave_0_Foundation.md` (P0-DK-05).
//!
//! Production assets are content-addressed from a
//! [`CanonicalIdentityRecord`](crate::CanonicalIdentityRecord). The factory
//! mints *synthetic* ids (tests, unnamed intermediates, ephemeral simulation
//! instruments) from a labelled entropy stream so they remain reproducible.

use crate::asset_id::AssetId;
use prismatik_determinism::{ContentHash, Entropy};
use std::fmt;

/// Mints reproducible synthetic [`AssetId`]s from an [`Entropy`] stream.
///
/// Never use this for cryptographic material — entropy here is deterministic
/// Monte Carlo / simulation entropy, not OS CSPRNG.
pub struct AssetIdFactory<'a> {
    entropy: &'a mut dyn Entropy,
}

impl fmt::Debug for AssetIdFactory<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AssetIdFactory")
            .field("entropy", &"<dyn Entropy>")
            .finish()
    }
}

impl<'a> AssetIdFactory<'a> {
    /// Bind a factory to an entropy stream (typically a labelled split such
    /// as `ctx.entropy.split("identity")`).
    pub fn new(entropy: &'a mut dyn Entropy) -> Self {
        Self { entropy }
    }

    /// Mint a fresh synthetic `AssetId` by consuming 32 bytes of entropy.
    pub fn mint(&mut self) -> AssetId {
        let mut buf = [0u8; 32];
        self.entropy.fill_bytes(&mut buf);
        AssetId::from_hash(ContentHash::from(buf))
    }

    /// Mint `n` synthetic ids. Order is deterministic for a given stream.
    pub fn mint_n(&mut self, n: usize) -> Vec<AssetId> {
        (0..n).map(|_| self.mint()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prismatik_determinism::SplitEntropy;

    #[test]
    fn same_seed_mints_same_ids() {
        let mut stream_a = SplitEntropy::from_seed(42).split("identity");
        let mut stream_b = SplitEntropy::from_seed(42).split("identity");
        let mut a = AssetIdFactory::new(stream_a.as_mut());
        let mut b = AssetIdFactory::new(stream_b.as_mut());
        assert_eq!(a.mint_n(8), b.mint_n(8));
    }

    #[test]
    fn different_labels_diverge() {
        let parent = SplitEntropy::from_seed(7);
        let mut stream_a = parent.split("a");
        let mut stream_b = parent.split("b");
        let mut left = AssetIdFactory::new(stream_a.as_mut());
        let mut right = AssetIdFactory::new(stream_b.as_mut());
        assert_ne!(left.mint(), right.mint());
    }
}

use runtime_primitives::{
    generic::DigestItem,
};
use parity_codec::{Encode, Decode};

pub const FINALITY_TRACKER_MODULE_LOG_PREFIX: u8 = 4;

/// Digest item for finality-tracker.
pub trait FinalityTrackerDigestItem: Sized {
    /// revert to finality_tracker data.
    fn as_finality_tracker(&self) -> Option<u64>;
}

impl<Hash, AuthorityId, SealSignature> FinalityTrackerDigestItem for DigestItem<Hash, AuthorityId, SealSignature> {
    fn as_finality_tracker(&self) -> Option<u64> {
        match self {
            DigestItem::Other(data) if data.len() >= 2 && data[0] == FINALITY_TRACKER_MODULE_LOG_PREFIX && data[1] == 0
            => {
                let input = &mut &data[2..];
                let num = Decode::decode(input)?;
                Some(num)
            }
            _ => None
        }
    }
}
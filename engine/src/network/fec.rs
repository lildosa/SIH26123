use crate::protocol::{Envelope, FleetMessage};

/// Low-latency forward error correction without external crates.
///
/// Design (see AGENT_IMPROVEMENT_PLAN Review Finding 1):
/// - High-priority control messages (`Intent`, `Conflict`, `Yield`) use
///   adaptive dual-burst redundancy (N=2): the same envelope (identical
///   sequence number) is transmitted twice back-to-back. The existing
///   O(1) dedup in `decide_phase` drops the duplicate, giving survival
///   probability `(1 - p^2)` at packet-drop rate `p` with zero buffering
///   latency (e.g. 96% delivery at 20% loss).
/// - Bulk stream / map-sync payloads use chunked XOR parity blocks where all
///   chunks are ready simultaneously, so no artificial tick buffering occurs.

/// Number of transmissions for high-priority control envelopes.
pub const BURST_REPLICATION: usize = 2;

/// Returns true for messages that require dual-burst protection.
pub fn is_high_priority(payload: &FleetMessage) -> bool {
    matches!(
        payload,
        FleetMessage::Intent(_) | FleetMessage::Conflict(_) | FleetMessage::Yield(_)
    )
}

/// Survival probability after dual-burst at per-packet drop rate `p`.
pub fn burst_survival_probability(drop_rate: f64) -> f64 {
    let p = drop_rate.clamp(0.0, 1.0);
    1.0 - p * p
}

/// Build the N=2 burst frames for an envelope.
///
/// Frames share the identical sequence number so the receiver's
/// `last_seq_seen` dedup accepts the first arrival and drops the copy.
pub fn burst_envelopes(env: &Envelope) -> Vec<Envelope> {
    vec![env.clone(), env.clone()]
}

/// Helper for high-priority control envelopes.
pub struct AdaptiveBurstTransport;

impl AdaptiveBurstTransport {
    /// Returns 1 frame for bulk traffic, 2 identical frames for control traffic.
    pub fn encode(env: &Envelope) -> Vec<Envelope> {
        if is_high_priority(&env.payload) {
            burst_envelopes(env)
        } else {
            vec![env.clone()]
        }
    }

    pub fn survival_probability(drop_rate: f64) -> f64 {
        burst_survival_probability(drop_rate)
    }
}

/// XOR parity over a block of chunks, zero-padded to the longest chunk.
///
/// `parity[i] = chunk_0[i] ^ chunk_1[i] ^ ...` with missing bytes as 0.
/// Returns an empty vec for an empty block.
pub fn encode_fec_block(chunks: &[Vec<u8>]) -> Vec<u8> {
    let max_len = chunks.iter().map(|c| c.len()).max().unwrap_or(0);
    let mut parity = vec![0u8; max_len];
    for chunk in chunks {
        for (i, b) in chunk.iter().enumerate() {
            parity[i] ^= b;
        }
    }
    parity
}

/// Recover exactly one missing chunk from a block given the parity.
///
/// `available` holds `Some(bytes)` for received chunks and `None` for the
/// single missing slot. Returns `None` when zero or 2+ chunks are missing.
pub fn decode_missing_single(available: &[Option<Vec<u8>>], parity: &[u8]) -> Option<Vec<u8>> {
    let missing: Vec<usize> = available
        .iter()
        .enumerate()
        .filter_map(|(i, c)| if c.is_none() { Some(i) } else { None })
        .collect();
    if missing.len() != 1 {
        return None;
    }
    let max_len = parity.len();
    let mut recovered = parity.to_vec();
    for slot in available.iter().flatten() {
        for i in 0..slot.len().min(max_len) {
            recovered[i] ^= slot[i];
        }
    }
    Some(recovered)
}

/// Incremental decoder for bulk blocks: collect K chunks + 1 parity,
/// tolerate any single-chunk loss.
pub struct FecDecoder {
    expected_chunks: usize,
    slots: Vec<Option<Vec<u8>>>,
    parity: Option<Vec<u8>>,
}

impl FecDecoder {
    pub fn new(expected_chunks: usize) -> Self {
        Self {
            expected_chunks,
            slots: vec![None; expected_chunks],
            parity: None,
        }
    }

    pub fn push_chunk(&mut self, index: usize, bytes: Vec<u8>) {
        if index < self.expected_chunks {
            self.slots[index] = Some(bytes);
        }
    }

    pub fn push_parity(&mut self, parity: Vec<u8>) {
        self.parity = Some(parity);
    }

    /// Returns the full ordered block when recoverable (0 or 1 missing).
    pub fn try_decode(&self) -> Option<Vec<Vec<u8>>> {
        let missing = self.slots.iter().filter(|s| s.is_none()).count();
        if missing == 0 {
            return Some(self.slots.iter().map(|s| s.clone().unwrap()).collect());
        }
        if missing == 1 {
            let parity = self.parity.as_ref()?;
            let recovered = decode_missing_single(&self.slots, parity)?;
            let mut out = Vec::with_capacity(self.expected_chunks);
            let mut used_recovered = false;
            for slot in &self.slots {
                match slot {
                    Some(b) => out.push(b.clone()),
                    None if !used_recovered => {
                        out.push(recovered.clone());
                        used_recovered = true;
                    }
                    None => return None,
                }
            }
            return Some(out);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parity_roundtrip_single_loss() {
        let chunks = vec![vec![1u8, 2, 3], vec![4u8, 5], vec![9u8, 9, 9, 9]];
        let parity = encode_fec_block(&chunks);
        let mut slots: Vec<Option<Vec<u8>>> = chunks.into_iter().map(Some).collect();
        slots[1] = None;
        let recovered = decode_missing_single(&slots, &parity).unwrap();
        assert_eq!(&recovered[..2], &[4u8, 5]);
    }
}

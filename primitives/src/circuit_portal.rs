use crate::ProofTriePointer;
use snowbridge_core::Verifier;
use sp_std::vec::Vec;
use sp_trie::StorageProof;

pub trait CircuitPortal<T: frame_system::Config> {
    type EthVerifier: Verifier;

    fn confirm_parachain_header(
        gateway_id: [u8; 4],
        block_hash: Vec<u8>,
        proof: StorageProof,
    ) -> Result<Vec<u8>, &'static str>;

    fn confirm_event_inclusion(
        gateway_id: [u8; 4],
        encoded_event: Vec<u8>,
        maybe_proof: Option<Vec<Vec<u8>>>,
        maybe_block_hash: Option<Vec<u8>>,
    ) -> Result<(), &'static str>;
}

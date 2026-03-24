use super::sequential::SequentialTreeHash;
use thiserror::Error;

#[derive(Debug)]
pub struct HashLeaf {
    pub start: u64,
    pub stop: u64,
    pub hash: [u8; 32],
}

pub struct RandomInsertTreeHash {
    have_end_leaf: bool,
    part_size: u64,
    map_by_start: std::collections::HashMap<u64, HashLeaf>,
}

#[derive(Debug, Error)]
pub enum RandomInsertTreeHashError {
    #[error("Invalid range: start {start} stop {stop}")]
    InvalidRange { start: u64, stop: u64 },
    #[error("Leaf overlap at start {start} stop {stop}")]
    LeafOverlap { start: u64, stop: u64 },
    #[error("Leaf too big")]
    LeafTooBig,
    #[error("Unaligned leaf start at {0}")]
    UnalignedLeafStart(u64),
    #[error("Multiple short leaves encountered")]
    MultipleShortLeaves,
    #[error("Data gap between leaves at start {start} stop {stop}")]
    DataGap { start: u64, stop: u64 },
    #[error("Empty tree cannot compute hash")]
    EmptyTree,
}

impl RandomInsertTreeHash {
    pub fn new(part_size: u64) -> Self {
        RandomInsertTreeHash {
            have_end_leaf: false,
            part_size,
            map_by_start: std::collections::HashMap::new(),
        }
    }

    pub fn try_insert(
        &mut self,
        start: u64,
        stop: u64,
        hash: [u8; 32],
    ) -> Result<(), RandomInsertTreeHashError> {
        if start >= stop {
            return Err(RandomInsertTreeHashError::InvalidRange { start, stop });
        }
        if self.map_by_start.contains_key(&start) {
            return Err(RandomInsertTreeHashError::LeafOverlap { start, stop });
        }
        if !start.is_multiple_of(self.part_size) {
            return Err(RandomInsertTreeHashError::UnalignedLeafStart(start));
        }
        let size = stop - start;
        if size > self.part_size {
            return Err(RandomInsertTreeHashError::LeafTooBig);
        }
        if size < self.part_size {
            if self.have_end_leaf {
                return Err(RandomInsertTreeHashError::MultipleShortLeaves);
            }
            self.have_end_leaf = true;
        }
        let leaf = HashLeaf { start, stop, hash };
        self.map_by_start.insert(start, leaf);
        Ok(())
    }

    pub fn compute_hash(self) -> Result<[u8; 32], RandomInsertTreeHashError> {
        if self.map_by_start.is_empty() {
            return Err(RandomInsertTreeHashError::EmptyTree);
        }
        let mut leaves: Vec<HashLeaf> = self.map_by_start.into_values().collect();
        leaves.sort_by_key(|leaf| leaf.start);
        let mut last_stop = 0;
        // Ensure that the leaves are non-overlapping and cover the range from 0 to the end of the last leaf
        for leaf in &leaves {
            if leaf.start < last_stop {
                return Err(RandomInsertTreeHashError::LeafOverlap {
                    start: leaf.start,
                    stop: leaf.stop,
                });
            }
            if leaf.start > last_stop {
                return Err(RandomInsertTreeHashError::DataGap {
                    start: last_stop,
                    stop: leaf.start,
                });
            }
            last_stop = leaf.stop;
        }
        let mut tree = SequentialTreeHash::new();
        for leaf in leaves {
            tree.insert(leaf.hash);
        }
        Ok(tree.finalize().expect("Tree must have been empty!"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn test_hash_tree() {
        let mut tree = RandomInsertTreeHash::new(4);
        tree.try_insert(0, 4, [0; 32]).unwrap();
        tree.try_insert(4, 8, [1; 32]).unwrap();
        let hash = tree.compute_hash().unwrap();
        let mut hasher = Sha256::new();
        hasher.update([0; 32]);
        hasher.update([1; 32]);
        let result: [u8; 32] = hasher.finalize().into();
        assert_eq!(hash, result);
    }

    // Test unaligned leaf start
    #[test]
    fn test_unaligned_leaf_start() {
        let mut tree = RandomInsertTreeHash::new(4);
        let result = tree.try_insert(1, 5, [0; 32]);
        assert!(matches!(
            result,
            Err(RandomInsertTreeHashError::UnalignedLeafStart(1))
        ));
    }

    // Test leaf too big
    #[test]
    fn test_leaf_too_big() {
        let mut tree = RandomInsertTreeHash::new(4);
        let result = tree.try_insert(0, 5, [0; 32]);
        assert!(matches!(result, Err(RandomInsertTreeHashError::LeafTooBig)));
    }

    // Test multiple short leaves
    #[test]
    fn test_multiple_short_leaves() {
        let mut tree = RandomInsertTreeHash::new(4);
        tree.try_insert(0, 2, [0; 32]).unwrap();
        let result = tree.try_insert(4, 6, [0; 32]);
        assert!(matches!(
            result,
            Err(RandomInsertTreeHashError::MultipleShortLeaves)
        ));
    }
}

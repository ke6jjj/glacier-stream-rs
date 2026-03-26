use super::sequential::{SequentialTreeHash, SequentialTreeHashError};

#[derive(Debug)]
pub struct ReservedTreeHash {
    reserve_depth: i64,
    reserve: SequentialTreeHash,
    main: SequentialTreeHash,
}

impl Default for ReservedTreeHash {
    fn default() -> Self {
        Self::new(0)
    }
}

impl ReservedTreeHash {
    // Using a negative reserve depth will cause no reserve to be held
    // and all inserts will go directly to the main tree.
    pub fn new(reserve_depth: i64) -> Self {
        ReservedTreeHash {
            reserve_depth,
            reserve: SequentialTreeHash::new(),
            main: SequentialTreeHash::new(),
        }
    }

    pub fn insert(&mut self, hash: [u8; 32]) {
        if self.reserve.depth() < self.reserve_depth as i64 {
            self.reserve.insert(hash);
        } else {
            self.main.insert(hash);
        }
    }

    // Merges the reserve tree and then the main tree into the provided sequential tree.
    pub fn merge_into(self, sequential: &mut SequentialTreeHash) -> Result<(), SequentialTreeHashError> {
        sequential.merge(self.reserve)?;
        sequential.merge(self.main)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reserved_tree_hash() {
        let mut straight = SequentialTreeHash::new();
        straight.insert([0; 32]);
        let mut reserved = ReservedTreeHash::new(0);
        reserved.insert([1; 32]);
        reserved.insert([2; 32]);
        reserved.insert([3; 32]);
        reserved.merge_into(&mut straight)
            .expect("Merging reserved tree hash should not fail");
        let scenario1_hash = straight
            .finalize()
            .expect("Finalizing tree hash should not fail");
        let mut straight2 = SequentialTreeHash::new();
        straight2.insert([0; 32]);
        straight2.insert([1; 32]);
        straight2.insert([2; 32]);
        straight2.insert([3; 32]);
        let scenario2_hash = straight2
            .finalize()
            .expect("Finalizing tree hash should not fail");
        assert_eq!(scenario1_hash, scenario2_hash);
    }

    #[test]
    fn test_reserved_tree_hash_negative_depth() {
        let mut straight = SequentialTreeHash::new();
        let mut reserved = ReservedTreeHash::new(-1);
        reserved.insert([0; 32]);
        reserved.insert([1; 32]);
        reserved.insert([2; 32]);
        reserved.insert([3; 32]);
        reserved.merge_into(&mut straight)
            .expect("Merging reserved tree hash should not fail");
        let scenario1_hash = straight
            .finalize()
            .expect("Finalizing tree hash should not fail");
        let mut straight2 = SequentialTreeHash::new();
        straight2.insert([0; 32]);
        straight2.insert([1; 32]);
        straight2.insert([2; 32]);
        straight2.insert([3; 32]);
        let scenario2_hash = straight2
            .finalize()
            .expect("Finalizing tree hash should not fail");
        assert_eq!(scenario1_hash, scenario2_hash);
    }
}
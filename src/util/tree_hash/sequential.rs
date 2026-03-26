use sha2::{Digest, Sha256};
use thiserror::Error;
use std::cmp::Ordering;

#[derive(Debug, Error)]
pub enum SequentialTreeHashError {
    #[error("Tree hash is empty")]
    EmptyTree,
    #[error("Depth mismatch during merge")]
    DepthMismatch,
}

#[derive(Debug)]
pub struct SequentialTreeHash {
    stack: Vec<HashNode>,
}

#[derive(Debug)]
struct HashNode {
    hash: [u8; 32],
    depth: usize,
}

impl Default for SequentialTreeHash {
    fn default() -> Self {
        Self::new()
    }
}

impl SequentialTreeHash {
    pub fn new() -> Self {
        SequentialTreeHash { stack: Vec::new() }
    }

    // Current depth of the tree (0-based). Returns -1 if the tree is empty.
    pub fn depth(&self) -> i64 {
        self.stack
            .last()
            .map(|hd| hd.depth as i64)
            .unwrap_or(-1)
    }

    pub fn insert(&mut self, hash: [u8; 32]) {
        self.insert_node(HashNode { hash, depth: 0 })
            .expect("Zero depth insertion should never fail");
    }

    fn insert_node(&mut self, node: HashNode) -> Result<(), SequentialTreeHashError> {
        let mut current = node;
        while let Some(top) = self.stack.last() {
            match top.depth.cmp(&current.depth) {
                Ordering::Greater => break,
                Ordering::Equal => {
                    let left = self.stack.pop().unwrap();
                    let mut hasher = Sha256::new();
                    hasher.update(left.hash);
                    hasher.update(current.hash);
                    current = HashNode {
                        hash: hasher.finalize().into(),
                        depth: left.depth + 1,
                    };
                }
                Ordering::Less => return Err(SequentialTreeHashError::DepthMismatch),
            }
        }
        self.stack.push(current);
        Ok(())
    }

    pub fn merge(&mut self, other: SequentialTreeHash) -> Result<(), SequentialTreeHashError> {
        for hash_node in other.stack {
            self.insert_node(hash_node)?
        }
        Ok(())
    }

    pub fn finalize(mut self) -> Result<[u8; 32], SequentialTreeHashError> {
        if self.stack.is_empty() {
            return Err(SequentialTreeHashError::EmptyTree);
        }
        while self.stack.len() > 1 {
            let right = self.stack.pop().unwrap();
            let left = self.stack.pop().unwrap();
            let mut hasher = Sha256::new();
            hasher.update(left.hash);
            hasher.update(right.hash);
            self.stack.push(HashNode {
                hash: hasher.finalize().into(),
                depth: left.depth + 1,
            });
        }
        Ok(self.stack.pop().unwrap().hash)
    }
}

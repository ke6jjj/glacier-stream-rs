use sha2::{Digest, Sha256};

pub struct SequentialTreeHash {
    stack: Vec<HashDepth>,
}

struct HashDepth {
    hash: [u8; 32],
    depth: usize,
}

impl SequentialTreeHash {
    pub fn new() -> Self {
        SequentialTreeHash {
            stack: Vec::new(),
        }
    }

    pub fn insert(&mut self, hash: [u8; 32]) {
        let mut current = HashDepth { hash, depth: 0 };
        while let Some(top) = self.stack.last() {
            if top.depth == current.depth {
                let left = self.stack.pop().unwrap();
                let mut hasher = Sha256::new();
                hasher.update(left.hash);
                hasher.update(current.hash);
                current = HashDepth {
                    hash: hasher.finalize().into(),
                    depth: left.depth + 1,
                };
            } else {
                break;
            }
        }
        self.stack.push(current);
    }

    pub fn finalize(mut self) -> [u8; 32] {
        while self.stack.len() > 1 {
            let right = self.stack.pop().unwrap();
            let left = self.stack.pop().unwrap();
            let mut hasher = Sha256::new();
            hasher.update(left.hash);
            hasher.update(right.hash);
            self.stack.push(HashDepth {
                hash: hasher.finalize().into(),
                depth: left.depth + 1,
            });
        }
        self.stack.pop().unwrap().hash
    }
}

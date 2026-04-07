// Transposition table

use crate::board::Move;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NodeType {
    Exact,
    LowerBound,
    UpperBound,
}

#[derive(Clone, Copy, Debug)]
pub struct TTEntry {
    pub key: u64,
    pub best_move: Option<Move>,
    pub score: i32,
    pub depth: i32,
    pub node_type: NodeType,
    pub age: u8,
}

impl TTEntry {
    /// An empty/default entry with key 0.
    fn empty() -> Self {
        TTEntry {
            key: 0,
            best_move: None,
            score: 0,
            depth: 0,
            node_type: NodeType::Exact,
            age: 0,
        }
    }
}

pub struct TranspositionTable {
    entries: Vec<TTEntry>,
    size: usize,
    generation: u8,
}

/// Entry size for capacity calculation (24 bytes per entry).
const ENTRY_SIZE: usize = 24;

impl TranspositionTable {
    /// Create a new transposition table with the given size in megabytes.
    pub fn new(size_mb: usize) -> Self {
        let num_entries = (size_mb * 1024 * 1024) / ENTRY_SIZE;
        let num_entries = num_entries.max(1); // at least 1 entry
        TranspositionTable {
            entries: vec![TTEntry::empty(); num_entries],
            size: num_entries,
            generation: 0,
        }
    }

    /// Probe the table for an entry matching the given hash.
    /// Returns `Some(&TTEntry)` if found, `None` otherwise.
    pub fn probe(&self, hash: u64) -> Option<&TTEntry> {
        let index = (hash as usize) % self.size;
        let entry = &self.entries[index];
        if entry.key == hash && entry.key != 0 {
            Some(entry)
        } else {
            None
        }
    }

    /// Store an entry in the table using the replacement policy:
    /// - Empty slot (key == 0): always store
    /// - New entry from current generation, existing from older generation: replace regardless of depth
    /// - Same generation: replace if new entry has >= depth
    /// - Otherwise: don't replace
    pub fn store(&mut self, hash: u64, entry: TTEntry) {
        let index = (hash as usize) % self.size;
        let existing = &self.entries[index];

        let should_replace = if existing.key == 0 {
            // Empty slot
            true
        } else if entry.age == self.generation && existing.age != self.generation {
            // New entry is current generation, existing is older: always replace
            true
        } else if entry.age == existing.age {
            // Same generation: replace if new depth >= existing depth
            entry.depth >= existing.depth
        } else {
            // New entry is older generation than existing: don't replace
            false
        };

        if should_replace {
            self.entries[index] = entry;
        }
    }

    /// Clear all entries in the table.
    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = TTEntry::empty();
        }
    }

    /// Increment the generation counter (wraps around at u8::MAX).
    pub fn new_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// Resize the table to the given size in megabytes.
    pub fn resize(&mut self, size_mb: usize) {
        let num_entries = (size_mb * 1024 * 1024) / ENTRY_SIZE;
        let num_entries = num_entries.max(1);
        self.entries = vec![TTEntry::empty(); num_entries];
        self.size = num_entries;
        self.generation = 0;
    }

    /// Returns the current generation value.
    pub fn generation(&self) -> u8 {
        self.generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{MoveFlags, Piece};

    fn make_test_move() -> Move {
        Move::new(12, 28, Piece::Pawn, None, None, MoveFlags::DOUBLE_PUSH)
    }

    fn make_entry(key: u64, depth: i32, age: u8) -> TTEntry {
        TTEntry {
            key,
            best_move: Some(make_test_move()),
            score: 42,
            depth,
            node_type: NodeType::Exact,
            age,
        }
    }

    #[test]
    fn store_and_probe_returns_entry() {
        let mut tt = TranspositionTable::new(1);
        let hash: u64 = 0xDEAD_BEEF_CAFE_BABE;
        let entry = make_entry(hash, 5, 0);
        tt.store(hash, entry);

        let probed = tt.probe(hash).expect("should find stored entry");
        assert_eq!(probed.key, hash);
        assert_eq!(probed.score, 42);
        assert_eq!(probed.depth, 5);
        assert_eq!(probed.node_type, NodeType::Exact);
        assert!(probed.best_move.is_some());
        let mv = probed.best_move.unwrap();
        assert_eq!(mv.from, 12);
        assert_eq!(mv.to, 28);
    }

    #[test]
    fn probe_wrong_hash_returns_none() {
        let mut tt = TranspositionTable::new(1);
        let hash: u64 = 0xDEAD_BEEF_CAFE_BABE;
        let entry = make_entry(hash, 5, 0);
        tt.store(hash, entry);

        let wrong_hash: u64 = 0x1234_5678_9ABC_DEF0;
        assert!(tt.probe(wrong_hash).is_none());
    }

    #[test]
    fn clear_empties_table() {
        let mut tt = TranspositionTable::new(1);
        let hash: u64 = 0xDEAD_BEEF_CAFE_BABE;
        let entry = make_entry(hash, 5, 0);
        tt.store(hash, entry);
        assert!(tt.probe(hash).is_some());

        tt.clear();
        assert!(tt.probe(hash).is_none());
    }

    #[test]
    fn new_generation_increments_counter() {
        let mut tt = TranspositionTable::new(1);
        assert_eq!(tt.generation(), 0);
        tt.new_generation();
        assert_eq!(tt.generation(), 1);
        tt.new_generation();
        assert_eq!(tt.generation(), 2);
    }

    #[test]
    fn newer_generation_replaces_older() {
        let mut tt = TranspositionTable::new(1);

        // Store an entry at generation 0 with high depth
        let hash: u64 = 0xAAAA_BBBB_CCCC_DDDD;
        let old_entry = TTEntry {
            key: hash,
            best_move: None,
            score: 100,
            depth: 20,
            node_type: NodeType::Exact,
            age: 0,
        };
        tt.store(hash, old_entry);

        // Advance generation
        tt.new_generation();

        // Store a new entry at generation 1 with lower depth — should still replace
        // because current generation replaces older generation regardless of depth.
        // We need to compute the same index, so use same hash.
        let new_entry = TTEntry {
            key: hash,
            best_move: None,
            score: 200,
            depth: 1,
            node_type: NodeType::LowerBound,
            age: tt.generation(),
        };
        tt.store(hash, new_entry);

        let probed = tt.probe(hash).expect("should find entry");
        assert_eq!(probed.score, 200);
        assert_eq!(probed.depth, 1);
        assert_eq!(probed.node_type, NodeType::LowerBound);
    }

    #[test]
    fn same_generation_deeper_replaces_shallower() {
        let mut tt = TranspositionTable::new(1);
        let gen = tt.generation();

        let hash: u64 = 0x1111_2222_3333_4444;

        // Store shallow entry
        let shallow = TTEntry {
            key: hash,
            best_move: None,
            score: 50,
            depth: 3,
            node_type: NodeType::Exact,
            age: gen,
        };
        tt.store(hash, shallow);

        // Store deeper entry at same generation — should replace
        let deeper = TTEntry {
            key: hash,
            best_move: None,
            score: 150,
            depth: 8,
            node_type: NodeType::UpperBound,
            age: gen,
        };
        tt.store(hash, deeper);

        let probed = tt.probe(hash).expect("should find entry");
        assert_eq!(probed.score, 150);
        assert_eq!(probed.depth, 8);
        assert_eq!(probed.node_type, NodeType::UpperBound);
    }

    #[test]
    fn same_generation_shallower_does_not_replace_deeper() {
        let mut tt = TranspositionTable::new(1);
        let gen = tt.generation();

        let hash: u64 = 0x5555_6666_7777_8888;

        // Store deep entry
        let deep = TTEntry {
            key: hash,
            best_move: None,
            score: 300,
            depth: 10,
            node_type: NodeType::Exact,
            age: gen,
        };
        tt.store(hash, deep);

        // Try to store shallower entry at same generation — should NOT replace
        let shallow = TTEntry {
            key: hash,
            best_move: None,
            score: 50,
            depth: 2,
            node_type: NodeType::LowerBound,
            age: gen,
        };
        tt.store(hash, shallow);

        let probed = tt.probe(hash).expect("should find entry");
        assert_eq!(probed.score, 300);
        assert_eq!(probed.depth, 10);
    }

    #[test]
    fn resize_clears_and_changes_capacity() {
        let mut tt = TranspositionTable::new(1);
        let hash: u64 = 0xDEAD_BEEF_CAFE_BABE;
        let entry = make_entry(hash, 5, 0);
        tt.store(hash, entry);
        assert!(tt.probe(hash).is_some());

        tt.resize(2);
        // After resize, old entries are gone
        assert!(tt.probe(hash).is_none());
        // New capacity should be based on 2 MB
        let expected = (2 * 1024 * 1024) / ENTRY_SIZE;
        assert_eq!(tt.size, expected);
    }
}

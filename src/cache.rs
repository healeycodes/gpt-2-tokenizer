use std::num::NonZeroUsize;

use lru::LruCache;

// Matches encoder.py's shape, with a production-sized LRU cap.
pub struct BpeCache(LruCache<String, String>);

impl BpeCache {
    pub fn get(&mut self, piece: &str) -> Option<String> {
        self.0.get(piece).cloned()
    }

    pub fn put(&mut self, piece: String, merged: String) {
        self.0.put(piece, merged);
    }
}

impl Default for BpeCache {
    fn default() -> Self {
        Self(LruCache::new(NonZeroUsize::new(256).unwrap()))
    }
}

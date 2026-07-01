use lightframe_core::media::ThumbnailSize;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

pub struct ThumbCache {
    micro: Mutex<LruCache<i64, Vec<u8>>>,
    standard: Mutex<LruCache<(i64, ThumbnailSize), Vec<u8>>>,
}

impl ThumbCache {
    pub fn new() -> Self {
        Self::with_capacity(2000, 500)
    }

    pub fn with_capacity(micro_cap: usize, standard_cap: usize) -> Self {
        Self {
            micro: Mutex::new(LruCache::new(
                NonZeroUsize::new(micro_cap.max(1)).expect("micro cache capacity"),
            )),
            standard: Mutex::new(LruCache::new(
                NonZeroUsize::new(standard_cap.max(1)).expect("standard cache capacity"),
            )),
        }
    }

    pub fn get(&self, media_id: i64, size: ThumbnailSize) -> Option<Vec<u8>> {
        match size {
            ThumbnailSize::Micro => self
                .micro
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&media_id)
                .cloned(),
            ThumbnailSize::Small | ThumbnailSize::Large => self
                .standard
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&(media_id, size))
                .cloned(),
        }
    }

    pub fn insert(&self, media_id: i64, size: ThumbnailSize, bytes: Vec<u8>) {
        match size {
            ThumbnailSize::Micro => {
                self.micro
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .put(media_id, bytes);
            }
            ThumbnailSize::Small | ThumbnailSize::Large => {
                self.standard
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .put((media_id, size), bytes);
            }
        }
    }

    pub fn invalidate_media(&self, media_id: i64) {
        {
            let mut micro = self.micro.lock().unwrap_or_else(|e| e.into_inner());
            micro.pop(&media_id);
        }
        {
            let mut standard = self.standard.lock().unwrap_or_else(|e| e.into_inner());
            standard.pop(&(media_id, ThumbnailSize::Small));
            standard.pop(&(media_id, ThumbnailSize::Large));
        }
    }

    /// Resize the LRU caches. Entries beyond the new capacity are evicted (LRU order).
    pub fn resize(&self, micro_cap: usize, standard_cap: usize) {
        let micro_cap = NonZeroUsize::new(micro_cap.max(1)).unwrap();
        let standard_cap = NonZeroUsize::new(standard_cap.max(1)).unwrap();
        {
            let mut micro = self.micro.lock().unwrap_or_else(|e| e.into_inner());
            micro.resize(micro_cap);
        }
        {
            let mut standard = self.standard.lock().unwrap_or_else(|e| e.into_inner());
            standard.resize(standard_cap);
        }
    }

    /// Current number of entries in both caches.
    pub fn len(&self) -> (usize, usize) {
        let micro_len = self.micro.lock().unwrap_or_else(|e| e.into_inner()).len();
        let standard_len = self
            .standard
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len();
        (micro_len, standard_len)
    }

    /// Current LRU capacity limits for both caches.
    pub fn capacity(&self) -> (usize, usize) {
        let micro_cap = self
            .micro
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .cap()
            .get();
        let standard_cap = self
            .standard
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .cap()
            .get();
        (micro_cap, standard_cap)
    }
}

impl Default for ThumbCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_retrieves_micro_thumbnails() {
        let cache = ThumbCache::new();
        cache.insert(1, ThumbnailSize::Micro, vec![1, 2, 3]);
        assert_eq!(cache.get(1, ThumbnailSize::Micro), Some(vec![1, 2, 3]));
        assert_eq!(cache.get(1, ThumbnailSize::Small), None);
    }

    #[test]
    fn stores_and_retrieves_standard_thumbnails() {
        let cache = ThumbCache::new();
        cache.insert(2, ThumbnailSize::Small, vec![4, 5]);
        cache.insert(2, ThumbnailSize::Large, vec![6, 7, 8]);
        assert_eq!(cache.get(2, ThumbnailSize::Small), Some(vec![4, 5]));
        assert_eq!(cache.get(2, ThumbnailSize::Large), Some(vec![6, 7, 8]));
    }

    #[test]
    fn invalidate_media_removes_all_sizes() {
        let cache = ThumbCache::new();
        cache.insert(3, ThumbnailSize::Micro, vec![1]);
        cache.insert(3, ThumbnailSize::Small, vec![2]);
        cache.insert(3, ThumbnailSize::Large, vec![3]);
        cache.invalidate_media(3);
        assert_eq!(cache.get(3, ThumbnailSize::Micro), None);
        assert_eq!(cache.get(3, ThumbnailSize::Small), None);
        assert_eq!(cache.get(3, ThumbnailSize::Large), None);
    }

    #[test]
    fn get_returns_none_for_missing_entries() {
        let cache = ThumbCache::new();
        assert_eq!(cache.get(99, ThumbnailSize::Micro), None);
        assert_eq!(cache.get(99, ThumbnailSize::Small), None);
    }

    #[test]
    fn insert_overwrites_existing_thumbnail() {
        let cache = ThumbCache::new();
        cache.insert(5, ThumbnailSize::Small, vec![1, 2]);
        cache.insert(5, ThumbnailSize::Small, vec![9, 8, 7]);
        assert_eq!(cache.get(5, ThumbnailSize::Small), Some(vec![9, 8, 7]));
    }

    #[test]
    fn invalidate_does_not_remove_other_media() {
        let cache = ThumbCache::new();
        cache.insert(1, ThumbnailSize::Micro, vec![1]);
        cache.insert(2, ThumbnailSize::Micro, vec![2]);
        cache.invalidate_media(1);
        assert_eq!(cache.get(1, ThumbnailSize::Micro), None);
        assert_eq!(cache.get(2, ThumbnailSize::Micro), Some(vec![2]));
    }

    #[test]
    fn micro_and_standard_caches_are_independent() {
        let cache = ThumbCache::new();
        cache.insert(7, ThumbnailSize::Micro, vec![10]);
        cache.insert(7, ThumbnailSize::Small, vec![20]);
        cache.invalidate_media(7);
        assert_eq!(cache.get(7, ThumbnailSize::Micro), None);
        assert_eq!(cache.get(7, ThumbnailSize::Small), None);
    }

    #[test]
    fn with_capacity_creates_cache_with_specified_sizes() {
        let cache = ThumbCache::with_capacity(3, 2);
        for i in 0..5 {
            cache.insert(i, ThumbnailSize::Micro, vec![i as u8]);
        }
        // Only last 3 should remain (LRU eviction)
        assert_eq!(cache.get(0, ThumbnailSize::Micro), None);
        assert_eq!(cache.get(1, ThumbnailSize::Micro), None);
        assert_eq!(cache.get(2, ThumbnailSize::Micro), Some(vec![2]));
        assert_eq!(cache.get(3, ThumbnailSize::Micro), Some(vec![3]));
        assert_eq!(cache.get(4, ThumbnailSize::Micro), Some(vec![4]));
    }

    #[test]
    fn with_capacity_standard_evicts_beyond_limit() {
        let cache = ThumbCache::with_capacity(100, 2);
        cache.insert(1, ThumbnailSize::Small, vec![1]);
        cache.insert(2, ThumbnailSize::Small, vec![2]);
        cache.insert(3, ThumbnailSize::Small, vec![3]);
        // Only last 2 should remain
        assert_eq!(cache.get(1, ThumbnailSize::Small), None);
        assert_eq!(cache.get(2, ThumbnailSize::Small), Some(vec![2]));
        assert_eq!(cache.get(3, ThumbnailSize::Small), Some(vec![3]));
    }

    #[test]
    fn resize_shrinks_micro_cache_evicts_lru() {
        let cache = ThumbCache::with_capacity(10, 10);
        for i in 0..10 {
            cache.insert(i, ThumbnailSize::Micro, vec![i as u8]);
        }
        assert_eq!(cache.len().0, 10);

        // Shrink to 3 — oldest 7 entries evicted
        cache.resize(3, 10);
        assert_eq!(cache.len().0, 3);
        // LRU entries (0..7) should be gone
        for i in 0..7 {
            assert_eq!(cache.get(i, ThumbnailSize::Micro), None);
        }
        // Most recent (7, 8, 9) should survive
        assert_eq!(cache.get(7, ThumbnailSize::Micro), Some(vec![7]));
        assert_eq!(cache.get(8, ThumbnailSize::Micro), Some(vec![8]));
        assert_eq!(cache.get(9, ThumbnailSize::Micro), Some(vec![9]));
    }

    #[test]
    fn resize_shrinks_standard_cache_evicts_lru() {
        let cache = ThumbCache::with_capacity(10, 5);
        for i in 0..5 {
            cache.insert(i, ThumbnailSize::Small, vec![i as u8]);
        }
        assert_eq!(cache.len().1, 5);

        cache.resize(10, 2);
        assert_eq!(cache.len().1, 2);
        // Oldest 3 gone
        assert_eq!(cache.get(0, ThumbnailSize::Small), None);
        assert_eq!(cache.get(1, ThumbnailSize::Small), None);
        assert_eq!(cache.get(2, ThumbnailSize::Small), None);
        // Recent 2 survive
        assert_eq!(cache.get(3, ThumbnailSize::Small), Some(vec![3]));
        assert_eq!(cache.get(4, ThumbnailSize::Small), Some(vec![4]));
    }

    #[test]
    fn resize_grow_does_not_lose_entries() {
        let cache = ThumbCache::with_capacity(3, 3);
        cache.insert(1, ThumbnailSize::Micro, vec![1]);
        cache.insert(2, ThumbnailSize::Micro, vec![2]);
        cache.insert(3, ThumbnailSize::Micro, vec![3]);

        cache.resize(100, 100);
        assert_eq!(cache.get(1, ThumbnailSize::Micro), Some(vec![1]));
        assert_eq!(cache.get(2, ThumbnailSize::Micro), Some(vec![2]));
        assert_eq!(cache.get(3, ThumbnailSize::Micro), Some(vec![3]));
    }

    #[test]
    fn resize_to_one_keeps_most_recent_only() {
        let cache = ThumbCache::with_capacity(10, 10);
        cache.insert(1, ThumbnailSize::Micro, vec![1]);
        cache.insert(2, ThumbnailSize::Micro, vec![2]);
        cache.insert(3, ThumbnailSize::Micro, vec![3]);

        cache.resize(1, 1);
        assert_eq!(cache.len().0, 1);
        assert_eq!(cache.get(3, ThumbnailSize::Micro), Some(vec![3]));
        assert_eq!(cache.get(1, ThumbnailSize::Micro), None);
        assert_eq!(cache.get(2, ThumbnailSize::Micro), None);
    }

    #[test]
    fn resize_zero_cap_treated_as_one() {
        let cache = ThumbCache::with_capacity(10, 10);
        cache.insert(1, ThumbnailSize::Micro, vec![1]);
        cache.resize(0, 0);
        // Should not panic; at least 1 entry capacity
        assert!(cache.len().0 <= 1);
    }

    #[test]
    fn len_reports_correct_counts() {
        let cache = ThumbCache::with_capacity(100, 100);
        assert_eq!(cache.len(), (0, 0));
        cache.insert(1, ThumbnailSize::Micro, vec![1]);
        cache.insert(2, ThumbnailSize::Micro, vec![2]);
        cache.insert(1, ThumbnailSize::Small, vec![3]);
        assert_eq!(cache.len(), (2, 1));
    }
}

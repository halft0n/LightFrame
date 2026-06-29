use lightframe_core::media::ThumbnailSize;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

const MICRO_CAPACITY: usize = 2000;
const STANDARD_CAPACITY: usize = 500;

pub struct ThumbCache {
    micro: Mutex<LruCache<i64, Vec<u8>>>,
    standard: Mutex<LruCache<(i64, ThumbnailSize), Vec<u8>>>,
}

impl ThumbCache {
    pub fn new() -> Self {
        Self {
            micro: Mutex::new(LruCache::new(
                NonZeroUsize::new(MICRO_CAPACITY).expect("micro cache capacity"),
            )),
            standard: Mutex::new(LruCache::new(
                NonZeroUsize::new(STANDARD_CAPACITY).expect("standard cache capacity"),
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
}

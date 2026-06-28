use catchlight_core::media::ThumbnailSize;
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
                .expect("micro thumb cache mutex poisoned")
                .get(&media_id)
                .cloned(),
            ThumbnailSize::Small | ThumbnailSize::Large => self
                .standard
                .lock()
                .expect("standard thumb cache mutex poisoned")
                .get(&(media_id, size))
                .cloned(),
        }
    }

    pub fn insert(&self, media_id: i64, size: ThumbnailSize, bytes: Vec<u8>) {
        match size {
            ThumbnailSize::Micro => {
                self.micro
                    .lock()
                    .expect("micro thumb cache mutex poisoned")
                    .put(media_id, bytes);
            }
            ThumbnailSize::Small | ThumbnailSize::Large => {
                self.standard
                    .lock()
                    .expect("standard thumb cache mutex poisoned")
                    .put((media_id, size), bytes);
            }
        }
    }

    pub fn invalidate_media(&self, media_id: i64) {
        {
            let mut micro = self.micro.lock().expect("micro thumb cache mutex poisoned");
            micro.pop(&media_id);
        }
        {
            let mut standard = self
                .standard
                .lock()
                .expect("standard thumb cache mutex poisoned");
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
}

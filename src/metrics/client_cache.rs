use moka::sync::Cache;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct ClientCache {
    cache1: Option<Arc<Cache<IpAddr, u64>>>,
    cache2: Option<Arc<Cache<IpAddr, u64>>>,
    cache3: Option<Arc<Cache<IpAddr, u64>>>,
}

impl ClientCache {
    pub fn new(limit1: u64, limit2: u64, limit3: u64) -> Self {
        Self::new_with_ttls(limit1, limit2, limit3, 60, 3600, 86400)
    }

    pub fn new_with_ttls(limit1: u64, limit2: u64, limit3: u64, ttl1: u64, ttl2: u64, ttl3: u64) -> Self {
        let cache1 = if limit1 > 0 {
            Some(Arc::new(Cache::builder()
                .max_capacity(limit1)
                .time_to_live(Duration::from_secs(ttl1))
                .build()))
        } else {
            None
        };

        let cache2 = if limit2 > 0 {
            Some(Arc::new(Cache::builder()
                .max_capacity(limit2)
                .time_to_live(Duration::from_secs(ttl2))
                .build()))
        } else {
            None
        };

        let cache3 = if limit3 > 0 {
            Some(Arc::new(Cache::builder()
                .max_capacity(limit3)
                .time_to_live(Duration::from_secs(ttl3))
                .build()))
        } else {
            None
        };

        Self {
            cache1,
            cache2,
            cache3,
        }
    }

    pub fn inc_client(&self, ip: IpAddr) -> (u64, u64, u64) {
        let count1 = if let Some(cache) = &self.cache1 {
            let count = cache.get(&ip).unwrap_or(0) + 1;
            cache.insert(ip, count);
            count
        } else {
            0
        };

        let count2 = if let Some(cache) = &self.cache2 {
            let count = cache.get(&ip).unwrap_or(0) + 1;
            cache.insert(ip, count);
            count
        } else {
            0
        };

        let count3 = if let Some(cache) = &self.cache3 {
            let count = cache.get(&ip).unwrap_or(0) + 1;
            cache.insert(ip, count);
            count
        } else {
            0
        };

        (count1, count2, count3)
    }

    pub fn get_counts(&self) -> (u64, u64, u64) {
        (
            self.cache1.as_ref().map_or(0, |c| c.entry_count()),
            self.cache2.as_ref().map_or(0, |c| c.entry_count()),
            self.cache3.as_ref().map_or(0, |c| c.entry_count()),
        )
    }
}

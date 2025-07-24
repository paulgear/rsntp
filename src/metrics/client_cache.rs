use moka::sync::Cache;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct ClientCache {
    cache1: Arc<Cache<IpAddr, u64>>,
    cache2: Arc<Cache<IpAddr, u64>>,
    cache3: Arc<Cache<IpAddr, u64>>,
}

impl ClientCache {
    pub fn new(limit1: u64, limit2: u64, limit3: u64) -> Self {
        Self::new_with_ttls(limit1, limit2, limit3, 60, 3600, 86400)
    }

    pub fn new_with_ttls(limit1: u64, limit2: u64, limit3: u64, ttl1: u64, ttl2: u64, ttl3: u64) -> Self {
        let cache1 = Cache::builder()
            .max_capacity(limit1)
            .time_to_live(Duration::from_secs(ttl1))
            .build();

        let cache2 = Cache::builder()
            .max_capacity(limit2)
            .time_to_live(Duration::from_secs(ttl2))
            .build();

        let cache3 = Cache::builder()
            .max_capacity(limit3)
            .time_to_live(Duration::from_secs(ttl3))
            .build();

        Self {
            cache1: Arc::new(cache1),
            cache2: Arc::new(cache2),
            cache3: Arc::new(cache3),
        }
    }

    pub fn inc_client(&self, ip: IpAddr) -> (u64, u64, u64) {
        let count1 = self.cache1.get(&ip).unwrap_or(0) + 1;
        let count2 = self.cache2.get(&ip).unwrap_or(0) + 1;
        let count3 = self.cache3.get(&ip).unwrap_or(0) + 1;

        self.cache1.insert(ip, count1);
        self.cache2.insert(ip, count2);
        self.cache3.insert(ip, count3);

        (count1, count2, count3)
    }

    pub fn get_counts(&self) -> (u64, u64, u64) {
        (
            self.cache1.entry_count(),
            self.cache2.entry_count(),
            self.cache3.entry_count(),
        )
    }
}

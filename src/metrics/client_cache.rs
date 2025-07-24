use moka::sync::Cache;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct ClientCache {
    caches: Vec<Option<Arc<Cache<IpAddr, u64>>>>,
}

impl ClientCache {
    pub fn new(configs: &[(u64, u64)]) -> Self {
        let caches = configs
            .iter()
            .map(|&(limit, ttl)| Self::create_cache(limit, ttl))
            .collect();

        Self { caches }
    }

    fn create_cache(limit: u64, ttl: u64) -> Option<Arc<Cache<IpAddr, u64>>> {
        if limit > 0 {
            Some(Arc::new(
                Cache::builder()
                    .max_capacity(limit)
                    .time_to_live(Duration::from_secs(ttl))
                    .build(),
            ))
        } else {
            None
        }
    }

    pub fn get_client(&self, ip: IpAddr) -> Vec<u64> {
        self.caches
            .iter()
            .map(|cache| cache.as_ref().map_or(0, |c| c.get(&ip).unwrap_or(0)))
            .collect()
    }

    pub fn get_counts(&self) -> Vec<u64> {
        self.caches
            .iter()
            .map(|cache| cache.as_ref().map_or(0, |c| c.entry_count()))
            .collect()
    }

    fn increment_cache(cache: &Option<Arc<Cache<IpAddr, u64>>>, ip: IpAddr) -> u64 {
        cache.as_ref().map_or(0, |c| {
            let count = c.get(&ip).unwrap_or(0) + 1;
            c.insert(ip, count);
            count
        })
    }

    pub fn inc_client(&self, ip: IpAddr) -> Vec<u64> {
        self.caches
            .iter()
            .map(|cache| Self::increment_cache(cache, ip))
            .collect()
    }

}

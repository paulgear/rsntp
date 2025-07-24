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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_inc_and_get_client() {
        let cache = ClientCache::new(&[(10, 60)]);
        let ip = IpAddr::from_str("192.0.2.1").unwrap();

        for i in 1..=10 {
            let result = cache.inc_client(ip);
            assert_eq!(result.len(), 1);
            assert_eq!(result[0], i);
        }
    }

    #[test]
    fn test_cache_limit() {
        let cache = ClientCache::new(&[(10, 60)]);

        // Add 20 clients with contiguous IP addresses
        for i in 1..=20 {
            let ip = IpAddr::from_str(&format!("192.0.2.{}", i)).unwrap();
            cache.inc_client(ip);
        }

        // Wait for cache eviction
        std::thread::sleep(std::time::Duration::from_millis(1000));

        // Check that the cache has at most 10 entries
        let counts = cache.get_counts();
        assert_eq!(counts.len(), 1);
        assert!(counts[0] <= 10, "Cache should have at most 10 entries, got {}", counts[0]);

        // Verify that some entries were evicted by checking total unique entries
        let mut present_count = 0;
        for i in 1..=20 {
            let ip = IpAddr::from_str(&format!("192.0.2.{}", i)).unwrap();
            let result = cache.get_client(ip);
            if result[0] > 0 {
                present_count += 1;
            }
        }

        assert!(present_count <= 10, "Should have at most 10 present entries, got {}", present_count);
        assert!(present_count > 0, "Should have some entries present");
    }
}

use moka::sync::Cache;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct ClientCache {
    caches: Vec<Option<Arc<Cache<IpAddr, u64>>>>,
    ttls: Vec<u64>,
}

impl ClientCache {
    pub fn new(configs: &[(u64, u64)]) -> Self {
        // Sort caches by descending TTL so that we can use the keys in the first one
        // as the definitive list of clients.
        let mut sorted_configs = configs.to_vec();
        sorted_configs.sort_by(|a, b| b.1.cmp(&a.1));

        let caches = sorted_configs
            .iter()
            .map(|&(limit, ttl)| Self::create_cache(limit, ttl))
            .collect();

        let ttls = sorted_configs.iter().map(|&(_, ttl)| ttl).collect();

        Self { caches, ttls }
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

    pub fn get_clients(&self) -> Vec<(IpAddr, u64)> {
        self.caches
            .iter()
            .filter_map(|cache| cache.as_ref())
            .flat_map(|cache| cache.iter().map(|(ip, count)| (*ip, count)))
            .collect()
    }

    pub fn get_counts(&self) -> Vec<u64> {
        self.caches
            .iter()
            .map(|cache| cache.as_ref().map_or(0, |c| c.entry_count()))
            .collect()
    }

    fn get_ttls(&self) -> &[u64] {
        &self.ttls
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

    fn run_pending_tasks(&self) {
        self.caches
            .iter()
            .filter_map(|cache| cache.as_ref())
            .for_each(|cache| cache.run_pending_tasks());
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
    fn test_multi_cache() {
        let cache = ClientCache::new(&[(10, 2), (10, 10), (10, 1)]);

        // Confirm that the caches are sorted by descending TTL
        assert_eq!(cache.get_ttls(), &[10, 2, 1]);

        // Increment a client's counter a few times and make sure it returns the
        // same count for each cache.
        let ip = IpAddr::from_str("192.0.2.1").unwrap();
        for i in 1..=10 {
            let result = cache.inc_client(ip);
            assert_eq!(result.len(), 3);
            let result = cache.get_client(ip);
            assert_eq!(result[0], i);
            assert_eq!(result[1], i);
            assert_eq!(result[2], i);
        }

        // Wait for the 3rd cache to expire and make sure the client is no longer
        // present in the 3rd cache but present in the others.
        std::thread::sleep(std::time::Duration::from_millis(1_001));
        let result = cache.get_client(ip);
        assert_eq!(result[0], 10);
        assert_eq!(result[1], 10);
        assert_eq!(result[2], 0);

        // Get the list of clients and make sure there are only 2 entries, and the IP
        // is present in them.
        let result = cache.get_clients();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, ip);
        assert_eq!(result[1].0, ip);

        // Wait for the 2nd cache to expire and make sure the client is no longer
        // present in the 2nd cache but present in the others.
        std::thread::sleep(std::time::Duration::from_millis(1_001));
        let result = cache.get_client(ip);
        assert_eq!(result[0], 10);
        assert_eq!(result[1], 0);
        assert_eq!(result[2], 0);
    }

    // WARNING: This test seems rather timing sensitive.  The sleep durations have been
    // chosen to try to favour the second set of IP addresses, but it may not always work.
    #[test]
    fn test_cache_limit() {
        let cache = ClientCache::new(&[(10, 60)]);

        // Add 10 clients with contiguous IP addresses
        for i in 1..=10 {
            let ip = IpAddr::from_str(&format!("192.0.2.{}", i)).unwrap();
            cache.inc_client(ip);
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        // Add 10 more clients with contiguous IP addresses
        for i in 11..=20 {
            let ip = IpAddr::from_str(&format!("192.0.2.{}", i)).unwrap();
            cache.inc_client(ip);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        // Check that the last 10 IPs are present
        for i in 11..=20 {
            let ip = IpAddr::from_str(&format!("192.0.2.{}", i)).unwrap();
            let result = cache.get_client(ip);
            assert_eq!(result[0], 1);
        }

        // Increment those again
        for i in 11..=20 {
            let ip = IpAddr::from_str(&format!("192.0.2.{}", i)).unwrap();
            cache.inc_client(ip);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Force cache eviction
        cache.run_pending_tasks();

        // Display all clients
        let clients = cache.get_clients();
        println!("Cached clients: {:?}", clients);

        // Check that the cache has at most 10 entries
        let counts = cache.get_counts();
        assert_eq!(counts.len(), 1);
        assert!(counts[0] <= 10, "Cache should have at most 10 entries, got {}", counts[0]);

        // Check that the last 10 IPs are present (192.0.2.11 to 192.0.2.20)
        for i in 11..=20 {
            let ip = IpAddr::from_str(&format!("192.0.2.{}", i)).unwrap();
            let result = cache.get_client(ip);
            assert!(result[0] > 0, "Cache for {} should be > 0, got {}", ip, result[0]);
        }

        // Check that the first 10 IPs are evicted (192.0.2.1 to 192.0.2.10)
        for i in 1..=10 {
            let ip = IpAddr::from_str(&format!("192.0.2.{}", i)).unwrap();
            let result = cache.get_client(ip);
            assert_eq!(result[0], 0);
        }
    }
}

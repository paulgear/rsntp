use moka::future::Cache;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct ClientCache {
    minute_cache: Arc<Cache<IpAddr, ()>>,
    hour_cache: Arc<Cache<IpAddr, ()>>,
    day_cache: Arc<Cache<IpAddr, ()>>,
}

impl ClientCache {
    pub fn new(minute_limit: u64, hour_limit: u64, day_limit: u64) -> Self {
        let minute_cache = Cache::builder()
            .max_capacity(minute_limit)
            .time_to_live(Duration::from_secs(60))
            .build();
            
        let hour_cache = Cache::builder()
            .max_capacity(hour_limit)
            .time_to_live(Duration::from_secs(3600))
            .build();
            
        let day_cache = Cache::builder()
            .max_capacity(day_limit)
            .time_to_live(Duration::from_secs(86400))
            .build();
        
        Self {
            minute_cache: Arc::new(minute_cache),
            hour_cache: Arc::new(hour_cache),
            day_cache: Arc::new(day_cache),
        }
    }
    
    pub async fn add_client(&self, ip: IpAddr) -> Result<(bool, bool, bool), Box<dyn std::error::Error + Send + Sync>> {
        let minute_new = !self.minute_cache.contains_key(&ip);
        let hour_new = !self.hour_cache.contains_key(&ip);
        let day_new = !self.day_cache.contains_key(&ip);
        
        if minute_new {
            self.minute_cache.insert(ip, ()).await;
        }
        if hour_new {
            self.hour_cache.insert(ip, ()).await;
        }
        if day_new {
            self.day_cache.insert(ip, ()).await;
        }
        
        Ok((minute_new, hour_new, day_new))
    }
    
    pub fn get_counts(&self) -> (u64, u64, u64) {
        (
            self.minute_cache.entry_count(),
            self.hour_cache.entry_count(),
            self.day_cache.entry_count(),
        )
    }
}

use crate::error::RelayError;
use ech_board_common::RelayDragonflyConfig;
use fred::prelude::*;
use fred::types::Message;
use futures::stream::TryStreamExt;
use moka::sync::Cache as MokaCache;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const L1_MAX_BYTES: u64 = 64 * 1024 * 1024;
const L2_POOL_SIZE: usize = 4;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Invalidation {
    pub(crate) flush: bool,
    pub(crate) scopes: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct Cache {
    pool: Pool,
    subscriber: Client,
    l1: L1Cache,
    channel: String,
    healthy: Arc<AtomicBool>,
}

#[derive(Clone)]
struct L1Cache {
    cache: MokaCache<String, Arc<Vec<u8>>>,
}

impl L1Cache {
    fn new(max_bytes: u64) -> Self {
        let cache = MokaCache::builder()
            .weigher(|key: &String, val: &Arc<Vec<u8>>| (key.len() + val.len()) as u32)
            .max_capacity(max_bytes)
            .build();
        Self { cache }
    }

    fn get(&self, key: &str) -> Option<Arc<Vec<u8>>> {
        self.cache.get(key)
    }

    fn put(&self, key: String, val: Vec<u8>) {
        self.cache.insert(key, Arc::new(val));
    }

    fn invalidate_prefix(&self, prefix: &str) -> usize {
        let keys: Vec<String> = self
            .cache
            .iter()
            .filter(|(key, _)| key.starts_with(prefix))
            .map(|(key, _)| key.to_string())
            .collect();
        for key in &keys {
            self.cache.invalidate(key);
        }
        keys.len()
    }

    fn flush(&self) {
        self.cache.invalidate_all();
    }
}

fn cache_err(err: fred::error::Error) -> RelayError {
    RelayError::Internal(format!("dragonfly cache: {err}"))
}

impl Cache {
    pub(crate) async fn new(cfg: &RelayDragonflyConfig) -> Result<Self, RelayError> {
        let config = Config::from_url(&cfg.url)
            .map_err(|e| RelayError::ConfigInvalid(format!("dragonfly url: {e}")))?;
        let pool = Builder::from_config(config.clone())
            .build_pool(L2_POOL_SIZE)
            .map_err(|e| RelayError::ConfigInvalid(format!("dragonfly pool: {e}")))?;
        let subscriber = Builder::from_config(config)
            .build()
            .map_err(|e| RelayError::ConfigInvalid(format!("dragonfly subscriber: {e}")))?;
        pool.init()
            .await
            .map_err(|e| RelayError::ConfigInvalid(format!("dragonfly pool init: {e}")))?;
        subscriber
            .init()
            .await
            .map_err(|e| RelayError::ConfigInvalid(format!("dragonfly subscriber init: {e}")))?;
        Ok(Self {
            pool,
            subscriber,
            l1: L1Cache::new(L1_MAX_BYTES),
            channel: cfg.channel.clone(),
            healthy: Arc::new(AtomicBool::new(true)),
        })
    }

    fn healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }

    fn mark_ok(&self) {
        self.healthy.store(true, Ordering::Relaxed);
    }

    fn mark_err(&self) {
        self.healthy.store(false, Ordering::Relaxed);
    }

    pub(crate) async fn start_listener(&self) -> Result<(), RelayError> {
        let reconnect_cache = self.clone();
        let _reconnect = self.subscriber.on_reconnect(move |_| {
            let cache = reconnect_cache.clone();
            async move {
                cache.l1_flush();
                if let Err(e) = cache.l2_flush().await {
                    eprintln!("cache flush on reconnect: {e}");
                } else {
                    cache.mark_ok();
                }
                Ok::<(), fred::error::Error>(())
            }
        });
        let msg_l1 = self.l1.clone();
        let msg_channel = self.channel.clone();
        let _messages = self.subscriber.on_message(move |message: Message| {
            let l1 = msg_l1.clone();
            let channel = msg_channel.clone();
            async move {
                if message.channel.as_bytes() == channel.as_bytes() {
                    let payload: &[u8] = match &message.value {
                        Value::Bytes(bytes) => bytes,
                        Value::String(s) => s.as_bytes(),
                        _ => &[],
                    };
                    if !payload.is_empty() {
                        if let Ok(inv) = serde_json::from_slice::<Invalidation>(payload) {
                            if inv.flush {
                                l1.flush();
                            } else {
                                for scope in &inv.scopes {
                                    l1.invalidate_prefix(scope);
                                }
                            }
                        }
                    }
                }
                Ok::<(), fred::error::Error>(())
            }
        });
        let _: () = self
            .subscriber
            .subscribe(self.channel.clone())
            .await
            .map_err(cache_err)?;
        Ok(())
    }

    pub(crate) async fn publish(&self, inv: &Invalidation) -> Result<(), RelayError> {
        let payload = serde_json::to_vec(inv)
            .map_err(|e| RelayError::Internal(format!("invalidation encode: {e}")))?;
        let _: Value = self
            .subscriber
            .publish(self.channel.clone(), payload)
            .await
            .map_err(cache_err)?;
        Ok(())
    }

    pub(crate) async fn l2_get(&self, key: &str) -> Result<Option<Vec<u8>>, RelayError> {
        let val: Option<Vec<u8>> = self.pool.get(key).await.map_err(cache_err)?;
        Ok(val)
    }

    pub(crate) async fn l2_put(&self, key: &str, val: &[u8]) -> Result<(), RelayError> {
        let _: () = self
            .pool
            .set(key, val.to_vec(), None, None, false)
            .await
            .map_err(cache_err)?;
        Ok(())
    }

    pub(crate) async fn l2_del(&self, keys: &[String]) -> Result<(), RelayError> {
        if keys.is_empty() {
            return Ok(());
        }
        let _: () = self.pool.del(keys.to_vec()).await.map_err(cache_err)?;
        Ok(())
    }

    pub(crate) async fn l2_incr(&self, key: &str) -> Result<u64, RelayError> {
        let val: u64 = self.pool.incr(key).await.map_err(cache_err)?;
        Ok(val)
    }

    pub(crate) async fn l2_pattern_del(&self, pattern: &str) -> Result<(), RelayError> {
        let keys: Vec<String> = self
            .pool
            .next()
            .scan_buffered(pattern, Some(100), None)
            .map_ok(|key| key.as_str_lossy().into_owned())
            .try_collect()
            .await
            .map_err(cache_err)?;
        self.l2_del(&keys).await
    }

    pub(crate) async fn l2_flush(&self) -> Result<(), RelayError> {
        self.l2_pattern_del("v:*").await?;
        self.l2_pattern_del("gen:*").await?;
        Ok(())
    }

    pub(crate) async fn gen_get(&self, key: &str) -> u64 {
        match self.pool.get::<Option<u64>, _>(key).await {
            Ok(val) => {
                self.mark_ok();
                val.unwrap_or(0)
            }
            Err(e) => {
                self.mark_err();
                eprintln!("cache gen {key}: {e}");
                0
            }
        }
    }

    pub(crate) async fn peek(&self, key: &str) -> Option<Vec<u8>> {
        if self.healthy() {
            if let Some(v) = self.l1.get(key) {
                return Some(v.to_vec());
            }
        }
        match self.l2_get(key).await {
            Ok(Some(v)) => {
                self.mark_ok();
                self.l1.put(key.to_string(), v.clone());
                Some(v)
            }
            Ok(None) => {
                self.mark_ok();
                None
            }
            Err(e) => {
                self.mark_err();
                eprintln!("cache get {key}: {e}");
                None
            }
        }
    }

    pub(crate) async fn store(&self, key: String, val: &[u8]) {
        if !self.healthy() {
            return;
        }
        self.l1.put(key.clone(), val.to_vec());
        match self.l2_put(&key, val).await {
            Ok(()) => self.mark_ok(),
            Err(e) => {
                self.mark_err();
                eprintln!("cache put {key}: {e}");
            }
        }
    }

    pub(crate) async fn get_or_build<F>(&self, key: String, build: F) -> Result<Vec<u8>, RelayError>
    where
        F: Future<Output = Result<Vec<u8>, RelayError>> + Send,
    {
        if let Some(v) = self.l1.get(&key) {
            return Ok(v.to_vec());
        }
        match self.l2_get(&key).await {
            Ok(Some(v)) => {
                self.l1.put(key.clone(), v.clone());
                Ok(v)
            }
            Ok(None) => {
                let v = build.await?;
                self.l1.put(key.clone(), v.clone());
                if let Err(e) = self.l2_put(&key, &v).await {
                    eprintln!("cache put {key}: {e}");
                }
                Ok(v)
            }
            Err(e) => {
                eprintln!("cache get {key}: {e}");
                build.await
            }
        }
    }

    pub(crate) fn l1_invalidate_prefix(&self, prefix: &str) -> usize {
        self.l1.invalidate_prefix(prefix)
    }

    pub(crate) fn l1_flush(&self) {
        self.l1.flush();
    }
}

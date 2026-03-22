use std::collections::HashMap;
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use crate::theory::pdg::contracts::{
    PdgCacheBucketDiagnostics, PdgCacheDiagnostics, PdgDecayTable, PdgParticleRecord,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvictionStrategy {
    Lru,
    Lfu,
}

#[derive(Debug, Clone)]
struct CacheMeta {
    last_access: u64,
    frequency: u64,
}

#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    meta: CacheMeta,
}

#[derive(Debug, Clone, Copy, Default)]
struct CacheCounters {
    hits: u64,
    misses: u64,
    evictions: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct BoundedCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    capacity: usize,
    strategy: EvictionStrategy,
    clock: u64,
    counters: CacheCounters,
    entries: HashMap<K, CacheEntry<V>>,
}

impl<K, V> BoundedCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    pub(crate) fn new(capacity: usize, strategy: EvictionStrategy) -> Self {
        Self {
            capacity: capacity.max(1),
            strategy,
            clock: 0,
            counters: CacheCounters::default(),
            entries: HashMap::with_capacity(capacity.max(1)),
        }
    }

    pub(crate) fn get(&mut self, key: &K) -> Option<V> {
        self.clock = self.clock.saturating_add(1);
        if let Some(entry) = self.entries.get_mut(key) {
            entry.meta.last_access = self.clock;
            entry.meta.frequency = entry.meta.frequency.saturating_add(1);
            self.counters.hits = self.counters.hits.saturating_add(1);
            return Some(entry.value.clone());
        }

        self.counters.misses = self.counters.misses.saturating_add(1);
        None
    }

    pub(crate) fn put(&mut self, key: K, value: V) {
        self.clock = self.clock.saturating_add(1);

        if let Some(entry) = self.entries.get_mut(&key) {
            entry.value = value;
            entry.meta.last_access = self.clock;
            entry.meta.frequency = entry.meta.frequency.saturating_add(1);
            return;
        }

        if self.entries.len() >= self.capacity {
            if let Some(victim) = self.choose_victim() {
                self.entries.remove(&victim);
                self.counters.evictions = self.counters.evictions.saturating_add(1);
            }
        }

        self.entries.insert(
            key,
            CacheEntry {
                value,
                meta: CacheMeta {
                    last_access: self.clock,
                    frequency: 1,
                },
            },
        );
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.clock = 0;
    }

    pub(crate) fn diagnostics(&self) -> PdgCacheBucketDiagnostics {
        let total = self.counters.hits.saturating_add(self.counters.misses);
        let hit_rate = if total == 0 {
            0.0
        } else {
            self.counters.hits as f64 / total as f64
        };

        PdgCacheBucketDiagnostics {
            hits: self.counters.hits,
            misses: self.counters.misses,
            evictions: self.counters.evictions,
            size: self.entries.len(),
            capacity: self.capacity,
            hit_rate,
        }
    }

    fn choose_victim(&self) -> Option<K> {
        match self.strategy {
            EvictionStrategy::Lru => self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.meta.last_access)
                .map(|(key, _)| key.clone()),
            EvictionStrategy::Lfu => self
                .entries
                .iter()
                .min_by_key(|(_, entry)| (entry.meta.frequency, entry.meta.last_access))
                .map(|(key, _)| key.clone()),
        }
    }
}

#[derive(Debug)]
pub(crate) struct SharedBoundedCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    inner: Mutex<BoundedCache<K, V>>,
}

impl<K, V> SharedBoundedCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    pub(crate) fn new(capacity: usize, strategy: EvictionStrategy) -> Self {
        Self {
            inner: Mutex::new(BoundedCache::new(capacity, strategy)),
        }
    }

    pub(crate) fn get(&self, key: &K) -> Option<V> {
        match self.inner.lock() {
            Ok(mut guard) => guard.get(key),
            Err(_) => None,
        }
    }

    pub(crate) fn put(&self, key: K, value: V) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.put(key, value);
        }
    }

    pub(crate) fn clear(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.clear();
        }
    }

    pub(crate) fn diagnostics(&self) -> PdgCacheBucketDiagnostics {
        match self.inner.lock() {
            Ok(guard) => guard.diagnostics(),
            Err(_) => PdgCacheBucketDiagnostics {
                hits: 0,
                misses: 0,
                evictions: 0,
                size: 0,
                capacity: 0,
                hit_rate: 0.0,
            },
        }
    }
}

#[derive(Debug)]
pub(crate) struct PdgCacheSet {
    particle_records: SharedBoundedCache<i32, PdgParticleRecord>,
    decay_tables: SharedBoundedCache<(i32, String), PdgDecayTable>,
    id_resolution: SharedBoundedCache<String, i32>,
    db_queries: AtomicU64,
    db_query_time_ns: AtomicU64,
    db_last_query_ns: AtomicU64,
}

impl PdgCacheSet {
    pub(crate) fn with_strategies(
        particle_capacity: usize,
        decay_capacity: usize,
        id_capacity: usize,
        particle_strategy: EvictionStrategy,
        decay_strategy: EvictionStrategy,
        id_strategy: EvictionStrategy,
    ) -> Self {
        Self {
            particle_records: SharedBoundedCache::new(particle_capacity, particle_strategy),
            decay_tables: SharedBoundedCache::new(decay_capacity, decay_strategy),
            id_resolution: SharedBoundedCache::new(id_capacity, id_strategy),
            db_queries: AtomicU64::new(0),
            db_query_time_ns: AtomicU64::new(0),
            db_last_query_ns: AtomicU64::new(0),
        }
    }

    pub(crate) fn new_default() -> Self {
        Self::with_strategies(
            500,
            200,
            2000,
            EvictionStrategy::Lru,
            EvictionStrategy::Lru,
            EvictionStrategy::Lfu,
        )
    }

    pub(crate) fn get_particle_record(&self, mcid: i32) -> Option<PdgParticleRecord> {
        self.particle_records.get(&mcid)
    }

    pub(crate) fn put_particle_record(&self, mcid: i32, record: PdgParticleRecord) {
        self.particle_records.put(mcid, record);
    }

    pub(crate) fn get_decay_table(&self, mcid: i32, policy: &str) -> Option<PdgDecayTable> {
        self.decay_tables.get(&(mcid, policy.to_string()))
    }

    pub(crate) fn put_decay_table(&self, mcid: i32, policy: &str, table: PdgDecayTable) {
        self.decay_tables.put((mcid, policy.to_string()), table);
    }

    pub(crate) fn get_id_resolution(&self, key: &str) -> Option<i32> {
        self.id_resolution.get(&key.to_ascii_lowercase())
    }

    pub(crate) fn put_id_resolution(&self, key: &str, mcid: i32) {
        self.id_resolution.put(key.to_ascii_lowercase(), mcid);
    }

    pub(crate) fn clear(&self) {
        self.particle_records.clear();
        self.decay_tables.clear();
        self.id_resolution.clear();
    }

    pub(crate) fn record_db_query(&self, elapsed: Duration) {
        let nanos = elapsed.as_nanos().min(u64::MAX as u128) as u64;
        self.db_queries.fetch_add(1, Ordering::Relaxed);
        self.db_query_time_ns.fetch_add(nanos, Ordering::Relaxed);
        self.db_last_query_ns.store(nanos, Ordering::Relaxed);
    }

    pub(crate) fn diagnostics(&self) -> PdgCacheDiagnostics {
        let particle = self.particle_records.diagnostics();
        let decay = self.decay_tables.diagnostics();
        let id = self.id_resolution.diagnostics();

        let total_hits = particle.hits + decay.hits + id.hits;
        let total_misses = particle.misses + decay.misses + id.misses;
        let total_evictions = particle.evictions + decay.evictions + id.evictions;
        let total_entries = particle.size + decay.size + id.size;

        let query_count = self.db_queries.load(Ordering::Relaxed);
        let query_time_ns = self.db_query_time_ns.load(Ordering::Relaxed);
        let avg_us = if query_count == 0 {
            0.0
        } else {
            (query_time_ns as f64 / query_count as f64) / 1_000.0
        };
        let last_us = self.db_last_query_ns.load(Ordering::Relaxed) as f64 / 1_000.0;

        PdgCacheDiagnostics {
            particle_records: particle,
            decay_tables: decay,
            id_resolution: id,
            total_hits,
            total_misses,
            total_evictions,
            total_entries,
            db_queries: query_count,
            db_average_latency_us: avg_us,
            db_last_latency_us: last_us,
        }
    }
}

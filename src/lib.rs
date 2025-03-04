use wasm_bindgen::prelude::*;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::cmp;

pub struct ElasticHashTable<K, V> {
    delta: f64,
    max_inserts: usize,
    num_inserts: usize,
    levels: Vec<Vec<Option<(K, V)>>>,
    occupancies: Vec<usize>,
    c: f64,
}
const THRESHOLD: f64 = 0.25;

impl<K, V> ElasticHashTable<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    /// capacity: total capacity
    /// delta: ratio of empty slots
    pub fn new(capacity: usize, delta: f64) -> Self {
        if capacity == 0 {
            panic!("Capacity must be positive.");
        }
        if !(0.0 < delta && delta < 1.0) {
            panic!("delta must be between 0 and 1.");
        }
        // max inserts = capacity - floor(delta * capacity)
        let max_inserts = capacity - (delta * capacity as f64).floor() as usize;

        // calculate number of levels: floor(log₂(capacity)), at least 1 level
        let mut levels = Vec::new();
        let mut remaining = capacity;
        let mut cap = remaining;
        while remaining > 0 {
            cap = std::cmp::min(remaining, (cap as f64 / 2.0).ceil() as usize);
            levels.push(vec![None; cap]);
            remaining = remaining - cap;
        }

        let occupancies = vec![0; levels.len()];
        let c = 4.0; // constant c

        Self {
            delta,
            max_inserts,
            num_inserts: 0,
            levels,
            occupancies,
            c,
        }
    }

    /// use DefaultHasher to calculate hash value, combine key and level println
    fn hash<Q: ?Sized>(&self, key: &Q, level: usize) -> u64
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash,
    {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        level.hash(&mut hasher);
        hasher.finish() & 0x7FFFFFFF
    }

    /// quadratic probe function: return the index of the j-th probe
    fn quad_probe<Q: ?Sized>(&self, key: &Q, level: usize, j: usize, table_size: usize) -> usize
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash,
    {
        let h = self.hash(key, level);
        ((h as usize) + j * j) % table_size
    }

    /// calculate the free ratio of the specified level: free/size
    fn level_load(&self, level: usize) -> f64 {
        let size = self.levels[level].len() as f64;
        let occ = self.occupancies[level] as f64;
        let free = size - occ;
        free / size
    }

    /// insert (key, value)
    /// according to the strategy described in the paper:
    /// - for non-last levels, first calculate the load of the current level, then calculate the probe_limit based on the load,
    ///   then decide which strategy to use based on the state of the next level (load_next and 0.25 threshold).
    /// - for the last level, scan the entire level.
    pub fn insert(&mut self, key: K, value: V) -> Result<(usize, usize), String> {
        if self.num_inserts >= self.max_inserts {
            self.print_status();
            return Err("Hash table is full (maximum allowed insertions reached).".into());
        }
        for i in 0..self.levels.len() - 1 {
            let level_size = self.levels[i].len();
            let load = self.level_load(i);

            // non-last level: calculate the load of the next level
            let next_load = self.level_load(i + 1);
            if load > (self.delta / 2.0) && next_load > THRESHOLD {
                // calculate probe_limit, simulate f(ε)=c×min(log₂(1/ε), log₂(1/δ))
                let log_inv_load = if load > 0.0 { (1.0 / load).log2() } else { 0.0 };
                let log_inv_delta = (1.0 / self.delta).log2();
                let probe_limit = cmp::max(
                    1,
                    (self.c * log_inv_load.min(log_inv_delta)).ceil() as usize,
                );
                // Case 1: try limited probes in the current level
                for j in 0..probe_limit {
                    let idx = self.quad_probe(&key, i, j, level_size);
                    if self.levels[i][idx].is_none() {
                        self.levels[i][idx] = Some((key.clone(), value.clone()));
                        self.occupancies[i] += 1;
                        self.num_inserts += 1;
                        return Ok((i, idx));
                    }
                }
                // if insertion fails in the current level, try a fixed number of probes in the next level (here using the ceiling of c)
                let next_size = self.levels[i + 1].len();
                for j in 0..self.c.ceil() as usize{
                    let idx = self.quad_probe(&key, i + 1, j, next_size);
                    if self.levels[i + 1][idx].is_none() {
                        self.levels[i + 1][idx] = Some((key.clone(), value.clone()));
                        self.occupancies[i + 1] += 1;
                        self.num_inserts += 1;
                        return Ok((i + 1, idx));
                    }
                }
            } else if load <= (self.delta / 2.0) {
                // Case 2: current level has too few empty slots, skip and try the next level
                continue;
            } else if next_load <= THRESHOLD {
                // Case 3: next level is full, must scan all slots in the current level
                for j in 0..level_size {
                    let idx = self.quad_probe(&key, i, j, level_size);
                    if self.levels[i][idx].is_none() {
                        self.levels[i][idx] = Some((key.clone(), value.clone()));
                        self.occupancies[i] += 1;
                        self.num_inserts += 1;
                        return Ok((i, idx));
                    }
                }
            }
        }
        // last level: scan the entire level by borrowing it directly
        let last_level_size = self.levels[self.levels.len() - 1].len();
        for j in 0..last_level_size {
            let idx = self.quad_probe(&key, self.levels.len() - 1, j, last_level_size);
            {
                let last = self.levels.len() - 1;
                let last_level = &mut self.levels[last];
                if last_level[idx].is_none() {
                    last_level[idx] = Some((key.clone(), value.clone()));
                    self.occupancies[last] += 1;
                    self.num_inserts += 1;
                    return Ok((last, idx));
                }
            }
        }
        Err("Insertion failed in all levels; hash table is full.".into())
    }

    // search algorithm is not correct
    pub fn search<Q: ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq,
    {
        for i in 0..self.levels.len() - 1 {
            for j in 0..self.levels[i].len() {
                let idx = self.quad_probe(&key, i, j, self.levels[i].len());
                if let Some((ref k, ref v)) = self.levels[i][idx] {
                    if k.borrow() == key {
                        return Some(v);
                    }
                }
            }
        }
        None
    }

    pub fn print_status(&self) {
        println!("Occupancies: {:?}", self.occupancies);
        println!("Num inserts: {}", self.num_inserts);
        println!("Max inserts: {}", self.max_inserts);
        for i in 0..self.levels.len() {
            println!("Level {}: {}/{}", i, self.levels[i].len() - self.occupancies[i], self.levels[i].len());
        }
    }
}

#[wasm_bindgen]
pub struct JsElasticHashTable {
    table: ElasticHashTable<String, String>
}

#[wasm_bindgen]
impl JsElasticHashTable {
    #[wasm_bindgen(constructor)]
    pub fn new(capacity: usize, delta: f64) -> Self {
        JsElasticHashTable {
            table: ElasticHashTable::new(capacity, delta)
        }
    }

    #[wasm_bindgen]
    pub fn insert(&mut self, key: String, value: String) {
        self.table.insert(key, value).expect("Insertion failed");
    }

    #[wasm_bindgen]
    pub fn search(&self, key: String) -> Option<String> {
        self.table.search(&key).map(|v| v.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use env_logger;
    use log::LevelFilter;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(LevelFilter::Debug)
            .is_test(true)
            .try_init();
    }

    #[test]
    fn test_elastic_hash_table() {
        init();
        let n = 10000;
        let delta = 0.01;
        let mut table = ElasticHashTable::new(n, delta);

        for i in 0..(n as f64 * (1.0 - delta)) as usize {
            table.insert(i, i << 1).expect("Insertion failed");
        }
        table.print_status();

        // test search
        for i in 0..(n as f64 * (1.0 - delta)) as usize {
            let res = table.search(&i);
            assert!(res.is_some(), "Key {} not found", i);
            assert_eq!(res.unwrap(), &(i << 1));
        }
    }

    #[test]
    fn test_small_elastic_hash_table() {
        init();
        let n = 10;
        let delta = 0.1;
        let mut table = ElasticHashTable::new(n, delta);

        for i in 0..9 {
            let res = table.insert(i, i).expect("Insertion failed");
            println!("{:?}", res);
        }
        table.print_status();

        for i in 0..9 {
            let res = table.search(&i);
            assert!(res.is_some(), "Key {} not found", i);
            assert_eq!(res.unwrap(), &i);
        }
    }

}

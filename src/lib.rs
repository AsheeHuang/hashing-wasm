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
    pub fn insert(&mut self, key: K, value: V) -> Result<(), String> {
        if self.num_inserts >= self.max_inserts {
            self.print_status();
            return Err("Hash table is full (maximum allowed insertions reached).".into());
        }
        for i in 0..self.levels.len() {
            let level_size = self.levels[i].len();
            let load = self.level_load(i);
            // calculate probe_limit, simulate f(ε)=c×min(log₂(1/ε), log₂(1/δ))
            let log_inv_load = if load > 0.0 { (1.0 / load).log2() } else { 0.0 };
            let log_inv_delta = (1.0 / self.delta).log2();
            let probe_limit = cmp::max(
                1,
                (self.c * log_inv_load.min(log_inv_delta)).ceil() as usize,
            );

            if i < self.levels.len() - 1 {
                // non-last level: calculate the load of the next level
                let next_load = self.level_load(i + 1);
                let threshold = 0.25;
                if load > (self.delta / 2.0) && next_load > threshold {
                    // Case 1: try limited probes in the current level
                    for j in 0..probe_limit {
                        let idx = self.quad_probe(&key, i, j, level_size);
                        if self.levels[i][idx].is_none() {
                            self.levels[i][idx] = Some((key.clone(), value.clone()));
                            self.occupancies[i] += 1;
                            self.num_inserts += 1;
                            return Ok(());
                        }
                    }
                    // if insertion fails in the current level, try a fixed number of probes in the next level (here using the ceiling of c)
                    let next_probe_limit = cmp::max(1, self.c.ceil() as usize);
                    let next_size = self.levels[i + 1].len();
                    for j in 0..next_probe_limit {
                        let idx = self.quad_probe(&key, i + 1, j, next_size);
                        if self.levels[i + 1][idx].is_none() {
                            self.levels[i + 1][idx] = Some((key.clone(), value.clone()));
                            self.occupancies[i + 1] += 1;
                            self.num_inserts += 1;
                            return Ok(());
                        }
                    }
                } else if load <= (self.delta / 2.0) {
                    // Case 2: current level has too few empty slots, skip and try the next level
                    continue;
                } else if next_load <= threshold {
                    // Case 3: next level is full, must scan all slots in the current level
                    for j in 0..level_size {
                        let idx = self.quad_probe(&key, i, j, level_size);
                        if self.levels[i][idx].is_none() {
                            self.levels[i][idx] = Some((key.clone(), value.clone()));
                            self.occupancies[i] += 1;
                            self.num_inserts += 1;
                            return Ok(());
                        }
                    }
                }
            } else {
                // last level: scan the entire level
                for j in 0..level_size {
                    let idx = self.quad_probe(&key, i, j, level_size);
                    if self.levels[i][idx].is_none() {
                        self.levels[i][idx] = Some((key.clone(), value.clone()));
                        self.occupancies[i] += 1;
                        self.num_inserts += 1;
                        return Ok(());
                    }
                }
            }
        }
        self.print_status();
        Err("Insertion failed in all levels; hash table is full.".into())
    }

    /// search for the given key, return Some(&value) if found
    pub fn search<Q: ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq,
    {
        for i in 0..self.levels.len() {
            let level_size = self.levels[i].len();
            for j in 0..level_size {
                let idx = self.quad_probe(key, i, j, level_size);
                match &self.levels[i][idx] {
                    Some((k, v)) if k.borrow() == key => return Some(v),
                    None => break,
                    _ => continue,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elastic_hash_table() {
        let n = 100000;
        let delta = 0.0001;
        let mut table = ElasticHashTable::new(n, delta);
        // insert some data
        for i in 0..(n as f64 * (1.0 - delta)) as usize {
            table.insert(i, format!("Value {}", i)).expect("Insertion failed");
        }
        table.print_status();
        // test search
        for i in 0..n * (1.0 - delta) as usize {
            let res = table.search(&i);
            assert!(res.is_some(), "Key {} not found", i);
            assert_eq!(res.unwrap(), &format!("Value {}", i));
        }
    }
}
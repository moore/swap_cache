use std::cmp::max;
use hashbrown::HashMap;


struct CacheEntry<K, V> {
    key: K,
    value: V,
    index: usize,
}

const DEFAULT_MOVE: usize = 50;

struct Lru<K> {
        ring: Vec<K>,
        top: usize,
        max_pointer: usize,
        move_ratio: usize,
        min_update_distance: usize,
        min_update_limit: usize,
        long_distance: usize,
    }

impl<K> Lru<K> {

    fn set_min_update_limit(&mut self) {
        self.min_update_limit = 1 + self.max_pointer - (self.max_pointer * self.move_ratio) / 100;
    }
}


pub struct SwapCache<K, V> {
    mapping: HashMap<K, usize>,
    lru: Lru<usize>,
    entries: Vec<CacheEntry<K,V>>
}

// BUG: lets get rid of all the magic constants
impl<K, V> SwapCache<K, V>
where
    K: Clone + std::cmp::Eq + std::hash::Hash
{
    pub fn new(size: usize) -> SwapCache<K, V> {
        // BUG: Should we size mapping, ring, and entries to size right away?
        let mut cache = SwapCache {
            mapping: HashMap::new(),
            lru: Lru {
                ring: Vec::new(),
                top: 0,
                max_pointer: size - 1,
                move_ratio: DEFAULT_MOVE,
                min_update_distance: (size * DEFAULT_MOVE) / 100,
                min_update_limit: 0,
                long_distance: size / 4,
            },
            entries: Vec::new(),
        };

        cache.lru.set_min_update_limit();

        cache
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        let entry = self.update(key, 10);

        match entry {
            None => None,
            Some(e) => Some(&e.value),
        }
    }

    pub fn put(&mut self, key: K, value: V) {
        // What should we do if we do have the key? self.update()?
        if self.mapping.contains_key(&key) {
            return;
        }

        let lru = &mut self.lru;
        let mapping = &mut self.mapping;
        let entries = &mut self.entries;
        
        let slot =
            if lru.ring.len() <= lru.top {
                let slot = entries.len();
                
                let entry = CacheEntry {
                    key: key.clone(),
                    value,
                    index: slot, //same as index
                };
            
                entries.push(entry);
                lru.ring.push(slot);
                
                slot
            } else {
                let slot = lru.ring[lru.top];

                let dead_key = {
                    entries[slot].key.clone()
                };
            
                mapping.remove(&dead_key);

                entries[slot] = CacheEntry {
                    key: key.clone(),
                    value,
                    index: lru.top,
                };
            
                if lru.min_update_distance > (lru.move_ratio / 100) {
                    lru.min_update_distance -= 1;
                }

                slot
            };

        mapping.insert(
            key,
            slot,
        );

        lru.top += 1;

        if lru.top > lru.max_pointer {
            lru.top = 0;
        } 
    }

    fn update<'a>(&'a mut self, key: &K, count: usize) -> Option<&'a mut CacheEntry<K, V>> {
        let mapping = &self.mapping;
        let lru     = &mut self.lru;
        let entries = &mut self.entries;
        
        let slot = match mapping.get(key) {
            None => return None,
            Some(slot) => *slot,
        };

        let mut currnet_index = entries[slot].index;
        
        let distance = if currnet_index <= lru.top {
            lru.top - currnet_index
        } else {
            lru.top + lru.max_pointer - currnet_index
        };

        if distance <= lru.min_update_distance {
            return Some(&mut entries[slot]);
        }

        let mut move_distance = (distance * lru.move_ratio) / 100;

        let steep_size = max(move_distance/count, 1);

        let next_index = loop {
            let mut next_index = currnet_index + steep_size;

            if next_index >= lru.max_pointer {
                next_index -= lru.max_pointer;
            }

            let demoted = lru.ring[next_index];

            entries[demoted].index = currnet_index;
            lru.ring[currnet_index] = demoted;

            currnet_index = next_index;

            if move_distance > steep_size {
                move_distance -= steep_size;
            } else {
                break next_index;
            }
        };

        if lru.min_update_distance < lru.min_update_limit {
            lru.min_update_distance += 1;
        }

        if (distance < lru.long_distance) && (lru.move_ratio >= 1) {
            lru.move_ratio -= 1;
            lru.set_min_update_limit();
        } else if lru.move_ratio < 99 {
            lru.move_ratio += 1;
            lru.set_min_update_limit();
        }

        let entry = &mut entries[slot];

        entry.index = next_index;

        lru.ring[next_index] = slot;

        Some(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple() {
        let mut cache = SwapCache::new(10);

        let pairs: Vec<(usize, char)> = "abcdefg".chars().enumerate().collect();

        for (value, key) in pairs.clone() {
            cache.put(key, value);
        }

        for (value, key) in pairs.clone() {
            assert_eq!(cache.get(&key), Some(&value))
        }
    }

    #[test]
    fn test_expire() {

        let mut cache = SwapCache::new(10);

        let pairs: Vec<(usize, char)> = "abcdefghijklmnopqrstuvwxyz".chars().enumerate().collect();
        
        for (value, key) in pairs.clone() {
            cache.put( key, value );
        }

        for (value, key) in &pairs[16..26] {            
            assert_eq!(cache.get( key ), Some(value));
        }

        for (_, key) in &pairs[0..16] {            
            assert_eq!(cache.get( key ), None);
        }
    }


    #[test]
    fn test_update() {

        let mut cache = SwapCache::new(20);
        
        let pairs: Vec<(usize, char)> = "abcdefghijklmnopqrstuvwxyz".chars().enumerate().collect();
     
        let (update_value, update_key) = pairs[0];
        let (_, displaced_key) = pairs[6];
        
        for (value, key) in pairs.clone() {
            cache.put(key, value);
            cache.get(&update_key);
        }
        assert_eq!(cache.get(&update_key), Some(&update_value));
        assert_eq!(cache.get(&displaced_key), None);
    }


}

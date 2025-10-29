use core::hash::{Hash, Hasher};
use core::mem;
use arceos_api::random;
#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
#[doc(no_inline)]
use alloc::vec::Vec;

// ---------- SipHasher24 支持 ----------
use siphasher::sip::SipHasher;

pub struct DefaultHasher(SipHasher);

impl DefaultHasher {
    pub fn new_with_keys(k0: u64, k1: u64) -> Self {
        Self(SipHasher::new())
    }
}

impl Hasher for DefaultHasher {
    fn write(&mut self, bytes: &[u8]) {
        self.0.write(bytes);
    }
    fn finish(&self) -> u64 {
        self.0.finish()
    }
}

// ---------- BuildHasher ----------
pub struct MyRandomState {
    k0: u64,
    k1: u64,
}

impl MyRandomState {
    pub fn new() -> Self {
        let buf: [u8; 16] = random().to_ne_bytes();
        let k0 = u64::from_ne_bytes(buf[0..8].try_into().unwrap());
        let k1 = u64::from_ne_bytes(buf[8..16].try_into().unwrap());
        Self { k0, k1 }
    }

    pub fn build_hasher(&self) -> DefaultHasher {
        DefaultHasher::new_with_keys(self.k0, self.k1)
    }
}

// ---------- Entry ----------
#[derive(Clone)]
pub(crate) enum Entry<K, V> {
    Occupied(K, V),
    Tombstone,
    Empty,
}

// ---------- HashMap ----------
const INITIAL_CAPACITY: usize = 16;
const LOAD_FACTOR: f64 = 0.75;
#[cfg(feature = "alloc")]
pub struct HashMap<K, V> {
    entries: Vec<Entry<K, V>>,
    len: usize,
    hasher_builder: MyRandomState,
}
#[cfg(feature = "alloc")]
impl<K, V> HashMap<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    pub fn new() -> Self {
        let mut entries = Vec::with_capacity(INITIAL_CAPACITY);
        entries.resize_with(INITIAL_CAPACITY, || Entry::Empty);
        Self {
            entries,
            len: 0,
            hasher_builder: MyRandomState::new(),
        }
    }

    fn hash(&self, key: &K) -> usize {
        let mut hasher = self.hasher_builder.build_hasher();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.entries.len()
    }

    fn resize(&mut self) {
        let old_entries = mem::replace(&mut self.entries, Vec::new());
        let new_capacity = old_entries.len() * 2;
        self.entries = (0..new_capacity).map(|_| Entry::Empty).collect();
        self.len = 0;

        for entry in old_entries {
            if let Entry::Occupied(k, v) = entry {
                self.insert_no_resize(k, v);
            }
        }
    }

    fn insert_no_resize(&mut self, key: K, value: V) {
        let mut idx = self.hash(&key);
        loop {
            match &self.entries[idx] {
                Entry::Empty | Entry::Tombstone => {
                    self.entries[idx] = Entry::Occupied(key, value);
                    self.len += 1;
                    return;
                }
                Entry::Occupied(k, _) if k == &key => {
                    self.entries[idx] = Entry::Occupied(key, value);
                    return;
                }
                _ => {
                    idx = (idx + 1) % self.entries.len();
                }
            }
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        if self.len >= (self.entries.len() as f64 * LOAD_FACTOR) as usize {
            self.resize();
        }
        self.insert_no_resize(key, value);
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: core::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let mut idx = {
            let mut hasher = self.hasher_builder.build_hasher();
            key.hash(&mut hasher);
            (hasher.finish() as usize) % self.entries.len()
        };

        loop {
            match &self.entries[idx] {
                Entry::Empty => return None,
                Entry::Tombstone => {}
                Entry::Occupied(k, v) if k.borrow() == key => return Some(v),
                _ => {}
            }
            idx = (idx + 1) % self.entries.len();
        }
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: core::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let mut idx = {
            let mut hasher = self.hasher_builder.build_hasher();
            key.hash(&mut hasher);
            (hasher.finish() as usize) % self.entries.len()
        };

        loop {
            match &self.entries[idx] {
                Entry::Empty => return None,
                Entry::Occupied(k, _) if k.borrow() == key => {
                    if let Entry::Occupied(_, v) = mem::replace(&mut self.entries[idx], Entry::Tombstone) {
                        self.len -= 1;
                        return Some(v);
                    }
                    unreachable!();
                }
                _ => {}
            }
            idx = (idx + 1) % self.entries.len();
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

// ---------- Iter ----------
#[cfg(feature = "alloc")]
pub struct Iter<'a, K: 'a, V: 'a> {
    entries: core::slice::Iter<'a, Entry<K, V>>,
}
#[cfg(feature = "alloc")]
impl<'a, K: 'a, V: 'a> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.entries.next() {
                Some(Entry::Occupied(k, v)) => return Some((k, v)),
                Some(_) => continue,
                None => return None,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.entries.size_hint().1)
    }
}
#[cfg(feature = "alloc")]
impl<K, V> HashMap<K, V> {
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            entries: self.entries.iter(),
        }
    }
}

// ---------- IntoIterator for &HashMap ----------
#[cfg(feature = "alloc")]
impl<'a, K, V> IntoIterator for &'a HashMap<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
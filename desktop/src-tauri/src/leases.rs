use std::collections::HashMap;

/// Leases never reuse IDs, so delayed callbacks cannot reach a replacement controller.
pub struct Leases<T> {
    next: u64,
    entries: HashMap<String, T>,
}
impl<T> Default for Leases<T> {
    fn default() -> Self {
        Self {
            next: 0,
            entries: HashMap::new(),
        }
    }
}
impl<T> Leases<T> {
    pub fn has_capacity(&self) -> bool {
        self.entries.len() < 32
    }
    pub fn insert(&mut self, value: T) -> Result<String, String> {
        if !self.has_capacity() {
            return Err("At most 32 terminal views may be attached".into());
        }
        self.next = self
            .next
            .checked_add(1)
            .ok_or("Controller lease IDs exhausted")?;
        let id = self.next.to_string();
        self.entries.insert(id.clone(), value);
        Ok(id)
    }
    pub fn get(&mut self, id: &str) -> Result<&mut T, String> {
        self.entries
            .get_mut(id)
            .ok_or("Terminal view is detached".into())
    }
    pub fn remove(&mut self, id: &str) -> Option<T> {
        self.entries.remove(id)
    }
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_lease_cannot_access_or_remove_replacement() {
        let mut pool = Leases::default();
        let old = pool.insert("first").unwrap();
        pool.remove(&old);
        let new = pool.insert("second").unwrap();
        pool.remove(&old);
        assert_ne!(old, new);
        assert!(pool.get(&old).is_err());
        assert_eq!(*pool.get(&new).unwrap(), "second");
    }
    #[test]
    fn controllers_are_independent_and_bounded() {
        let mut pool = Leases::default();
        for i in 0..32 {
            pool.insert(i).unwrap();
        }
        assert!(!pool.has_capacity());
        assert!(pool.insert(33).is_err());
        pool.remove("1");
        assert_eq!(*pool.get("2").unwrap(), 1);
        assert!(pool.insert(33).is_ok());
        pool.clear();
        assert!(pool.get("2").is_err());
    }
}

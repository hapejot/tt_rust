use std::{collections::HashSet, sync::Mutex};

use once_cell::sync::Lazy;
use tracing::info;

pub struct SelectorSet {
    cache: Mutex<HashSet<&'static str>>,
}

impl SelectorSet {
    pub fn get(name: &str) -> &'static str {
        let mut lck = SELECTOR_SET.cache.lock().unwrap();
        match lck.get(name) {
            Some(s) => *s,
            None => {
                let s0 = Box::new(name.to_string());
                let s1: &'static String = Box::leak(s0);
                lck.insert(s1);
                s1
            }
        }
    }

    pub fn stats() {
        for x in SELECTOR_SET.cache.lock().unwrap().iter() {
            info!("selector {}", x);
        }
    }
}

static SELECTOR_SET: Lazy<SelectorSet> = Lazy::new(|| SelectorSet {
    cache: Mutex::new(HashSet::new()),
});

#[cfg(test)]
mod test {
    use super::SelectorSet;

    #[test]
    fn selector() {
        let sel1 = SelectorSet::get("a:b:");
        let sel2 = SelectorSet::get(format!("{}:{}:", "a", "b").as_str());
        assert_eq!(sel1, sel2);
        assert_eq!(sel1.as_ptr(), sel2.as_ptr());
    }
}

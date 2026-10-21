//! Typed retained results and caller-selected presentation interests.
use crate::session::{ResultHandle, ResultKey};
use std::{any::Any, collections::HashMap, hash::Hash, ops::Deref, sync::Arc};

/// Loading's padded ranks are display-only; finalized ranks borrow the shared front.
#[derive(Clone, Debug)]
pub enum ParetoRanks {
    Pending(Vec<u32>),
    Finalized(SharedResult<crate::pareto::ParetoResult>),
}
impl Deref for ParetoRanks {
    type Target = Vec<u32>;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Pending(ranks) => ranks,
            Self::Finalized(front) => &front.ranks,
        }
    }
}
impl PartialEq<Vec<u32>> for ParetoRanks {
    fn eq(&self, other: &Vec<u32>) -> bool {
        **self == *other
    }
}

pub struct KeyedResult<K, T> {
    pub selection: K,
    pub value: Arc<T>,
    pub rows: Vec<tdashboard_study::TrialKey>,
}

#[derive(Debug)]
pub struct SharedResult<T> {
    handle: ResultHandle,
    value: Arc<T>,
    rows: Vec<tdashboard_study::TrialKey>,
}
impl<T> Clone for SharedResult<T> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
            value: self.value.clone(),
            rows: self.rows.clone(),
        }
    }
}
impl<T: Any + Send + Sync> SharedResult<T> {
    pub fn new(handle: ResultHandle) -> Result<Self, String> {
        let value = handle.payload::<T>().ok_or("Analysis is still running")??;
        let rows = handle.rows().first().cloned().unwrap_or_default();
        Ok(Self {
            handle,
            value,
            rows,
        })
    }
    pub fn keyed<K: Any + Send + Sync + Clone>(handle: ResultHandle) -> Result<(K, Self), String> {
        let payload = handle
            .payload::<KeyedResult<K, T>>()
            .ok_or("Analysis is still running")??;
        Ok((
            payload.selection.clone(),
            Self {
                value: payload.value.clone(),
                rows: payload.rows.clone(),
                handle,
            },
        ))
    }
}
impl<T> SharedResult<T> {
    pub fn key(&self) -> &ResultKey {
        self.handle.key()
    }
    pub fn handle(&self) -> &ResultHandle {
        &self.handle
    }
    pub fn rows(&self) -> &[tdashboard_study::TrialKey] {
        &self.rows
    }
}
impl<T> Deref for SharedResult<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

/// A caller's selected handles, not a second copy of computed payloads.
#[derive(Debug)]
pub struct ResultSelections<K, T> {
    entries: HashMap<K, SharedResult<T>>,
}
impl<K, T> Default for ResultSelections<K, T> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}
impl<K: Eq + Hash, T> ResultSelections<K, T> {
    pub fn get(&self, key: &K) -> Option<&T> {
        self.entries.get(key).map(|r| &**r)
    }
    pub fn selected(&self, key: &K) -> Option<&SharedResult<T>> {
        self.entries.get(key)
    }
    pub fn contains_key(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn clear(&mut self) {
        self.entries.clear();
    }
    pub fn select(&mut self, key: K, result: SharedResult<T>) {
        self.entries.insert(key, result);
    }
    pub fn retain(&mut self, mut keep: impl FnMut(&K) -> bool) {
        self.entries.retain(|key, _| keep(key));
    }
    pub fn keys(&self) -> impl Iterator<Item = &ResultKey> {
        self.entries.values().map(SharedResult::key)
    }
}
impl<K: Eq + Hash + Clone + Any + Send + Sync, T: Any + Send + Sync> ResultSelections<K, T> {
    pub fn adopt(&mut self, handle: ResultHandle) -> Result<SharedResult<T>, String> {
        let (key, result) = SharedResult::<T>::keyed::<K>(handle)?;
        self.select(key, result.clone());
        Ok(result)
    }
}

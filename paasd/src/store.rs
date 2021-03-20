use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use tonic::Status;
use uuid::Uuid;

use worker::Process;

use crate::user::UserId;

#[derive(Debug, thiserror::Error)]
pub enum GetError {
    #[error("Process with the given id not found")]
    NotFound,
}

impl Into<Status> for GetError {
    fn into(self) -> Status {
        use GetError::*;
        match self {
            NotFound => Status::not_found(format!("{}", self)),
        }
    }
}

pub type ProcessStore = Store<Process>;

type StoreMap<V> = HashMap<Uuid, (UserId, Arc<V>)>;

#[derive(Default)]
pub struct Store<V>(RwLock<StoreMap<V>>);

impl<V> Store<V> {
    pub fn new() -> Self {
        Self(RwLock::new(StoreMap::new()))
    }

    pub fn get(&self, pid: Uuid, uid: &UserId) -> Result<Arc<V>, GetError> {
        let read = self.0.read().unwrap();
        let (owner, process) = read.get(&pid).ok_or(GetError::NotFound)?;
        // TODO: does this need constant-time EQ?
        if owner == uid {
            Ok(process.clone())
        } else {
            Err(GetError::NotFound)
        }
    }

    pub fn insert(&self, uid: &UserId, process: V) -> Uuid {
        let pid = Uuid::new_v4();
        let prev = self
            .0
            .write()
            .unwrap()
            .insert(pid, (uid.clone(), Arc::new(process)));
        if prev.is_some() {
            unreachable!("Duplicate UUID generated");
        }
        pid
    }
}

#[cfg(test)]
mod test {
    use super::*;
    type Store = super::Store<()>;

    #[test]
    fn test_store_get_not_found() {
        let uid = UserId("alice".into());
        let store = Store::new();
        let pid = Uuid::new_v4();
        assert!(matches!(store.get(pid, &uid), Err(GetError::NotFound)));
    }

    #[test]
    fn test_store_get_unauthorized() {
        let uid1 = UserId("alice".into());
        let uid2 = UserId("eve".into());
        let store = Store::new();
        let pid = store.insert(&uid1, ());
        assert!(matches!(store.get(pid, &uid2), Err(GetError::NotFound)))
    }

    #[test]
    fn test_store_get_authorized() {
        let uid = UserId("alice".into());
        let store = Store::new();
        let pid = store.insert(&uid, ());
        assert!(store.get(pid, &uid).is_ok())
    }
}

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use uuid::Uuid;

use crate::user::UserId;

#[derive(Debug, thiserror::Error)]
pub enum GetError {
    #[error("Process with the given id not found")]
    NotFound,
    #[error("User unauthorized to access this process")]
    Unauthorized,
}

type Process = (); // TODO: worker::Process instead
type ProcessMap = HashMap<Uuid, (UserId, Arc<Process>)>;

#[derive(Default)]
pub struct ProcessStore(RwLock<ProcessMap>);

impl ProcessStore {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get(&self, pid: Uuid, uid: &UserId) -> Result<Arc<Process>, GetError> {
        let read = self.0.read().unwrap();
        let (owner, process) = read.get(&pid).ok_or(GetError::NotFound)?;
        // TODO: does this need constant-time EQ?
        if owner == uid {
            Ok(process.clone())
        } else {
            Err(GetError::Unauthorized)
        }
    }

    pub fn insert(&self, uid: &UserId, process: Process) -> Uuid {
        let pid = Uuid::new_v4();
        let prev = self
            .0
            .write()
            .unwrap()
            .insert(pid, (uid.clone(), Arc::new(process)));
        if prev.is_some() {
            panic!("Duplicate UUID generated: this should never happen");
        }
        pid
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_pstore_get_not_found() {
        let uid = UserId("alice".into());
        let store = ProcessStore::new();
        let pid = Uuid::new_v4();
        assert!(matches!(store.get(pid, &uid), Err(GetError::NotFound)));
    }

    #[test]
    fn test_pstore_get_unauthorized() {
        let uid1 = UserId("alice".into());
        let uid2 = UserId("eve".into());
        let store = ProcessStore::new();
        let pid = store.insert(&uid1, ());
        assert!(matches!(store.get(pid, &uid2), Err(GetError::Unauthorized)))
    }

    #[test]
    fn test_pstore_get_authorized() {
        let uid = UserId("alice".into());
        let store = ProcessStore::new();
        let pid = store.insert(&uid, ());
        assert!(store.get(pid, &uid).is_ok())
    }
}

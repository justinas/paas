use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use tonic::Status;
use uuid::Uuid;

use worker::Process;

use crate::user::UserId;

/// Represents a resource stored in state, and *owned* by some user.
/// The `owner` data is used for authorization in `Store` impl.
struct Owned<T> {
    value: Arc<T>,
    owner: UserId,
}

impl<T> Owned<T> {
    fn new(value: T, owner: UserId) -> Self {
        Self {
            value: Arc::new(value),
            owner,
        }
    }
}

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

/// A concretization of Store that holds processes.
pub type ProcessStore = Store<Process>;

/// An in-memory storage for objects such as processes,
/// with ownership data attached.
#[derive(Default)]
pub struct Store<V>(RwLock<HashMap<Uuid, Owned<V>>>);

impl<V> Store<V> {
    /// Constructs a new, empty store.
    pub fn new() -> Self {
        Self(RwLock::new(HashMap::new()))
    }

    /// Tries to get a reference to a resource with the given `id`,
    /// owned by the user identified by the given `uid`.
    /// Returns an error if a resource is not found by the given ID,
    /// or the user is not the owner of the resource.
    pub fn get(&self, id: Uuid, uid: &UserId) -> Result<Arc<V>, GetError> {
        let read = self.0.read().unwrap();
        let Owned { value, owner } = read.get(&id).ok_or(GetError::NotFound)?;
        // TODO: does this need constant-time EQ?
        if owner == uid {
            Ok(value.clone())
        } else {
            Err(GetError::NotFound)
        }
    }

    /// Puts the given resource into the store,
    /// marking the resource as being owned by the given user.
    /// Generates and returns a `Uuid` that can be used to later retrieve the resource.
    pub fn insert(&self, uid: &UserId, value: V) -> Uuid {
        let pid = Uuid::new_v4();
        let prev = self
            .0
            .write()
            .unwrap()
            .insert(pid, Owned::new(value, uid.clone()));
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

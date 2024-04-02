//! A queue that can be extended while iterating over it.

use std::{
    collections::VecDeque,
    sync::{Arc, RwLock, Weak},
};

/// A queue that can be extended while iterating over it.
#[derive(Debug, Clone)]
pub struct ExtendableQueue<T> {
    queue: Arc<RwLock<VecDeque<T>>>,
}

impl Default for ExtendableQueue<String> {
    fn default() -> Self {
        Self {
            queue: Arc::new(RwLock::new(VecDeque::new())),
        }
    }
}

impl<T, V> From<V> for ExtendableQueue<T>
where
    V: Into<VecDeque<T>>,
{
    fn from(value: V) -> Self {
        Self {
            queue: Arc::new(RwLock::new(value.into())),
        }
    }
}

impl<T> ExtendableQueue<T> {
    /// Add an element to the queue.
    pub fn push(&self, value: T) {
        self.queue.write().unwrap().push_back(value);
    }

    /// Get the queue.
    pub fn get(&self) -> &Arc<RwLock<VecDeque<T>>> {
        &self.queue
    }

    /// Get a weak reference to the queue.
    pub fn get_weak(&self) -> Weak<RwLock<VecDeque<T>>> {
        Arc::downgrade(&self.queue)
    }

    /// Clear the queue.
    pub fn clear(&self) {
        self.queue.write().unwrap().clear();
    }

    /// Get the length of the queue.
    pub fn len(&self) -> usize {
        self.queue.read().unwrap().len()
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.read().unwrap().is_empty()
    }

    /// Get and remove the next item without needing mutable access.
    pub fn pop_front(&self) -> Option<T> {
        self.queue.write().unwrap().pop_front()
    }
}

impl<A> Extend<A> for ExtendableQueue<A> {
    fn extend<T: IntoIterator<Item = A>>(&mut self, iter: T) {
        self.queue.write().unwrap().extend(iter);
    }
}

impl<T> Iterator for ExtendableQueue<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.write().unwrap().pop_front()
    }
}

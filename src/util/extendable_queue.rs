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

impl<T> Default for ExtendableQueue<T> {
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
    #[must_use]
    pub fn get_arc(&self) -> &Arc<RwLock<VecDeque<T>>> {
        &self.queue
    }

    /// Get a weak reference to the queue.
    #[must_use]
    pub fn get_weak(&self) -> Weak<RwLock<VecDeque<T>>> {
        Arc::downgrade(&self.queue)
    }

    /// Clear the queue.
    pub fn clear(&self) {
        self.queue.write().unwrap().clear();
    }

    /// Get the length of the queue.
    #[must_use]
    pub fn len(&self) -> usize {
        self.queue.read().unwrap().len()
    }

    /// Check if the queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queue.read().unwrap().is_empty()
    }

    /// Get and remove the next item without needing mutable access.
    #[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue() {
        let mut queue = ExtendableQueue::default();
        queue.push(1);
        queue.push(2);
        queue.push(3);

        assert_eq!(queue.len(), 3);

        let mut count = 0;

        while let Some(el) = queue.next() {
            count += el;

            if el == 1 {
                queue.extend(vec![4, 5, 6]);
            }
        }

        assert_eq!(count, 21);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_from() {
        let base = vec![1, 2, 3, 4];
        let queue = ExtendableQueue::from(base.clone());

        assert!(queue.into_iter().zip(base).all(|(a, b)| a == b));
    }
}

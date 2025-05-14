use std::collections::HashSet;

/// A queue that maintains unique elements.
pub struct UniqueQueue<T> {
    queue: Vec<T>,
    set: HashSet<T>,
}

impl<T: Eq + std::hash::Hash + Clone> UniqueQueue<T> {
    /// Creates a new `UniqueQueue`.
    pub fn new() -> Self {
        UniqueQueue {
            queue: Vec::new(),
            set: HashSet::new(),
        }
    }

    /// Push an item into the queue.
    ///
    /// # Arguments
    /// `item` - The item to be pushed into the queue.
    pub fn push(&mut self, item: T) {
        if self.set.insert(item.clone()) {
            self.queue.push(item);
        }
    }

    /// Pop an item from the queue.
    ///
    /// # Returns
    /// `Some(item)` if the queue is not empty, otherwise `None`.
    pub fn pop(&mut self) -> Option<T> {
        if let Some(item) = self.queue.pop() {
            self.set.remove(&item);
            Some(item)
        } else {
            None
        }
    }

    /// Returns whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

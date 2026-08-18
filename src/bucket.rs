use std::collections::VecDeque;

use crate::id::NodeId;

pub struct KBucket {
    nodes: VecDeque<NodeId>, // A double-ended queue to store NodeIds in the bucket
    capacity: usize,         // Maximum number of nodes the bucket can hold
}

pub enum InsertionResult {
    Inserted,
    Refreshed,
    Full(NodeId), // The node that would be evicted if the bucket is full
}

impl KBucket {
    pub fn new(capacity: usize) -> Self {
        KBucket {
            nodes: VecDeque::with_capacity(capacity), // Initialize the VecDeque with the specified capacity
            capacity,
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len() // Return the current number of nodes in the bucket
    }

    pub fn is_full(&self) -> bool {
        self.nodes.len() >= self.capacity // Check if the bucket has reached its capacity
    }

    pub fn contains(&self, node_id: &NodeId) -> bool {
        self.nodes.contains(node_id) // Check if the bucket contains the specified NodeId
    }

    pub fn remove_node(&mut self, node_id: &NodeId) -> bool {
        if let Some(pos) = self.nodes.iter().position(|x| x == node_id) {
            self.nodes.remove(pos); // Remove the node if it exists
            true
        } else {
            false // Node not found
        }
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty() // Check if the bucket is empty
    }

    pub fn iter(&self) -> impl Iterator<Item = &NodeId> {
        self.nodes.iter() // Return an iterator over the NodeIds in the bucket
    }

    pub fn add_node(&mut self, node_id: &NodeId) -> InsertionResult {
        if self.contains(node_id) {
            // If the node already exists, move it to the back (most recently seen)
            self.nodes.retain(|&x| x != *node_id); // Remove the existing node
            self.nodes.push_back(*node_id); // Add it to the back
            InsertionResult::Refreshed
        } else {
            if !self.is_full() {
                // If there's space, simply add the new node
                self.nodes.push_back(*node_id);
                InsertionResult::Inserted
            } else {
                // If the bucket is full, we could implement eviction logic here
                // For simplicity, we'll just ignore new nodes when full
                // In a real implementation, we might ping the least recently seen node and evict it if unresponsive
                InsertionResult::Full(*self.nodes.front().unwrap()) // Return the node that would be evicted (the least recently seen)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kbucket_add_node() {
        let mut bucket = KBucket::new(3);
        let node1 = NodeId([1; 32]);
        let node2 = NodeId([2; 32]);
        let node3 = NodeId([3; 32]);
        let node4 = NodeId([4; 32]);

        assert!(matches!(bucket.add_node(&node1), InsertionResult::Inserted));
        assert!(matches!(bucket.add_node(&node2), InsertionResult::Inserted));
        assert!(matches!(bucket.add_node(&node3), InsertionResult::Inserted));
        assert!(matches!(bucket.add_node(&node4), InsertionResult::Full(_))); // Bucket is full
    }

    #[test]
    fn test_kbucket_remove_node() {
        let mut bucket = KBucket::new(3);
        let node1 = NodeId([1; 32]);
        let node2 = NodeId([2; 32]);
        let node3 = NodeId([3; 32]);

        assert!(matches!(bucket.add_node(&node1), InsertionResult::Inserted));
        assert!(matches!(bucket.add_node(&node2), InsertionResult::Inserted));
        assert!(matches!(bucket.add_node(&node3), InsertionResult::Inserted));

        assert!(matches!(bucket.remove_node(&node1), true));
        assert!(matches!(bucket.remove_node(&node1), false)); // Node is already removed
    }

    #[test]
    fn test_kbucket_is_empty() {
        let bucket = KBucket::new(3);
        assert!(bucket.is_empty());
    }

    #[test]
    fn test_kbucket_iter() {
        let mut bucket = KBucket::new(3);
        let node1 = NodeId([1; 32]);
        let node2 = NodeId([2; 32]);
        let node3 = NodeId([3; 32]);

        assert!(matches!(bucket.add_node(&node1), InsertionResult::Inserted));
        assert!(matches!(bucket.add_node(&node2), InsertionResult::Inserted));
        assert!(matches!(bucket.add_node(&node3), InsertionResult::Inserted));

        let nodes: Vec<&NodeId> = bucket.iter().collect();
        assert_eq!(nodes.len(), 3);
    }
}

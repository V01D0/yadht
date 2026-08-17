use std::collections::VecDeque;
use tracing::info;
use tracing_subscriber;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct NodeId(pub [u8; 32]); // Represents a 256-bit identifier for a node in the network

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, PartialOrd, Ord)]
pub struct Distance(pub [u8; 32]); // Represents the XOR distance between two NodeIds

pub fn init_tracing() {

    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        tracing_subscriber::fmt::init();
    });
}

pub fn distance(a: &NodeId, b: &NodeId) -> Distance {
    // Calculate the XOR distance between two NodeIds
    // XOR each byte of the two NodeIds to get the distance
    let mut result = [0; 32];
    for i in 0..32 {
        result[i] = a.0[i] ^ b.0[i]; // XOR bits to calculate distance
    }
    Distance(result)
}

pub fn bucket_index(distance: &Distance) -> Option<usize> {
    // Find the index of the first non-zero byte in the distance
    // Each byte represents 8 bits, so we need to find the first non-zero byte and then calculate the bit index
    for (i, byte) in distance.0.iter().enumerate() {
        if *byte != 0 {
            // Find the first non-zero byte
            let leading_zeros = byte.leading_zeros() as usize; // Find the highest set bit in the byte (0-7)
            return Some(255 - (i * 8 + leading_zeros)); // Calculate the bit index from the end of the array
        }
    }
    None // All bytes are zero, meaning the distance is zero
}

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

pub struct RoutingTable {
    buckets: Vec<KBucket>, // A vector of K-buckets, each representing a range of distances
    own_id: NodeId,        // The NodeId of the current node
}

impl RoutingTable {
    pub fn new(own_id: NodeId) -> Self {
        RoutingTable {
            buckets: Vec::new(),
            own_id,
        }
    }

    pub fn add_node(&mut self, node_id: NodeId) -> Option<InsertionResult> {
        let dist = distance(&self.own_id, &node_id); // Calculate the distance to the new node
        if let Some(index) = bucket_index(&dist) {
            // Ensure the buckets vector is large enough
            while self.buckets.len() <= index {
                self.buckets.push(KBucket::new(20)); // Assuming a default capacity of 20 for each bucket
            }
            let bucket = &mut self.buckets[index];
            let result = bucket.add_node(&node_id);
            match &result {
                InsertionResult::Inserted => {
                    println!("Node {:?} inserted into bucket {}", node_id, index);
                }
                InsertionResult::Refreshed => {
                    println!("Node {:?} refreshed in bucket {}", node_id, index);
                }
                InsertionResult::Full(evicted_node) => {
                    println!(
                        "Bucket {} is full. Node {:?} would be evicted.",
                        index, evicted_node
                    );
                }
            }
            Some(result)
        } else {
            println!(
                "Distance is zero; node {:?} is the same as own_id.",
                node_id
            );
            None
        }
    }

    pub fn find_closest_nodes(&self, target_id: &NodeId, count: usize) -> Vec<NodeId> {
        let mut nodes = Vec::new();
        let dist = distance(&self.own_id, target_id);
        if let Some(index) = bucket_index(&dist) {
            // Start searching from the bucket corresponding to the distance
            for i in (0..=index).rev() {
                if i < self.buckets.len() {
                    nodes.extend(self.buckets[i].iter().cloned());
                }
                if nodes.len() >= count {
                    break;
                }
            }
            for i in (index + 1)..self.buckets.len() {
                nodes.extend(self.buckets[i].iter().cloned());
                if nodes.len() >= count {
                    break;
                }
            }
        }
        nodes.sort_by_key(|id| distance(target_id, id));
        nodes.truncate(count); // Ensure we only return the requested number of nodes
        nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Simple splitmix64 PRNG — deterministic, no external crate needed for tests.
    fn random_node_id(seed: &mut u64) -> NodeId {
        let mut bytes = [0u8; 32];
        for chunk in bytes.chunks_mut(8) {
            *seed = seed.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = *seed;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^= z >> 31;
            chunk.copy_from_slice(&z.to_be_bytes());
        }
        NodeId(bytes)
    }

    #[test]
    fn test_distance_function() {
        let node_id_a = NodeId([1; 32]);
        let node_id_b = NodeId([2; 32]);
        let dist = distance(&node_id_a, &node_id_b);
        assert_eq!(dist, Distance([3; 32])); // 1 XOR 2 = 3 for each byte
    }

    #[test]
    fn distance_to_self_is_zero() {
        let node_id = NodeId([1; 32]);
        let dist = distance(&node_id, &node_id);
        assert_eq!(dist, Distance([0; 32]));
    }

    #[test]
    fn distance_is_symmetric() {
        let node_id_a = NodeId([1; 32]);
        let node_id_b = NodeId([2; 32]);
        let dist_ab = distance(&node_id_a, &node_id_b);
        let dist_ba = distance(&node_id_b, &node_id_a);
        assert_eq!(dist_ab, dist_ba);
    }

    #[test]
    fn distance_is_nonzero_for_distinct_ids() {
        let node_id_a = NodeId([1; 32]);
        let node_id_b = NodeId([2; 32]);
        let dist = distance(&node_id_a, &node_id_b);
        assert_ne!(dist, Distance([0; 32]));
    }

    #[test]
    fn test_bucket_index() {
        let node_id_a = NodeId([0; 32]);
        let node_id_b = NodeId([
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]); // Only the 8th byte
        let dist = distance(&node_id_a, &node_id_b);
        let index = bucket_index(&dist);
        assert_eq!(index, Some(192)); // The first non-zero byte is at index 7, leading zeros in that byte is 7, so 255 - (7 * 8 + 7) = 192
    }

    #[test]
    fn test_bucket_index_zero_distance() {
        let node_id_a = NodeId([0; 32]);
        let node_id_b = NodeId([0; 32]);
        let dist = distance(&node_id_a, &node_id_b);
        let index = bucket_index(&dist);
        assert_eq!(index, None); // Distance is zero, so no bucket index
    }

    #[test]
    fn test_bucket_index_all_bytes_nonzero() {
        let node_id_a = NodeId([0; 32]);
        let node_id_b = NodeId([255; 32]); // All bytes are non-zero
        let dist = distance(&node_id_a, &node_id_b);
        let index = bucket_index(&dist);
        assert_eq!(index, Some(255)); // The first non-zero byte is at index 0, leading zeros in that byte is 0, so 255 - (0 * 8 + 0) = 255
    }

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

    #[test]
    fn test_routing_table_add_node_basic() {
        let own_id = NodeId([0; 32]);
        let mut table = RoutingTable::new(own_id);
        let peer = NodeId([1; 32]);

        assert!(matches!(
            table.add_node(peer),
            Some(InsertionResult::Inserted)
        ));
        assert!(matches!(
            table.add_node(peer),
            Some(InsertionResult::Refreshed)
        ));
    }

    #[test]
    fn test_routing_table_self_insert_is_noop() {
        let own_id = NodeId([7; 32]);
        let mut table = RoutingTable::new(own_id);

        assert!(table.add_node(own_id).is_none());
    }

    #[test]
    fn test_routing_table_bucket_overflow() {
        let own_id = NodeId([0; 32]);
        let mut table = RoutingTable::new(own_id);

        // Varying only byte 0, keeping its top bit set, keeps every one of these
        // in the same (farthest) bucket relative to own_id — lets us fill one
        // bucket to capacity through the RoutingTable's own routing logic,
        // not by touching KBucket directly.
        for i in 1u8..=20 {
            let mut bytes = [0u8; 32];
            bytes[0] = 0x80 + i;
            assert!(matches!(
                table.add_node(NodeId(bytes)),
                Some(InsertionResult::Inserted)
            ));
        }

        let mut overflow_bytes = [0u8; 32];
        overflow_bytes[0] = 0x95; // 21st distinct id targeting the same bucket
        assert!(matches!(
            table.add_node(NodeId(overflow_bytes)),
            Some(InsertionResult::Full(_))
        ));
    }

    #[test]
    fn test_find_closest_nodes_matches_brute_force() {
        let own_id = NodeId([0; 32]);
        let mut table = RoutingTable::new(own_id);
        let mut seed = 42u64;

        // Some of these 200 will be rejected (bucket full) — that's expected,
        // not a bug. "Ground truth" has to be built from what actually made it
        // into the table, not the full random pool.
        let mut inserted = Vec::new();
        for _ in 0..200 {
            let peer = random_node_id(&mut seed);
            let result = table.add_node(peer);
            if matches!(
                result,
                Some(InsertionResult::Inserted) | Some(InsertionResult::Refreshed)
            ) {
                inserted.push(peer);
            }
        }

        let target = random_node_id(&mut seed);
        let count = 20;

        let mut expected = inserted.clone();
        expected.sort_by_key(|id| distance(&target, id));
        expected.truncate(count);

        let actual = table.find_closest_nodes(&target, count);
        assert_eq!(actual, expected);
    }
}

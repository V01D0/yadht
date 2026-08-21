use crate::bucket::{InsertionResult, KBucket};
use crate::id::{bucket_index, distance, NodeId};

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

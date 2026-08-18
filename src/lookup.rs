use std::collections::HashMap;

// Map node to routing table and find closest nodes
use crate::distance;
use crate::{NodeId, RoutingTable};

// // Finds the closest nodes to a given node_id in the provided routing table.
// pub fn find_closest_nodes(node_id: &NodeId, routing_table: &RoutingTable, count: usize) -> Vec<NodeId> {
//     routing_table.find_closest_nodes(node_id, count)
// }

pub struct Network {
    nodes: HashMap<NodeId, RoutingTable>,
}

impl Network {
    pub fn new() -> Self {
        Network {
            nodes: HashMap::new(),
        }
    }

    pub fn register_peer(&mut self, id: NodeId) {
        self.nodes
            .entry(id)
            .or_insert_with(|| RoutingTable::new(id));
    }

    pub fn introduce(&mut self, from: &NodeId, about: NodeId) {
        // Introduce a node to another node's routing table
        if let Some(table) = self.nodes.get_mut(from) {
            table.add_node(about);
        }
    }

    pub fn query_node(&self, from: &NodeId, target: &NodeId, count: usize) -> Vec<NodeId> {
        // Return vector of closest nodes or empty vector if from node not found
        self.nodes
            .get(from)
            .map(|table| table.find_closest_nodes(target, count))
            .unwrap_or_else(Vec::new) // Return vector of closest nodes or empty vector if from node not found
    }

    pub fn iterative_lookup(&self, start: &NodeId, target: &NodeId, count: usize) -> Vec<NodeId> {
        let mut queried = std::collections::HashSet::new();
        let mut closest_nodes = self.query_node(start, target, count);

        loop {
            let mut new_closest = Vec::new();
            for node in closest_nodes.iter() {
                if !queried.contains(node) {
                    queried.insert(*node);
                    new_closest.extend(self.query_node(node, target, count));
                }
            }
            new_closest.extend(closest_nodes.iter().cloned());
            new_closest.sort_by_key(|n| distance(n, target));
            new_closest.dedup();
            let updated: Vec<_> = new_closest.into_iter().take(count).collect();

            if updated == closest_nodes {
                break; // a round produced nothing new — converged
            }
            closest_nodes = updated;
        }

        closest_nodes
    }
}

#[cfg(test)]
mod tests {
    use crate::NodeId;
    use crate::lookup::Network;
    #[test]
    pub fn simulate_lookup() {
        // Simulate a lookup operation in the routing table
        let mut mock_network = Network::new();
        mock_network.register_peer(NodeId([0; 32]));
        mock_network.register_peer(NodeId([1; 32]));
        mock_network.introduce(&NodeId([0; 32]), NodeId([1; 32]));

        let closest_nodes = mock_network.query_node(&NodeId([0; 32]), &NodeId([1; 32]), 5);
        assert_eq!(closest_nodes.len(), 1);
        assert_eq!(closest_nodes[0], NodeId([1; 32]));
    }

    #[test]
    fn test_iterative_lookup_converges_across_fragmented_network() {
        let mut network = Network::new();
        let a = NodeId([0; 32]);
        let b = NodeId([1; 32]);
        let c = NodeId([2; 32]);
        let target = NodeId([9; 32]);

        network.register_peer(a);
        network.register_peer(b);
        network.register_peer(c);

        network.introduce(&a, b); // a only knows about b
        network.introduce(&b, c); // b only knows about c
        network.introduce(&c, target); // c is the only one who knows about the actual target

        // a has no direct knowledge of `target` — only reachable via b, then c
        let result = network.iterative_lookup(&a, &target, 5);
        assert!(result.contains(&target));
    }
}

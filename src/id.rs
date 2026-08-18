#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct NodeId(pub [u8; 32]); // Represents a 256-bit identifier for a node in the network

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, PartialOrd, Ord)]
pub struct Distance(pub [u8; 32]); // Represents the XOR distance between two NodeIds

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

#[cfg(test)]
mod tests {
    use super::*;

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
}

mod bucket;
mod id;
mod routing_table;

pub use bucket::{InsertionResult, KBucket};
pub use id::{bucket_index, distance, Distance, NodeId};
pub use routing_table::RoutingTable;

pub fn init_tracing() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        tracing_subscriber::fmt::init();
    });
}

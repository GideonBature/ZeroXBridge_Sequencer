pub mod l1_event_watcher;
pub mod l2_event_watcher;

pub use l2_event_watcher::{fetch_l2_events, CommitmentLog};
pub use l1_event_watcher::{fetch_l1_deposit_events_with_provider, TestEthereumProvider};

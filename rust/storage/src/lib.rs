#![warn(clippy::dbg_macro)]
#![warn(clippy::disallowed_methods)]
#![warn(clippy::doc_markdown)]
#![warn(clippy::explicit_into_iter_loop)]
#![warn(clippy::explicit_iter_loop)]
#![warn(clippy::inconsistent_struct_constructor)]
#![warn(clippy::map_flatten)]
#![warn(clippy::no_effect_underscore_binding)]
#![warn(clippy::await_holding_lock)]
#![deny(unused_must_use)]
#![feature(trait_alias)]
#![feature(generic_associated_types)]
#![feature(binary_heap_drain_sorted)]
#![feature(drain_filter)]
#![feature(bound_map)]
#![feature(backtrace)]
#![feature(map_first_last)]
#![feature(let_chains)]

pub mod cell_based_row_deserializer;
pub mod cell_based_row_serializer;
pub mod hummock;
pub mod keyspace;
pub mod memory;
pub mod monitor;
pub mod object;
pub mod panic_store;
pub mod store;
pub mod store_impl;
pub mod table;
pub mod write_batch;

#[cfg(feature = "rocksdb-local")]
pub mod rocksdb_local;
#[cfg(not(feature = "rocksdb-local"))]
#[path = "rocksdb_local_mock.rs"]
pub mod rocksdb_local;

#[cfg(feature = "tikv")]
pub mod tikv;
#[cfg(not(feature = "tikv"))]
#[path = "tikv_mock.rs"]
pub mod tikv;

pub use keyspace::{Keyspace, Segment};
pub use store::{StateStore, StateStoreIter};
pub use store_impl::StateStoreImpl;

pub enum TableScanOptions {
    SequentialScan,
    SparseIndexScan,
}

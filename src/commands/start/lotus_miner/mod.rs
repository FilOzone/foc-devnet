//! Lotus-Miner step.
//!
//! This module handles starting the Lotus-Miner container, which is the first
//! generation miner node that builds tipsets and performs PoRep (Proof of Replication).

pub mod constants;
pub mod container_ops;
pub mod docker_command;
pub mod lotus_miner_step;
pub mod setup;
pub mod verification;

pub use lotus_miner_step::LotusMinerStep;

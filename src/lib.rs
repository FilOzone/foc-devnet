//! foc-localnet library crate
//!
//! This crate provides the core functionality for managing local Filecoin
//! on-chain cloud clusters for testing purposes.

pub mod app;
pub mod cli;
pub mod commands;
pub mod config;
pub mod crypto;
pub mod docker;
pub mod embedded_assets;
pub mod paths;
pub mod poison;

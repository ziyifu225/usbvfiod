//! # Device Emulation Code
//!
//! This crate contains device emulation code. It
//! should never depend on hypervisor, x86 or Linux specific parts.

#![deny(missing_docs)]
#![deny(rustdoc::all)]
#![deny(clippy::must_use_candidate)]
#![deny(missing_debug_implementations)]

pub mod bus;
pub mod interval;
pub mod msi_receiver;
pub mod pci;
pub mod register_set;

//! # PCI Local Bus Emulation
//!
//! The PCI Local Bus is the central component for attaching devices to a virtual machine. This
//! module contains the generic PCI emulation logic in [`root`]. Specific devices are implemented in
//! their own modules.
//!
//! ## Example
//!
//! To use the PCI emulation code, you must select a host bridge. So far we implement the [`i440FX`
//! host bridge](i440fx) that is also used by Qemu and other VMMs. With this host bridge, you can
//! then create a [builder](root::PciRootBuilder) that allows you to [add
//! devices](root::PciRootBuilder::add_device) and eventually
//! [finalize](root::PciRootBuilder::pci_root) the PCI root.
//!
//! ```rust
//! use std::sync::{Arc, Mutex};
//! use particle_devices::{bus::Bus, pci::{i440fx, root}};
//! use particle_devices::interrupt_line::DummyInterruptLine;
//! use particle_devices::power_button::PowerButton;
//!
//! #[derive(Clone, Debug)]
//! struct HandleDummy;
//!
//! # use particle_devices::lifecycle;
//! # impl lifecycle::EventSender for HandleDummy {
//! #     fn send_event(&self, _: lifecycle::Event) {
//! #     }
//! # }
//! // Pass a real VM lifecycle handle here.
//! let vm_handle = HandleDummy;
//!
//! let host_bridge = Mutex::new(i440fx::I440fxHostBridge::new(
//!     vm_handle,
//!     Box::new(DummyInterruptLine::default()),
//!     Arc::new(Mutex::new(PowerButton::new())))
//! );
//! let pci_root = root::PciRootBuilder::new(host_bridge)
//!                  .pci_root();
//!
//! let mut memory_bus = Bus::new_with_default("memory", Arc::new(pci_root.mmio_space()));
//! let mut pio_bus = Bus::new_with_default("port I/O", Arc::new(pci_root.pio_space()));
//! ```

pub mod config_space;
pub mod constants;
pub mod msix_table;
pub mod traits;

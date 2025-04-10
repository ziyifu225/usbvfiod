//! # PCI Core Traits
//!
//! This module contains the core traits for PCI emulation. See [`PciDevice`].

use std::fmt::Debug;
use std::{collections::BTreeMap, sync::Arc};

use crate::device::bus::Request;

/// The type of I/O region request for a PCI device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestKind {
    /// A legacy x86 port I/O request. Usually made via `IN` or `OUT` instructions.
    PortIO,

    /// A MMIO request.
    Memory,
}

/// The interface a device has to implement to be added to a PCI bus.
///
/// PCI devices have to respond to requests in three different "address spaces":
///
/// - PCI Configuration Space,
/// - Port I/O, and
/// - memory-mapped I/O.
///
/// The most straight-forward one is the PCI Configuration Space. Devices always have to respond to
/// requests to it to be recognized on the bus.
///
/// Whether devices claim Port I/O or memory requests depend on their configuration. In a better
/// world, the PCI bus would only have to look at their Base Address Registers (BARs) in their
/// Configuration Space to see which port I/O and memory requests to send to them. In reality, this
/// is not sufficient, because devices may claim more requests than their BARs suggest. Examples
/// here are VGA controllers or the PIIX4 PM device that has non-standard BARs for certain I/O
/// regions.
///
/// To avoid having to deal with device-specific quirks, the PCI bus leaves it up to devices whether
/// they claim port I/O or memory requests.
pub trait PciDevice: Debug {
    /// Write to the PCI Configuration Space of the device.
    ///
    /// # Parameters
    ///
    /// `req`: The address and size of the request.
    /// `value`: The value to be written.
    fn write_cfg(&self, req: Request, value: u64);

    /// Read from the PCI Configuration Space of the device.
    ///
    /// # Parameters
    ///
    /// `req`: The address and size of the request.
    fn read_cfg(&self, req: Request) -> u64;

    /// Write a value to an I/O region.
    ///
    /// The device may or may not claim the request. If the request is claimed, this function
    /// returns `Some(())` otherwise `None`.
    ///
    /// A device that only claims requests via standard BARs can use
    /// [`try_match_bar`](super::config_space::ConfigSpace::try_match_bar) to implement this function.
    ///
    /// # Parameters
    ///
    /// - `kind`: Specifies the type of request.
    /// - `req`: The offset and size of the request. Offsets are relative to the beginning of each
    ///          I/O region.
    /// - `value`: The value to be written.
    #[must_use]
    fn try_io_write(&self, kind: RequestKind, req: Request, value: u64) -> Option<()>;

    /// Read a value from an I/O region.
    ///
    /// The device may or may not claim the request. If the request is claimed, this function
    /// returns `Some(value)` otherwise `None`.
    ///
    /// A device that only claims requests via standard BARs can use
    /// [`try_match_bar`](super::config_space::ConfigSpace::try_match_bar) to implement this function.
    ///
    /// # Parameters
    ///
    /// - `kind`: Specifies the type of request.
    /// - `req`: The offset and size of the request. Offsets are relative to the beginning of each
    ///          I/O region.
    #[must_use]
    fn try_io_read(&self, kind: RequestKind, req: Request) -> Option<u64>;
}

/// An integer denoting a device/function pair.
///
/// This is used to address devices on a single PCI bus.
///
/// The device part are bits 7:3 and the function part is bits 2:0.
pub type DeviceFunctionIndex = u8;

/// A map from device-function to a PCI device reference.
pub type DeviceFunctionMap = BTreeMap<DeviceFunctionIndex, PciDeviceRef>;

/// A special PCI device that can be a PCI host bridge at BDF 0:0.0.
pub trait HostBridge: PciDevice {
    /// Returns a map of companion devices for the host bridge.
    ///
    /// These devices are always automatically added to the PCI bus when it is created.
    fn reserved_devices(&self) -> DeviceFunctionMap;
}

/// A reference-counted reference to a PCI device.
pub type PciDeviceRef = Arc<dyn PciDevice + Send + Sync>;

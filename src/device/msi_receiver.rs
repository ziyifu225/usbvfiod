//! # MSI Receiver
//!
//! This module contains a trait [`MsiReceiver`] which allows receiving Message-Signaled Interrupts
//! (MSIs) with custom Address and Data type. Objects that implement this trait are supposed to be
//! used by virtual devices that send MSIs.

use std::fmt::Debug;

/// The address/data pair for an MSI.
///
/// The PCI specification makes no limitations here. The interpretation of address and data is
/// entirely platform specific. For x86, refer to the Intel SDM Vol 3. Chapter 10.11 "Message
/// Signalled Interrupts".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MsiMessage {
    /// The guest physical address where the MSI message is sent to.
    ///
    /// As a rule of thumb, this value determines which CPU the interrupt is routed to.
    pub address: u64,

    /// The payload of the MSI.
    ///
    /// As a rule of thumb, this value determines what interrupt vector is triggered on the selected
    /// CPU.
    pub data: u16,
}

impl MsiMessage {
    /// Create a new [`MsiMessage`] struct.
    #[must_use]
    pub fn new(address: u64, data: u16) -> Self {
        Self { address, data }
    }
}

/// Trait for all receivers of [`MsiMessage`]s.
///
/// It provides a single function for sending MSIs with a 64-bit address field and a 16-bit data
/// field.
///
/// Devices using this trait to send interrupts are supposed to be implemented like this:
///
/// ```
/// use usbvfiod::device::msi_receiver::{MsiReceiver, MsiMessage};
///
/// struct Device {
///     vector: MsiMessage,
///     msi_receiver: std::rc::Rc<dyn MsiReceiver>,
/// }
///
/// impl Device {
///     fn send_interrupt(&self) {
///         self.msi_receiver.send_msi(self.vector);
///     }
/// }
///```
pub trait MsiReceiver: Debug + Send + Sync {
    /// Sends a single MSI to the receiver.
    fn send_msi(&self, msi: MsiMessage);
}

/// A do-nothing implementation of [`MsiReceiver`] useful for testing or prototyping.
///
/// Any MSIs sent to this receiver are silently dropped.
#[derive(Debug, Clone, Copy, Default)]
pub struct DummyMsiReceiver {}

impl DummyMsiReceiver {
    /// Create a new dummy MSI receiver.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl MsiReceiver for DummyMsiReceiver {
    fn send_msi(&self, _msi: MsiMessage) {
        // The dummy receiver intentionally does nothing when it receives an MSI.
    }
}

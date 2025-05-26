//! # Interrupt Line
//!
//! This module exposes an abstract [`InterruptLine`] trait, which is supposed
//! to be implemented by interrupt controller devices or other mechanisms that
//! can raise an interrupt.
//! Other devices can then use such an interrupt line to trigger
//! interrupts without any knowledge about the receiving controller.

use std::fmt::Debug;

/// An interrupt line with a single operation: [`InterruptLine::interrupt`].
pub trait InterruptLine: Debug + Send + Sync + 'static {
    /// Send a single edge-triggered interrupt to the interrupt controller.
    fn interrupt(&self);
}

/// A dummy interrupt line that is intended to be used by devices whose
/// interrupts aren't wired to any interrupt controller.
#[derive(Default, Debug, Clone, Copy)]
pub struct DummyInterruptLine {}

impl InterruptLine for DummyInterruptLine {
    fn interrupt(&self) {}
}

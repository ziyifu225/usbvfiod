//! # PCI Constants
//!
//! This module collects PCI related constants. All definitions are derived from the PCI
//! Spec, either the "PCI Local Bus Specification" or newer "PCI Express Base Specification"
//! documents.

// Allow missing docs to avoid duplicating the PCI spec for all constants.
#![allow(missing_docs)]

/// The maximum number of busses on a PCI segment.
pub const MAX_BUSES: usize = 256;

/// The maximum number of devices on a PCI bus.
pub const MAX_BUS_DEVICES: usize = 32;

/// The maximum number of functions in a PCI device.
pub const MAX_DEVICE_FUNCTIONS: usize = 8;

/// The maximum number of devices on a PCI segment.
pub const MAX_DEVICES: usize = MAX_BUSES * MAX_BUS_DEVICES * MAX_DEVICE_FUNCTIONS;

/// Constants related to the configuration space.
pub mod config_space {

    /// The config space size of a single PCI device in bytes.
    pub const SIZE: usize = 256;

    /// The maximum number of Base Address Registers (BARs) per device.
    pub const MAX_BARS: usize = 6;

    /// The size in bytes of a single BAR.
    pub const BAR_ENTRY_SIZE: usize = 4;

    /// Masks for various configuration space fields.
    pub mod mask {
        pub const CAPABILITIES_POINTER: u8 = 0xfc;
        pub const PIO_BAR_MARKER: u64 = 0x1;
        pub const PIO_BAR_ADDRESS: u64 = 0xffff_fffc;
        pub const MMIO_BAR_TYPE: u64 = 0x6;
        pub const MMIO_BAR_64_BIT: u64 = 0x4;
        pub const MMIO_BAR_ADDRESS: u64 = 0xffff_fff0;
    }

    /// The offsets of various fields in the configuration space.
    pub mod offset {
        pub const VENDOR: usize = 0x0;
        pub const DEVICE: usize = 0x2;
        pub const COMMAND: usize = 0x4;
        pub const STATUS: usize = 0x6;
        pub const REVISION: usize = 0x8;
        pub const PROG_IF: usize = 0x9;
        pub const SUBCLASS: usize = 0xA;
        pub const CLASS: usize = 0xB;
        pub const CACHE_LINE_SIZE: usize = 0xC;
        pub const LATENCY_TIMER: usize = 0xD;
        pub const HEADER_TYPE: usize = 0xE;
        pub const BIST: usize = 0xF;

        pub const BAR_0: usize = 0x10;
        pub const BAR_1: usize = 0x14;
        pub const BAR_2: usize = 0x18;
        pub const BAR_3: usize = 0x1C;
        pub const BAR_4: usize = 0x20;
        pub const BAR_5: usize = 0x24;

        pub const SUBSYSTEM_VENDOR_ID: usize = 0x2C;
        pub const SUBSYSTEM_ID: usize = 0x2E;
        pub const ROM_BAR: usize = 0x30;
        pub const CAPABILITIES_POINTER: usize = 0x34;
        pub const IRQ_LINE: usize = 0x3C;
        pub const IRQ_PIN: usize = 0x3D;
        pub const MIN_GNT: usize = 0x3E;
        pub const MAX_LAT: usize = 0x3F;
    }

    /// The device vendor.
    pub mod vendor {
        pub const INVALID: u16 = 0xFFFF;
        pub const INTEL: u16 = 0x8086;
        pub const REDHAT: u16 = 0x1b36;
        pub const VIRTIO: u16 = 0x1AF4;
    }

    pub mod device {
        pub const INVALID: u16 = 0xFFFF;
        pub const I440FX_HOST_BRIDGE: u16 = 0x1237;
        pub const PIIX4_ISA_BRIDGE: u16 = 0x7110;
        pub const PIIX4_PM_DEVICE: u16 = 0x7113;
        pub const REDHAT_XHCI: u16 = 0x000d;

        /// Virtio devices occupy a range of device IDs.
        ///
        /// The concrete device ID is computed by adding the virtio-specific type ID to this value.
        pub const VIRTIO_DEVICE: u16 = 0x1040;
    }

    /// Command Register Constants.
    pub mod command {
        pub const WRITABLE_BITS: u16 = 0x077F;
    }

    /// Status Register Constants.
    pub mod status {
        /// The device has a list of capabilities starting at
        /// [`CAPABILITIES_POINTER`](super::offset::CAPABILITIES_POINTER).
        pub const CAPABILITIES: u16 = 1 << 4;
    }

    /// PCI class constants.
    pub mod class {
        pub const BRIDGE: u8 = 0x6;
        pub const SERIAL: u8 = 0xc;
        pub const UNASSIGNED: u8 = 0xFF;
    }

    /// PCI sub-class constants.
    pub mod subclass {
        pub const HOST_BRIDGE: u8 = 0x0;
        pub const PCI_TO_ISA_BRIDGE: u8 = 0x1;
        pub const OTHER_BRIDGE: u8 = 0x80;
        pub const SERIAL_USB: u8 = 0x03;
        pub const UNASSIGNED: u8 = 0xFF;
    }

    /// PCI programming interface constants.
    pub mod progif {
        pub const USB_XHCI: u8 = 0x30;
    }

    /// PCI header type.
    ///
    /// This is usually type 0, except for PCI-to-PCI bridges and other exotic devices such as
    /// Cardbus bridges.
    pub mod header_type {
        pub const TYPE_00: u8 = 0;
        pub const MULTIFUNCTION: u8 = 1 << 7;
    }

    /// IDs for PCI Capabilities.
    pub mod capability_id {
        pub const MSI: u8 = 0x05;
        pub const VENDOR_SPECIFIC: u8 = 0x09;
        pub const MSI_X: u8 = 0x11;
    }

    /// Markers for iterating the list of capabilities.
    pub mod capability_list {
        pub const END_OF_LIST: u8 = 0;
    }

    /// Constants for the MSI capability.
    pub mod msi {
        /// Size of the capability in bytes.
        pub const SIZE: usize = 16;

        /// The offset of the message control register.
        pub const CONTROL: u64 = 2;
        /// The offset of the lower address part.
        pub const ADDRESS_LOW: u64 = 4;
        /// The offset of the high address part of a 64 bit address.
        pub const ADDRESS_HIGH: u64 = 8;
        /// The offset of the data field.
        pub const DATA: u64 = 12;

        /// Constants for the Control field.
        pub mod control {
            pub const ENABLE: u16 = 1 << 0;
        }
    }

    /// Constants for the MSI-X capability.
    pub mod msix {
        /// The size of the MSI-X capability.
        pub const SIZE: usize = 12;

        /// The maximum number of MSI-X vectors.
        ///
        /// Note that the table size field in the [`control`] register contains the _last valid
        /// index_, not the maximum number.
        pub const MAX_VECTORS: u16 = 0x800;

        /// The offset of the message control register.
        pub const CONTROL: u64 = 2;
        /// The offset for MSI-X Table Offset and BAR indicator.
        pub const TABLE_INFO: u64 = 4;
        /// The offset for MSI-X Pending Bit Array Offset and BAR indicator.
        pub const PBA_INFO: u64 = 8;

        /// Masks of the table info field.
        pub mod table_info {
            pub const REGION: u8 = 0b111;
            pub const OFFSET: u32 = !0b111;
        }

        /// Constants for the Control field.
        pub mod control {
            pub const ENABLE: u16 = 1 << 15;
            pub const FUNCTION_MASK: u16 = 1 << 14;

            pub const WRITABLE_BITS: u16 = ENABLE | FUNCTION_MASK;
        }
    }
}

/// Constants related to the XHCI MMIO space.
pub mod xhci {

    /// Value for the operational base as returned for reading CAPLENGTH.
    pub const OP_BASE: u64 = 0x68;
    /// Runtime register base offset.
    pub const RUN_BASE: u64 = 0x3000;

    /// Offsets of various fields from the start of the XHCI MMIO region.
    pub mod offset {
        /// Capability Register Offsets
        pub const CAPLENGTH: u64 = 0x0;
        pub const HCIVERSION: u64 = 0x2;
        pub const HCSPARAMS1: u64 = 0x4;
        pub const HCSPARAMS2: u64 = 0x8;
        pub const HCSPARAMS3: u64 = 0xc;
        pub const HCCPARAMS1: u64 = 0x10;
        pub const DBOFF: u64 = 0x14;
        pub const RTSOFF: u64 = 0x18;
        pub const HCCPARAMS2: u64 = 0x1c;

        /// Operational Register Offsets
        pub const USBCMD: u64 = super::OP_BASE;
        pub const USBSTS: u64 = super::OP_BASE + 0x4;
        pub const PAGESIZE: u64 = super::OP_BASE + 0x8;
        pub const CONFIG: u64 = super::OP_BASE + 0x38;

        /// Runtime Register Offsets
        pub const IMAN: u64 = super::RUN_BASE;
        pub const IMOD: u64 = super::RUN_BASE + 0x4;
        pub const ERSTSZ: u64 = super::RUN_BASE + 0x8;
        pub const ERSTBA: u64 = super::RUN_BASE + 0x10;
        pub const ERDB: u64 = super::RUN_BASE + 0x18;
    }

    /// Constants for the capability register.
    pub mod capability {}

    /// Constants for the operational registers.
    pub mod operational {}
}

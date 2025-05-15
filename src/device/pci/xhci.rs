//! Emulation of a USB3 Host (XHCI) controller.
//!
//! The specification is available
//! [here](https://www.intel.com/content/dam/www/public/us/en/documents/technical-specifications/extensible-host-controler-interface-usb-xhci.pdf).

use std::sync::Mutex;
use tracing::{debug, warn};

use crate::device::{
    bus::{BusDeviceRef, Request, SingleThreadedBusDevice},
    pci::{
        config_space::{ConfigSpace, ConfigSpaceBuilder},
        constants::xhci::{capability, offset, operational::crcr, MAX_SLOTS, OP_BASE, RUN_BASE},
        traits::PciDevice,
    },
};

/// A Basic Event Ring.
#[derive(Debug, Default, Clone)]
pub struct EventRing {
    base_address: u64,
    dequeue_pointer: u64,
}

/// The emulation of a XHCI controller.
#[derive(Debug, Clone)]
pub struct XhciController {
    /// A reference to the VM memory to perform DMA on.
    #[allow(unused)]
    dma_bus: BusDeviceRef,

    /// The PCI Configuration Space of the controller.
    config_space: ConfigSpace,

    /// The current Run/Stop status of the controller.
    running: bool,

    /// The current Run/Stop status of the command ring.
    command_ring_running: bool,

    /// Internal Command Ring position.
    command_ring_dequeue_pointer: u64,

    /// The Event Ring of the single Interrupt Register Set.
    event_ring: EventRing,

    /// Internal Consumer Cycle State for the next TRB fetch.
    consumer_cycle_state: bool,

    /// Configured device slots.
    slots: Vec<()>,

    /// Device Context Array
    /// TODO: currently just the raw pointer configured by the OS
    device_contexts: Vec<u64>,
}

impl XhciController {
    /// Create a new XHCI controller with default settings.
    ///
    /// `dma_bus` is the device on which we will perform DMA
    /// operations. This is typically VM guest memory.
    #[must_use]
    pub fn new(dma_bus: BusDeviceRef) -> Self {
        use crate::device::pci::constants::config_space::*;

        Self {
            dma_bus,
            config_space: ConfigSpaceBuilder::new(vendor::REDHAT, device::REDHAT_XHCI)
                .class(class::SERIAL, subclass::SERIAL_USB, progif::USB_XHCI)
                // TODO Should be a 64-bit BAR.
                .mem32_nonprefetchable_bar(0, 4 * 0x1000)
                .config_space(),
            running: false,
            command_ring_running: false,
            command_ring_dequeue_pointer: 0,
            consumer_cycle_state: false,
            event_ring: EventRing::default(),
            slots: vec![],
            device_contexts: vec![],
        }
    }

    /// Obtain the current host controller status as defined for the `USBSTS` register.
    #[must_use]
    pub fn status(&self) -> u64 {
        !u64::from(self.running) & 0x1u64
    }

    /// Obtain the current command ring status as defined for reading the `CRCR` register.
    #[must_use]
    pub fn command_ring_status(&self) -> u64 {
        // All fields except CRR (command ring running) read as zero.
        (u64::from(self.command_ring_running) << 3) & 0b100
    }

    /// Obtain the current host controller configuration as defined for the `CONFIG` register.
    #[must_use]
    pub fn config(&self) -> u64 {
        u64::try_from(self.slots.len()).unwrap() & 0x8u64
    }

    /// Obtain the Event Ring Segment Table Base Address.
    #[must_use]
    pub fn event_ring_base_address(&self) -> u64 {
        self.event_ring.base_address
    }

    /// Enable device slots.
    pub fn enable_slots(&mut self, count: u64) {
        assert!(count <= MAX_SLOTS);

        self.slots = (0..count).map(|_| ()).collect();

        debug!("enabled {} device slots", self.slots.len());
    }

    /// Configure the device context array from the array base pointer.
    pub fn configure_device_contexts(&mut self, device_context_base_array_ptr: u64) {
        debug!(
            "configuring device contexts from pointer {:#x}",
            device_context_base_array_ptr
        );
        self.device_contexts.clear();
        self.device_contexts.push(device_context_base_array_ptr);
    }

    /// Configure the Event Ring Segment Table from the base address.
    pub fn configure_event_ring_segment_table(&mut self, erstba: u64) {
        assert_eq!(erstba & 0x3f, 0, "unaligned event ring base address");

        self.event_ring.base_address = erstba;

        debug!("event ring segment table at {:#x}", erstba);
    }

    /// Handle writes to the Event Ring Dequeue Pointer (ERDP).
    pub fn update_event_ring(&mut self, value: u64) {
        debug!("event ring dequeue pointer advanced to {:#x}", value);
        self.event_ring.dequeue_pointer = value;
    }

    /// Handle Command Ring Control Register (CRCR) updates.
    pub fn update_command_ring(&mut self, value: u64) {
        if self.command_ring_running {
            match value {
                abort if abort & crcr::CA != 0 => todo!(),
                stop if stop & crcr::CS != 0 => todo!(),
                ignored => {
                    warn!(
                        "received useless write to CRCR while running {:#x}",
                        ignored
                    )
                }
            }
        } else {
            let dequeue_ptr = value & crcr::DEQUEUE_POINTER_MASK;
            if self.command_ring_dequeue_pointer != dequeue_ptr {
                debug!(
                    "updating command ring dequeue ptr from {:#x} to {:#x}",
                    self.command_ring_dequeue_pointer, dequeue_ptr
                );
                self.command_ring_dequeue_pointer = dequeue_ptr;
            }
            // Update internal consumer cycle state for next TRB fetch.
            self.consumer_cycle_state = value & crcr::RCS != 0;
        }
    }
}

impl PciDevice for Mutex<XhciController> {
    fn write_cfg(&self, req: Request, value: u64) {
        self.lock().unwrap().config_space.write(req, value);
    }

    fn read_cfg(&self, req: Request) -> u64 {
        self.lock().unwrap().config_space.read(req)
    }

    fn write_io(&self, region: u32, req: Request, value: u64) {
        // The XHCI Controller has a single MMIO BAR.
        assert_eq!(region, 0);

        match req.addr {
            // xHC Operational Registers
            offset::USBCMD => match value {
                val if val & 0x1 == 0 => (), /* stop */
                _ => todo!(),
            },
            offset::CRCR => self.lock().unwrap().update_command_ring(value),
            offset::CRCR_HI => assert_eq!(value, 0, "no support for configuration above 4G"),
            offset::DCBAAP => self.lock().unwrap().configure_device_contexts(value),
            offset::DCBAAP_HI => assert_eq!(value, 0, "no support for configuration above 4G"),
            offset::CONFIG => self.lock().unwrap().enable_slots(value),

            // xHC Runtime Registers
            offset::ERSTSZ => assert_eq!(value, 1, "only a single segment supported"),
            offset::ERSTBA => self
                .lock()
                .unwrap()
                .configure_event_ring_segment_table(value),
            offset::ERSTBA_HI => assert_eq!(value, 0, "no support for configuration above 4G"),
            offset::ERDP => self.lock().unwrap().update_event_ring(value),
            offset::ERDP_HI => assert_eq!(value, 0, "no support for configuration above 4G"),
            _ => todo!(),
        }
    }

    fn read_io(&self, region: u32, req: Request) -> u64 {
        // The XHCI Controller has a single MMIO BAR.
        assert_eq!(region, 0);

        match req.addr {
            // xHC Capability Registers
            offset::CAPLENGTH => OP_BASE,
            offset::HCIVERSION => capability::HCIVERSION,
            offset::HCSPARAMS1 => capability::HCSPARAMS1,
            offset::HCSPARAMS2 => 0, /* ERST Max size is a single segment */
            offset::HCSPARAMS3 => 0,
            offset::HCCPARAMS1 => capability::HCCPARAMS1,
            offset::DBOFF => 0x2000,
            offset::RTSOFF => RUN_BASE,
            offset::HCCPARAMS2 => 0,

            // xHC Extended Capability ("Supported Protocols Capability")
            offset::SUPPORTED_PROTOCOLS => capability::supported_protocols::CAP_INFO,
            offset::SUPPORTED_PROTOCOLS_CONFIG => capability::supported_protocols::CONFIG,

            // xHC Operational Registers
            offset::USBCMD => 0,
            offset::USBSTS => self.lock().unwrap().status(),
            offset::CRCR => self.lock().unwrap().command_ring_status(),
            offset::CRCR_HI => 0,
            offset::PAGESIZE => 0x1, /* 4k Pages */
            offset::CONFIG => self.lock().unwrap().config(),

            // xHC Runtime Registers
            offset::ERSTSZ => 1,
            offset::ERSTBA => self.lock().unwrap().event_ring_base_address(),
            offset::ERSTBA_HI => 0,

            // Everything else is Reserved Zero
            _ => todo!(),
        }
    }
}

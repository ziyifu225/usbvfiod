//! Emulation of a USB3 Host (XHCI) controller.
//!
//! The specification is available
//! [here](https://www.intel.com/content/dam/www/public/us/en/documents/technical-specifications/extensible-host-controler-interface-usb-xhci.pdf).

use std::sync::Mutex;

use crate::device::{
    bus::{BusDeviceRef, Request, SingleThreadedBusDevice},
    pci::{
        config_space::{ConfigSpace, ConfigSpaceBuilder},
        constants::xhci::{capability, offset, OP_BASE, RUN_BASE},
        traits::PciDevice,
    },
};

/// The emulation of a XHCI controller.
#[derive(Debug, Clone)]
pub struct XhciController {
    /// A reference to the VM memory to perform DMA on.
    #[allow(unused)]
    dma_bus: BusDeviceRef,

    /// The PCI Configuration Space of the controller.
    config_space: ConfigSpace,
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
            offset::USBCMD => match value {
                val if val & 0x1 == 0 => (), /* stop */
                _ => todo!(),
            },
            offset::CONFIG => todo!(),
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
            offset::HCSPARAMS2 => 0,
            offset::HCSPARAMS3 => 0,
            offset::HCCPARAMS1 => 0,
            offset::DBOFF => 0x2000,
            offset::RTSOFF => RUN_BASE,
            offset::HCCPARAMS2 => 0,

            // xHC Operational Registers
            offset::USBCMD => 0,
            offset::USBSTS => 0x1,   /* HCHalted */
            offset::PAGESIZE => 0x1, /* 4k Pages */
            offset::CONFIG => 0,     /* No device slots enabled */

            // xHC Runtime Registers

            // Everything else is Reserved Zero
            _ => todo!(),
        }
    }
}

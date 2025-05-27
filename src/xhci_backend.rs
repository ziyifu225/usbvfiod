use std::{
    fs::File,
    path::Path,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use tracing::{debug, info, trace, warn};

use vfio_bindings::bindings::vfio::{
    vfio_region_info, VFIO_PCI_CONFIG_REGION_INDEX, VFIO_PCI_NUM_IRQS, VFIO_PCI_NUM_REGIONS,
    VFIO_REGION_INFO_FLAG_READ, VFIO_REGION_INFO_FLAG_WRITE,
};
use vfio_user::{IrqInfo, ServerBackend};

use usbvfiod::device::{
    bus::{Request, RequestSize},
    pci::{traits::PciDevice, xhci::XhciController},
};

use crate::{dynamic_bus::DynamicBus, memory_segment::MemorySegment};

#[derive(Debug)]
pub struct XhciBackend {
    dma_bus: Arc<DynamicBus>,
    controller: Mutex<XhciController>,
}

impl XhciBackend {
    /// Create a new virtual XHCI controller with the given USB
    /// devices attached at creation time.
    pub fn new<I>(devices: I) -> Result<Self>
    where
        I: IntoIterator,
        I::Item: AsRef<Path>,
    {
        let dma_bus = Arc::new(DynamicBus::new());

        let backend = Self {
            controller: Mutex::new(XhciController::new(dma_bus.clone())),
            dma_bus,
        };

        for device in devices {
            backend.add_device_from_path(device)?;
        }

        Ok(backend)
    }

    /// Add a USB device to the virtual XHCI controller.
    fn add_device(&self, device: nusb::Device) -> Result<()> {
        // The configuration is not super interesting, but as long as
        // we don't do anything else here this serves as a way to see
        // whether the file is actually a USB device.
        let active_configuration = device
            .active_configuration()
            .context("Failed to query active configuration")?;

        debug!("Device configuration: {active_configuration:?}");

        // TODO Actually add the device to the XHCI controller.
        warn!("Adding devices is not implemented yet.");

        Ok(())
    }

    /// Add a USB device via its path in `/dev/bus/usb`.
    pub fn add_device_from_path(&self, path: impl AsRef<Path>) -> Result<()> {
        let path: &Path = path.as_ref();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .with_context(|| format!("Failed to open USB device file: {}", path.display()))?;

        self.add_device(nusb::Device::from_fd(file.into())?)
    }
}

impl XhciBackend {
    /// Return a list of regions for [`vfio_user::Server::new`].
    pub fn regions(&self) -> Vec<vfio_region_info> {
        (0..VFIO_PCI_NUM_REGIONS)
            .map(|i| match i {
                VFIO_PCI_CONFIG_REGION_INDEX => vfio_region_info {
                    argsz: size_of::<vfio_region_info>() as u32,
                    index: i,
                    size: 256,
                    flags: VFIO_REGION_INFO_FLAG_READ | VFIO_REGION_INFO_FLAG_WRITE,
                    ..Default::default()
                },

                _ => vfio_region_info {
                    argsz: size_of::<vfio_region_info>() as u32,
                    index: i,
                    ..Default::default()
                },
            })
            .collect()
    }

    /// Return a list of IRQs for [`vfio_user::Server::new`].
    pub fn irqs(&self) -> Vec<IrqInfo> {
        let mut irqs = Vec::with_capacity(VFIO_PCI_NUM_IRQS as usize);
        for index in 0..VFIO_PCI_NUM_IRQS {
            let irq = IrqInfo {
                index,
                count: 1,
                flags: 0,
            };

            irqs.push(irq);
        }

        irqs
    }
}

impl ServerBackend for XhciBackend {
    fn region_read(
        &mut self,
        region: u32,
        offset: u64,
        data: &mut [u8],
    ) -> Result<(), std::io::Error> {
        trace!("read  region {region} offset {offset:#x}+{}", data.len());

        let value: u64 = match region {
            VFIO_PCI_CONFIG_REGION_INDEX => self.controller.read_cfg(Request::new(
                offset,
                RequestSize::try_from(data.len() as u64).expect("should use valid request size"),
            )),

            0 => self.controller.read_io(
                0,
                Request::new(
                    offset,
                    RequestSize::try_from(data.len() as u64)
                        .expect("should use valid request size"),
                ),
            ),

            _ => !0u64,
        };

        data.copy_from_slice(&value.to_le_bytes()[0..data.len()]);

        Ok(())
    }

    fn region_write(
        &mut self,
        region: u32,
        offset: u64,
        data: &[u8],
    ) -> Result<(), std::io::Error> {
        trace!(
            "write region {region} offset {offset:#x}+{} val {:?}",
            data.len(),
            data
        );

        match region {
            VFIO_PCI_CONFIG_REGION_INDEX => self.controller.write_cfg(
                Request::new(
                    offset,
                    RequestSize::try_from(data.len() as u64)
                        .expect("should use valid request size"),
                ),
                match data.len() {
                    1 => data[0].into(),
                    2 => {
                        let val: [u8; 2] = data.try_into().unwrap();
                        u16::from_le_bytes(val).into()
                    }

                    4 => {
                        let val: [u8; 4] = data.try_into().unwrap();
                        u32::from_le_bytes(val).into()
                    }
                    _ => todo!(),
                },
            ),

            0 => self.controller.write_io(
                0,
                Request::new(
                    offset,
                    RequestSize::try_from(data.len() as u64)
                        .expect("should use valid request size"),
                ),
                match data.len() {
                    1 => data[0].into(),
                    2 => {
                        let val: [u8; 2] = data.try_into().unwrap();
                        u16::from_le_bytes(val).into()
                    }

                    4 => {
                        let val: [u8; 4] = data.try_into().unwrap();
                        u32::from_le_bytes(val).into()
                    }
                    _ => todo!(),
                },
            ),
            _ => todo!(),
        }

        Ok(())
    }

    fn dma_map(
        &mut self,
        flags: vfio_user::DmaMapFlags,
        offset: u64,
        address: u64,
        size: u64,
        fd: Option<File>,
    ) -> Result<(), std::io::Error> {
        info!("dma_map flags = {flags:?} offset = {offset} address = {address} size = {size} fd = {fd:?}");

        if let Some(fd) = fd {
            let mseg = MemorySegment::new_from_fd(&fd, offset, size, flags.try_into().unwrap())?;

            self.dma_bus.add(address, Arc::new(mseg)).unwrap();
        } else {
            todo!("Memory region without file descriptor");
        }

        Ok(())
    }

    fn dma_unmap(
        &mut self,
        _flags: vfio_user::DmaUnmapFlags,
        _address: u64,
        _size: u64,
    ) -> Result<(), std::io::Error> {
        todo!()
    }

    fn reset(&mut self) -> Result<(), std::io::Error> {
        todo!()
    }

    fn set_irqs(
        &mut self,
        index: u32,
        flags: u32,
        start: u32,
        count: u32,
        fds: Vec<File>,
    ) -> Result<(), std::io::Error> {
        debug!(
            "set IRQs: {index} flags: {flags:#x} start: {start:#x} count: {count:#x} #fds: {}",
            fds.len()
        );

        Ok(())
    }
}

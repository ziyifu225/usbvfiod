use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::device::pci::usbrequest::UsbRequest;
use tracing::warn;

const LINKTYPE_USB_LINUX: u32 = 189;
const PCAP_MAGIC: u32 = 0xa1b2c3d4;
const SNAPLEN: u32 = 65_535;
pub const DEFAULT_BUS_NUMBER: u16 = 1;

#[derive(Clone, Copy)]
pub enum UsbEventType {
    Submission,
    Completion,
}

impl UsbEventType {
    fn code(self) -> u8 {
        match self {
            UsbEventType::Submission => b'S',
            UsbEventType::Completion => b'C',
        }
    }
}

#[derive(Clone, Copy)]
pub enum UsbTransferType {
    Control,
    Bulk,
    Interrupt,
}

impl UsbTransferType {
    fn code(self) -> u8 {
        match self {
            UsbTransferType::Control => 2,
            UsbTransferType::Bulk => 3,
            UsbTransferType::Interrupt => 1,
        }
    }
}

#[derive(Clone, Copy)]
pub enum UsbDirection {
    HostToDevice,
    DeviceToHost,
}

impl UsbDirection {
    fn endpoint_address(self, endpoint: u8) -> u8 {
        match self {
            UsbDirection::HostToDevice => endpoint & 0x7f,
            UsbDirection::DeviceToHost => endpoint | 0x80,
        }
    }
}

/// Timestamp of a packet in seconds and microseconds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Timestamp {
    pub seconds: u32,
    pub microseconds: u32,
}

impl From<std::time::SystemTime> for Timestamp {
    fn from(value: std::time::SystemTime) -> Self {
        let duration = value
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        Self {
            seconds: duration.as_secs() as u32,
            microseconds: duration.subsec_micros(),
        }
    }
}

pub struct UsbPacketLinktypeHeader {
    pub id: u64,
    pub event_type: u8,
    pub transfer_type: u8,
    pub endpoint_address: u8,
    pub device_address: u8,
    pub bus_number: u16,
    pub setup_flag: u8,
    pub data_flag: u8,
    pub status: i32,
    pub urb_len: u32,
    pub data_len: u32,
    pub setup: [u8; 8],
}

impl UsbPacketLinktypeHeader {
    pub fn header_bytes(&self, timestamp: Timestamp) -> [u8; 48] {
        let mut header = [0u8; 48];
        header[0..8].copy_from_slice(&self.id.to_le_bytes());
        header[8] = self.event_type;
        header[9] = self.transfer_type;
        header[10] = self.endpoint_address;
        header[11] = self.device_address;
        header[12..14].copy_from_slice(&self.bus_number.to_le_bytes());
        header[14] = self.setup_flag;
        header[15] = self.data_flag;
        header[16..24].copy_from_slice(&(timestamp.seconds as i64).to_le_bytes());
        header[24..28].copy_from_slice(&(timestamp.microseconds as i32).to_le_bytes());
        header[28..32].copy_from_slice(&self.status.to_le_bytes());
        header[32..36].copy_from_slice(&self.urb_len.to_le_bytes());
        header[36..40].copy_from_slice(&self.data_len.to_le_bytes());
        header[40..48].copy_from_slice(&self.setup);
        header
    }
}

struct PcapFileWriter {
    writer: Mutex<BufWriter<File>>,
}

impl PcapFileWriter {
    fn new(file: File) -> std::io::Result<Self> {
        let mut writer = BufWriter::new(file);
        writer.write_all(&PCAP_MAGIC.to_le_bytes())?;
        writer.write_all(&2u16.to_le_bytes())?;
        writer.write_all(&4u16.to_le_bytes())?;
        writer.write_all(&0u32.to_le_bytes())?;
        writer.write_all(&0u32.to_le_bytes())?;
        writer.write_all(&SNAPLEN.to_le_bytes())?;
        writer.write_all(&LINKTYPE_USB_LINUX.to_le_bytes())?;
        writer.flush()?;
        Ok(Self {
            writer: Mutex::new(writer),
        })
    }

    fn write_packet(
        &self,
        timestamp: Timestamp,
        meta: &UsbPacketLinktypeHeader,
        payload: &[u8],
    ) -> std::io::Result<()> {
        let header = meta.header_bytes(timestamp);
        let incl_len = (header.len() + payload.len()) as u32;
        let mut writer = self.writer.lock().unwrap();
        writer.write_all(&timestamp.seconds.to_le_bytes())?;
        writer.write_all(&timestamp.microseconds.to_le_bytes())?;
        writer.write_all(&incl_len.to_le_bytes())?;
        writer.write_all(&incl_len.to_le_bytes())?;
        writer.write_all(&header)?;
        writer.write_all(payload)?;
        writer.flush()?;
        Ok(())
    }
}

struct UsbPcapManagerState {
    dir: Option<PathBuf>,
    writer: Option<Arc<PcapFileWriter>>,
    warned: bool,
}

impl UsbPcapManagerState {
    fn new(path: Option<PathBuf>) -> Self {
        Self {
            dir: path,
            writer: None,
            warned: false,
        }
    }

    fn ensure_writer(&mut self) -> Option<Arc<PcapFileWriter>> {
        let file_path = self.dir.clone()?;

        if self.writer.is_some() {
            return self.writer.as_ref().map(Arc::clone);
        }

        if let Some(parent) = file_path.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                if !self.warned {
                    warn!(
                        "Disabling USB PCAP logging after failing to create {}: {}",
                        parent.display(),
                        error
                    );
                    self.warned = true;
                }
                self.dir = None;
                return None;
            }
        }

        let writer = match File::create(&file_path).and_then(PcapFileWriter::new) {
            Ok(writer) => Arc::new(writer),
            Err(error) => {
                if !self.warned {
                    warn!(
                        "Disabling USB PCAP logging after failing to open {}: {}",
                        file_path.display(),
                        error
                    );
                    self.warned = true;
                }
                self.dir = None;
                return None;
            }
        };

        self.writer = Some(writer.clone());
        Some(writer)
    }
}

static MANAGER: Mutex<Option<UsbPcapManagerState>> = Mutex::new(None);

pub struct UsbPcapManager;

impl UsbPcapManager {
    pub fn init(path: Option<PathBuf>) {
        *MANAGER.lock().unwrap() = Some(UsbPcapManagerState::new(path));
    }

    pub fn write(meta: &UsbPacketLinktypeHeader, payload: &[u8]) {
        let mut guard = MANAGER.lock().unwrap();
        let writer = match guard.as_mut().and_then(UsbPcapManagerState::ensure_writer) {
            Some(writer) => writer,
            None => return,
        };

        let timestamp = Timestamp::from(std::time::SystemTime::now());
        if let Err(error) = writer.write_packet(timestamp, meta, payload) {
            warn!("Failed to write USB PCAP packet: {}", error);
        }
    }
}

fn build_setup_bytes(request: &UsbRequest) -> [u8; 8] {
    [
        request.request_type,
        request.request,
        (request.value & 0x00ff) as u8,
        (request.value >> 8) as u8,
        (request.index & 0x00ff) as u8,
        (request.index >> 8) as u8,
        (request.length & 0x00ff) as u8,
        (request.length >> 8) as u8,
    ]
}

pub fn log_control_submission(
    slot_id: u8,
    bus_number: u16,
    request: &UsbRequest,
    direction: UsbDirection,
    payload: &[u8],
) {
    log_control_packet(
        request.address,
        slot_id,
        bus_number,
        UsbEventType::Submission,
        direction,
        0,
        u32::from(request.length),
        payload,
        Some(build_setup_bytes(request)),
    );
}

pub fn log_control_completion(
    request_id: u64,
    slot_id: u8,
    bus_number: u16,
    direction: UsbDirection,
    status: i32,
    actual_length: u32,
    payload: &[u8],
) {
    log_control_packet(
        request_id,
        slot_id,
        bus_number,
        UsbEventType::Completion,
        direction,
        status,
        actual_length,
        payload,
        None,
    );
}

fn log_control_packet(
    request_id: u64,
    slot_id: u8,
    bus_number: u16,
    event: UsbEventType,
    direction: UsbDirection,
    status: i32,
    urb_len: u32,
    payload: &[u8],
    setup: Option<[u8; 8]>,
) {
    let meta = UsbPacketLinktypeHeader {
        id: request_id,
        event_type: event.code(),
        transfer_type: UsbTransferType::Control.code(),
        endpoint_address: direction.endpoint_address(0),
        device_address: slot_id,
        bus_number,
        setup_flag: setup.is_some() as u8,
        data_flag: (!payload.is_empty()) as u8,
        status,
        urb_len,
        data_len: payload.len() as u32,
        setup: setup.unwrap_or([0; 8]),
    };
    UsbPcapManager::write(&meta, payload);
}

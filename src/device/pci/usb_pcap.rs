use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tracing::warn;

const LINKTYPE_USB_LINUX: u32 = 189;
const PCAP_MAGIC: u32 = 0xa1b2c3d4;
const SNAPLEN: u32 = 65_535;

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

pub struct UsbPacketMeta {
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

impl UsbPacketMeta {
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

struct PcapWriter {
    writer: Mutex<BufWriter<File>>,
}

impl PcapWriter {
    fn new(file: File) -> std::io::Result<Self> {
        let mut writer = BufWriter::new(file);
        writer.write_all(&PCAP_MAGIC.to_le_bytes())?;
        writer.write_all(&2u16.to_le_bytes())?;
        writer.write_all(&4u16.to_le_bytes())?;
        writer.write_all(&0u32.to_le_bytes())?;
        writer.write_all(&0u32.to_le_bytes())?;
        writer.write_all(&SNAPLEN.to_le_bytes())?;
        writer.write_all(&LINKTYPE_USB_LINUX.to_le_bytes())?;
        Ok(Self {
            writer: Mutex::new(writer),
        })
    }

    fn write_packet(
        &self,
        timestamp: Timestamp,
        meta: &UsbPacketMeta,
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
        Ok(())
    }
}

struct UsbPcapManagerState {
    dir: Option<PathBuf>,
    writer: Option<Arc<PcapWriter>>,
    warned: bool,
}

impl UsbPcapManagerState {
    fn new(dir: Option<PathBuf>) -> Self {
        Self {
            dir,
            writer: None,
            warned: false,
        }
    }

    fn ensure_writer(&mut self) -> Option<Arc<PcapWriter>> {
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

        let writer = match File::create(&file_path).and_then(PcapWriter::new) {
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
        *MANAGER.lock().unwrap() = path.map(UsbPcapManagerState::new);
    }

    pub fn write(meta: &UsbPacketMeta, payload: &[u8]) {
        let mut guard = MANAGER.lock().unwrap();
        let writer = guard
            .as_mut()
            .and_then(UsbPcapManagerState::ensure_writer)?;

        let timestamp = Timestamp::from(std::time::SystemTime::now());
        if let Err(error) = writer.write_packet(timestamp, meta, payload) {
            warn!("Failed to write USB PCAP packet: {}", error);
        }
    }
}

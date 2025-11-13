use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_usb::Builder;
use static_cell::StaticCell;

/// In-memory block device with a minimal FAT16 filesystem
pub struct RamBlockDevice {
    storage: [u8; Self::TOTAL_SIZE],
}

impl RamBlockDevice {
    const BLOCK_SIZE: usize = 512;
    const BLOCK_COUNT: usize = 128; // 64KB total
    const TOTAL_SIZE: usize = Self::BLOCK_SIZE * Self::BLOCK_COUNT;

    // FAT16 filesystem layout
    const BOOT_SECTOR: usize = 0;
    const FAT1_SECTOR: usize = 1;
    const FAT2_SECTOR: usize = 2;
    const ROOT_DIR_SECTOR: usize = 3;
    const DATA_START_SECTOR: usize = 4;

    pub fn new() -> Self {
        let mut device = Self {
            storage: [0u8; Self::TOTAL_SIZE],
        };
        device.format();
        device
    }

    /// Format the device as FAT16 and create a "hello" file
    fn format(&mut self) {
        self.write_boot_sector();
        self.write_fat_tables();
        self.write_root_directory();
        self.write_file_data();
    }

    /// Write the FAT16 boot sector
    fn write_boot_sector(&mut self) {
        let offset = Self::BOOT_SECTOR * Self::BLOCK_SIZE;
        let sector = &mut self.storage[offset..offset + Self::BLOCK_SIZE];

        // Jump instruction and OEM name
        sector[0..3].copy_from_slice(&[0xEB, 0x3C, 0x90]);
        sector[3..11].copy_from_slice(b"NUMCAL  ");

        // BIOS Parameter Block (BPB)
        Self::write_le_u16(&mut sector[11..], 512);              // Bytes per sector
        sector[13] = 1;                                          // Sectors per cluster
        Self::write_le_u16(&mut sector[14..], 1);                // Reserved sectors
        sector[16] = 2;                                          // Number of FATs
        Self::write_le_u16(&mut sector[17..], 16);               // Root directory entries
        Self::write_le_u16(&mut sector[19..], Self::BLOCK_COUNT as u16); // Total sectors
        sector[21] = 0xF8;                                       // Media descriptor
        Self::write_le_u16(&mut sector[22..], 1);                // Sectors per FAT
        Self::write_le_u16(&mut sector[24..], 1);                // Sectors per track
        Self::write_le_u16(&mut sector[26..], 1);                // Number of heads
        Self::write_le_u32(&mut sector[28..], 0);                // Hidden sectors
        Self::write_le_u32(&mut sector[32..], 0);                // Large sector count

        // Extended BPB
        sector[36] = 0x80;                                       // Drive number
        sector[37] = 0x00;                                       // Reserved
        sector[38] = 0x29;                                       // Extended boot signature
        Self::write_le_u32(&mut sector[39..], 0x12345678);       // Volume serial number
        sector[43..54].copy_from_slice(b"NUMCAL     ");         // Volume label
        sector[54..62].copy_from_slice(b"FAT16   ");            // Filesystem type

        // Boot signature
        sector[510] = 0x55;
        sector[511] = 0xAA;
    }

    /// Write FAT tables (FAT1 and FAT2)
    fn write_fat_tables(&mut self) {
        // Initialize FAT1
        let fat1_offset = Self::FAT1_SECTOR * Self::BLOCK_SIZE;
        let fat1 = &mut self.storage[fat1_offset..fat1_offset + Self::BLOCK_SIZE];

        // Media descriptor and end-of-chain markers
        fat1[0] = 0xF8;
        fat1[1] = 0xFF;
        fat1[2] = 0xFF;
        fat1[3] = 0xFF;

        // Cluster 2 (first data cluster for "hello" file) - end of chain
        fat1[4] = 0xFF;
        fat1[5] = 0xFF;

        // Copy FAT1 to FAT2
        let fat2_offset = Self::FAT2_SECTOR * Self::BLOCK_SIZE;
        self.storage.copy_within(fat1_offset..fat1_offset + Self::BLOCK_SIZE, fat2_offset);
    }

    /// Write root directory with "hello" file entry
    fn write_root_directory(&mut self) {
        let offset = Self::ROOT_DIR_SECTOR * Self::BLOCK_SIZE;
        let root_dir = &mut self.storage[offset..offset + Self::BLOCK_SIZE];

        // Directory entry for "hello" file (32 bytes)
        root_dir[0..8].copy_from_slice(b"HELLO   ");              // Filename (8.3 format)
        root_dir[8..11].copy_from_slice(b"   ");                  // Extension
        root_dir[11] = 0x20;                                       // Archive attribute
        root_dir[12..22].fill(0);                                  // Reserved
        Self::write_le_u16(&mut root_dir[22..], 0x0000);          // Creation time
        Self::write_le_u16(&mut root_dir[24..], 0x0021);          // Creation date
        Self::write_le_u16(&mut root_dir[26..], 2);               // First cluster (cluster 2)
        Self::write_le_u32(&mut root_dir[28..], 5);               // File size (5 bytes for "world")
    }

    /// Write file data in the data area
    fn write_file_data(&mut self) {
        // Cluster 2 starts at DATA_START_SECTOR
        let offset = Self::DATA_START_SECTOR * Self::BLOCK_SIZE;
        self.storage[offset..offset + 5].copy_from_slice(b"world");
    }

    /// Helper: Write 16-bit little-endian value
    fn write_le_u16(buf: &mut [u8], value: u16) {
        buf[0] = value as u8;
        buf[1] = (value >> 8) as u8;
    }

    /// Helper: Write 32-bit little-endian value
    fn write_le_u32(buf: &mut [u8], value: u32) {
        buf[0] = value as u8;
        buf[1] = (value >> 8) as u8;
        buf[2] = (value >> 16) as u8;
        buf[3] = (value >> 24) as u8;
    }

    pub fn read_block(&self, block: u32, buf: &mut [u8]) {
        let offset = block as usize * Self::BLOCK_SIZE;
        if offset + Self::BLOCK_SIZE <= self.storage.len() {
            buf[..Self::BLOCK_SIZE]
                .copy_from_slice(&self.storage[offset..offset + Self::BLOCK_SIZE]);
        }
    }

    pub fn write_block(&mut self, _block: u32, _buf: &[u8]) {
        // Ignore writes for now as requested
        log::debug!("Write request ignored (read-only mode)");
    }

    pub fn block_count(&self) -> u32 {
        Self::BLOCK_COUNT as u32
    }

    pub fn block_size(&self) -> u32 {
        Self::BLOCK_SIZE as u32
    }
}

// USB Mass Storage Class implementation
use embassy_usb::control::{InResponse, OutResponse, Recipient, Request, RequestType};
use embassy_usb::driver::{EndpointIn, EndpointOut};
use embassy_usb::types::InterfaceNumber;
use embassy_usb::Handler;

const USB_CLASS_MSC: u8 = 0x08;
const MSC_SUBCLASS_SCSI: u8 = 0x06;
const MSC_PROTOCOL_BBB: u8 = 0x50; // Bulk-Only Transport

pub struct MscClass<'d, D: embassy_usb::driver::Driver<'d>> {
    read_ep: D::EndpointOut,
    write_ep: D::EndpointIn,
    if_num: InterfaceNumber,
    block_device: &'static mut RamBlockDevice,
}

impl<'d, D: embassy_usb::driver::Driver<'d>> MscClass<'d, D> {
    pub fn new(
        builder: &mut Builder<'d, D>,
        block_device: &'static mut RamBlockDevice,
    ) -> Self {
        let mut func = builder.function(USB_CLASS_MSC, MSC_SUBCLASS_SCSI, MSC_PROTOCOL_BBB);
        let mut iface = func.interface();
        let if_num = iface.interface_number();
        let mut alt = iface.alt_setting(USB_CLASS_MSC, MSC_SUBCLASS_SCSI, MSC_PROTOCOL_BBB, None);

        let read_ep = alt.endpoint_bulk_out(None, 64);
        let write_ep = alt.endpoint_bulk_in(None, 64);

        MscClass {
            read_ep,
            write_ep,
            if_num,
            block_device,
        }
    }

    pub async fn run(&mut self) -> ! {
        log::info!("MSC: USB Mass Storage ready with FAT16 filesystem");

        let mut cbw_buf = [0u8; 31];
        let mut data_buf = [0u8; 512];

        loop {
            // Read Command Block Wrapper (CBW)
            match self.read_ep.read(&mut cbw_buf).await {
                Ok(n) if n == 31 => {
                    if let Err(e) = self.process_cbw(&cbw_buf, &mut data_buf).await {
                        log::error!("MSC: Error processing CBW: {:?}", e);
                    }
                }
                Ok(n) => log::warn!("MSC: Invalid CBW length: {}", n),
                Err(e) => log::error!("MSC: Read error: {:?}", e),
            }
        }
    }

    async fn process_cbw(&mut self, cbw_buf: &[u8], data_buf: &mut [u8]) -> Result<(), ()> {
        // Validate CBW signature
        let signature = u32::from_le_bytes([cbw_buf[0], cbw_buf[1], cbw_buf[2], cbw_buf[3]]);
        if signature != 0x43425355 { // "USBC"
            log::warn!("MSC: Invalid CBW signature: 0x{:08x}", signature);
            return Err(());
        }

        let tag = u32::from_le_bytes([cbw_buf[4], cbw_buf[5], cbw_buf[6], cbw_buf[7]]);
        let data_transfer_length = u32::from_le_bytes([cbw_buf[8], cbw_buf[9], cbw_buf[10], cbw_buf[11]]);
        let cb = &cbw_buf[15..31]; // Command block

        log::debug!("MSC: CBW tag=0x{:08x} cmd=0x{:02x}", tag, cb[0]);

        // Process SCSI command
        let (status, data_len) = match cb[0] {
            0x00 => {
                log::debug!("MSC: TEST UNIT READY");
                (0u8, 0u32)
            }
            0x12 => {
                log::debug!("MSC: INQUIRY");
                self.handle_inquiry(data_buf).await?;
                (0u8, 36u32)
            }
            0x25 => {
                log::debug!("MSC: READ CAPACITY");
                self.handle_read_capacity(data_buf).await?;
                (0u8, 8u32)
            }
            0x28 => {
                let lba = u32::from_be_bytes([cb[2], cb[3], cb[4], cb[5]]);
                let block_count = u16::from_be_bytes([cb[7], cb[8]]) as u32;
                log::debug!("MSC: READ(10) lba={} count={}", lba, block_count);
                self.handle_read(lba, block_count).await?;
                (0u8, block_count * 512)
            }
            0x2A => {
                let lba = u32::from_be_bytes([cb[2], cb[3], cb[4], cb[5]]);
                let block_count = u16::from_be_bytes([cb[7], cb[8]]) as u32;
                log::debug!("MSC: WRITE(10) lba={} count={} (ignored)", lba, block_count);
                // Drain write data
                for _ in 0..block_count {
                    let _ = self.read_ep.read(data_buf).await;
                }
                (0u8, 0u32)
            }
            0x1A => {
                log::debug!("MSC: MODE SENSE(6)");
                data_buf[0..4].copy_from_slice(&[0x03, 0x00, 0x00, 0x00]);
                self.write_ep.write(&data_buf[0..4]).await.map_err(|_| ())?;
                (0u8, 4u32)
            }
            cmd => {
                log::warn!("MSC: Unsupported SCSI command: 0x{:02x}", cmd);
                (1u8, 0u32)
            }
        };

        // Send Command Status Wrapper (CSW)
        self.send_csw(tag, data_transfer_length.saturating_sub(data_len), status).await
    }

    async fn handle_inquiry(&mut self, buf: &mut [u8]) -> Result<(), ()> {
        buf[0] = 0x00;   // Direct access block device
        buf[1] = 0x80;   // Removable
        buf[2] = 0x04;   // SPC-2
        buf[3] = 0x02;   // Response data format
        buf[4] = 31;     // Additional length
        buf[5..8].fill(0);
        buf[8..16].copy_from_slice(b"NumCal  ");
        buf[16..32].copy_from_slice(b"Flash Drive     ");
        buf[32..36].copy_from_slice(b"1.0 ");

        self.write_ep.write(&buf[0..36]).await.map_err(|_| ())
    }

    async fn handle_read_capacity(&mut self, buf: &mut [u8]) -> Result<(), ()> {
        let last_lba = self.block_device.block_count() - 1;
        let block_size = self.block_device.block_size();

        buf[0..4].copy_from_slice(&last_lba.to_be_bytes());
        buf[4..8].copy_from_slice(&block_size.to_be_bytes());

        self.write_ep.write(&buf[0..8]).await.map_err(|_| ())
    }

    async fn handle_read(&mut self, lba: u32, block_count: u32) -> Result<(), ()> {
        let mut buf = [0u8; 512];
        for i in 0..block_count {
            self.block_device.read_block(lba + i, &mut buf);
            self.write_ep.write(&buf).await.map_err(|_| ())?;
        }
        Ok(())
    }

    async fn send_csw(&mut self, tag: u32, data_residue: u32, status: u8) -> Result<(), ()> {
        let csw = [
            0x55, 0x53, 0x42, 0x53, // Signature "USBS"
            (tag & 0xFF) as u8,
            ((tag >> 8) & 0xFF) as u8,
            ((tag >> 16) & 0xFF) as u8,
            ((tag >> 24) & 0xFF) as u8,
            (data_residue & 0xFF) as u8,
            ((data_residue >> 8) & 0xFF) as u8,
            ((data_residue >> 16) & 0xFF) as u8,
            ((data_residue >> 24) & 0xFF) as u8,
            status,
        ];

        self.write_ep.write(&csw).await.map_err(|_| ())
    }
}

impl<'d, D: embassy_usb::driver::Driver<'d>> Handler for MscClass<'d, D> {
    fn control_out(&mut self, req: Request, _data: &[u8]) -> Option<OutResponse> {
        if req.request_type == RequestType::Class
            && req.recipient == Recipient::Interface
            && req.index == self.if_num.0 as u16
        {
            match req.request {
                0xFF => {
                    log::debug!("MSC: Bulk-Only Mass Storage Reset");
                    return Some(OutResponse::Accepted);
                }
                _ => {}
            }
        }
        None
    }

    fn control_in<'a>(&'a mut self, req: Request, buf: &'a mut [u8]) -> Option<InResponse<'a>> {
        if req.request_type == RequestType::Class
            && req.recipient == Recipient::Interface
            && req.index == self.if_num.0 as u16
        {
            match req.request {
                0xFE => {
                    log::debug!("MSC: Get Max LUN");
                    buf[0] = 0; // Single LUN (LUN 0)
                    return Some(InResponse::Accepted(&buf[0..1]));
                }
                _ => {}
            }
        }
        None
    }
}

static BLOCK_DEVICE: StaticCell<RamBlockDevice> = StaticCell::new();

pub fn init(
    spawner: &Spawner,
    builder: &mut Builder<'static, Driver<'static, USB>>,
) -> Result<(), ()> {
    let block_device = BLOCK_DEVICE.init(RamBlockDevice::new());
    let msc = MscClass::new(builder, block_device);
    spawner.spawn(msc_task(msc).unwrap());

    log::info!("File system initialized: FAT16 with 'hello' file");
    Ok(())
}

#[embassy_executor::task]
async fn msc_task(mut msc: MscClass<'static, Driver<'static, USB>>) {
    msc.run().await;
}

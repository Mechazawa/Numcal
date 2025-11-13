use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_usb::Builder;
use static_cell::StaticCell;

/// Simple in-memory block device with a FAT16 filesystem containing "hello" file
pub struct RamBlockDevice {
    storage: [u8; Self::TOTAL_SIZE],
}

impl RamBlockDevice {
    const BLOCK_SIZE: usize = 512;
    const BLOCK_COUNT: usize = 128; // 64KB total
    const TOTAL_SIZE: usize = Self::BLOCK_SIZE * Self::BLOCK_COUNT;

    pub fn new() -> Self {
        let mut device = Self {
            storage: [0u8; Self::TOTAL_SIZE],
        };
        device.init_filesystem();
        device
    }

    fn init_filesystem(&mut self) {
        // Create a minimal FAT16 filesystem with "hello" file containing "world"

        // Boot Sector (Block 0)
        let boot_sector = &mut self.storage[0..512];

        // Jump instruction
        boot_sector[0] = 0xEB;
        boot_sector[1] = 0x3C;
        boot_sector[2] = 0x90;

        // OEM Name
        boot_sector[3..11].copy_from_slice(b"NUMCAL  ");

        // Bytes per sector (512)
        boot_sector[11] = 0x00;
        boot_sector[12] = 0x02;

        // Sectors per cluster (1)
        boot_sector[13] = 0x01;

        // Reserved sectors (1)
        boot_sector[14] = 0x01;
        boot_sector[15] = 0x00;

        // Number of FATs (2)
        boot_sector[16] = 0x02;

        // Root directory entries (16)
        boot_sector[17] = 0x10;
        boot_sector[18] = 0x00;

        // Total sectors (128)
        boot_sector[19] = Self::BLOCK_COUNT as u8;
        boot_sector[20] = 0x00;

        // Media descriptor (0xF8 = hard disk)
        boot_sector[21] = 0xF8;

        // Sectors per FAT (1)
        boot_sector[22] = 0x01;
        boot_sector[23] = 0x00;

        // Sectors per track (1)
        boot_sector[24] = 0x01;
        boot_sector[25] = 0x00;

        // Number of heads (1)
        boot_sector[26] = 0x01;
        boot_sector[27] = 0x00;

        // Hidden sectors (0)
        boot_sector[28..32].fill(0);

        // Large sector count (0)
        boot_sector[32..36].fill(0);

        // Drive number (0x80 = hard disk)
        boot_sector[36] = 0x80;
        boot_sector[37] = 0x00;

        // Extended boot signature
        boot_sector[38] = 0x29;

        // Volume serial number
        boot_sector[39..43].copy_from_slice(&[0x12, 0x34, 0x56, 0x78]);

        // Volume label
        boot_sector[43..54].copy_from_slice(b"NUMCAL     ");

        // File system type
        boot_sector[54..62].copy_from_slice(b"FAT16   ");

        // Boot signature
        boot_sector[510] = 0x55;
        boot_sector[511] = 0xAA;

        // FAT 1 (Block 1)
        let fat1 = &mut self.storage[512..1024];
        fat1[0] = 0xF8; // Media descriptor
        fat1[1] = 0xFF;
        fat1[2] = 0xFF;
        fat1[3] = 0xFF;
        fat1[4] = 0xFF; // Cluster 2 (hello file) - end of chain
        fat1[5] = 0xFF;

        // FAT 2 (Block 2) - copy of FAT 1
        self.storage.copy_within(512..1024, 1024);

        // Root Directory (Block 3)
        let root_dir = &mut self.storage[1536..2048];

        // Directory entry for "hello" file
        root_dir[0..8].copy_from_slice(b"HELLO   ");
        root_dir[8..11].copy_from_slice(b"   ");
        root_dir[11] = 0x20; // Archive attribute
        root_dir[12..22].fill(0);

        // Time and date (arbitrary)
        root_dir[22] = 0x00;
        root_dir[23] = 0x00;
        root_dir[24] = 0x21;
        root_dir[25] = 0x00;

        // First cluster (2)
        root_dir[26] = 0x02;
        root_dir[27] = 0x00;

        // File size (5 bytes for "world")
        root_dir[28] = 0x05;
        root_dir[29] = 0x00;
        root_dir[30] = 0x00;
        root_dir[31] = 0x00;

        // Data area starts at block 4
        // Cluster 2 data (first data cluster) at block 4
        let data_start = 4 * 512;
        self.storage[data_start..data_start + 5].copy_from_slice(b"world");
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

// USB Mass Storage Class implementation using raw endpoints
use embassy_usb::control::{InResponse, OutResponse, Recipient, Request, RequestType};
use embassy_usb::driver::{EndpointIn, EndpointOut};
use embassy_usb::types::InterfaceNumber;
use embassy_usb::Handler;

const USB_CLASS_MSC: u8 = 0x08;
const MSC_SUBCLASS_SCSI: u8 = 0x06;
const MSC_PROTOCOL_BBB: u8 = 0x50; // Bulk-Only Transport

// CBW (Command Block Wrapper) structure
#[repr(C, packed)]
struct Cbw {
    signature: u32,
    tag: u32,
    data_transfer_length: u32,
    flags: u8,
    lun: u8,
    cb_length: u8,
    cb: [u8; 16],
}

// CSW (Command Status Wrapper) structure
#[repr(C, packed)]
struct Csw {
    signature: u32,
    tag: u32,
    data_residue: u32,
    status: u8,
}

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
        log::info!("MSC: Starting USB Mass Storage task");

        let mut cbw_buf = [0u8; 31];
        let mut data_buf = [0u8; 512];

        loop {
            // Read CBW
            match self.read_ep.read(&mut cbw_buf).await {
                Ok(n) if n == 31 => {
                    if let Err(e) = self.process_cbw(&cbw_buf, &mut data_buf).await {
                        log::error!("MSC: Error processing CBW: {:?}", e);
                    }
                }
                Ok(n) => {
                    log::warn!("MSC: Invalid CBW length: {}", n);
                }
                Err(e) => {
                    log::error!("MSC: Read error: {:?}", e);
                }
            }
        }
    }

    async fn process_cbw(&mut self, cbw_buf: &[u8], data_buf: &mut [u8]) -> Result<(), ()> {
        // Parse CBW
        let signature = u32::from_le_bytes([cbw_buf[0], cbw_buf[1], cbw_buf[2], cbw_buf[3]]);
        if signature != 0x43425355 {
            // "USBC"
            log::warn!("MSC: Invalid CBW signature: 0x{:08x}", signature);
            return Err(());
        }

        let tag = u32::from_le_bytes([cbw_buf[4], cbw_buf[5], cbw_buf[6], cbw_buf[7]]);
        let data_transfer_length =
            u32::from_le_bytes([cbw_buf[8], cbw_buf[9], cbw_buf[10], cbw_buf[11]]);
        let _flags = cbw_buf[12];
        let _cb_length = cbw_buf[14];
        let cb = &cbw_buf[15..31];

        log::debug!(
            "MSC: CBW tag=0x{:08x} len={} cmd=0x{:02x}",
            tag,
            data_transfer_length,
            cb[0]
        );

        // Process SCSI command
        let (status, data_len) = match cb[0] {
            0x00 => {
                // TEST UNIT READY
                log::debug!("MSC: TEST UNIT READY");
                (0u8, 0u32)
            }
            0x12 => {
                // INQUIRY
                log::debug!("MSC: INQUIRY");
                self.handle_inquiry(data_buf).await?;
                (0u8, 36u32)
            }
            0x25 => {
                // READ CAPACITY(10)
                log::debug!("MSC: READ CAPACITY");
                self.handle_read_capacity(data_buf).await?;
                (0u8, 8u32)
            }
            0x28 => {
                // READ(10)
                let lba = u32::from_be_bytes([cb[2], cb[3], cb[4], cb[5]]);
                let block_count = u16::from_be_bytes([cb[7], cb[8]]) as u32;
                log::debug!("MSC: READ(10) lba={} count={}", lba, block_count);
                self.handle_read(lba, block_count).await?;
                (0u8, block_count * 512)
            }
            0x2A => {
                // WRITE(10)
                let lba = u32::from_be_bytes([cb[2], cb[3], cb[4], cb[5]]);
                let block_count = u16::from_be_bytes([cb[7], cb[8]]) as u32;
                log::debug!("MSC: WRITE(10) lba={} count={} (ignored)", lba, block_count);
                // Drain the data without storing it
                for _ in 0..block_count {
                    let _ = self.read_ep.read(data_buf).await;
                }
                (0u8, 0u32)
            }
            0x1A => {
                // MODE SENSE(6)
                log::debug!("MSC: MODE SENSE(6)");
                data_buf[0..4].copy_from_slice(&[0x03, 0x00, 0x00, 0x00]);
                self.write_ep.write(&data_buf[0..4]).await.map_err(|_| ())?;
                (0u8, 4u32)
            }
            cmd => {
                log::warn!("MSC: Unsupported SCSI command: 0x{:02x}", cmd);
                (1u8, 0u32) // Error status
            }
        };

        // Send CSW
        self.send_csw(tag, data_transfer_length.saturating_sub(data_len), status)
            .await?;

        Ok(())
    }

    async fn handle_inquiry(&mut self, buf: &mut [u8]) -> Result<(), ()> {
        buf[0] = 0x00; // Direct access block device
        buf[1] = 0x80; // Removable
        buf[2] = 0x04; // SPC-2
        buf[3] = 0x02; // Response data format
        buf[4] = 31; // Additional length
        buf[5] = 0x00;
        buf[6] = 0x00;
        buf[7] = 0x00;
        buf[8..16].copy_from_slice(b"NumCal  "); // Vendor
        buf[16..32].copy_from_slice(b"Flash Drive     "); // Product
        buf[32..36].copy_from_slice(b"1.0 "); // Revision

        self.write_ep.write(&buf[0..36]).await.map_err(|_| ())?;
        Ok(())
    }

    async fn handle_read_capacity(&mut self, buf: &mut [u8]) -> Result<(), ()> {
        let last_lba = self.block_device.block_count() - 1;
        let block_size = self.block_device.block_size();

        buf[0..4].copy_from_slice(&last_lba.to_be_bytes());
        buf[4..8].copy_from_slice(&block_size.to_be_bytes());

        self.write_ep.write(&buf[0..8]).await.map_err(|_| ())?;
        Ok(())
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
            tag as u8,
            (tag >> 8) as u8,
            (tag >> 16) as u8,
            (tag >> 24) as u8, // Tag
            data_residue as u8,
            (data_residue >> 8) as u8,
            (data_residue >> 16) as u8,
            (data_residue >> 24) as u8, // Data residue
            status, // Status
        ];

        self.write_ep.write(&csw).await.map_err(|_| ())?;
        Ok(())
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
                    // Bulk-Only Mass Storage Reset
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
                    // Get Max LUN
                    log::debug!("MSC: Get Max LUN");
                    buf[0] = 0; // We have only 1 LUN (LUN 0)
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

    log::info!("File system initialized with 'hello' file");
    Ok(())
}

#[embassy_executor::task]
async fn msc_task(mut msc: MscClass<'static, Driver<'static, USB>>) {
    msc.run().await;
}

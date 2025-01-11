use rp2040_flash::flash;

pub const FLASH_SECTOR_SIZE: usize = 4096;
pub const FLASH_SLOTS: [(u32, &str); 4] = [
    (0x100000, "slot1"),
    (0x101000, "slot2"),
    (0x102000, "slot3"),
    (0x103000, "slot4"),
];

#[repr(C)]
pub struct StoredData {
    size: u32,
    data: [u8; FLASH_SECTOR_SIZE - 4],
}

pub fn write_to_flash(slot: usize, data: &[u8]) -> Result<(), &'static str> {
    if slot >= FLASH_SLOTS.len() {
        return Err("Invalid slot number");
    }

    if data.len() > FLASH_SECTOR_SIZE - 4 {
        return Err("Data too large");
    }

    let (offset, _) = FLASH_SLOTS[slot];

    let mut stored = StoredData {
        size: data.len() as u32,
        data: [0xff; FLASH_SECTOR_SIZE - 4],
    };
    stored.data[..data.len()].copy_from_slice(data);

    let stored_bytes =
        unsafe { core::slice::from_raw_parts(&stored as *const _ as *const u8, FLASH_SECTOR_SIZE) };

    cortex_m::interrupt::free(|_cs| unsafe {
        flash::flash_range_erase(offset, FLASH_SECTOR_SIZE as u32, true);
        flash::flash_range_program(offset, stored_bytes, true);
    });

    Ok(())
}

pub fn read_from_flash(slot: usize) -> Result<&'static [u8], &'static str> {
    if slot >= FLASH_SLOTS.len() {
        return Err("Invalid slot number");
    }

    let (offset, _) = FLASH_SLOTS[slot];

    let stored = unsafe { &*((0x10000000 + offset) as *const StoredData) };

    Ok(&stored.data[..stored.size as usize])
}

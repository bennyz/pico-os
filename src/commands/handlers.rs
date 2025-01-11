use crate::commands::{CommandResult, COMMANDS};
use crate::flash;
use core::fmt::Write;
use rp_pico::hal;
use usbd_serial::SerialPort;

pub fn handle_buffer(serial: &mut SerialPort<hal::usb::UsbBus>, args: &[&str]) -> CommandResult {
    CommandResult::Ok(Some(b"Buffer size\r\n"))
}

pub fn handle_reboot(serial: &mut SerialPort<hal::usb::UsbBus>, args: &[&str]) -> CommandResult {
    CommandResult::Ok(Some(b"Rebooting...\r\n"))
}

pub fn handle_bootloader(
    serial: &mut SerialPort<hal::usb::UsbBus>,
    args: &[&str],
) -> CommandResult {
    CommandResult::Ok(Some(b"Rebooting into bootloader mode...\r\n"))
}

pub fn handle_help(serial: &mut SerialPort<hal::usb::UsbBus>, _args: &[&str]) -> CommandResult {
    let _ = serial.write(b"\r\nAvailable commands:\r\n");

    for cmd in COMMANDS {
        let _ = serial.write(b"  ");
        let _ = serial.write(cmd.name.as_bytes());

        let padding = 12_usize.saturating_sub(cmd.name.len());
        for _ in 0..padding {
            let _ = serial.write(b" ");
        }

        let _ = serial.write(b" - ");
        let _ = serial.write(cmd.help.as_bytes());
        let _ = serial.write(b"\r\n");
    }

    CommandResult::Ok(None)
}

static mut WRITE_BUFFER: [u8; 256] = [0; 256];
static mut SLOTS_BUFFER: [u8; 256] = [0; 256];

pub struct ByteWriter<'a> {
    buffer: &'a mut [u8],
    position: usize,
}

impl<'a> ByteWriter<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Self {
            buffer,
            position: 0,
        }
    }

    pub fn position(&self) -> usize {
        self.position
    }
}

impl<'a> core::fmt::Write for ByteWriter<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let remaining_buf = &mut self.buffer[self.position..];
        if bytes.len() > remaining_buf.len() {
            return Err(core::fmt::Error);
        }
        remaining_buf[..bytes.len()].copy_from_slice(bytes);
        self.position += bytes.len();
        Ok(())
    }
}

fn write_to_buffer(buffer: &'static mut [u8], f: impl FnOnce(&mut ByteWriter)) -> &'static [u8] {
    let mut writer = ByteWriter {
        buffer,
        position: 0,
    };
    f(&mut writer);

    let position = writer.position;
    drop(writer);

    &buffer[..position]
}

pub fn handle_write(serial: &mut SerialPort<hal::usb::UsbBus>, args: &[&str]) -> CommandResult {
    if args.len() < 2 {
        return CommandResult::Error("Usage: write <slot> <text>");
    }

    match args[0].parse::<usize>() {
        Ok(slot) => {
            let text = unsafe {
                write_to_buffer(&mut WRITE_BUFFER, |writer| {
                    for (i, &arg) in args[1..].iter().enumerate() {
                        if i > 0 {
                            let _ = writer.write_str(" ");
                        }
                        let _ = writer.write_str(arg);
                    }
                })
            };

            match flash::write_to_flash(slot - 1, text) {
                Ok(_) => CommandResult::Ok(None),
                Err(e) => CommandResult::Error(e),
            }
        }
        Err(_) => CommandResult::Error("Invalid slot number"),
    }
}

pub fn handle_read(serial: &mut SerialPort<hal::usb::UsbBus>, args: &[&str]) -> CommandResult {
    if args.len() != 1 {
        return CommandResult::Error("Usage: read <slot>");
    }

    match args[0].parse::<usize>() {
        Ok(slot) => match flash::read_from_flash(slot - 1) {
            Ok(data) => {
                unsafe {
                    write_to_buffer(&mut SLOTS_BUFFER, |writer| {
                        let _ = writer.write_str("\r\n");
                        let text = core::str::from_utf8(data).unwrap_or("<invalid utf8>");
                        let _ = writer.write_str(text);
                    })
                };
                CommandResult::Ok(Some(unsafe { &SLOTS_BUFFER[..] }))
            }
            Err(e) => CommandResult::Error(e),
        },
        Err(_) => CommandResult::Error("Invalid slot number"),
    }
}

pub fn handle_slots(serial: &mut SerialPort<hal::usb::UsbBus>, args: &[&str]) -> CommandResult {
    let output = unsafe {
        write_to_buffer(&mut SLOTS_BUFFER, |writer| {
            for (i, (_, name)) in flash::FLASH_SLOTS.iter().enumerate() {
                let _ = write!(writer, "  {}: {}\r\n", i + 1, name);
            }
        })
    };

    CommandResult::Ok(Some(output))
}

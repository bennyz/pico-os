use super::{Command, CommandArgs};
use crate::commands::{CommandResult, COMMANDS};
use crate::context::Context;
use crate::flash;
use core::fmt::Write;
use fugit::MicrosDurationU32;
use hal::rom_data::reset_to_usb_boot;
use heapless::String;
use rp_pico::hal;

pub struct WriteCommand;
pub struct ReadCommand;
pub struct SlotsCommand;
pub struct HelpCommand;
pub struct RebootCommand;
pub struct BootloaderCommand;

impl Command for HelpCommand {
    type Args = CommandArgs;

    fn name(&self) -> &'static str {
        "help"
    }
    fn help(&self) -> &'static str {
        "Show this help message"
    }

    fn parse(&self, args: &[&str]) -> Result<Self::Args, &'static str> {
        if !args.is_empty() {
            return Err("help command takes no arguments");
        }
        Ok(CommandArgs::None(()))
    }

    fn execute(&self, _: Self::Args, context: &Context) -> CommandResult {
        let mut serial = context.serial.borrow_mut();

        for cmd in COMMANDS {
            let _ = serial.write(b"  ");
            let _ = serial.write(cmd.name().as_bytes());

            let padding = 12_usize.saturating_sub(cmd.name().len());
            for _ in 0..padding {
                let _ = serial.write(b" ");
            }

            let _ = serial.write(b" - ");
            let _ = serial.write(cmd.help().as_bytes());
            let _ = serial.write(b"\r\n");
        }

        CommandResult::Ok(None)
    }
}

impl Command for RebootCommand {
    type Args = CommandArgs;

    fn name(&self) -> &'static str {
        "reboot"
    }
    fn help(&self) -> &'static str {
        "Reboot the device into App mode"
    }

    fn parse(&self, args: &[&str]) -> Result<Self::Args, &'static str> {
        if !args.is_empty() {
            return Err("reboot command takes no arguments");
        }
        Ok(CommandArgs::None(()))
    }

    fn execute(&self, _: Self::Args, context: &Context) -> CommandResult {
        context
            .watchdog
            .borrow_mut()
            .start(MicrosDurationU32::millis(1));
        loop {}
    }
}

impl Command for BootloaderCommand {
    type Args = CommandArgs;

    fn name(&self) -> &'static str {
        "bootloader"
    }
    fn help(&self) -> &'static str {
        "Reboot the device into Bootloader mode"
    }

    fn parse(&self, args: &[&str]) -> Result<Self::Args, &'static str> {
        if !args.is_empty() {
            return Err("bootloader command takes no arguments");
        }
        Ok(CommandArgs::None(()))
    }

    fn execute(&self, _: Self::Args, _context: &Context) -> CommandResult {
        reset_to_usb_boot(0, 0);
        CommandResult::Ok(None)
    }
}

impl Command for ReadCommand {
    type Args = CommandArgs;

    fn name(&self) -> &'static str {
        "read"
    }
    fn help(&self) -> &'static str {
        "Read text from slot n: read <n>"
    }

    fn parse(&self, args: &[&str]) -> Result<Self::Args, &'static str> {
        if args.len() != 1 {
            return Err("Usage: read <slot>");
        }

        let slot = args[0]
            .parse::<usize>()
            .map_err(|_| "Invalid slot number")?;

        if slot > flash::FLASH_SLOTS.len() {
            return Err("Invalid slot number");
        }

        Ok(CommandArgs::Slot(slot))
    }

    fn execute(&self, args: Self::Args, _context: &Context) -> CommandResult {
        let slot = match args {
            CommandArgs::Slot(slot) => slot,
            _ => return CommandResult::Error("Invalid arguments"),
        };
        match flash::read_from_flash(slot) {
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
        }
    }
}

impl Command for WriteCommand {
    type Args = CommandArgs;

    fn name(&self) -> &'static str {
        "write"
    }
    fn help(&self) -> &'static str {
        "Write text to slot n: write <n> <text>"
    }

    fn parse(&self, args: &[&str]) -> Result<Self::Args, &'static str> {
        if args.len() < 2 {
            return Err("Usage: write <slot> <text>");
        }

        let slot = args[0]
            .parse::<usize>()
            .map_err(|_| "Invalid slot number")?;

        if slot > flash::FLASH_SLOTS.len() {
            return Err("Invalid slot number");
        }

        // Convert to a heapless String
        let mut data = String::<64>::new();
        for (i, part) in args.iter().skip(1).enumerate() {
            if i > 0 {
                let _ = data.write_str(" ");
            }
            let _ = data.write_str(part);
        }

        Ok(CommandArgs::WriteSlot(slot, data))
    }

    fn execute(&self, args: Self::Args, _context: &Context) -> CommandResult {
        let (slot, data) = match args {
            CommandArgs::WriteSlot(slot, data) => (slot, data),
            _ => return CommandResult::Error("Invalid arguments"),
        };
        match flash::write_to_flash(slot, data.as_bytes()) {
            Ok(_) => CommandResult::Ok(None),
            Err(e) => CommandResult::Error(e),
        }
    }
}

impl Command for SlotsCommand {
    type Args = CommandArgs;

    fn name(&self) -> &'static str {
        "slots"
    }
    fn help(&self) -> &'static str {
        "List available storage slots"
    }

    fn parse(&self, args: &[&str]) -> Result<Self::Args, &'static str> {
        if !args.is_empty() {
            return Err("slots command takes no arguments");
        }
        Ok(CommandArgs::None(()))
    }

    fn execute(&self, _: Self::Args, _: &Context) -> CommandResult {
        let output = unsafe {
            write_to_buffer(&mut SLOTS_BUFFER, |writer| {
                for (i, (_, name)) in flash::FLASH_SLOTS.iter().enumerate() {
                    let _ = write!(writer, "  {}: {}\r\n", i + 1, name);
                }
            })
        };

        CommandResult::Ok(Some(output))
    }
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

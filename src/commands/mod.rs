mod handlers;
use handlers::*;

use heapless::Vec;
use rp_pico::hal;
use usbd_serial::SerialPort;

pub static COMMANDS: &[Command] = &[
    Command {
        name: "write",
        help: "Write text to slot n: write <n> <text>",
        handler: handle_write,
    },
    Command {
        name: "read",
        help: "Read text from slot n: read <n>",
        handler: handle_read,
    },
    Command {
        name: "slots",
        help: "List available storage slots",
        handler: handle_slots,
    },
    Command {
        name: "help",
        help: "Show available commands",
        handler: handle_help,
    },
    Command {
        name: "reboot",
        help: "Restart the device",
        handler: handle_reboot,
    },
    Command {
        name: "bootloader",
        help: "Enter USB bootloader mode",
        handler: handle_bootloader,
    },
    Command {
        name: "buffer",
        help: "Print HELP_BUFFER_SIZE",
        handler: handle_buffer,
    },
];

pub enum CommandResult {
    Ok(Option<&'static [u8]>),
    Error(&'static str),
}

pub struct Command {
    pub name: &'static str,
    pub help: &'static str,
    pub handler: fn(&mut SerialPort<hal::usb::UsbBus>, &[&str]) -> CommandResult,
}

pub struct CommandRegistry {
    commands: &'static [Command],
}

impl CommandRegistry {
    pub const fn new(commands: &'static [Command]) -> Self {
        Self { commands }
    }

    pub fn execute(&self, serial: &mut SerialPort<hal::usb::UsbBus>, line: &str) -> CommandResult {
        let mut parts: Vec<&str, 8> = Vec::new();
        for part in line.split(' ') {
            if parts.push(part).is_err() {
                return CommandResult::Error("Too many arguments");
            }
        }

        let command_name = match parts.first() {
            Some(name) => *name,
            None => return CommandResult::Error("Empty command"),
        };

        for cmd in self.commands {
            if cmd.name == command_name {
                return (cmd.handler)(serial, &parts[1..]);
            }
        }

        CommandResult::Error("Unknown command")
    }
}

mod handlers;
use handlers::*;

use crate::context::{self, Context};

use heapless::String;
use heapless::Vec;
use rp_pico::hal;
use usbd_serial::SerialPort;

#[derive(Debug)]
pub enum CommandArgs {
    None(()),
    Slot(usize),
    WriteSlot(usize, String<64>),
    Led(&'static str),
}

pub trait Command: Send + Sync {
    type Args;
    fn name(&self) -> &'static str;
    fn help(&self) -> &'static str;
    fn parse(&self, args: &[&str]) -> Result<Self::Args, &'static str>;
    fn execute(
        &self,
        args: Self::Args,
        context: &Context,
        serial: &mut SerialPort<'static, hal::usb::UsbBus>,
    ) -> CommandResult;
}

pub static COMMANDS: &[&dyn Command<Args = CommandArgs>] = &[
    &WriteCommand,
    &ReadCommand,
    &SlotsCommand,
    &HelpCommand,
    &RebootCommand,
    &BootloaderCommand,
    &LedCommand,
    &TempCommand,
];

pub enum CommandResult {
    Ok(Option<&'static [u8]>),
    Error(&'static str),
}

pub struct CommandRegistry {
    commands: &'static [&'static dyn Command<Args = CommandArgs>],
}

impl CommandRegistry {
    pub const fn new(commands: &'static [&'static dyn Command<Args = CommandArgs>]) -> Self {
        Self { commands }
    }

    pub fn execute(
        &self,
        line: &str,
        serial: &mut SerialPort<'static, hal::usb::UsbBus>,
    ) -> CommandResult {
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

        for &cmd in self.commands {
            if cmd.name() == command_name {
                return context::with_context(|ctx| match cmd.parse(&parts[1..]) {
                    Ok(args) => cmd.execute(args, ctx, serial),
                    Err(e) => CommandResult::Error(e),
                });
            }
        }

        CommandResult::Error("Unknown command")
    }
}

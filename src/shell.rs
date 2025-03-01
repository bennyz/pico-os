use crate::commands;
use crate::{Error, MAX_COMMAND_LENGTH, NEWLINE, PROMPT};
use defmt::*;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_time::{Duration, Timer};
use embassy_usb::class::cdc_acm::CdcAcmClass;
use embassy_usb::driver::EndpointError;

/// Handles conversion of USB endpoint errors to our Error type
impl From<EndpointError> for Error {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => Error::BufferOverflow,
            EndpointError::Disabled => Error::Disconnected,
        }
    }
}

/// Shell handler that processes commands from the USB serial interface
pub struct Shell<'d> {
    class: &'d mut CdcAcmClass<'d, Driver<'d, USB>>,
    command_buf: [u8; MAX_COMMAND_LENGTH],
    command_pos: usize,
}

impl<'d> Shell<'d> {
    pub fn new(class: &'d mut CdcAcmClass<'d, Driver<'d, USB>>) -> Self {
        Self {
            class,
            command_buf: [0; MAX_COMMAND_LENGTH],
            command_pos: 0,
        }
    }

    pub async fn wait_connection(&mut self) {
        while !self.class.dtr() {
            Timer::after(Duration::from_millis(10)).await;
            debug!("Waiting for DTR...");
        }
    }

    pub async fn send_welcome(&mut self) -> Result<(), Error> {
        let welcome_msg = b"Welcome to Pico OS\r\nType 'exit' to quit\r\n> ";
        self.class.write_packet(welcome_msg).await.map_err(Into::into)
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        self.wait_connection().await;
        self.send_welcome().await?;

        let mut read_buf = [0; 64];

        loop {
            match self.class.read_packet(&mut read_buf).await {
                Ok(n) if n > 0 => {
                    let data = &read_buf[..n];
                    info!("Received: {:?}", core::str::from_utf8(data).unwrap_or("Invalid UTF-8"));
                    
                    self.class.write_packet(data).await?;

                    if let Err(e) = self.process_input(data).await {
                        return Err(e);
                    }
                }
                Ok(_) => {} // Zero-length packet, ignore
                Err(e) => return Err(e.into()),
            }
        }
    }

    async fn process_input(&mut self, data: &[u8]) -> Result<(), Error> {
        for &byte in data {
            // Only add printable ASCII or newlines
            if (byte >= 32 && byte <= 126) || byte == b'\r' || byte == b'\n' {
                if self.command_pos < self.command_buf.len() {
                    self.command_buf[self.command_pos] = byte;
                    self.command_pos += 1;
                }
            }
        }

        if data.contains(&b'\r') || data.contains(&b'\n') {
            let cmd_str = {
                let cmd = core::str::from_utf8(&self.command_buf[..self.command_pos])
                    .unwrap_or("")
                    .trim_end_matches(&['\r', '\n'][..])
                    .trim();
                
                let mut cmd_copy = heapless::String::<MAX_COMMAND_LENGTH>::new();
                let _ = cmd_copy.push_str(cmd);
                cmd_copy
            };

            let result = self.execute_command(&cmd_str).await?;
            
            // Reset command buffer for next command
            self.command_pos = 0;
            
            if !result {
                return Ok(());
            }
            
            self.class.write_packet(NEWLINE).await?;
            self.class.write_packet(PROMPT).await?;
        }

        Ok(())
    }

    /// Execute a command and return true to continue, false to exit
    async fn execute_command(&mut self, cmd: &str) -> Result<bool, Error> {
        if cmd.is_empty() {
            return Ok(true);
        }

        match cmd {
            "exit" => {
                self.class.write_packet(b"\r\nGoodbye!\r\n").await?;
                Timer::after(Duration::from_millis(100)).await;
                Ok(false) // Signal to exit
            },
            // Execute from commands module
            _ => {
                commands::execute_command(self.class, cmd).await?;
                Ok(true)
            }
        }
    }
}
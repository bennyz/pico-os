use crate::commands::{CommandRegistry, CommandResult};
use fugit::MicrosDurationU32;
use hal::rom_data::reset_to_usb_boot;
use hal::Watchdog;
use heapless::String;
use rp_pico::hal;
use usb_device::class_prelude::*;
use usb_device::prelude::*;
use usbd_serial::SerialPort;

pub struct UsbSerial {
    serial: SerialPort<'static, hal::usb::UsbBus>,
    device: UsbDevice<'static, hal::usb::UsbBus>,
    line_buffer: String<64>,
}

static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;

impl UsbSerial {
    pub fn new(usb_bus: UsbBusAllocator<hal::usb::UsbBus>) -> Self {
        // SAFETY: This is safe as we only call this once during initialization
        unsafe {
            USB_BUS = Some(usb_bus);
            let bus_ref = USB_BUS.as_ref().unwrap();

            let serial = SerialPort::new(bus_ref);
            let device = UsbDeviceBuilder::new(bus_ref, UsbVidPid(0x2E8A, 0x000A))
                .strings(&[StringDescriptors::default()
                    .manufacturer("RPI")
                    .product("Pico OS")
                    .serial_number("TEST")])
                .unwrap()
                .device_class(2)
                .build();

            Self {
                serial,
                device,
                line_buffer: String::new(),
            }
        }
    }

    pub fn init(&mut self) {
        let _ = self.serial.write(b"\r\n=== USB Serial Example ===\r\n");
        let _ = self
            .serial
            .write(b"Type 'help' for available commands.\r\n> ");
    }

    pub fn poll(&mut self, registry: &CommandRegistry, watchdog: &mut Watchdog) {
        if self.device.poll(&mut [&mut self.serial]) {
            let mut buf = [0u8; 64];
            match self.serial.read(&mut buf) {
                Ok(count) if count > 0 => {
                    self.handle_input(&buf[..count], registry, watchdog);
                }
                _ => {}
            }
        }
    }

    fn handle_input(&mut self, input: &[u8], registry: &CommandRegistry, watchdog: &mut Watchdog) {
        for &b in input {
            match b {
                b'\r' | b'\n' => self.handle_line(registry, watchdog),
                8 | 127 => self.handle_backspace(),
                b if (32..127).contains(&b) => self.handle_char(b),
                _ => {}
            }
        }
    }

    fn handle_line(&mut self, registry: &CommandRegistry, watchdog: &mut Watchdog) {
        if !self.line_buffer.is_empty() {
            match registry.execute(&mut self.serial, &self.line_buffer) {
                CommandResult::Ok(Some(data)) => {
                    let _ = self.serial.write(b"\r\n");
                    let _ = self.serial.write(data);

                    match self.line_buffer.as_str() {
                        "reboot" => {
                            watchdog.start(MicrosDurationU32::millis(1));
                            loop {}
                        }
                        "bootloader" => reset_to_usb_boot(0, 0),
                        _ => {}
                    }
                }
                CommandResult::Ok(None) => {
                    let _ = self.serial.write(b"\r\nOK");
                }
                CommandResult::Error(e) => {
                    let _ = self.serial.write(b"\r\nError: ");
                    let _ = self.serial.write(e.as_bytes());
                }
            }
            let _ = self.serial.write(b"\r\n> ");
            self.line_buffer.clear();
        } else {
            let _ = self.serial.write(b"\r\n> ");
        }
    }

    fn handle_backspace(&mut self) {
        if !self.line_buffer.is_empty() {
            self.line_buffer.pop();
            let _ = self.serial.write(b"\x08 \x08");
        }
    }

    fn handle_char(&mut self, b: u8) {
        if self.line_buffer.push(b as char).is_ok() {
            let _ = self.serial.write(&[b]);
        }
    }
}

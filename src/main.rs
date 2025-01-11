#![no_std]
#![no_main]

use core::fmt::Write;
use cortex_m;
use fugit::MicrosDurationU32;
use hal::rom_data::reset_to_usb_boot;
use heapless::String;
use panic_halt as _;
use rp_pico::entry;
use rp_pico::hal;
use rp_pico::hal::pac;
use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    #[cfg(feature = "rp2040-e5")]
    {
        let sio = hal::Sio::new(pac.SIO);
        let _pins = rp_pico::Pins::new(
            pac.IO_BANK0,
            pac.PADS_BANK0,
            sio.gpio_bank0,
            &mut pac.RESETS,
        );
    }

    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut serial = SerialPort::new(&usb_bus);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .strings(&[StringDescriptors::default()
            .manufacturer("Fake company")
            .product("Serial port")
            .serial_number("TEST")])
        .unwrap()
        .device_class(2)
        .build();

    let mut counter = 0;
    let mut last_time = 0;
    let mut init_done = false;
    let mut line_buffer = heapless::String::<64>::new();

    loop {
        if !init_done {
            let _ = serial.write(b"\r\n=== USB Serial Example ===\r\n");
            let _ = serial.write(b"Type something and press enter.\r\n> ");
            init_done = true;
        }

        let current_time = timer.get_counter().ticks();

        if current_time - last_time >= 125_000_000 {
            let mut text: String<64> = String::new();
            writeln!(&mut text, "Heartbeat #{}\r\n", counter).unwrap();
            let _ = serial.write(text.as_bytes());

            counter += 1;
            last_time = current_time;
        }

        if usb_dev.poll(&mut [&mut serial]) {
            let mut buf = [0u8; 64];
            match serial.read(&mut buf) {
                Ok(count) if count > 0 => {
                    for &b in &buf[..count] {
                        match b {
                            b'\r' | b'\n' => {
                                if !line_buffer.is_empty() {
                                    match line_buffer.as_str() {
                                        "reboot" => {
                                            let _ = serial.write(b"\r\nRebooting...\r\n");
                                            watchdog.start(MicrosDurationU32::millis(1));
                                            loop {}
                                        }
                                        "bootloader" => {
                                            let _ = serial.write(
                                                b"\r\nRebooting into bootloader mode...\r\n",
                                            );
                                            reset_to_usb_boot(0, 0);
                                        }
                                        _ => {
                                            let _ = serial.write(b"\r\nYou typed: ");
                                            let _ = serial.write(line_buffer.as_bytes());
                                            let _ = serial.write(b"\r\n> ");
                                        }
                                    }
                                    line_buffer.clear();
                                }
                            }
                            8 | 127 => {
                                if !line_buffer.is_empty() {
                                    line_buffer.pop();
                                    let _ = serial.write(b"\x08 \x08");
                                }
                            }
                            b if b >= 32 && b < 127 => {
                                if line_buffer.push(b as char).is_ok() {
                                    let _ = serial.write(&[b]);
                                }
                            }
                            _ => { /* ignore other characters */ }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

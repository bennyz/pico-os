#![no_std]
#![no_main]

mod commands;
mod flash;
mod usb;

use crate::commands::CommandRegistry;

use panic_halt as _;
use rp_pico::hal::pac;
use rp_pico::{entry, hal};
use usb::UsbSerial;
use usb_device::class_prelude::UsbBusAllocator;

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

    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut usb = UsbSerial::new(usb_bus);
    let registry = CommandRegistry::new(commands::COMMANDS);

    usb.init();

    loop {
        usb.poll(&registry, &mut watchdog);
    }
}

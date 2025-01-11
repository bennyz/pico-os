#![no_std]
#![no_main]

mod commands;
mod context;
mod flash;
mod usb;

use crate::commands::CommandRegistry;

use cortex_m::asm::delay;
use panic_halt as _;
use rp_pico::hal::{pac, Watchdog};
use rp_pico::{entry, hal};
use usb::UsbSerial;
use usb_device::class_prelude::UsbBusAllocator;

use context::init as init_context;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    static mut WATCHDOG: Option<Watchdog> = None;
    let watchdog = unsafe {
        WATCHDOG = Some(hal::Watchdog::new(pac.WATCHDOG));
        WATCHDOG.as_mut().unwrap()
    };

    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        watchdog,
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

    init_context(watchdog);
    usb.init();

    loop {
        usb.poll(&registry);
    }
}

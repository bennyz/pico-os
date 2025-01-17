#![no_std]
#![no_main]

mod commands;
mod context;
mod flash;
mod usb;

use crate::commands::CommandRegistry;

use panic_halt as _;
use rp_pico::hal::{pac, Adc, Clock, Watchdog};
use rp_pico::{entry, hal};
use usb::UsbSerial;
use usb_device::class_prelude::UsbBusAllocator;

use context::init as init_context;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

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

    let delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let adc = Adc::new(pac.ADC, &mut pac.RESETS);

    let sio = hal::Sio::new(pac.SIO);
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    let led_pin = pins.led.into_push_pull_output();

    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut usb = UsbSerial::new(usb_bus);
    let registry = CommandRegistry::new(commands::COMMANDS);

    init_context(watchdog, led_pin, delay, adc);
    usb.init();

    loop {
        usb.poll(&registry);
    }
}

use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::{Builder, Config};

// Bind USB interrupts
bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

/// Initializes the USB CDC ACM (serial) device
pub fn setup_usb<'d>(
    usb_peripheral: USB,
    state: &'d mut State<'d>,
    config_desc: &'d mut [u8; 256],
    bos_desc: &'d mut [u8; 256],
    control_buf: &'d mut [u8; 64],
) -> (
    Builder<'d, Driver<'d, USB>>,
    CdcAcmClass<'d, Driver<'d, USB>>,
) {
    let driver = Driver::new(usb_peripheral, Irqs);

    // Configure USB device
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Pico OS");
    config.product = Some("USB Serial");
    config.serial_number = Some("123456");

    // Create USB builder
    let mut builder = Builder::new(
        driver,
        config,
        config_desc,
        bos_desc,
        &mut [],
        control_buf,
    );

    // Create CDC ACM class
    let class = CdcAcmClass::new(&mut builder, state, 64);

    (builder, class)
}
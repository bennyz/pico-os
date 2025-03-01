#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::block::ImageDef;
use embassy_time::{Duration, Timer};
use pico_os_embassy::{info, usb, shell::Shell};
use {defmt_rtt as _, panic_probe as _};

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: ImageDef = ImageDef::secure_exe();

#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"Pico OS"),
    embassy_rp::binary_info::rp_program_description!(c"Pico OS"),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    info!("Pico OS starting...");
    Timer::after(Duration::from_millis(1000)).await;
    
    // Initialize USB and CDC-ACM class
    let mut state = embassy_usb::class::cdc_acm::State::new();
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut control_buf = [0; 64];
    
    let (builder, mut class) = usb::setup_usb(
        p.USB, 
        &mut state,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut control_buf,
    );
    
    let mut usb = builder.build();
    info!("USB device initialized");

    // Create a shell instance
    let mut shell = Shell::new(&mut class);

    // Run USB device and shell tasks
    let usb_fut = usb.run();
    let shell_fut = async {
        loop {
            match shell.run().await {
                Ok(()) => {
                    info!("Shell exited normally");
                },
                Err(e) => {
                    warn!("Shell disconnected: {}", e);
                    // Wait before reconnecting
                    Timer::after(Duration::from_millis(1000)).await;
                }
            }
        }
    };

    // Run both tasks concurrently
    join(usb_fut, shell_fut).await;
}
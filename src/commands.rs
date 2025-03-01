use crate::{Error, NEWLINE, Timer, Duration};
use embassy_rp::usb::{Driver, Instance};
use embassy_rp::bind_interrupts;
use embassy_rp::adc::{Adc, Channel, Config, InterruptHandler};
use embassy_rp::watchdog::Watchdog;
use heapless::Vec;
use embassy_usb::class::cdc_acm::CdcAcmClass;
use core::fmt::Write;

bind_interrupts!(struct AdcIrqs {
    ADC_IRQ_FIFO => InterruptHandler;
});

pub async fn write_chunked<'d, T: Instance + 'd>(
    class: &mut CdcAcmClass<'d, Driver<'d, T>>,
    data: &[u8],
) -> Result<(), Error> {
    for chunk in data.chunks(64) {
        class.write_packet(chunk).await?;
        Timer::after(Duration::from_millis(10)).await;
    }
    Ok(())
}

pub struct CommandInfo {
    pub name: &'static str,
    pub description: &'static str,
}

pub fn get_commands() -> &'static [CommandInfo] {
    &[
        CommandInfo {
            name: "help",
            description: "Show this help information",
        },
        CommandInfo {
            name: "version",
            description: "Show firmware version",
        },
        CommandInfo {
            name: "echo",
            description: "Echo the arguments",
        },
        CommandInfo {
            name: "reboot",
            description: "Reboot the Pico",
        },
        CommandInfo {
            name: "bootloader",
            description: "Reboot into USB bootloader mode",
        },
        CommandInfo {
            name: "temp",
            description: "Show the Pico's temperature",
        },
    ]
}

pub async fn execute_command<'d, T: Instance + 'd>(
    class: &mut CdcAcmClass<'d, Driver<'d, T>>,
    cmd: &str,
) -> Result<(), Error> {
    let parts: Vec<&str, 8> = heapless::Vec::from_iter(cmd.split_whitespace());
    
    if parts.is_empty() {
        return Ok(());
    }

    let command_name = parts[0];
    let args = &parts[1..];
    
    match command_name {
        "help" => cmd_help(class).await,
        "version" => cmd_version(class).await,
        "echo" => cmd_echo(class, args).await,
        "reboot" => cmd_reboot(class).await,
        "bootloader" => cmd_bootloader(class).await,
        "temp" => cmd_temp(class).await,
        _ => {
            class.write_packet(b"\r\nUnknown command: ").await?;
            class.write_packet(command_name.as_bytes()).await?;
            class.write_packet(NEWLINE).await?;
            Ok(())
        }
    }
}

async fn cmd_help<'d, T: Instance + 'd>(
    class: &mut CdcAcmClass<'d, Driver<'d, T>>,
) -> Result<(), Error> {
    class.write_packet(b"\r\nAvailable commands:\r\n").await?;
    
    for cmd in get_commands() {
        let prefix = b"  ";
        class.write_packet(prefix).await?;
        class.write_packet(cmd.name.as_bytes()).await?;
        
        // Pad command name to align descriptions
        let padding = 10 - cmd.name.len();
        for _ in 0..padding {
            class.write_packet(b" ").await?;
        }
        
        class.write_packet(b"- ").await?;
        class.write_packet(cmd.description.as_bytes()).await?;
        class.write_packet(NEWLINE).await?;
    }
    
    Ok(())
}

async fn cmd_version<'d, T: Instance + 'd>(
    class: &mut CdcAcmClass<'d, Driver<'d, T>>,
) -> Result<(), Error> {
    class.write_packet(b"\r\nPico OS v0.1.0\r\n").await?;
    Ok(())
}

async fn cmd_echo<'d, T: Instance + 'd>(
    class: &mut CdcAcmClass<'d, Driver<'d, T>>,
    args: &[&str],
) -> Result<(), Error> {
    class.write_packet(NEWLINE).await?;
    
    if args.is_empty() {
        return Ok(());
    }
    
    let mut output = heapless::String::<64>::new();
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            let _ = output.push_str(" ");
        }
        let _ = output.push_str(arg);
    }
    
    class.write_packet(output.as_bytes()).await?;
    class.write_packet(NEWLINE).await?;
    Ok(())
}

async fn cmd_reboot<'d, T: Instance + 'd>(
    class: &mut CdcAcmClass<'d, Driver<'d, T>>,
) -> Result<(), Error> {
    class.write_packet(b"\r\nRebooting Pico in 1 second...\r\n").await?;
    
    // Let the USB transaction complete before rebooting
    Timer::after(Duration::from_millis(1000)).await;
    
    // Note: In a more integrated system, we would use p.WATCHDOG
    // instead of using unwrap(), but this works for demonstration
    let mut watchdog = Watchdog::new(unsafe { 
        embassy_rp::peripherals::WATCHDOG::steal() 
    });
    
    // Set watchdog to reset if not fed within 1 millisecond
    watchdog.start(Duration::from_millis(1));
    
    // Wait for the watchdog to reset us
    loop {
        Timer::after(Duration::from_millis(10)).await;
    }
}

async fn cmd_bootloader<'d, T: Instance + 'd>(
    class: &mut CdcAcmClass<'d, Driver<'d, T>>,
) -> Result<(), Error> {
    class.write_packet(b"\r\nEntering USB bootloader mode...\r\n").await?;
    
    Timer::after(Duration::from_millis(1000)).await;
    
    embassy_rp::rom_data::reboot(0x01000000, 100, 0, 0);
    
    loop {
        Timer::after(Duration::from_millis(100)).await;
    }
}

fn convert_to_celsius(raw_temp: u16) -> f32 {
    let temp = 27.0 - (raw_temp as f32 * 3.3 / 4096.0 - 0.706) / 0.001721;
    let sign = if temp < 0.0 { -1.0 } else { 1.0 };
    let rounded_temp_x10: i16 = ((temp * 10.0) + 0.5 * sign) as i16;
    (rounded_temp_x10 as f32) / 10.0
}

async fn cmd_temp<'d, T: Instance + 'd>(
    class: &mut CdcAcmClass<'d, Driver<'d, T>>,
) -> Result<(), Error> {
    class.write_packet(b"\r\nReading temperature sensor...\r\n").await?;
    
    let p_adc = unsafe { embassy_rp::peripherals::ADC::steal() };
    let mut adc = Adc::new(p_adc, AdcIrqs {}, Config::default());
    
    let p_temp_sensor = unsafe { embassy_rp::peripherals::ADC_TEMP_SENSOR::steal() };
    let mut temp_sensor = Channel::new_temp_sensor(p_temp_sensor);
    
    match adc.read(&mut temp_sensor).await {
        Ok(raw_temp) => {
            let temp_c = convert_to_celsius(raw_temp);
            
            let mut temp_str = heapless::String::<64>::new();
            let _ = write!(temp_str, "Temperature: {:.1} °C\r\n", temp_c);
            class.write_packet(temp_str.as_bytes()).await?;
        },
        Err(_) => {
            class.write_packet(b"Error reading temperature sensor\r\n").await?;
        }
    }
    
    Ok(())
}
# Pico OS

A minimal operating system for the Raspberry Pi Pico 2350

## Features

- USB serial interface
- Command-line interface with basic commands:
  - `help` - Show available commands
  - `version` - Display firmware version
  - `echo` - Echo back arguments
  - `reboot` - Reboot the Pico
  - `bootloader` - Enter USB bootloader mode
  - `temp` - Display the Pico's temperature

## Development

### Prerequisites

- Rust toolchain with `thumbv8m-none-eabi` target
- `cargo-embed` or similar flashing tool
- Raspberry Pi Pico

### Building

```bash
cargo build --release
```

### Flashing

Connect your Pico while holding the BOOTSEL button, then:

```bash
cargo run --release
```

## Project Structure

- `src/commands.rs` - Command implementations
- `memory.x` - Memory layout configuration
- `build.rs` - Build configuration

## License

[Add your chosen license here]

## Contributing

[Add contribution guidelines here]
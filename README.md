# Rusty NES 🦀🎮

A Nintendo Entertainment System (NES) emulator written in Rust. This project aims to explore low-level system emulation and eventually run classic NES games and custom homebrew.

## Current Status

*   **Phase 1: Bootstrapping & Rust Warm-Up**
    *   Project setup with Rust and Cargo.
    *   iNES file format header parsing: Can load `.nes` files, read their headers, and extract information like PRG/CHR ROM sizes, mapper number, and mirroring type.
    *   Basic PRG and CHR ROM data loading.
    *   Unit tests for header parsing logic.

## Getting Started

### Prerequisites

*   [Rust programming language and Cargo](https://www.rust-lang.org/tools/install) (latest stable version recommended).

### Building

1.  Clone the repository (if you haven't already):
    ```bash
    # If you are setting this up from scratch elsewhere:
    # git clone <your-repo-url>
    cd rusty-nes
    ```
2.  Build the project:
    ```bash
    cargo build
    ```
    For a release build (optimized):
    ```bash
    cargo build --release
    ```

### Running

The emulator loads a `.nes` ROM file specified on the command line and prints its
header information.

1.  Ensure you have a `.nes` ROM file. For initial testing, a simple Mapper 0 ROM is recommended.
2.  Run the emulator and pass the path to your ROM file:

    ```bash
    cargo run -- path/to/your_rom.nes
    ```
    The output will display the parsed header information from the ROM.

    *(Future versions will start actual emulation.)*

### Running Tests

To run the unit tests for components like the cartridge loader:
```bash
cargo test
```

## Project Structure

*   `src/`: Contains the Rust source code for the emulator components.
    *   `main.rs`: Main entry point, currently used for testing ROM loading.
    *   `cartridge.rs`: Logic for loading and parsing `.nes` ROM files and their headers.
    *   `cpu.rs`: (Planned) NES CPU (Ricoh 2A03, based on 6502) emulation.
    *   `ppu.rs`: (Planned) Picture Processing Unit emulation.
    *   `bus.rs`: (Planned) System bus connecting CPU, PPU, RAM, and cartridge.
    *   `apu.rs`: (Planned) Audio Processing Unit emulation.
*   `project/`: Contains project planning documents.
    *   `rusty-nes.md`: Detailed development roadmap and phases.
*   `Cargo.toml`: Project manifest and dependencies.

## Development Roadmap

The emulator is being developed in phases. The next major phase involves implementing the CPU.
My goal is learn a trick or two contribute to the community (maybe) and have fun.

---

This README will be updated as the project progresses. 
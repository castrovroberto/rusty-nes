# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

NES emulator in Rust (edition 2024, no external dependencies). Built in phases: iNES cartridge loading (phase 1) and a full 6502 CPU (phase 2) are done; the system bus is partially wired; `ppu.rs` and `apu.rs` are empty placeholders. The `main` binary only loads a ROM and prints its header. It doesn't run emulation yet.

## Commands

```bash
cargo build
cargo run -- path/to/rom.nes        # prints iNES header info
cargo test                          # unit tests + nestest integration test
cargo test --test nestest           # only the nestest CPU conformance test
cargo test nestest_official_opcodes # a single test by name
cargo test cpu::tests::             # tests in one module
```

Run tests from the repo root: `tests/nestest.rs` opens `tests/roms/nestest.nes` with a relative path.

## Architecture

- **`CpuBus` trait (`src/cpu.rs`)** is the seam between the CPU and memory. `Cpu<B: CpuBus>` is generic over its bus and owns it (`cpu.bus`). There are two implementations in `src/bus.rs`:
  - `Bus`: the real NES memory map (2 KB RAM mirrored through `$1FFF`, cartridge PRG at `$8000–$FFFF` with NROM-128 mirroring). PPU (`$2000–$3FFF`) and APU/IO (`$4000–$4017`) ranges are stubbed to return 0. Only mapper 0 is handled.
  - `FlatBus`: a flat 64 KB array used by the CPU unit tests and nestest, so they don't depend on the real memory map. Keep CPU tests on `FlatBus`.
- **CPU (`src/cpu.rs`)**: `step()` runs one instruction through a single big `match opcode`, and each arm returns that instruction's cycle count. Addressing-mode helpers return `(addr, page_crossed)`, and the arm adds the page-cross penalty itself (e.g. `4 + p as u8`). `step()` adds that count to `cpu.cycles` and also returns it, and `nmi()`/`irq()` add 7. Unofficial opcodes (LAX, SAX, DCP, ISB, SLO, RLA, SRE, RRA, unofficial NOPs) are implemented. `read_u16_bugged` reproduces the 6502 JMP-indirect page-wrap bug. `disassemble()` produces nestest-format text for logging.
- **Cartridge (`src/cartridge.rs`)**: `Cartridge::from_file` parses the 16-byte iNES header into `NesHeader` (mapper number, `Mirroring`, battery, trainer, four-screen) and loads the trainer, PRG and CHR data. Errors are `String`s.

## nestest conformance

`tests/nestest.rs` loads the 16 KB PRG into `FlatBus` at both `$8000` and `$C000`, sets PC=`$C000`, SP=`$FD`, P=`$24`, cycles=7, then steps the CPU and compares **PC and CYC only** against `tests/roms/nestest.log` line by line. Register mismatches alone won't fail it. `nestest_official_opcodes` covers lines 1–5003, and `nestest_all_opcodes` also covers the unofficial opcodes. Any CPU change has to keep both passing. The test file keeps its own `opcode_len` table, so update it if instruction lengths change.

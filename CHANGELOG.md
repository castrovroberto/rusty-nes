# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `GLOSSARY.md` defining the project's domain vocabulary (cartridges, memory, timing).
- ADR 0001: the CPU runs one instruction per step and the PPU catches up afterwards.
- `CLAUDE.md` with build/test commands and an architecture overview.
- System bus with the NES CPU memory map: 2 KB internal RAM with its mirrored range up to `$1FFF`, and Mapper 0 (NROM) PRG ROM at `$8000–$FFFF`, mirrored at `$C000` for 16 KB images. PPU and APU/IO registers are stubbed.
- `FlatBus`, a flat 64 KB bus for CPU testing.
- Ricoh 2A03 (6502) CPU with the full official and unofficial instruction set, CPU cycle counting, and NMI/IRQ handling.
- Disassembler producing nestest-format trace lines.
- nestest integration test checking PC and CPU cycle count against the reference log for every instruction.
- Command-line ROM image path: `cargo run -- path/to/rom.nes`.
- iNES ROM image loading: header parsing (PRG/CHR ROM sizes, mapper number, nametable mirroring, battery, trainer) and PRG ROM, CHR ROM and trainer data.

### Changed

- README updated to reflect CPU and bus progress.

### Removed

- `NesHeader::four_screen_mode`, which duplicated `Mirroring::FourScreen`.

### Fixed

- Header parsing unit test.

# Rusty NES

An emulator for the Nintendo Entertainment System: the console's CPU, PPU, APU and memory map, driven by game cartridges loaded from ROM images.

## Language

### Cartridges

**ROM image**:
A `.nes` file on disk in iNES format, from which a Cartridge is loaded.
_Avoid_: ROM, ROM file, game file

**Cartridge**:
The emulated game board: its PRG ROM, CHR ROM, mapper and nametable mirroring.
_Avoid_: ROM, cart, game

**PRG ROM**:
The cartridge memory holding the program the CPU executes.
_Avoid_: program ROM, ROM

**CHR ROM**:
The cartridge memory holding the graphics tile data the PPU reads.
_Avoid_: character ROM, pattern ROM, ROM

### Memory

**Nametable mirroring**:
How a cartridge arranges the PPU's nametables: horizontal, vertical or four-screen. Uses the NESdev names, even though they describe the mirroring direction rather than the layout.
_Avoid_: mirroring (unqualified), arrangement

**Mirrored range**:
An address range that repeats another region of memory, so different addresses reach the same bytes.
_Avoid_: mirroring (unqualified), mirror

### Timing

**CPU cycle**:
One tick of the CPU's clock, the unit in which instruction timing is measured.
_Avoid_: cycle (unqualified), clock

**PPU dot**:
One tick of the PPU's clock, producing one pixel position. There are three PPU dots per CPU cycle on NTSC.
_Avoid_: cycle (unqualified), PPU cycle, pixel clock

**Step**:
The execution of one CPU instruction, including servicing a pending interrupt.
_Avoid_: tick, clock

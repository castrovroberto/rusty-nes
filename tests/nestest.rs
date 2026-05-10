// nestest.nes integration test
// Loads the ROM at $C000/$8000, starts at $C000, and compares logged state
// against the reference log line by line.

use rusty_nes::cpu::{Cpu, disassemble};
use rusty_nes::cartridge::Cartridge;

fn load_nestest(cpu: &mut Cpu) {
    let cart = Cartridge::from_file("tests/roms/nestest.nes")
        .expect("tests/roms/nestest.nes not found — run from project root");

    // nestest is 16KB PRG — mirror it at $8000 and $C000
    for i in 0..cart.prg_rom.len() {
        cpu.memory[0x8000 + i] = cart.prg_rom[i];
        cpu.memory[0xC000 + i] = cart.prg_rom[i];
    }
}

fn cpu_log_line(cpu: &Cpu) -> String {
    let pc = cpu.pc;
    let op0 = cpu.read_u8(pc);
    let op1 = cpu.read_u8(pc.wrapping_add(1));
    let op2 = cpu.read_u8(pc.wrapping_add(2));

    let instr_len = opcode_len(op0);
    let bytes_str = match instr_len {
        1 => format!("{:02X}      ", op0),
        2 => format!("{:02X} {:02X}   ", op0, op1),
        3 => format!("{:02X} {:02X} {:02X}", op0, op1, op2),
        _ => format!("{:02X}      ", op0),
    };

    let disasm = disassemble(cpu, pc);

    format!(
        "{:04X}  {}  {:<31} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} CYC:{}",
        pc,
        bytes_str,
        disasm,
        cpu.a,
        cpu.x,
        cpu.y,
        cpu.status,
        cpu.sp,
        cpu.cycles
    )
}

fn opcode_len(op: u8) -> u8 {
    match op {
        // 1-byte opcodes
        0x00 | 0x08 | 0x0A | 0x18 | 0x28 | 0x2A | 0x38 | 0x40 | 0x48 | 0x4A |
        0x58 | 0x60 | 0x68 | 0x6A | 0x78 | 0x88 | 0x8A | 0x98 | 0x9A | 0xA8 |
        0xAA | 0xB8 | 0xBA | 0xC8 | 0xCA | 0xD8 | 0xE8 | 0xEA | 0xF8 |
        // unofficial 1-byte NOPs
        0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => 1,

        // 2-byte opcodes
        0x01 | 0x03 | 0x04 | 0x05 | 0x06 | 0x07 | 0x09 | 0x10 | 0x11 | 0x13 |
        0x14 | 0x15 | 0x16 | 0x17 | 0x21 | 0x23 | 0x24 | 0x25 | 0x26 | 0x27 |
        0x29 | 0x30 | 0x31 | 0x33 | 0x34 | 0x35 | 0x36 | 0x37 | 0x41 | 0x43 |
        0x44 | 0x45 | 0x46 | 0x47 | 0x49 | 0x50 | 0x51 | 0x53 | 0x54 | 0x55 |
        0x56 | 0x57 | 0x61 | 0x63 | 0x64 | 0x65 | 0x66 | 0x67 | 0x69 | 0x70 |
        0x71 | 0x73 | 0x74 | 0x75 | 0x76 | 0x77 | 0x80 | 0x81 | 0x82 | 0x83 |
        0x84 | 0x85 | 0x86 | 0x87 | 0x89 | 0x90 | 0x91 | 0x94 | 0x95 | 0x96 |
        0x97 | 0xA0 | 0xA1 | 0xA2 | 0xA3 | 0xA4 | 0xA5 | 0xA6 | 0xA7 | 0xA9 |
        0xB0 | 0xB1 | 0xB3 | 0xB4 | 0xB5 | 0xB6 | 0xB7 | 0xC0 | 0xC1 | 0xC2 |
        0xC3 | 0xC4 | 0xC5 | 0xC6 | 0xC7 | 0xC9 | 0xD0 | 0xD1 | 0xD3 | 0xD4 |
        0xD5 | 0xD6 | 0xD7 | 0xE0 | 0xE1 | 0xE2 | 0xE3 | 0xE4 | 0xE5 | 0xE6 |
        0xE7 | 0xE9 | 0xEB | 0xF0 | 0xF1 | 0xF3 | 0xF4 | 0xF5 | 0xF6 | 0xF7 | 0xF9 => 2,

        // 3-byte opcodes
        _ => 3,
    }
}

fn parse_cyc(line: &str) -> u64 {
    line.split("CYC:").nth(1)
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn parse_pc(line: &str) -> u16 {
    u16::from_str_radix(&line[0..4], 16).unwrap_or(0)
}

#[test]
fn nestest_official_opcodes() {
    let log = include_str!("roms/nestest.log");
    let reference_lines: Vec<&str> = log.lines().collect();

    let mut cpu = Cpu::new();
    load_nestest(&mut cpu);

    // nestest requires starting at $C000 with cycles=7 (as if after reset)
    cpu.pc = 0xC000;
    cpu.sp = 0xFD;
    cpu.status = 0x24;
    cpu.cycles = 7;

    let official_line_count = 5003; // Lines 1-5003 test official opcodes only

    for (i, &ref_line) in reference_lines.iter().enumerate().take(official_line_count) {
        let our_line = cpu_log_line(&cpu);

        // Compare PC and CYC which are the most critical fields
        let ref_pc = parse_pc(ref_line);
        let our_pc = parse_pc(&our_line);
        let ref_cyc = parse_cyc(ref_line);
        let our_cyc = parse_cyc(&our_line);

        if ref_pc != our_pc || ref_cyc != our_cyc {
            panic!(
                "Mismatch at log line {}:\n  expected: {}\n  got:      {}",
                i + 1, ref_line, our_line
            );
        }

        cpu.step();
    }
}

#[test]
fn nestest_all_opcodes() {
    let log = include_str!("roms/nestest.log");
    let reference_lines: Vec<&str> = log.lines().collect();

    let mut cpu = Cpu::new();
    load_nestest(&mut cpu);

    cpu.pc = 0xC000;
    cpu.sp = 0xFD;
    cpu.status = 0x24;
    cpu.cycles = 7;

    for (i, &ref_line) in reference_lines.iter().enumerate() {
        let our_line = cpu_log_line(&cpu);

        let ref_pc = parse_pc(ref_line);
        let our_pc = parse_pc(&our_line);
        let ref_cyc = parse_cyc(ref_line);
        let our_cyc = parse_cyc(&our_line);

        if ref_pc != our_pc || ref_cyc != our_cyc {
            panic!(
                "Mismatch at log line {}:\n  expected: {}\n  got:      {}",
                i + 1, ref_line, our_line
            );
        }

        cpu.step();
    }
}

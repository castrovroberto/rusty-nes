// NES CPU (Ricoh 2A03, based on MOS 6502) implementation

// Status flag bit masks
pub const FLAG_CARRY: u8 = 0x01;
pub const FLAG_ZERO: u8 = 0x02;
pub const FLAG_IRQ_DISABLE: u8 = 0x04;
pub const FLAG_DECIMAL: u8 = 0x08;
pub const FLAG_BREAK: u8 = 0x10;
pub const FLAG_UNUSED: u8 = 0x20;
pub const FLAG_OVERFLOW: u8 = 0x40;
pub const FLAG_NEGATIVE: u8 = 0x80;

pub struct Cpu {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub pc: u16,
    pub status: u8,
    pub cycles: u64,
    pub memory: [u8; 0x10000],
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            a: 0,
            x: 0,
            y: 0,
            sp: 0xFD,
            pc: 0,
            status: FLAG_IRQ_DISABLE | FLAG_UNUSED,
            cycles: 0,
            memory: [0; 0x10000],
        }
    }

    pub fn reset(&mut self) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0xFD;
        self.status = FLAG_IRQ_DISABLE | FLAG_UNUSED;
        self.pc = self.read_u16(0xFFFC);
        self.cycles = 7; // reset takes 7 cycles
    }

    // --- Memory access ---

    pub fn read_u8(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    pub fn write_u8(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }

    pub fn read_u16(&self, addr: u16) -> u16 {
        let lo = self.read_u8(addr) as u16;
        let hi = self.read_u8(addr.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    // 6502 page-wrap bug: high byte wraps within the same page
    fn read_u16_bugged(&self, addr: u16) -> u16 {
        let lo = self.read_u8(addr) as u16;
        let hi_addr = (addr & 0xFF00) | ((addr.wrapping_add(1)) & 0x00FF);
        let hi = self.read_u8(hi_addr) as u16;
        (hi << 8) | lo
    }

    // --- Stack ---

    fn stack_push(&mut self, data: u8) {
        self.write_u8(0x0100 | self.sp as u16, data);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn stack_pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        self.read_u8(0x0100 | self.sp as u16)
    }

    fn stack_push_u16(&mut self, data: u16) {
        self.stack_push((data >> 8) as u8);
        self.stack_push((data & 0xFF) as u8);
    }

    fn stack_pop_u16(&mut self) -> u16 {
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;
        (hi << 8) | lo
    }

    // --- Flags ---

    pub fn get_flag(&self, flag: u8) -> bool {
        self.status & flag != 0
    }

    pub fn set_flag(&mut self, flag: u8, value: bool) {
        if value {
            self.status |= flag;
        } else {
            self.status &= !flag;
        }
    }

    // Set N and Z flags based on a result value
    fn update_nz(&mut self, value: u8) {
        self.set_flag(FLAG_ZERO, value == 0);
        self.set_flag(FLAG_NEGATIVE, value & 0x80 != 0);
    }

    // --- Addressing modes ---
    // Returns (effective_address, page_crossed)

    fn addr_immediate(&mut self) -> (u16, bool) {
        let addr = self.pc;
        self.pc = self.pc.wrapping_add(1);
        (addr, false)
    }

    fn addr_zero_page(&mut self) -> (u16, bool) {
        let addr = self.read_u8(self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        (addr, false)
    }

    fn addr_zero_page_x(&mut self) -> (u16, bool) {
        let base = self.read_u8(self.pc);
        self.pc = self.pc.wrapping_add(1);
        (base.wrapping_add(self.x) as u16, false)
    }

    fn addr_zero_page_y(&mut self) -> (u16, bool) {
        let base = self.read_u8(self.pc);
        self.pc = self.pc.wrapping_add(1);
        (base.wrapping_add(self.y) as u16, false)
    }

    fn addr_absolute(&mut self) -> (u16, bool) {
        let addr = self.read_u16(self.pc);
        self.pc = self.pc.wrapping_add(2);
        (addr, false)
    }

    fn addr_absolute_x(&mut self) -> (u16, bool) {
        let base = self.read_u16(self.pc);
        self.pc = self.pc.wrapping_add(2);
        let addr = base.wrapping_add(self.x as u16);
        let crossed = (base & 0xFF00) != (addr & 0xFF00);
        (addr, crossed)
    }

    fn addr_absolute_y(&mut self) -> (u16, bool) {
        let base = self.read_u16(self.pc);
        self.pc = self.pc.wrapping_add(2);
        let addr = base.wrapping_add(self.y as u16);
        let crossed = (base & 0xFF00) != (addr & 0xFF00);
        (addr, crossed)
    }

    fn addr_indirect(&mut self) -> (u16, bool) {
        let ptr = self.read_u16(self.pc);
        self.pc = self.pc.wrapping_add(2);
        (self.read_u16_bugged(ptr), false)
    }

    fn addr_indexed_indirect_x(&mut self) -> (u16, bool) {
        let base = self.read_u8(self.pc);
        self.pc = self.pc.wrapping_add(1);
        let ptr = base.wrapping_add(self.x) as u16;
        let lo = self.read_u8(ptr) as u16;
        let hi = self.read_u8(ptr.wrapping_add(1) & 0xFF) as u16;
        ((hi << 8) | lo, false)
    }

    fn addr_indirect_indexed_y(&mut self) -> (u16, bool) {
        let ptr = self.read_u8(self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        let lo = self.read_u8(ptr) as u16;
        let hi = self.read_u8((ptr.wrapping_add(1)) & 0xFF) as u16;
        let base = (hi << 8) | lo;
        let addr = base.wrapping_add(self.y as u16);
        let crossed = (base & 0xFF00) != (addr & 0xFF00);
        (addr, crossed)
    }

    // --- Interrupts ---

    pub fn nmi(&mut self) {
        self.stack_push_u16(self.pc);
        let status = self.status & !FLAG_BREAK | FLAG_UNUSED;
        self.stack_push(status);
        self.set_flag(FLAG_IRQ_DISABLE, true);
        self.pc = self.read_u16(0xFFFA);
        self.cycles += 7;
    }

    pub fn irq(&mut self) {
        if !self.get_flag(FLAG_IRQ_DISABLE) {
            self.stack_push_u16(self.pc);
            let status = self.status & !FLAG_BREAK | FLAG_UNUSED;
            self.stack_push(status);
            self.set_flag(FLAG_IRQ_DISABLE, true);
            self.pc = self.read_u16(0xFFFE);
            self.cycles += 7;
        }
    }

    // --- Main execution step ---
    // Returns the number of cycles this instruction took.

    pub fn step(&mut self) -> u8 {
        let opcode = self.read_u8(self.pc);
        self.pc = self.pc.wrapping_add(1);

        let cycles = match opcode {
            // BRK
            0x00 => {
                self.pc = self.pc.wrapping_add(1);
                self.stack_push_u16(self.pc);
                let status = self.status | FLAG_BREAK | FLAG_UNUSED;
                self.stack_push(status);
                self.set_flag(FLAG_IRQ_DISABLE, true);
                self.pc = self.read_u16(0xFFFE);
                7
            }

            // ORA
            0x01 => { let (a, _) = self.addr_indexed_indirect_x(); self.ora(a); 6 }
            0x05 => { let (a, _) = self.addr_zero_page();           self.ora(a); 3 }
            0x09 => { let (a, _) = self.addr_immediate();           self.ora(a); 2 }
            0x0D => { let (a, _) = self.addr_absolute();            self.ora(a); 4 }
            0x11 => { let (a, p) = self.addr_indirect_indexed_y();  self.ora(a); 5 + p as u8 }
            0x15 => { let (a, _) = self.addr_zero_page_x();         self.ora(a); 4 }
            0x19 => { let (a, p) = self.addr_absolute_y();          self.ora(a); 4 + p as u8 }
            0x1D => { let (a, p) = self.addr_absolute_x();          self.ora(a); 4 + p as u8 }

            // ASL
            0x06 => { let (a, _) = self.addr_zero_page();   self.asl_mem(a); 5 }
            0x0A => {                                        self.asl_acc();  2 }
            0x0E => { let (a, _) = self.addr_absolute();    self.asl_mem(a); 6 }
            0x16 => { let (a, _) = self.addr_zero_page_x(); self.asl_mem(a); 6 }
            0x1E => { let (a, _) = self.addr_absolute_x();  self.asl_mem(a); 7 }

            // PHP
            0x08 => { let s = self.status | FLAG_BREAK | FLAG_UNUSED; self.stack_push(s); 3 }

            // BPL
            0x10 => self.branch(!self.get_flag(FLAG_NEGATIVE)),

            // CLC
            0x18 => { self.set_flag(FLAG_CARRY, false); 2 }

            // JSR
            0x20 => {
                let addr = self.read_u16(self.pc);
                self.pc = self.pc.wrapping_add(2);
                self.stack_push_u16(self.pc.wrapping_sub(1));
                self.pc = addr;
                6
            }

            // AND
            0x21 => { let (a, _) = self.addr_indexed_indirect_x(); self.and(a); 6 }
            0x25 => { let (a, _) = self.addr_zero_page();           self.and(a); 3 }
            0x29 => { let (a, _) = self.addr_immediate();           self.and(a); 2 }
            0x2D => { let (a, _) = self.addr_absolute();            self.and(a); 4 }
            0x31 => { let (a, p) = self.addr_indirect_indexed_y();  self.and(a); 5 + p as u8 }
            0x35 => { let (a, _) = self.addr_zero_page_x();         self.and(a); 4 }
            0x39 => { let (a, p) = self.addr_absolute_y();          self.and(a); 4 + p as u8 }
            0x3D => { let (a, p) = self.addr_absolute_x();          self.and(a); 4 + p as u8 }

            // BIT
            0x24 => { let (a, _) = self.addr_zero_page(); self.bit(a); 3 }
            0x2C => { let (a, _) = self.addr_absolute();  self.bit(a); 4 }

            // ROL
            0x26 => { let (a, _) = self.addr_zero_page();   self.rol_mem(a); 5 }
            0x2A => {                                        self.rol_acc();  2 }
            0x2E => { let (a, _) = self.addr_absolute();    self.rol_mem(a); 6 }
            0x36 => { let (a, _) = self.addr_zero_page_x(); self.rol_mem(a); 6 }
            0x3E => { let (a, _) = self.addr_absolute_x();  self.rol_mem(a); 7 }

            // PLP
            0x28 => {
                let s = self.stack_pop();
                self.status = (s & !FLAG_BREAK) | FLAG_UNUSED;
                4
            }

            // BMI
            0x30 => self.branch(self.get_flag(FLAG_NEGATIVE)),

            // SEC
            0x38 => { self.set_flag(FLAG_CARRY, true); 2 }

            // RTI
            0x40 => {
                let s = self.stack_pop();
                self.status = (s & !FLAG_BREAK) | FLAG_UNUSED;
                self.pc = self.stack_pop_u16();
                6
            }

            // EOR
            0x41 => { let (a, _) = self.addr_indexed_indirect_x(); self.eor(a); 6 }
            0x45 => { let (a, _) = self.addr_zero_page();           self.eor(a); 3 }
            0x49 => { let (a, _) = self.addr_immediate();           self.eor(a); 2 }
            0x4D => { let (a, _) = self.addr_absolute();            self.eor(a); 4 }
            0x51 => { let (a, p) = self.addr_indirect_indexed_y();  self.eor(a); 5 + p as u8 }
            0x55 => { let (a, _) = self.addr_zero_page_x();         self.eor(a); 4 }
            0x59 => { let (a, p) = self.addr_absolute_y();          self.eor(a); 4 + p as u8 }
            0x5D => { let (a, p) = self.addr_absolute_x();          self.eor(a); 4 + p as u8 }

            // LSR
            0x46 => { let (a, _) = self.addr_zero_page();   self.lsr_mem(a); 5 }
            0x4A => {                                        self.lsr_acc();  2 }
            0x4E => { let (a, _) = self.addr_absolute();    self.lsr_mem(a); 6 }
            0x56 => { let (a, _) = self.addr_zero_page_x(); self.lsr_mem(a); 6 }
            0x5E => { let (a, _) = self.addr_absolute_x();  self.lsr_mem(a); 7 }

            // PHA
            0x48 => { let a = self.a; self.stack_push(a); 3 }

            // JMP absolute
            0x4C => { let (a, _) = self.addr_absolute(); self.pc = a; 3 }

            // BVC
            0x50 => self.branch(!self.get_flag(FLAG_OVERFLOW)),

            // CLI
            0x58 => { self.set_flag(FLAG_IRQ_DISABLE, false); 2 }

            // RTS
            0x60 => {
                self.pc = self.stack_pop_u16().wrapping_add(1);
                6
            }

            // ADC
            0x61 => { let (a, _) = self.addr_indexed_indirect_x(); self.adc(a); 6 }
            0x65 => { let (a, _) = self.addr_zero_page();           self.adc(a); 3 }
            0x69 => { let (a, _) = self.addr_immediate();           self.adc(a); 2 }
            0x6D => { let (a, _) = self.addr_absolute();            self.adc(a); 4 }
            0x71 => { let (a, p) = self.addr_indirect_indexed_y();  self.adc(a); 5 + p as u8 }
            0x75 => { let (a, _) = self.addr_zero_page_x();         self.adc(a); 4 }
            0x79 => { let (a, p) = self.addr_absolute_y();          self.adc(a); 4 + p as u8 }
            0x7D => { let (a, p) = self.addr_absolute_x();          self.adc(a); 4 + p as u8 }

            // ROR
            0x66 => { let (a, _) = self.addr_zero_page();   self.ror_mem(a); 5 }
            0x6A => {                                        self.ror_acc();  2 }
            0x6E => { let (a, _) = self.addr_absolute();    self.ror_mem(a); 6 }
            0x76 => { let (a, _) = self.addr_zero_page_x(); self.ror_mem(a); 6 }
            0x7E => { let (a, _) = self.addr_absolute_x();  self.ror_mem(a); 7 }

            // PLA
            0x68 => {
                self.a = self.stack_pop();
                let a = self.a;
                self.update_nz(a);
                4
            }

            // JMP indirect
            0x6C => { let (a, _) = self.addr_indirect(); self.pc = a; 5 }

            // BVS
            0x70 => self.branch(self.get_flag(FLAG_OVERFLOW)),

            // SEI
            0x78 => { self.set_flag(FLAG_IRQ_DISABLE, true); 2 }

            // STA
            0x81 => { let (a, _) = self.addr_indexed_indirect_x(); let v = self.a; self.write_u8(a, v); 6 }
            0x85 => { let (a, _) = self.addr_zero_page();           let v = self.a; self.write_u8(a, v); 3 }
            0x8D => { let (a, _) = self.addr_absolute();            let v = self.a; self.write_u8(a, v); 4 }
            0x91 => { let (a, _) = self.addr_indirect_indexed_y();  let v = self.a; self.write_u8(a, v); 6 }
            0x95 => { let (a, _) = self.addr_zero_page_x();         let v = self.a; self.write_u8(a, v); 4 }
            0x99 => { let (a, _) = self.addr_absolute_y();          let v = self.a; self.write_u8(a, v); 5 }
            0x9D => { let (a, _) = self.addr_absolute_x();          let v = self.a; self.write_u8(a, v); 5 }

            // STX
            0x86 => { let (a, _) = self.addr_zero_page();   let v = self.x; self.write_u8(a, v); 3 }
            0x8E => { let (a, _) = self.addr_absolute();    let v = self.x; self.write_u8(a, v); 4 }
            0x96 => { let (a, _) = self.addr_zero_page_y(); let v = self.x; self.write_u8(a, v); 4 }

            // STY
            0x84 => { let (a, _) = self.addr_zero_page();   let v = self.y; self.write_u8(a, v); 3 }
            0x8C => { let (a, _) = self.addr_absolute();    let v = self.y; self.write_u8(a, v); 4 }
            0x94 => { let (a, _) = self.addr_zero_page_x(); let v = self.y; self.write_u8(a, v); 4 }

            // DEY
            0x88 => {
                self.y = self.y.wrapping_sub(1);
                let y = self.y;
                self.update_nz(y);
                2
            }

            // TXA
            0x8A => { self.a = self.x; let a = self.a; self.update_nz(a); 2 }

            // TYA
            0x98 => { self.a = self.y; let a = self.a; self.update_nz(a); 2 }

            // TXS
            0x9A => { self.sp = self.x; 2 }

            // BCC
            0x90 => self.branch(!self.get_flag(FLAG_CARRY)),

            // LDY
            0xA0 => { let (a, _) = self.addr_immediate();           self.ldy(a); 2 }
            0xA4 => { let (a, _) = self.addr_zero_page();           self.ldy(a); 3 }
            0xAC => { let (a, _) = self.addr_absolute();            self.ldy(a); 4 }
            0xB4 => { let (a, _) = self.addr_zero_page_x();         self.ldy(a); 4 }
            0xBC => { let (a, p) = self.addr_absolute_x();          self.ldy(a); 4 + p as u8 }

            // LDA
            0xA1 => { let (a, _) = self.addr_indexed_indirect_x(); self.lda(a); 6 }
            0xA5 => { let (a, _) = self.addr_zero_page();           self.lda(a); 3 }
            0xA9 => { let (a, _) = self.addr_immediate();           self.lda(a); 2 }
            0xAD => { let (a, _) = self.addr_absolute();            self.lda(a); 4 }
            0xB1 => { let (a, p) = self.addr_indirect_indexed_y();  self.lda(a); 5 + p as u8 }
            0xB5 => { let (a, _) = self.addr_zero_page_x();         self.lda(a); 4 }
            0xB9 => { let (a, p) = self.addr_absolute_y();          self.lda(a); 4 + p as u8 }
            0xBD => { let (a, p) = self.addr_absolute_x();          self.lda(a); 4 + p as u8 }

            // LDX
            0xA2 => { let (a, _) = self.addr_immediate();           self.ldx(a); 2 }
            0xA6 => { let (a, _) = self.addr_zero_page();           self.ldx(a); 3 }
            0xAE => { let (a, _) = self.addr_absolute();            self.ldx(a); 4 }
            0xB6 => { let (a, _) = self.addr_zero_page_y();         self.ldx(a); 4 }
            0xBE => { let (a, p) = self.addr_absolute_y();          self.ldx(a); 4 + p as u8 }

            // TAY
            0xA8 => { self.y = self.a; let y = self.y; self.update_nz(y); 2 }

            // TAX
            0xAA => { self.x = self.a; let x = self.x; self.update_nz(x); 2 }

            // TSX
            0xBA => { self.x = self.sp; let x = self.x; self.update_nz(x); 2 }

            // BCS
            0xB0 => self.branch(self.get_flag(FLAG_CARRY)),

            // CLV
            0xB8 => { self.set_flag(FLAG_OVERFLOW, false); 2 }

            // CPY
            0xC0 => { let (a, _) = self.addr_immediate(); self.cpy(a); 2 }
            0xC4 => { let (a, _) = self.addr_zero_page(); self.cpy(a); 3 }
            0xCC => { let (a, _) = self.addr_absolute();  self.cpy(a); 4 }

            // CMP
            0xC1 => { let (a, _) = self.addr_indexed_indirect_x(); self.cmp(a); 6 }
            0xC5 => { let (a, _) = self.addr_zero_page();           self.cmp(a); 3 }
            0xC9 => { let (a, _) = self.addr_immediate();           self.cmp(a); 2 }
            0xCD => { let (a, _) = self.addr_absolute();            self.cmp(a); 4 }
            0xD1 => { let (a, p) = self.addr_indirect_indexed_y();  self.cmp(a); 5 + p as u8 }
            0xD5 => { let (a, _) = self.addr_zero_page_x();         self.cmp(a); 4 }
            0xD9 => { let (a, p) = self.addr_absolute_y();          self.cmp(a); 4 + p as u8 }
            0xDD => { let (a, p) = self.addr_absolute_x();          self.cmp(a); 4 + p as u8 }

            // DEC
            0xC6 => { let (a, _) = self.addr_zero_page();   self.dec(a); 5 }
            0xCE => { let (a, _) = self.addr_absolute();    self.dec(a); 6 }
            0xD6 => { let (a, _) = self.addr_zero_page_x(); self.dec(a); 6 }
            0xDE => { let (a, _) = self.addr_absolute_x();  self.dec(a); 7 }

            // INY
            0xC8 => {
                self.y = self.y.wrapping_add(1);
                let y = self.y;
                self.update_nz(y);
                2
            }

            // DEX
            0xCA => {
                self.x = self.x.wrapping_sub(1);
                let x = self.x;
                self.update_nz(x);
                2
            }

            // BNE
            0xD0 => self.branch(!self.get_flag(FLAG_ZERO)),

            // CLD
            0xD8 => { self.set_flag(FLAG_DECIMAL, false); 2 }

            // CPX
            0xE0 => { let (a, _) = self.addr_immediate(); self.cpx(a); 2 }
            0xE4 => { let (a, _) = self.addr_zero_page(); self.cpx(a); 3 }
            0xEC => { let (a, _) = self.addr_absolute();  self.cpx(a); 4 }

            // SBC
            0xE1 => { let (a, _) = self.addr_indexed_indirect_x(); self.sbc(a); 6 }
            0xE5 => { let (a, _) = self.addr_zero_page();           self.sbc(a); 3 }
            0xE9 | 0xEB => { let (a, _) = self.addr_immediate();    self.sbc(a); 2 } // 0xEB is unofficial SBC
            0xED => { let (a, _) = self.addr_absolute();            self.sbc(a); 4 }
            0xF1 => { let (a, p) = self.addr_indirect_indexed_y();  self.sbc(a); 5 + p as u8 }
            0xF5 => { let (a, _) = self.addr_zero_page_x();         self.sbc(a); 4 }
            0xF9 => { let (a, p) = self.addr_absolute_y();          self.sbc(a); 4 + p as u8 }
            0xFD => { let (a, p) = self.addr_absolute_x();          self.sbc(a); 4 + p as u8 }

            // INC
            0xE6 => { let (a, _) = self.addr_zero_page();   self.inc(a); 5 }
            0xEE => { let (a, _) = self.addr_absolute();    self.inc(a); 6 }
            0xF6 => { let (a, _) = self.addr_zero_page_x(); self.inc(a); 6 }
            0xFE => { let (a, _) = self.addr_absolute_x();  self.inc(a); 7 }

            // INX
            0xE8 => {
                self.x = self.x.wrapping_add(1);
                let x = self.x;
                self.update_nz(x);
                2
            }

            // NOP (official)
            0xEA => 2,

            // BEQ
            0xF0 => self.branch(self.get_flag(FLAG_ZERO)),

            // SED
            0xF8 => { self.set_flag(FLAG_DECIMAL, true); 2 }

            // --- Unofficial opcodes ---

            // NOP variants (single byte)
            0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => 2,

            // NOP variants (immediate, 2 bytes)
            0x80 | 0x82 | 0x89 | 0xC2 | 0xE2 => { self.pc = self.pc.wrapping_add(1); 2 }

            // NOP variants (zero page, 2 bytes)
            0x04 | 0x44 | 0x64 => { self.pc = self.pc.wrapping_add(1); 3 }

            // NOP variants (zero page,X, 2 bytes)
            0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 => { self.pc = self.pc.wrapping_add(1); 4 }

            // NOP variants (absolute, 3 bytes)
            0x0C => { self.pc = self.pc.wrapping_add(2); 4 }

            // NOP variants (absolute,X, 3 bytes)
            0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => {
                let base = self.read_u16(self.pc);
                self.pc = self.pc.wrapping_add(2);
                let addr = base.wrapping_add(self.x as u16);
                let crossed = (base & 0xFF00) != (addr & 0xFF00);
                4 + crossed as u8
            }

            // LAX (LDA then TAX) — indexed indirect X
            0xA3 => { let (a, _) = self.addr_indexed_indirect_x(); self.lax(a); 6 }
            // LAX — zero page
            0xA7 => { let (a, _) = self.addr_zero_page();           self.lax(a); 3 }
            // LAX — absolute
            0xAF => { let (a, _) = self.addr_absolute();            self.lax(a); 4 }
            // LAX — indirect indexed Y
            0xB3 => { let (a, p) = self.addr_indirect_indexed_y();  self.lax(a); 5 + p as u8 }
            // LAX — zero page Y
            0xB7 => { let (a, _) = self.addr_zero_page_y();         self.lax(a); 4 }
            // LAX — absolute Y
            0xBF => { let (a, p) = self.addr_absolute_y();          self.lax(a); 4 + p as u8 }

            // SAX (store A & X) — indexed indirect X
            0x83 => { let (a, _) = self.addr_indexed_indirect_x(); self.sax(a); 6 }
            // SAX — zero page
            0x87 => { let (a, _) = self.addr_zero_page();           self.sax(a); 3 }
            // SAX — absolute
            0x8F => { let (a, _) = self.addr_absolute();            self.sax(a); 4 }
            // SAX — zero page Y
            0x97 => { let (a, _) = self.addr_zero_page_y();         self.sax(a); 4 }

            // DCP (DEC then CMP) — all modes
            0xC3 => { let (a, _) = self.addr_indexed_indirect_x(); self.dcp(a); 8 }
            0xC7 => { let (a, _) = self.addr_zero_page();           self.dcp(a); 5 }
            0xCF => { let (a, _) = self.addr_absolute();            self.dcp(a); 6 }
            0xD3 => { let (a, _) = self.addr_indirect_indexed_y();  self.dcp(a); 8 }
            0xD7 => { let (a, _) = self.addr_zero_page_x();         self.dcp(a); 6 }
            0xDB => { let (a, _) = self.addr_absolute_y();          self.dcp(a); 7 }
            0xDF => { let (a, _) = self.addr_absolute_x();          self.dcp(a); 7 }

            // ISB/ISC (INC then SBC) — all modes
            0xE3 => { let (a, _) = self.addr_indexed_indirect_x(); self.isb(a); 8 }
            0xE7 => { let (a, _) = self.addr_zero_page();           self.isb(a); 5 }
            0xEF => { let (a, _) = self.addr_absolute();            self.isb(a); 6 }
            0xF3 => { let (a, _) = self.addr_indirect_indexed_y();  self.isb(a); 8 }
            0xF7 => { let (a, _) = self.addr_zero_page_x();         self.isb(a); 6 }
            0xFB => { let (a, _) = self.addr_absolute_y();          self.isb(a); 7 }
            0xFF => { let (a, _) = self.addr_absolute_x();          self.isb(a); 7 }

            // SLO (ASL then ORA) — all modes
            0x03 => { let (a, _) = self.addr_indexed_indirect_x(); self.slo(a); 8 }
            0x07 => { let (a, _) = self.addr_zero_page();           self.slo(a); 5 }
            0x0F => { let (a, _) = self.addr_absolute();            self.slo(a); 6 }
            0x13 => { let (a, _) = self.addr_indirect_indexed_y();  self.slo(a); 8 }
            0x17 => { let (a, _) = self.addr_zero_page_x();         self.slo(a); 6 }
            0x1B => { let (a, _) = self.addr_absolute_y();          self.slo(a); 7 }
            0x1F => { let (a, _) = self.addr_absolute_x();          self.slo(a); 7 }

            // RLA (ROL then AND) — all modes
            0x23 => { let (a, _) = self.addr_indexed_indirect_x(); self.rla(a); 8 }
            0x27 => { let (a, _) = self.addr_zero_page();           self.rla(a); 5 }
            0x2F => { let (a, _) = self.addr_absolute();            self.rla(a); 6 }
            0x33 => { let (a, _) = self.addr_indirect_indexed_y();  self.rla(a); 8 }
            0x37 => { let (a, _) = self.addr_zero_page_x();         self.rla(a); 6 }
            0x3B => { let (a, _) = self.addr_absolute_y();          self.rla(a); 7 }
            0x3F => { let (a, _) = self.addr_absolute_x();          self.rla(a); 7 }

            // SRE (LSR then EOR) — all modes
            0x43 => { let (a, _) = self.addr_indexed_indirect_x(); self.sre(a); 8 }
            0x47 => { let (a, _) = self.addr_zero_page();           self.sre(a); 5 }
            0x4F => { let (a, _) = self.addr_absolute();            self.sre(a); 6 }
            0x53 => { let (a, _) = self.addr_indirect_indexed_y();  self.sre(a); 8 }
            0x57 => { let (a, _) = self.addr_zero_page_x();         self.sre(a); 6 }
            0x5B => { let (a, _) = self.addr_absolute_y();          self.sre(a); 7 }
            0x5F => { let (a, _) = self.addr_absolute_x();          self.sre(a); 7 }

            // RRA (ROR then ADC) — all modes
            0x63 => { let (a, _) = self.addr_indexed_indirect_x(); self.rra(a); 8 }
            0x67 => { let (a, _) = self.addr_zero_page();           self.rra(a); 5 }
            0x6F => { let (a, _) = self.addr_absolute();            self.rra(a); 6 }
            0x73 => { let (a, _) = self.addr_indirect_indexed_y();  self.rra(a); 8 }
            0x77 => { let (a, _) = self.addr_zero_page_x();         self.rra(a); 6 }
            0x7B => { let (a, _) = self.addr_absolute_y();          self.rra(a); 7 }
            0x7F => { let (a, _) = self.addr_absolute_x();          self.rra(a); 7 }

            // Catch-all for truly unhandled opcodes
            _ => {
                // Treat unknown opcodes as 1-cycle NOPs to avoid panicking
                1
            }
        };

        self.cycles += cycles as u64;
        cycles
    }

    // --- Branch helper ---
    // Returns the number of cycles (2 base + 1 if taken + 1 if page crossed)
    fn branch(&mut self, condition: bool) -> u8 {
        let offset = self.read_u8(self.pc) as i8;
        self.pc = self.pc.wrapping_add(1);
        if condition {
            let new_pc = self.pc.wrapping_add(offset as u16);
            let crossed = (self.pc & 0xFF00) != (new_pc & 0xFF00);
            self.pc = new_pc;
            3 + crossed as u8
        } else {
            2
        }
    }

    // --- Instructions ---

    fn lda(&mut self, addr: u16) {
        self.a = self.read_u8(addr);
        let a = self.a;
        self.update_nz(a);
    }

    fn ldx(&mut self, addr: u16) {
        self.x = self.read_u8(addr);
        let x = self.x;
        self.update_nz(x);
    }

    fn ldy(&mut self, addr: u16) {
        self.y = self.read_u8(addr);
        let y = self.y;
        self.update_nz(y);
    }

    fn and(&mut self, addr: u16) {
        self.a &= self.read_u8(addr);
        let a = self.a;
        self.update_nz(a);
    }

    fn ora(&mut self, addr: u16) {
        self.a |= self.read_u8(addr);
        let a = self.a;
        self.update_nz(a);
    }

    fn eor(&mut self, addr: u16) {
        self.a ^= self.read_u8(addr);
        let a = self.a;
        self.update_nz(a);
    }

    fn adc(&mut self, addr: u16) {
        let val = self.read_u8(addr) as u16;
        let result = self.a as u16 + val + self.get_flag(FLAG_CARRY) as u16;
        self.set_flag(FLAG_CARRY, result > 0xFF);
        self.set_flag(FLAG_OVERFLOW, (!(self.a as u16 ^ val) & (self.a as u16 ^ result) & 0x80) != 0);
        self.a = result as u8;
        let a = self.a;
        self.update_nz(a);
    }

    fn sbc(&mut self, addr: u16) {
        let val = self.read_u8(addr) as u16 ^ 0xFF;
        let result = self.a as u16 + val + self.get_flag(FLAG_CARRY) as u16;
        self.set_flag(FLAG_CARRY, result > 0xFF);
        self.set_flag(FLAG_OVERFLOW, (!(self.a as u16 ^ val) & (self.a as u16 ^ result) & 0x80) != 0);
        self.a = result as u8;
        let a = self.a;
        self.update_nz(a);
    }

    fn cmp(&mut self, addr: u16) {
        let val = self.read_u8(addr);
        let result = self.a.wrapping_sub(val);
        self.set_flag(FLAG_CARRY, self.a >= val);
        self.update_nz(result);
    }

    fn cpx(&mut self, addr: u16) {
        let val = self.read_u8(addr);
        let result = self.x.wrapping_sub(val);
        self.set_flag(FLAG_CARRY, self.x >= val);
        self.update_nz(result);
    }

    fn cpy(&mut self, addr: u16) {
        let val = self.read_u8(addr);
        let result = self.y.wrapping_sub(val);
        self.set_flag(FLAG_CARRY, self.y >= val);
        self.update_nz(result);
    }

    fn bit(&mut self, addr: u16) {
        let val = self.read_u8(addr);
        self.set_flag(FLAG_ZERO, (self.a & val) == 0);
        self.set_flag(FLAG_OVERFLOW, val & 0x40 != 0);
        self.set_flag(FLAG_NEGATIVE, val & 0x80 != 0);
    }

    fn inc(&mut self, addr: u16) {
        let val = self.read_u8(addr).wrapping_add(1);
        self.write_u8(addr, val);
        self.update_nz(val);
    }

    fn dec(&mut self, addr: u16) {
        let val = self.read_u8(addr).wrapping_sub(1);
        self.write_u8(addr, val);
        self.update_nz(val);
    }

    fn asl_acc(&mut self) {
        self.set_flag(FLAG_CARRY, self.a & 0x80 != 0);
        self.a <<= 1;
        let a = self.a;
        self.update_nz(a);
    }

    fn asl_mem(&mut self, addr: u16) {
        let val = self.read_u8(addr);
        self.set_flag(FLAG_CARRY, val & 0x80 != 0);
        let result = val << 1;
        self.write_u8(addr, result);
        self.update_nz(result);
    }

    fn lsr_acc(&mut self) {
        self.set_flag(FLAG_CARRY, self.a & 0x01 != 0);
        self.a >>= 1;
        let a = self.a;
        self.update_nz(a);
    }

    fn lsr_mem(&mut self, addr: u16) {
        let val = self.read_u8(addr);
        self.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let result = val >> 1;
        self.write_u8(addr, result);
        self.update_nz(result);
    }

    fn rol_acc(&mut self) {
        let old_carry = self.get_flag(FLAG_CARRY) as u8;
        self.set_flag(FLAG_CARRY, self.a & 0x80 != 0);
        self.a = (self.a << 1) | old_carry;
        let a = self.a;
        self.update_nz(a);
    }

    fn rol_mem(&mut self, addr: u16) {
        let val = self.read_u8(addr);
        let old_carry = self.get_flag(FLAG_CARRY) as u8;
        self.set_flag(FLAG_CARRY, val & 0x80 != 0);
        let result = (val << 1) | old_carry;
        self.write_u8(addr, result);
        self.update_nz(result);
    }

    fn ror_acc(&mut self) {
        let old_carry = self.get_flag(FLAG_CARRY) as u8;
        self.set_flag(FLAG_CARRY, self.a & 0x01 != 0);
        self.a = (self.a >> 1) | (old_carry << 7);
        let a = self.a;
        self.update_nz(a);
    }

    fn ror_mem(&mut self, addr: u16) {
        let val = self.read_u8(addr);
        let old_carry = self.get_flag(FLAG_CARRY) as u8;
        self.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let result = (val >> 1) | (old_carry << 7);
        self.write_u8(addr, result);
        self.update_nz(result);
    }

    // --- Unofficial instructions ---

    fn lax(&mut self, addr: u16) {
        let val = self.read_u8(addr);
        self.a = val;
        self.x = val;
        self.update_nz(val);
    }

    fn sax(&mut self, addr: u16) {
        let val = self.a & self.x;
        self.write_u8(addr, val);
    }

    fn dcp(&mut self, addr: u16) {
        // DEC then CMP
        let val = self.read_u8(addr).wrapping_sub(1);
        self.write_u8(addr, val);
        let result = self.a.wrapping_sub(val);
        self.set_flag(FLAG_CARRY, self.a >= val);
        self.update_nz(result);
    }

    fn isb(&mut self, addr: u16) {
        // INC then SBC
        let val = self.read_u8(addr).wrapping_add(1);
        self.write_u8(addr, val);
        let val16 = val as u16 ^ 0xFF;
        let result = self.a as u16 + val16 + self.get_flag(FLAG_CARRY) as u16;
        self.set_flag(FLAG_CARRY, result > 0xFF);
        self.set_flag(FLAG_OVERFLOW, (!(self.a as u16 ^ val16) & (self.a as u16 ^ result) & 0x80) != 0);
        self.a = result as u8;
        let a = self.a;
        self.update_nz(a);
    }

    fn slo(&mut self, addr: u16) {
        // ASL then ORA
        let val = self.read_u8(addr);
        self.set_flag(FLAG_CARRY, val & 0x80 != 0);
        let shifted = val << 1;
        self.write_u8(addr, shifted);
        self.a |= shifted;
        let a = self.a;
        self.update_nz(a);
    }

    fn rla(&mut self, addr: u16) {
        // ROL then AND
        let val = self.read_u8(addr);
        let old_carry = self.get_flag(FLAG_CARRY) as u8;
        self.set_flag(FLAG_CARRY, val & 0x80 != 0);
        let rolled = (val << 1) | old_carry;
        self.write_u8(addr, rolled);
        self.a &= rolled;
        let a = self.a;
        self.update_nz(a);
    }

    fn sre(&mut self, addr: u16) {
        // LSR then EOR
        let val = self.read_u8(addr);
        self.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let shifted = val >> 1;
        self.write_u8(addr, shifted);
        self.a ^= shifted;
        let a = self.a;
        self.update_nz(a);
    }

    fn rra(&mut self, addr: u16) {
        // ROR then ADC
        let val = self.read_u8(addr);
        let old_carry = self.get_flag(FLAG_CARRY) as u8;
        self.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let rolled = (val >> 1) | (old_carry << 7);
        self.write_u8(addr, rolled);
        let v = rolled as u16;
        let result = self.a as u16 + v + self.get_flag(FLAG_CARRY) as u16;
        self.set_flag(FLAG_CARRY, result > 0xFF);
        self.set_flag(FLAG_OVERFLOW, (!(self.a as u16 ^ v) & (self.a as u16 ^ result) & 0x80) != 0);
        self.a = result as u8;
        let a = self.a;
        self.update_nz(a);
    }
}

// --- Disassembler (used for nestest logging) ---

pub fn disassemble(cpu: &Cpu, addr: u16) -> String {
    let opcode = cpu.read_u8(addr);
    let b1 = cpu.read_u8(addr.wrapping_add(1));
    let b2 = cpu.read_u8(addr.wrapping_add(2));

    match opcode {
        0x00 => format!("BRK"),
        0x01 => format!("ORA (${:02X},X)", b1),
        0x03 => format!("*SLO (${:02X},X)", b1),
        0x04 => format!("*NOP ${:02X}", b1),
        0x05 => format!("ORA ${:02X}", b1),
        0x06 => format!("ASL ${:02X}", b1),
        0x07 => format!("*SLO ${:02X}", b1),
        0x08 => format!("PHP"),
        0x09 => format!("ORA #${:02X}", b1),
        0x0A => format!("ASL A"),
        0x0C => format!("*NOP ${:02X}{:02X}", b2, b1),
        0x0D => format!("ORA ${:02X}{:02X}", b2, b1),
        0x0E => format!("ASL ${:02X}{:02X}", b2, b1),
        0x0F => format!("*SLO ${:02X}{:02X}", b2, b1),
        0x10 => format!("BPL ${:04X}", (addr.wrapping_add(2) as i16 + b1 as i8 as i16) as u16),
        0x11 => format!("ORA (${:02X}),Y", b1),
        0x13 => format!("*SLO (${:02X}),Y", b1),
        0x14 => format!("*NOP ${:02X},X", b1),
        0x15 => format!("ORA ${:02X},X", b1),
        0x16 => format!("ASL ${:02X},X", b1),
        0x17 => format!("*SLO ${:02X},X", b1),
        0x18 => format!("CLC"),
        0x19 => format!("ORA ${:02X}{:02X},Y", b2, b1),
        0x1A => format!("*NOP"),
        0x1B => format!("*SLO ${:02X}{:02X},Y", b2, b1),
        0x1C => format!("*NOP ${:02X}{:02X},X", b2, b1),
        0x1D => format!("ORA ${:02X}{:02X},X", b2, b1),
        0x1E => format!("ASL ${:02X}{:02X},X", b2, b1),
        0x1F => format!("*SLO ${:02X}{:02X},X", b2, b1),
        0x20 => format!("JSR ${:02X}{:02X}", b2, b1),
        0x21 => format!("AND (${:02X},X)", b1),
        0x23 => format!("*RLA (${:02X},X)", b1),
        0x24 => format!("BIT ${:02X}", b1),
        0x25 => format!("AND ${:02X}", b1),
        0x26 => format!("ROL ${:02X}", b1),
        0x27 => format!("*RLA ${:02X}", b1),
        0x28 => format!("PLP"),
        0x29 => format!("AND #${:02X}", b1),
        0x2A => format!("ROL A"),
        0x2C => format!("BIT ${:02X}{:02X}", b2, b1),
        0x2D => format!("AND ${:02X}{:02X}", b2, b1),
        0x2E => format!("ROL ${:02X}{:02X}", b2, b1),
        0x2F => format!("*RLA ${:02X}{:02X}", b2, b1),
        0x30 => format!("BMI ${:04X}", (addr.wrapping_add(2) as i16 + b1 as i8 as i16) as u16),
        0x31 => format!("AND (${:02X}),Y", b1),
        0x33 => format!("*RLA (${:02X}),Y", b1),
        0x34 => format!("*NOP ${:02X},X", b1),
        0x35 => format!("AND ${:02X},X", b1),
        0x36 => format!("ROL ${:02X},X", b1),
        0x37 => format!("*RLA ${:02X},X", b1),
        0x38 => format!("SEC"),
        0x39 => format!("AND ${:02X}{:02X},Y", b2, b1),
        0x3A => format!("*NOP"),
        0x3B => format!("*RLA ${:02X}{:02X},Y", b2, b1),
        0x3C => format!("*NOP ${:02X}{:02X},X", b2, b1),
        0x3D => format!("AND ${:02X}{:02X},X", b2, b1),
        0x3E => format!("ROL ${:02X}{:02X},X", b2, b1),
        0x3F => format!("*RLA ${:02X}{:02X},X", b2, b1),
        0x40 => format!("RTI"),
        0x41 => format!("EOR (${:02X},X)", b1),
        0x43 => format!("*SRE (${:02X},X)", b1),
        0x44 => format!("*NOP ${:02X}", b1),
        0x45 => format!("EOR ${:02X}", b1),
        0x46 => format!("LSR ${:02X}", b1),
        0x47 => format!("*SRE ${:02X}", b1),
        0x48 => format!("PHA"),
        0x49 => format!("EOR #${:02X}", b1),
        0x4A => format!("LSR A"),
        0x4C => format!("JMP ${:02X}{:02X}", b2, b1),
        0x4D => format!("EOR ${:02X}{:02X}", b2, b1),
        0x4E => format!("LSR ${:02X}{:02X}", b2, b1),
        0x4F => format!("*SRE ${:02X}{:02X}", b2, b1),
        0x50 => format!("BVC ${:04X}", (addr.wrapping_add(2) as i16 + b1 as i8 as i16) as u16),
        0x51 => format!("EOR (${:02X}),Y", b1),
        0x53 => format!("*SRE (${:02X}),Y", b1),
        0x54 => format!("*NOP ${:02X},X", b1),
        0x55 => format!("EOR ${:02X},X", b1),
        0x56 => format!("LSR ${:02X},X", b1),
        0x57 => format!("*SRE ${:02X},X", b1),
        0x58 => format!("CLI"),
        0x59 => format!("EOR ${:02X}{:02X},Y", b2, b1),
        0x5A => format!("*NOP"),
        0x5B => format!("*SRE ${:02X}{:02X},Y", b2, b1),
        0x5C => format!("*NOP ${:02X}{:02X},X", b2, b1),
        0x5D => format!("EOR ${:02X}{:02X},X", b2, b1),
        0x5E => format!("LSR ${:02X}{:02X},X", b2, b1),
        0x5F => format!("*SRE ${:02X}{:02X},X", b2, b1),
        0x60 => format!("RTS"),
        0x61 => format!("ADC (${:02X},X)", b1),
        0x63 => format!("*RRA (${:02X},X)", b1),
        0x64 => format!("*NOP ${:02X}", b1),
        0x65 => format!("ADC ${:02X}", b1),
        0x66 => format!("ROR ${:02X}", b1),
        0x67 => format!("*RRA ${:02X}", b1),
        0x68 => format!("PLA"),
        0x69 => format!("ADC #${:02X}", b1),
        0x6A => format!("ROR A"),
        0x6C => format!("JMP (${:02X}{:02X})", b2, b1),
        0x6D => format!("ADC ${:02X}{:02X}", b2, b1),
        0x6E => format!("ROR ${:02X}{:02X}", b2, b1),
        0x6F => format!("*RRA ${:02X}{:02X}", b2, b1),
        0x70 => format!("BVS ${:04X}", (addr.wrapping_add(2) as i16 + b1 as i8 as i16) as u16),
        0x71 => format!("ADC (${:02X}),Y", b1),
        0x73 => format!("*RRA (${:02X}),Y", b1),
        0x74 => format!("*NOP ${:02X},X", b1),
        0x75 => format!("ADC ${:02X},X", b1),
        0x76 => format!("ROR ${:02X},X", b1),
        0x77 => format!("*RRA ${:02X},X", b1),
        0x78 => format!("SEI"),
        0x79 => format!("ADC ${:02X}{:02X},Y", b2, b1),
        0x7A => format!("*NOP"),
        0x7B => format!("*RRA ${:02X}{:02X},Y", b2, b1),
        0x7C => format!("*NOP ${:02X}{:02X},X", b2, b1),
        0x7D => format!("ADC ${:02X}{:02X},X", b2, b1),
        0x7E => format!("ROR ${:02X}{:02X},X", b2, b1),
        0x7F => format!("*RRA ${:02X}{:02X},X", b2, b1),
        0x80 => format!("*NOP #${:02X}", b1),
        0x81 => format!("STA (${:02X},X)", b1),
        0x82 => format!("*NOP #${:02X}", b1),
        0x83 => format!("*SAX (${:02X},X)", b1),
        0x84 => format!("STY ${:02X}", b1),
        0x85 => format!("STA ${:02X}", b1),
        0x86 => format!("STX ${:02X}", b1),
        0x87 => format!("*SAX ${:02X}", b1),
        0x88 => format!("DEY"),
        0x89 => format!("*NOP #${:02X}", b1),
        0x8A => format!("TXA"),
        0x8C => format!("STY ${:02X}{:02X}", b2, b1),
        0x8D => format!("STA ${:02X}{:02X}", b2, b1),
        0x8E => format!("STX ${:02X}{:02X}", b2, b1),
        0x8F => format!("*SAX ${:02X}{:02X}", b2, b1),
        0x90 => format!("BCC ${:04X}", (addr.wrapping_add(2) as i16 + b1 as i8 as i16) as u16),
        0x91 => format!("STA (${:02X}),Y", b1),
        0x94 => format!("STY ${:02X},X", b1),
        0x95 => format!("STA ${:02X},X", b1),
        0x96 => format!("STX ${:02X},Y", b1),
        0x97 => format!("*SAX ${:02X},Y", b1),
        0x98 => format!("TYA"),
        0x99 => format!("STA ${:02X}{:02X},Y", b2, b1),
        0x9A => format!("TXS"),
        0x9D => format!("STA ${:02X}{:02X},X", b2, b1),
        0xA0 => format!("LDY #${:02X}", b1),
        0xA1 => format!("LDA (${:02X},X)", b1),
        0xA2 => format!("LDX #${:02X}", b1),
        0xA3 => format!("*LAX (${:02X},X)", b1),
        0xA4 => format!("LDY ${:02X}", b1),
        0xA5 => format!("LDA ${:02X}", b1),
        0xA6 => format!("LDX ${:02X}", b1),
        0xA7 => format!("*LAX ${:02X}", b1),
        0xA8 => format!("TAY"),
        0xA9 => format!("LDA #${:02X}", b1),
        0xAA => format!("TAX"),
        0xAC => format!("LDY ${:02X}{:02X}", b2, b1),
        0xAD => format!("LDA ${:02X}{:02X}", b2, b1),
        0xAE => format!("LDX ${:02X}{:02X}", b2, b1),
        0xAF => format!("*LAX ${:02X}{:02X}", b2, b1),
        0xB0 => format!("BCS ${:04X}", (addr.wrapping_add(2) as i16 + b1 as i8 as i16) as u16),
        0xB1 => format!("LDA (${:02X}),Y", b1),
        0xB3 => format!("*LAX (${:02X}),Y", b1),
        0xB4 => format!("LDY ${:02X},X", b1),
        0xB5 => format!("LDA ${:02X},X", b1),
        0xB6 => format!("LDX ${:02X},Y", b1),
        0xB7 => format!("*LAX ${:02X},Y", b1),
        0xB8 => format!("CLV"),
        0xB9 => format!("LDA ${:02X}{:02X},Y", b2, b1),
        0xBA => format!("TSX"),
        0xBC => format!("LDY ${:02X}{:02X},X", b2, b1),
        0xBD => format!("LDA ${:02X}{:02X},X", b2, b1),
        0xBE => format!("LDX ${:02X}{:02X},Y", b2, b1),
        0xBF => format!("*LAX ${:02X}{:02X},Y", b2, b1),
        0xC0 => format!("CPY #${:02X}", b1),
        0xC1 => format!("CMP (${:02X},X)", b1),
        0xC2 => format!("*NOP #${:02X}", b1),
        0xC3 => format!("*DCP (${:02X},X)", b1),
        0xC4 => format!("CPY ${:02X}", b1),
        0xC5 => format!("CMP ${:02X}", b1),
        0xC6 => format!("DEC ${:02X}", b1),
        0xC7 => format!("*DCP ${:02X}", b1),
        0xC8 => format!("INY"),
        0xC9 => format!("CMP #${:02X}", b1),
        0xCA => format!("DEX"),
        0xCC => format!("CPY ${:02X}{:02X}", b2, b1),
        0xCD => format!("CMP ${:02X}{:02X}", b2, b1),
        0xCE => format!("DEC ${:02X}{:02X}", b2, b1),
        0xCF => format!("*DCP ${:02X}{:02X}", b2, b1),
        0xD0 => format!("BNE ${:04X}", (addr.wrapping_add(2) as i16 + b1 as i8 as i16) as u16),
        0xD1 => format!("CMP (${:02X}),Y", b1),
        0xD3 => format!("*DCP (${:02X}),Y", b1),
        0xD4 => format!("*NOP ${:02X},X", b1),
        0xD5 => format!("CMP ${:02X},X", b1),
        0xD6 => format!("DEC ${:02X},X", b1),
        0xD7 => format!("*DCP ${:02X},X", b1),
        0xD8 => format!("CLD"),
        0xD9 => format!("CMP ${:02X}{:02X},Y", b2, b1),
        0xDA => format!("*NOP"),
        0xDB => format!("*DCP ${:02X}{:02X},Y", b2, b1),
        0xDC => format!("*NOP ${:02X}{:02X},X", b2, b1),
        0xDD => format!("CMP ${:02X}{:02X},X", b2, b1),
        0xDE => format!("DEC ${:02X}{:02X},X", b2, b1),
        0xDF => format!("*DCP ${:02X}{:02X},X", b2, b1),
        0xE0 => format!("CPX #${:02X}", b1),
        0xE1 => format!("SBC (${:02X},X)", b1),
        0xE2 => format!("*NOP #${:02X}", b1),
        0xE3 => format!("*ISB (${:02X},X)", b1),
        0xE4 => format!("CPX ${:02X}", b1),
        0xE5 => format!("SBC ${:02X}", b1),
        0xE6 => format!("INC ${:02X}", b1),
        0xE7 => format!("*ISB ${:02X}", b1),
        0xE8 => format!("INX"),
        0xE9 => format!("SBC #${:02X}", b1),
        0xEA => format!("NOP"),
        0xEB => format!("*SBC #${:02X}", b1),
        0xEC => format!("CPX ${:02X}{:02X}", b2, b1),
        0xED => format!("SBC ${:02X}{:02X}", b2, b1),
        0xEE => format!("INC ${:02X}{:02X}", b2, b1),
        0xEF => format!("*ISB ${:02X}{:02X}", b2, b1),
        0xF0 => format!("BEQ ${:04X}", (addr.wrapping_add(2) as i16 + b1 as i8 as i16) as u16),
        0xF1 => format!("SBC (${:02X}),Y", b1),
        0xF3 => format!("*ISB (${:02X}),Y", b1),
        0xF4 => format!("*NOP ${:02X},X", b1),
        0xF5 => format!("SBC ${:02X},X", b1),
        0xF6 => format!("INC ${:02X},X", b1),
        0xF7 => format!("*ISB ${:02X},X", b1),
        0xF8 => format!("SED"),
        0xF9 => format!("SBC ${:02X}{:02X},Y", b2, b1),
        0xFA => format!("*NOP"),
        0xFB => format!("*ISB ${:02X}{:02X},Y", b2, b1),
        0xFC => format!("*NOP ${:02X}{:02X},X", b2, b1),
        0xFD => format!("SBC ${:02X}{:02X},X", b2, b1),
        0xFE => format!("INC ${:02X}{:02X},X", b2, b1),
        0xFF => format!("*ISB ${:02X}{:02X},X", b2, b1),
        _ => format!("??? ({:02X})", opcode),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cpu(program: &[u8], start: u16) -> Cpu {
        let mut cpu = Cpu::new();
        for (i, &byte) in program.iter().enumerate() {
            cpu.memory[start as usize + i] = byte;
        }
        // Set reset vector to start
        cpu.memory[0xFFFC] = (start & 0xFF) as u8;
        cpu.memory[0xFFFD] = (start >> 8) as u8;
        cpu.reset();
        cpu
    }

    #[test]
    fn test_lda_immediate() {
        let mut cpu = make_cpu(&[0xA9, 0x42], 0x0200);
        cpu.step();
        assert_eq!(cpu.a, 0x42);
        assert!(!cpu.get_flag(FLAG_ZERO));
        assert!(!cpu.get_flag(FLAG_NEGATIVE));
    }

    #[test]
    fn test_lda_zero_sets_flag() {
        let mut cpu = make_cpu(&[0xA9, 0x00], 0x0200);
        cpu.step();
        assert_eq!(cpu.a, 0x00);
        assert!(cpu.get_flag(FLAG_ZERO));
        assert!(!cpu.get_flag(FLAG_NEGATIVE));
    }

    #[test]
    fn test_lda_negative_sets_flag() {
        let mut cpu = make_cpu(&[0xA9, 0x80], 0x0200);
        cpu.step();
        assert!(cpu.get_flag(FLAG_NEGATIVE));
        assert!(!cpu.get_flag(FLAG_ZERO));
    }

    #[test]
    fn test_sta_zero_page() {
        let mut cpu = make_cpu(&[0x85, 0x10], 0x0200);
        cpu.a = 0xAB;
        cpu.step();
        assert_eq!(cpu.memory[0x10], 0xAB);
    }

    #[test]
    fn test_tax() {
        let mut cpu = make_cpu(&[0xAA], 0x0200);
        cpu.a = 0x55;
        cpu.step();
        assert_eq!(cpu.x, 0x55);
    }

    #[test]
    fn test_inx_overflow() {
        let mut cpu = make_cpu(&[0xE8], 0x0200);
        cpu.x = 0xFF;
        cpu.step();
        assert_eq!(cpu.x, 0x00);
        assert!(cpu.get_flag(FLAG_ZERO));
    }

    #[test]
    fn test_adc_no_carry() {
        let mut cpu = make_cpu(&[0x69, 0x10], 0x0200);
        cpu.a = 0x20;
        cpu.step();
        assert_eq!(cpu.a, 0x30);
        assert!(!cpu.get_flag(FLAG_CARRY));
    }

    #[test]
    fn test_adc_with_carry_out() {
        let mut cpu = make_cpu(&[0x69, 0x01], 0x0200);
        cpu.a = 0xFF;
        cpu.step();
        assert_eq!(cpu.a, 0x00);
        assert!(cpu.get_flag(FLAG_CARRY));
        assert!(cpu.get_flag(FLAG_ZERO));
    }

    #[test]
    fn test_adc_overflow() {
        // 0x50 + 0x50 = 0xA0, positive + positive = negative → overflow
        let mut cpu = make_cpu(&[0x69, 0x50], 0x0200);
        cpu.a = 0x50;
        cpu.step();
        assert!(cpu.get_flag(FLAG_OVERFLOW));
        assert!(cpu.get_flag(FLAG_NEGATIVE));
    }

    #[test]
    fn test_sbc_basic() {
        let mut cpu = make_cpu(&[0xE9, 0x10], 0x0200);
        cpu.a = 0x50;
        cpu.set_flag(FLAG_CARRY, true);
        cpu.step();
        assert_eq!(cpu.a, 0x40);
        assert!(cpu.get_flag(FLAG_CARRY));
    }

    #[test]
    fn test_jmp_absolute() {
        let mut cpu = make_cpu(&[0x4C, 0x00, 0x03], 0x0200);
        cpu.step();
        assert_eq!(cpu.pc, 0x0300);
    }

    #[test]
    fn test_jsr_rts() {
        // JSR $0210, then at $0210: RTS
        let mut cpu = Cpu::new();
        cpu.memory[0x0200] = 0x20; // JSR
        cpu.memory[0x0201] = 0x10;
        cpu.memory[0x0202] = 0x02; // target $0210
        cpu.memory[0x0210] = 0x60; // RTS
        cpu.memory[0xFFFC] = 0x00;
        cpu.memory[0xFFFD] = 0x02;
        cpu.reset();
        cpu.step(); // JSR
        assert_eq!(cpu.pc, 0x0210);
        cpu.step(); // RTS
        assert_eq!(cpu.pc, 0x0203);
    }

    #[test]
    fn test_bne_taken() {
        let mut cpu = make_cpu(&[0xD0, 0x04], 0x0200); // BNE +4
        cpu.set_flag(FLAG_ZERO, false);
        cpu.step();
        assert_eq!(cpu.pc, 0x0206);
    }

    #[test]
    fn test_bne_not_taken() {
        let mut cpu = make_cpu(&[0xD0, 0x04], 0x0200);
        cpu.set_flag(FLAG_ZERO, true);
        cpu.step();
        assert_eq!(cpu.pc, 0x0202);
    }

    #[test]
    fn test_stack_push_pop() {
        let mut cpu = Cpu::new();
        cpu.memory[0xFFFC] = 0x00;
        cpu.memory[0xFFFD] = 0x02;
        cpu.reset();
        cpu.stack_push(0xAB);
        cpu.stack_push(0xCD);
        assert_eq!(cpu.stack_pop(), 0xCD);
        assert_eq!(cpu.stack_pop(), 0xAB);
    }

    #[test]
    fn test_jmp_indirect_page_wrap_bug() {
        // JMP ($10FF) should read lo from $10FF and hi from $1000 (not $1100)
        let mut cpu = Cpu::new();
        cpu.memory[0x10FF] = 0x34;
        cpu.memory[0x1000] = 0x12; // bug: wraps within page
        cpu.memory[0x1100] = 0xFF; // this should NOT be used
        cpu.memory[0x0200] = 0x6C; // JMP indirect
        cpu.memory[0x0201] = 0xFF;
        cpu.memory[0x0202] = 0x10;
        cpu.memory[0xFFFC] = 0x00;
        cpu.memory[0xFFFD] = 0x02;
        cpu.reset();
        cpu.step();
        assert_eq!(cpu.pc, 0x1234);
    }

    #[test]
    fn test_rol_carry_in_out() {
        let mut cpu = make_cpu(&[0x2A], 0x0200); // ROL A
        cpu.a = 0b1000_0001;
        cpu.set_flag(FLAG_CARRY, true);
        cpu.step();
        assert_eq!(cpu.a, 0b0000_0011);
        assert!(cpu.get_flag(FLAG_CARRY)); // old bit 7 becomes carry
    }

    #[test]
    fn test_bit_instruction() {
        let mut cpu = Cpu::new();
        cpu.memory[0x0010] = 0b1100_0000; // N=1, V=1
        cpu.memory[0x0200] = 0x24;        // BIT zero page
        cpu.memory[0x0201] = 0x10;
        cpu.memory[0xFFFC] = 0x00;
        cpu.memory[0xFFFD] = 0x02;
        cpu.reset();
        cpu.a = 0b0000_1111; // A & mem = 0 → Z set
        cpu.step();
        assert!(cpu.get_flag(FLAG_ZERO));
        assert!(cpu.get_flag(FLAG_NEGATIVE));
        assert!(cpu.get_flag(FLAG_OVERFLOW));
    }
}

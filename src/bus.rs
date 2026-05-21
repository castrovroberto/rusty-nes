use crate::cartridge::Cartridge;
use crate::cpu::CpuBus;

pub struct Bus {
    ram: [u8; 2048],
    pub cartridge: Option<Cartridge>,
}

impl Bus {
    pub fn new() -> Self {
        Bus {
            ram: [0; 2048],
            cartridge: None,
        }
    }

    pub fn load_cartridge(&mut self, cart: Cartridge) {
        self.cartridge = Some(cart);
    }

    fn read_prg(&self, addr: u16) -> u8 {
        let Some(cart) = &self.cartridge else {
            return 0;
        };
        let prg_len = cart.prg_rom.len();
        // NROM-128 (16 KB) mirrors at $C000; NROM-256 (32 KB) fills $8000–$FFFF
        let offset = (addr - 0x8000) as usize % prg_len;
        cart.prg_rom[offset]
    }
}

impl CpuBus for Bus {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize],
            0x2000..=0x3FFF => 0, // PPU registers — stubbed
            0x4000..=0x4017 => 0, // APU / IO — stubbed
            0x4018..=0x7FFF => 0, // Unmapped
            0x8000..=0xFFFF => self.read_prg(addr),
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize] = data,
            0x2000..=0x3FFF => {} // PPU registers — stubbed
            0x4000..=0x4017 => {} // APU / IO — stubbed
            _ => {}
        }
    }
}

// Flat 64 KB address space used by CPU unit tests and the nestest integration test.
// Avoids coupling tests to the real memory map before the PPU/cartridge are wired up.
pub struct FlatBus {
    pub mem: [u8; 0x10000],
}

impl FlatBus {
    pub fn new() -> Self {
        FlatBus { mem: [0; 0x10000] }
    }
}

impl CpuBus for FlatBus {
    fn read(&self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    fn write(&mut self, addr: u16, data: u8) {
        self.mem[addr as usize] = data;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ram_mirroring() {
        let mut bus = Bus::new();
        bus.write(0x0000, 0xAB);
        assert_eq!(bus.read(0x0800), 0xAB);
        assert_eq!(bus.read(0x1000), 0xAB);
        assert_eq!(bus.read(0x1800), 0xAB);
    }

    #[test]
    fn test_prg_mirror_16kb() {
        use crate::cartridge::{Cartridge, NesHeader, Mirroring};
        let mut prg = vec![0u8; 16 * 1024];
        prg[0] = 0x42;
        prg[16 * 1024 - 1] = 0xFF;
        let cart = Cartridge {
            header: NesHeader {
                prg_rom_size: 1,
                chr_rom_size: 0,
                flags6: 0,
                flags7: 0,
                mapper_number: 0,
                mirroring: Mirroring::Horizontal,
                has_battery_backed_ram: false,
                has_trainer: false,
                four_screen_mode: false,
            },
            prg_rom: prg,
            chr_rom: vec![],
            trainer: None,
        };
        let mut bus = Bus::new();
        bus.load_cartridge(cart);
        // $8000 and $C000 both map to offset 0 of the 16 KB ROM
        assert_eq!(bus.read(0x8000), 0x42);
        assert_eq!(bus.read(0xC000), 0x42);
        // Last byte of each mirror
        assert_eq!(bus.read(0xBFFF), 0xFF);
        assert_eq!(bus.read(0xFFFF), 0xFF);
    }
}

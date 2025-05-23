pub mod apu;
pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod ppu;

fn main() {
    let rom_path = "/Users/robertocastro/dev/nesasm/minimal.nes";

    println!("Attempting to load ROM: {}", rom_path);

    match cartridge::Cartridge::from_file(rom_path) {
        Ok(cartridge) => {
            println!("Successfully loaded ROM!");
            println!("--- Header Information ---");
            println!("PRG ROM Size (16KB units): {}", cartridge.header.prg_rom_size);
            println!("CHR ROM Size (8KB units): {}", cartridge.header.chr_rom_size);
            println!("Mapper Number: {}", cartridge.header.mapper_number);
            println!("Mirroring: {:?}", cartridge.header.mirroring);
            println!("Has Battery-Backed RAM: {}", cartridge.header.has_battery_backed_ram);
            println!("Has Trainer: {}", cartridge.header.has_trainer);
            println!("Four-Screen Mode: {}", cartridge.header.four_screen_mode);
            println!("Flags 6: 0b{:08b}", cartridge.header.flags6);
            println!("Flags 7: 0b{:08b}", cartridge.header.flags7);
            println!("--- ROM Data ---");
            println!("Actual PRG ROM size: {} bytes", cartridge.prg_rom.len());
            println!("Actual CHR ROM size: {} bytes", cartridge.chr_rom.len());
            if let Some(trainer_data) = &cartridge.trainer {
                println!("Trainer present, size: {} bytes", trainer_data.len());
            }
        }
        Err(e) => {
            eprintln!("Error loading ROM: {}", e);
        }
    }
}

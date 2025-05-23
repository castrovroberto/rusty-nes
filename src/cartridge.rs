use std::fs::File;
use std::io::{Read, BufReader, Seek};

// NES Cartridge and ROM handling 

#[derive(Debug)]
pub enum Mirroring {
    Vertical,
    Horizontal,
    FourScreen,
}

#[derive(Debug)]
pub struct NesHeader {
    pub prg_rom_size: u8, // Number of 16 KB units
    pub chr_rom_size: u8, // Number of 8 KB units (0 means CHR RAM)
    pub flags6: u8,
    pub flags7: u8,
    // pub nes2_indicator: u8, // For NES 2.0 format, not handled initially
    // pub console_type: u8, // For NES 2.0 / Famiclone types

    // Derived from flags
    pub mapper_number: u8,
    pub mirroring: Mirroring,
    pub has_battery_backed_ram: bool,
    pub has_trainer: bool, // 512-byte trainer at $7000-$71FF
    pub four_screen_mode: bool,
}

impl NesHeader {
    pub fn from_bytes(header_bytes: &[u8; 16]) -> Result<Self, String> {
        if header_bytes[0..4] != [0x4E, 0x45, 0x53, 0x1A] { // "NES\x1A"
            return Err("Not a valid iNES file format".to_string());
        }

        let prg_rom_size = header_bytes[4];
        let chr_rom_size = header_bytes[5];
        let flags6 = header_bytes[6];
        let flags7 = header_bytes[7];
        // Bytes 8-15 are typically padding or NES 2.0 specific, ignored for basic iNES

        let mirroring = if flags6 & 0x08 != 0 {
            Mirroring::FourScreen
        } else if flags6 & 0x01 != 0 {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };

        let has_battery_backed_ram = (flags6 & 0x02) != 0;
        let has_trainer = (flags6 & 0x04) != 0;

        // Mapper number is formed by the lower nibble of flags6 and upper nibble of flags7
        let mapper_lower_nibble = flags6 >> 4;
        let mapper_upper_nibble = flags7 & 0xF0; // Same as (flags7 >> 4) << 4
        let mapper_number = mapper_upper_nibble | mapper_lower_nibble;

        Ok(NesHeader {
            prg_rom_size,
            chr_rom_size,
            flags6,
            flags7,
            mapper_number,
            mirroring,
            has_battery_backed_ram,
            has_trainer,
            four_screen_mode: (flags6 & 0x08) != 0,
        })
    }
}

// Placeholder for the main Cartridge struct that will eventually hold the header and ROM data
#[derive(Debug)]
pub struct Cartridge {
    pub header: NesHeader,
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>, // CHR ROM or CHR RAM
    pub trainer: Option<Vec<u8>>,
}

impl Cartridge {
    pub fn from_file(path: &str) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open ROM file: {}", e))?;
        let mut reader = BufReader::new(file);

        let mut header_bytes = [0u8; 16];
        reader.read_exact(&mut header_bytes)
            .map_err(|e| format!("Failed to read iNES header: {}", e))?;
        
        // Diagnostic print for raw header bytes
        println!("DEBUG: Raw iNES Header Bytes: {:02X?}", header_bytes);

        let header = NesHeader::from_bytes(&header_bytes)?;

        // Diagnostic prints for parsed header values
        println!("DEBUG: Parsed Header: {:#?}", header);
        println!("DEBUG: Header PRG ROM Size (units): {}", header.prg_rom_size);
        println!("DEBUG: Header CHR ROM Size (units): {}", header.chr_rom_size);
        println!("DEBUG: Header Has Trainer: {}", header.has_trainer);

        let mut trainer: Option<Vec<u8>> = None;
        if header.has_trainer {
            println!("DEBUG: Trainer detected, attempting to read 512 bytes for trainer.");
            let mut trainer_data = vec![0u8; 512];
            reader.read_exact(&mut trainer_data)
                .map_err(|e| format!("Failed to read trainer data: {}", e))?;
            trainer = Some(trainer_data);
            println!("DEBUG: Successfully read trainer data.");
        }

        let prg_rom_size_bytes = header.prg_rom_size as usize * 16 * 1024; // 16KB units
        println!("DEBUG: Calculated PRG ROM size in bytes to read: {}", prg_rom_size_bytes);
        
        if prg_rom_size_bytes == 0 {
            return Err("PRG ROM size is 0, which is invalid.".to_string());
        }

        let mut prg_rom = vec![0u8; prg_rom_size_bytes];
        match reader.read_exact(&mut prg_rom) {
            Ok(_) => println!("DEBUG: Successfully read PRG ROM data."),
            Err(e) => {
                println!("ERROR_DETAIL: Failed during read_exact for PRG ROM: {}", e);
                // Attempt to get remaining file size for context
                // This is a bit hacky and might not be perfectly accurate depending on BufReader state
                let remaining_bytes = reader.buffer().len() as u64 + reader.get_ref().metadata().map_or(0, |m| m.len()) - reader.get_ref().stream_position().map_or(0, |p|p) ;
                println!("DEBUG: Approximate remaining bytes in file before PRG read attempt: {}", remaining_bytes);
                return Err(format!("Failed to read PRG ROM: {}", e));
            }
        }

        let chr_rom_size_bytes = header.chr_rom_size as usize * 8 * 1024; // 8KB units
        println!("DEBUG: Calculated CHR ROM size in bytes to read: {}", chr_rom_size_bytes);
        let mut chr_rom = Vec::new();
        if chr_rom_size_bytes > 0 {
            chr_rom = vec![0u8; chr_rom_size_bytes];
            reader.read_exact(&mut chr_rom)
                .map_err(|e| format!("Failed to read CHR ROM: {}", e))?;
        } else {
            // If chr_rom_size is 0, it often implies CHR RAM. 
            // For now, we'll leave chr_rom empty. Some mappers might allocate CHR RAM.
            // A common size for CHR RAM is 8KB if a game uses it.
            // For simplicity, we are not allocating CHR RAM here, 
            // this will be handled by the PPU or mapper logic later.
        }

        Ok(Cartridge {
            header,
            prg_rom,
            chr_rom,
            trainer,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_header_parsing() {
        // Sample iNES header: NES<EOF>, 1 PRG ROM, 1 CHR ROM, Mapper 0, Horizontal Mirroring, No Battery, No Trainer
        // Flags 6: 00000000 (Mapper lower nibble 0, No Trainer, No Battery, Horizontal Mirroring)
        // Flags 7: 00000000 (Mapper upper nibble 0)
        let header_data: [u8; 16] = [
            0x4E, 0x45, 0x53, 0x1A, // "NES\x1A"
            0x01,                   // PRG ROM size: 1 * 16KB
            0x01,                   // CHR ROM size: 1 * 8KB
            0x00,                   // Flags 6: Mapper 0 (lower), Horizontal mirroring, no battery, no trainer
            0x00,                   // Flags 7: Mapper 0 (upper)
            0x00, 0x00, 0x00, 0x00, // Padding / NES 2.0 - ignored for basic iNES
            0x00, 0x00, 0x00, 0x00
        ];

        let result = NesHeader::from_bytes(&header_data);
        assert!(result.is_ok());
        let header = result.unwrap();

        assert_eq!(header.prg_rom_size, 1);
        assert_eq!(header.chr_rom_size, 1);
        assert_eq!(header.mapper_number, 0);
        assert!(matches!(header.mirroring, Mirroring::Horizontal));
        assert!(!header.has_battery_backed_ram);
        assert!(!header.has_trainer);
        assert!(!header.four_screen_mode);
        assert_eq!(header.flags6, 0x00);
        assert_eq!(header.flags7, 0x00);
    }

    #[test]
    fn test_invalid_magic_number() {
        let header_data: [u8; 16] = [
            0x4E, 0x45, 0x53, 0x00, // Invalid last byte of magic number
            0x01, 0x01, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00
        ];

        let result = NesHeader::from_bytes(&header_data);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Not a valid iNES file format");
    }

    #[test]
    fn test_header_flags_and_mirroring() {
        // Flags 6: 00010111 (Mapper lower 1, FourScreen, Trainer, Battery, Vertical)
        // Mapper 1 (0x10 for upper nibble in Flags 7, 0x01 for lower nibble in Flags 6)
        let header_data: [u8; 16] = [
            0x4E, 0x45, 0x53, 0x1A, // "NES\x1A"
            0x02,                   // PRG ROM size: 2 * 16KB
            0x00,                   // CHR ROM size: 0 (implies CHR RAM)
            0b00010111,             // Flags 6: Mapper lower 1, FourScreen, Trainer, Battery, Vertical
            0b00010000,             // Flags 7: Mapper upper 1
            0x00, 0x00, 0x00, 0x00, 
            0x00, 0x00, 0x00, 0x00
        ];

        let result = NesHeader::from_bytes(&header_data);
        assert!(result.is_ok());
        let header = result.unwrap();

        assert_eq!(header.prg_rom_size, 2);
        assert_eq!(header.chr_rom_size, 0);
        assert_eq!(header.mapper_number, 0b00010001); // Mapper 17 (Upper 1, Lower 1)
        assert!(matches!(header.mirroring, Mirroring::FourScreen)); // FourScreen overrides Vertical bit
        assert!(header.has_battery_backed_ram);
        assert!(header.has_trainer);
        assert!(header.four_screen_mode);

        // Test Vertical Mirroring specifically when FourScreen is not set
        let header_data_vertical: [u8; 16] = [
            0x4E, 0x45, 0x53, 0x1A, 
            0x01, 0x01, 
            0b00000001, // Flags 6: Vertical mirroring, no four-screen
            0b00000000, // Flags 7 
            0x00, 0x00, 0x00, 0x00, 
            0x00, 0x00, 0x00, 0x00
        ];
        let header_vertical = NesHeader::from_bytes(&header_data_vertical).unwrap();
        assert!(matches!(header_vertical.mirroring, Mirroring::Vertical));
        assert!(!header_vertical.four_screen_mode);
    }
} 
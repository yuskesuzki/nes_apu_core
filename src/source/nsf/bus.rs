//! NSF 用メモリバスマッピング
//!
//! 6502 CPU のアドレス空間 ($0000..$FFFF) と、
//! NES の RAM、SRAM、APU レジスタ、バンクスイッチド ROM を接続します。

use mos6502::memory::Bus;
use crate::apu::NesApuCore;

pub struct NsfBus<'a> {
    pub ram: [u8; 0x800],        // $0000..$07FF (ミラーリング $0800..$1FFF)
    pub sram: [u8; 0x2000],      // $6000..$7FFF (ワークRAM)
    pub banks: Vec<[u8; 0x1000]>,// 4KB ごとの ROM バンク
    pub bank_map: [usize; 8],    // $8000..$FFFF の 8 個の 4KB スロットが指すバンク番号
    pub bankswitching_enabled: bool,
    pub rom_unbanked: Vec<u8>,   // バンクスイッチなしの場合の $8000..$FFFF
    pub apu: &'a mut NesApuCore,
}

impl<'a> NsfBus<'a> {
    pub fn new(
        rom_data: &[u8],
        load_addr: u16,
        bankswitch_init: [u8; 8],
        apu: &'a mut NesApuCore,
    ) -> Self {
        let bankswitching_enabled = bankswitch_init.iter().any(|&b| b != 0);

        if bankswitching_enabled {
            let num_banks = (rom_data.len() + 0xFFF) / 0x1000;
            let mut banks = Vec::with_capacity(num_banks.max(1));
            for chunk in rom_data.chunks(0x1000) {
                let mut bank = [0u8; 0x1000];
                bank[..chunk.len()].copy_from_slice(chunk);
                banks.push(bank);
            }
            if banks.is_empty() {
                banks.push([0u8; 0x1000]);
            }

            let mut bank_map = [0usize; 8];
            for i in 0..8 {
                let b = bankswitch_init[i] as usize;
                bank_map[i] = if b < banks.len() { b } else { 0 };
            }

            Self {
                ram: [0; 0x800],
                sram: [0; 0x2000],
                banks,
                bank_map,
                bankswitching_enabled: true,
                rom_unbanked: Vec::new(),
                apu,
            }
        } else {
            // バンクスイッチなし: load_addr からメモリ $8000..$FFFF にロード
            let mut rom_unbanked = vec![0u8; 0x10000]; // 64KB フラット領域として確保
            let start = load_addr as usize;
            let end = (start + rom_data.len()).min(0x10000);
            if start < 0x10000 {
                let len = end - start;
                rom_unbanked[start..end].copy_from_slice(&rom_data[..len]);
            }

            Self {
                ram: [0; 0x800],
                sram: [0; 0x2000],
                banks: Vec::new(),
                bank_map: [0; 8],
                bankswitching_enabled: false,
                rom_unbanked,
                apu,
            }
        }
    }
}

impl<'a> Bus for NsfBus<'a> {
    fn get_byte(&mut self, address: u16) -> u8 {
        match address {
            // $0000..$1FFF: 2KB RAM (ミラーリング)
            0x0000..=0x1FFF => self.ram[(address & 0x07FF) as usize],

            // $4015: APU Status Read
            0x4015 => self.apu.read_register(0x4015),

            // その他の $4000..$4017 レジスタ読み取り
            0x4000..=0x4017 => self.apu.read_register(address),

            // $6000..$7FFF: SRAM
            0x6000..=0x7FFF => self.sram[(address - 0x6000) as usize],

            // $8000..$FFFF: ROM 領域
            0x8000..=0xFFFF => {
                if self.bankswitching_enabled {
                    let slot = ((address - 0x8000) / 0x1000) as usize;
                    let offset = (address & 0x0FFF) as usize;
                    let bank_idx = self.bank_map[slot];
                    if bank_idx < self.banks.len() {
                        self.banks[bank_idx][offset]
                    } else {
                        0
                    }
                } else {
                    self.rom_unbanked[address as usize]
                }
            }

            _ => 0,
        }
    }

    fn set_byte(&mut self, address: u16, value: u8) {
        match address {
            // $0000..$1FFF: 2KB RAM (ミラーリング)
            0x0000..=0x1FFF => {
                self.ram[(address & 0x07FF) as usize] = value;
            }

            // $4000..$4017: APU レジスタ書き込み
            0x4000..=0x4017 => {
                self.apu.write_register(address, value);
            }

            // $5FF8..$5FFF: バンクスイッチングレジスタ
            0x5FF8..=0x5FFF => {
                if self.bankswitching_enabled {
                    let slot = (address - 0x5FF8) as usize;
                    let bank_idx = value as usize;
                    if bank_idx < self.banks.len() {
                        self.bank_map[slot] = bank_idx;
                    }
                }
            }

            // $6000..$7FFF: SRAM
            0x6000..=0x7FFF => {
                self.sram[(address - 0x6000) as usize] = value;
            }

            _ => {}
        }
    }
}

//! NSF (NES Sound Format) 演奏エンジン
//!
//! 6502 CPU (Ricoh 2A03) エミュレータを用いて、
//! NSF ファイル内の INIT ルーチンおよび 60Hz PLAY ルーチンを実行します。

pub mod bus;
pub mod header;

use std::path::Path;
use mos6502::cpu::CPU;
use mos6502::instruction::Nmos6502;
use mos6502::memory::Bus;
use mos6502::registers::{StackPointer, Status};

use crate::apu::NesApuCore;
use crate::source::SoundSource;
pub use bus::NsfBus;
pub use header::NsfHeader;

/// 最大実行ステップ数（無限ループやバグ防止のための安全リミット）
const MAX_EXEC_STEPS: usize = 100_000;

pub struct NsfSource {
    pub header: NsfHeader,
    pub rom_data: Vec<u8>,
    pub current_track: u8, // 1-based (1..=total_songs)
    ram: [u8; 0x800],
    sram: [u8; 0x2000],
    bank_map: [usize; 8],
    play_frame_count: u64,
    memory_buffer: Vec<u8>,
}

impl NsfSource {
    /// ファイルパスから NSF データを読み込んで `NsfSource` を構築します
    pub fn from_file(path: &Path, track_override: Option<u8>) -> Result<Self, String> {
        let data = std::fs::read(path)
            .map_err(|e| format!("Failed to read NSF file {}: {}", path.display(), e))?;
        Self::from_bytes(&data, track_override)
    }

    /// バイト列から NSF データを読み込んで `NsfSource` を構築します
    pub fn from_bytes(data: &[u8], track_override: Option<u8>) -> Result<Self, String> {
        let header = NsfHeader::parse(data)?;
        let rom_data = data[128..].to_vec();

        let track = track_override.unwrap_or(header.starting_song).clamp(1, header.total_songs);

        let mut bank_map = [0usize; 8];
        for i in 0..8 {
            bank_map[i] = header.bankswitch_init[i] as usize;
        }

        Ok(Self {
            header,
            rom_data,
            current_track: track,
            ram: [0; 0x800],
            sram: [0; 0x2000],
            bank_map,
            play_frame_count: 0,
            memory_buffer: vec![0u8; 0x10000],
        })
    }

    /// トラック番号を変更し、再初期化フラグを立てます
    pub fn set_track(&mut self, track: u8) {
        self.current_track = track.clamp(1, self.header.total_songs);
        self.play_frame_count = 0;
    }

    /// 6502 サブルーチンを JSR で呼び出して RTS まで実行します
    fn call_routine(&mut self, apu: &mut NesApuCore, target_addr: u16, a: u8, x: u8) {
        let mut bus = NsfBus {
            ram: self.ram,
            sram: self.sram,
            banks: Vec::new(),
            bank_map: self.bank_map,
            bankswitching_enabled: self.header.uses_bankswitching(),
            rom_unbanked: Vec::new(),
            apu,
        };

        // バンクデータの初期化
        if bus.bankswitching_enabled {
            let num_banks = (self.rom_data.len() + 0xFFF) / 0x1000;
            let mut banks = Vec::with_capacity(num_banks.max(1));
            for chunk in self.rom_data.chunks(0x1000) {
                let mut bank = [0u8; 0x1000];
                bank[..chunk.len()].copy_from_slice(chunk);
                banks.push(bank);
            }
            if banks.is_empty() {
                banks.push([0u8; 0x1000]);
            }
            bus.banks = banks;
        } else {
            let mut rom_unbanked = vec![0u8; 0x10000];
            let start = self.header.load_addr as usize;
            let end = (start + self.rom_data.len()).min(0x10000);
            if start < 0x10000 {
                let len = end - start;
                rom_unbanked[start..end].copy_from_slice(&self.rom_data[..len]);
            }
            bus.rom_unbanked = rom_unbanked;
        }

        let mut cpu = CPU::new(bus, Nmos6502);

        // スタックの初期化
        cpu.registers.stack_pointer = StackPointer(0xFD);
        cpu.registers.accumulator = a;
        cpu.registers.index_x = x;
        cpu.registers.index_y = 0;
        cpu.registers.status.insert(Status::PS_DISABLE_INTERRUPTS);

        // 復帰用アドレス $0000 をスタックに積む (JSR の戻り先 = return_addr - 1 = $FFFF)
        // 6502 の RTS はスタックから (low, high) をポップして +1 したアドレスにジャンプする
        cpu.memory.set_byte(0x01FD, 0xFF); // PC high
        cpu.memory.set_byte(0x01FC, 0xFF); // PC low
        cpu.registers.stack_pointer = StackPointer(0xFB);

        cpu.registers.program_counter = target_addr;

        // RTS で PC が $0000 に戻るまでシングルステップ実行
        let mut steps = 0;
        while steps < MAX_EXEC_STEPS {
            if cpu.registers.program_counter == 0x0000 {
                break;
            }
            if !cpu.single_step() {
                break;
            }
            steps += 1;
        }

        // 実行後の RAM・SRAM・バンク状態を退避
        self.ram = cpu.memory.ram;
        self.sram = cpu.memory.sram;
        self.bank_map = cpu.memory.bank_map;
    }

    /// DMC サンプルフェッチ用に最新のメモリマッピング（RAM/SRAM/ROMバンク）を APU DMC へ同期
    pub fn sync_dmc_memory(&mut self, apu: &mut NesApuCore) {
        if self.memory_buffer.len() != 0x10000 {
            self.memory_buffer = vec![0u8; 0x10000];
        }

        // $0000..$1FFF: 2KB RAM (ミラーリング)
        for mirror in 0..4 {
            let offset = mirror * 0x800;
            self.memory_buffer[offset..offset + 0x800].copy_from_slice(&self.ram);
        }

        // $6000..$7FFF: SRAM
        self.memory_buffer[0x6000..0x8000].copy_from_slice(&self.sram);

        // $8000..$FFFF: ROM 領域
        if self.header.uses_bankswitching() {
            let num_banks = (self.rom_data.len() + 0xFFF) / 0x1000;
            for slot in 0..8 {
                let bank_idx = self.bank_map[slot];
                let start_addr = 0x8000 + slot * 0x1000;
                if bank_idx < num_banks {
                    let rom_offset = bank_idx * 0x1000;
                    if rom_offset < self.rom_data.len() {
                        let end = (rom_offset + 0x1000).min(self.rom_data.len());
                        let len = end - rom_offset;
                        self.memory_buffer[start_addr..start_addr + len].copy_from_slice(&self.rom_data[rom_offset..end]);
                    }
                }
            }
        } else {
            let start = self.header.load_addr as usize;
            let end = (start + self.rom_data.len()).min(0x10000);
            if start < 0x10000 {
                let len = end - start;
                self.memory_buffer[start..end].copy_from_slice(&self.rom_data[..len]);
            }
        }

        apu.dmc.update_memory(&self.memory_buffer);
    }
}

impl SoundSource for NsfSource {
    fn title(&self) -> String {
        let t = if self.header.title.is_empty() {
            "Unknown"
        } else {
            &self.header.title
        };
        let a = if self.header.artist.is_empty() {
            "Unknown"
        } else {
            &self.header.artist
        };
        format!(
            "NSF [{}] by {} (Track {}/{})",
            t, a, self.current_track, self.header.total_songs
        )
    }

    fn init(&mut self, apu: &mut NesApuCore) {
        self.ram = [0; 0x800];
        self.sram = [0; 0x2000];
        for i in 0..8 {
            self.bank_map[i] = self.header.bankswitch_init[i] as usize;
        }
        self.play_frame_count = 0;

        // APU をデフォルト初期化 ($4015 全有効)
        apu.write_register(0x4015, 0x0F);

        // INIT ルーチン呼び出し (A = track - 1, X = 0 (NTSC))
        let track_idx = self.current_track.saturating_sub(1);
        self.call_routine(apu, self.header.init_addr, track_idx, 0);

        // DMC メモリを同期
        self.sync_dmc_memory(apu);
    }

    fn on_vblank(&mut self, apu: &mut NesApuCore) {
        self.play_frame_count += 1;
        // PLAY ルーチン呼び出し
        self.call_routine(apu, self.header.play_addr, 0, 0);

        // DMC メモリを同期
        self.sync_dmc_memory(apu);
    }
}

extern crate crc32fast;
extern crate rust_nes;

use crc32fast::Hasher;
use std::path::Path;

macro_rules! rom_tests {
    ($($name:ident: $value:expr,)*) => {
    $(
        #[test]
        fn $name() {
            let (cycles, expected_crc32, rom_path) = $value;
            let cartridge = rust_nes::get_cartridge(rom_path.to_str().unwrap()).unwrap();
            let framebuffer = rust_nes::run_headless_cycles(cartridge, cycles);
            let mut hasher = Hasher::new();
            hasher.update(&framebuffer);
            let actual_crc32 = hasher.finalize();

            assert_eq!(
                actual_crc32,
                expected_crc32,
                "{}",
                framebuffer_to_ascii_art(framebuffer)
            );
        }
    )*
    }
}

rom_tests! {
    // ----- General CPU Tests -----
    blargg_nes_cpu_test_official: (0x13399B3 * 3 as usize, 2605351162, Path::new("..").join("roms").join("test").join("blargg_nes_cpu_test5").join("official.nes")),
    instr_test_official_only: (0x33B7410 * 3 as usize, 216765697, Path::new("..").join("roms").join("test").join("instr_test-v3").join("official_only.nes")),
    cpu_timing_test: (0x11EB284 * 3 as usize, 377355712, Path::new("..").join("roms").join("test").join("cpu_timing_test6").join("cpu_timing_test.nes")),
    // instr_misc:  (0x11EB284 * 3 as usize, 377355712, Path::new("..").join("roms").join("test").join("instr_misc").join("instr_misc.nes")), - Requires unofficial opcodes
    // instr_timing:  (0x11EB284 * 3 as usize, 377355712, Path::new("..").join("roms").join("test").join("instr_timing").join("instr_timing.nes")), - Requires unofficial opcodes
    cpu_dummy_reads: (0x18F464 * 3 as usize, 2170164011, Path::new("..").join("roms").join("test").join("cpu_dummy_reads").join("cpu_dummy_reads.nes")),
    cpu_dummy_writes_oam: (0xB45D59 * 3 as usize, 3847704951, Path::new("..").join("roms").join("test").join("cpu_dummy_writes").join("cpu_dummy_writes_oam.nes")),
    // cpu_dummy_writes_ppumem: (0xB45D59 * 3 as usize, 3847704951, Path::new("..").join("roms").join("test").join("cpu_dummy_writes").join("cpu_dummy_writes_ppumem.nes")), # Opcodes are fine but open bus behaviour is wrong apparently
    cpu_exec_space_ppuio: (0x2367FD * 3 as usize, 2453696551, Path::new("..").join("roms").join("test").join("cpu_exec_space").join("test_cpu_exec_space_ppuio.nes")),
    branch_timing_basics: (0xCAF7C * 3 as usize, 880592341, Path::new("..").join("roms").join("test").join("branch_timing_tests").join("1.Branch_Basics.nes")),
    branch_timing_backward: (0xCAF7C * 3 as usize, 6166974, Path::new("..").join("roms").join("test").join("branch_timing_tests").join("2.Backward_Branch.nes")),
    branch_timing_forward: (0xCAF7C * 3 as usize, 1293237708, Path::new("..").join("roms").join("test").join("branch_timing_tests").join("3.Forward_Branch.nes")),
    cpu_interrupts_1_cli_delay:  (0x8987A * 3 as usize, 459637199, Path::new("..").join("roms").join("test").join("cpu_interrupts_v2").join("rom_singles").join("1-cli_latency.nes")),
    //cpu_interrupts_2_nmi_brk:  (0x138066 * 3 as usize, 459637199, Path::new("..").join("roms").join("test").join("cpu_interrupts_v2").join("rom_singles").join("2-nmi_and_brk.nes")),

    // ----- General PPU Tests -----
    blargg_nes_ppu_test_palette_ram: (0xD23D0 * 3 as usize, 1300901188, Path::new("..").join("roms").join("test").join("blargg_ppu_tests_2005.09.15b").join("palette_ram.nes")),
    blargg_nes_ppu_test_sprite_ram: (0xD23D0 * 3 as usize, 1300901188, Path::new("..").join("roms").join("test").join("blargg_ppu_tests_2005.09.15b").join("sprite_ram.nes")),
    blargg_nes_ppu_test_vbl_clear_time: (0xD23D0 * 3 as usize, 1300901188, Path::new("..").join("roms").join("test").join("blargg_ppu_tests_2005.09.15b").join("vbl_clear_time.nes")),
    blargg_nes_ppu_test_vram_access: (0xD23D0 * 3 as usize, 1300901188, Path::new("..").join("roms").join("test").join("blargg_ppu_tests_2005.09.15b").join("vram_access.nes")),
    // ppu_open_bus: (0x1C22B4 * 3 as usize, 3764449243, Path::new("..").join("roms").join("test").join("ppu_open_bus").join("ppu_open_bus.nes")), - Not working, claims because no decay
    // ppu_read_buffer: (0x1C22B4 * 3 as usize, 3764449243, Path::new("..").join("roms").join("test").join("ppu_read_buffer").join("test_ppu_read_buffer.nes")), - Fails on several counts, likely quite badly implemented read buffer

    // ----- DMA/DMC Specific Tests -----
    //dma_2007_read: (0xD23D0 * 3 as usize, 1300901188, Path::new("..").join("roms").join("test").join("dmc_dma_during_read4").join("dma_2007_read.nes")), - Fails, unclear why
    dma_2007_write: (0xFDDCD * 3 as usize, 1314372172, Path::new("..").join("roms").join("test").join("dmc_dma_during_read4").join("dma_2007_write.nes")),
    //dma_4016_read: (0xD23D0 * 3 as usize, 1300901188, Path::new("..").join("roms").join("test").join("dmc_dma_during_read4").join("dma_4016_read.nes")), - Fails, unclear why
    //double_2007_read: (0xD23D0 * 3 as usize, 1300901188, Path::new("..").join("roms").join("test").join("dmc_dma_during_read4").join("double_2007_read.nes")), - Fails, unclear why
    read_write_2007: (0xFDDCD * 3 as usize, 2762297165, Path::new("..").join("roms").join("test").join("dmc_dma_during_read4").join("read_write_2007.nes")),

    // ----- OAM Specific Tests -----
    oam_read: (0x1C22B4 * 3 as usize, 3764449243, Path::new("..").join("roms").join("test").join("oam_read").join("oam_read.nes")),
    oam_stress: (0x30E035C * 3 as usize, 2040203052, Path::new("..").join("roms").join("test").join("oam_stress").join("oam_stress.nes")),

    // ----- VBL/NMI Timing Tests -----
    ppu_vbl_nmi_01_basics: (0x4FF06A * 3 as usize, 3760518270, Path::new("..").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("01-vbl_basics.nes")),
    ppu_vbl_nmi_02_vbl_set_time: (0x5BC105 * 3 as usize, 98639598, Path::new("..").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("02-vbl_set_time.nes")),
    ppu_vbl_nmi_03_clear_time: (0x4DAAC6 * 3 as usize, 2257284403, Path::new("..").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("03-vbl_clear_time.nes")),
    ppu_vbl_nmi_04_nmi_control: (0x2621FA * 3 as usize, 1597701030, Path::new("..").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("04-nmi_control.nes")),
    ppu_vbl_nmi_05_nmi_timing: (0x6A4B9B * 3 as usize, 3525866603, Path::new("..").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("05-nmi_timing.nes")),
    ppu_vbl_nmi_06_suppression: (0x6ABFF0 * 3 as usize, 3592094813, Path::new("..").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("06-suppression.nes")),
    ppu_vbl_nmi_07_nmi_on_timing: (0x5EEF57 * 3 as usize, 1516309785, Path::new("..").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("07-nmi_on_timing.nes")),
    ppu_vbl_nmi_08_nmi_off_timing: (0x6791A1 * 3 as usize, 2373747886, Path::new("..").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("08-nmi_off_timing.nes")),
    ppu_vbl_nmi_09_even_odd_frames: (0x43AB75 * 3 as usize, 817319831, Path::new("..").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("09-even_odd_frames.nes")),
    // ppu_vbl_nmi_10_even_odd_timing: (0x4FF06A * 3 as usize, 3764449243, Path::new("..").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("10-even_odd_timing.nes")), - Failed #3 Clock is skipped too late relative to enabling bg
    vbl_nmi_timing_frame_basics: (0x5CA9A1 * 3 as usize, 3792590752, Path::new("..").join("roms").join("test").join("vbl_nmi_timing").join("1.frame_basics.nes")),
    vbl_nmi_timing_vbl_timing: (0x51C1BF * 3 as usize, 839309104, Path::new("..").join("roms").join("test").join("vbl_nmi_timing").join("2.vbl_timing.nes")),
    vbl_nmi_timing_even_odd_frames: (0x3A94DF * 3 as usize, 3404062440, Path::new("..").join("roms").join("test").join("vbl_nmi_timing").join("3.even_odd_frames.nes")),
    vbl_nmi_timing_vbl_clear_timing: (0x3BF1E1 * 3 as usize, 1325590663, Path::new("..").join("roms").join("test").join("vbl_nmi_timing").join("4.vbl_clear_timing.nes")),
    vbl_nmi_timing_nmi_suppression: (0x539313 * 3 as usize, 670688491, Path::new("..").join("roms").join("test").join("vbl_nmi_timing").join("5.nmi_suppression.nes")),
    vbl_nmi_timing_nmi_disable: (0x3CDA89 * 3 as usize, 324384964, Path::new("..").join("roms").join("test").join("vbl_nmi_timing").join("6.nmi_disable.nes")),
    vbl_nmi_timing_nmi_timing: (0x3C6634 * 3 as usize, 4107311669, Path::new("..").join("roms").join("test").join("vbl_nmi_timing").join("7.nmi_timing.nes")),

    // ----- Sprite Zero Hit Tests -----
    sprite_zero_hit_all: (0x10E56CB * 3 as usize, 1340789466, Path::new("..").join("roms").join("test").join("ppu_sprite_hit").join("ppu_sprite_hit.nes")),

    // ----- Sprite Overflow Tests
    sprite_overflow: (0xDAFD85 * 3 as usize, 1808572613, Path::new("..").join("roms").join("test").join("ppu_sprite_overflow").join("ppu_sprite_overflow.nes")),

    // ----- Mapper Tests -----
    mapper_0_p32k_c8k_v: (0x309599 * 3 as usize, 1798638175, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M0_P32K_C8K_V.nes")),
    mapper_0_p32k_cr8k_v: (0x50D915 * 3 as usize, 3474562170, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M0_P32K_CR8K_V.nes")),
    // TODO - Below is likely wrong, we don't have 32KB CHR RAM in the screenshot
    mapper_0_p32k_cr32k_v: (0x4C4DC8 * 3 as usize, 3474562170, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M0_P32K_CR32K_V.nes")),
    mapper_1_no_chrom: (0x4F7C0F * 3 as usize, 1531525988, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M1_P128K.nes")),
    mapper_1_p128k_c32k: (0x3C6627 * 3 as usize, 3934498320, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M1_P128K_C32K.nes")),
    mapper_1_p128k_c32k_s8k: (0x3C6627 * 3 as usize, 3934498320, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M1_P128K_C32K_S8K.nes")),
    mapper_1_p128k_c32k_w8k: (0x3C6627 * 3 as usize, 3934498320, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M1_P128K_C32K_W8K.nes")),
    mapper_1_p128k_c128k: (0x3C6627 * 3 as usize, 2354549445, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M1_P128K_C128K.nes")),
    mapper_1_p128k_c128k_s8k: (0x3C6627 * 3 as usize, 2354549445, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M1_P128K_C128K_S8K.nes")),
    mapper_1_p128k_c128k_w8k: (0x3C6627 * 3 as usize, 2354549445, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M1_P128K_C128K_W8K.nes")),
    mapper_2_p128k_cr8k_v: (0x253959 * 3 as usize, 1058817094, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M2_P128K_CR8K_V.nes")),
    mapper_2_p128k_v: (0x24C505 * 3 as usize, 3178533875, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M2_P128K_V.nes")),
    mapper_3: (0x2A38FA * 3 as usize, 2606110735, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M3_P32K_C32K_H.nes")),
    mapper_4_no_chrom: (0x30213C * 3 as usize, 3944012330, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M4_P128K.nes")),
    mapper_4_p128k_cr8k: (0x277EF7 * 3 as usize, 1769737631, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M4_P128K_CR8K.nes")),
    // TODO - Below is likely wrong, we don't have 32KB CHR RAM in the screenshot
    mapper_4_p128k_cr32k: (0x28DBF4 * 3 as usize, 1769737631, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M4_P128K_CR32K.nes")),
    mapper_4_p256k_c256k: (0xC3B1E * 3 as usize, 502837231, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M4_P256K_C256K.nes")),
    mapper_7_p128k: (0x262201 * 3 as usize, 2603256516, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M7_P128K.nes")),
    mapper_7_p128k_cr8k: (0x262201 * 3 as usize, 423779697, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M7_P128K_CR8K.nes")),
    mapper_9_p128k_c64k: (0x4F5DD * 3 as usize, 3084268463, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M9_P128K_C64K.nes")),
    mapper_10_p128k_c64k_s8k: (0x1C9707 * 3 as usize, 2938351879, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M10_P128K_C64K_S8K.nes")),
    mapper_10_p128k_c64k_w8k: (0x10521E * 3 as usize, 2938351879, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M10_P128K_C64K_W8K.nes")),
    mapper_11_p64k_c64k_v: (0x113AC6 * 3 as usize, 2383587170, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M11_P64K_C64K_V.nes")),
    // TODO - Below renders as BNROM in holy mapperel instead of color dreams because I don't bank CHRRAM
    // mapper_11_p64k_c64k_v: (0x113AC6 * 3 as usize, 2383587170, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M11_P64K_CR32K_V.nes")),
    mapper_34_p128k_h: (0x38C38A * 3 as usize, 3229261591, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M34_P128K_H.nes")),
    mapper_34_p128k_cr8k_h: (0x2A38FA * 3 as usize, 1108494498, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M34_P128K_CR8K_H.nes")),
    mapper_66_p64k_c16k_v: (0x19DD0C * 3 as usize, 2221445495, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M66_P64K_C16K_V.nes")),
    mapper_180_p128k_cr8k_h: (0x2A38FA * 3 as usize, 3038721105, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M180_P128K_CR8K_H.nes")),
    mapper_180_p128k_h: (0x2B95F7 * 3 as usize, 930604004, Path::new("..").join("roms").join("test").join("holy_mapperel").join("M180_P128K_H.nes")),

    // ----- MMC3 IRQ Tests -----
    mmc3_irq_clocking: (0x105218 * 3 as usize, 4185058565, Path::new("..").join("roms").join("test").join("mmc3_test").join("rom_singles").join("1-clocking.nes")),
    mmc3_irq_details: (0x113AC1 * 3 as usize, 1296344911, Path::new("..").join("roms").join("test").join("mmc3_test").join("rom_singles").join("2-details.nes")),
    //mmc3_irq_a12_clocking: (0x105218 * 3 as usize, 820133214, Path::new("..").join("roms").join("test").join("mmc3_test").join("rom_singles").join("3-A12_clocking.nes")),
    //mmc3_irq_scanline_timing: (0x90CD6 * 3 as usize, 3691845950, Path::new("..").join("roms").join("test").join("mmc3_test").join("rom_singles").join("4-scanline_timing.nes")), // Failed #14 - IRQ never occurred
    mmc3_irq_mmc3: (0x163A62 * 3 as usize, 144123581, Path::new("..").join("roms").join("test").join("mmc3_test").join("rom_singles").join("5-MMC3.nes")),
    //mmc3_irq_mmc3_alt: (0x90CD6 * 3 as usize, 3691845950, Path::new("..").join("roms").join("test").join("mmc3_test").join("rom_singles").join("6-MMC3_alt.nes")), // Failed #2 - Don't think I support the MMC3 alternate board

    // ----- APU Tests -----
    apu_test_1_length_counter: (0x1551B8 * 3 as usize, 1135491406, Path::new("..").join("roms").join("test").join("apu_test").join("rom_singles").join("1-len_ctr.nes")),
    apu_test_2_length_table: (0x1AC5AD * 3 as usize, 1850311913, Path::new("..").join("roms").join("test").join("apu_test").join("rom_singles").join("2-len_table.nes")),
    apu_test_3_irq_flag: (0x1D7FA9 * 3 as usize, 902361631, Path::new("..").join("roms").join("test").join("apu_test").join("rom_singles").join("3-irq_flag.nes")),
    apu_test_4_jitter: (0x18F45C * 3 as usize, 2672842930, Path::new("..").join("roms").join("test").join("apu_test").join("rom_singles").join("4-jitter.nes")),
    apu_test_5_length_timing: (0x3B7D82 * 3 as usize, 1825584722, Path::new("..").join("roms").join("test").join("apu_test").join("rom_singles").join("5-len_timing.nes")),
    apu_test_6_irq_flag_timing: (0x146910 * 3 as usize, 1222179157, Path::new("..").join("roms").join("test").join("apu_test").join("rom_singles").join("6-irq_flag_timing.nes")),
    //apu_test_7_dmc_basics: (0x1AC5AD * 3 as usize, 1850311913, Path::new("..").join("roms").join("test").join("apu_test").join("rom_singles").join("7-dmc_basics.nes")), // DMC channel not implemented
    //apu_test_8_dmc_rates: (0x1AC5AD * 3 as usize, 1850311913, Path::new("..").join("roms").join("test").join("apu_test").join("rom_singles").join("8-dmc_rates.nes")), // DMC channel not implemented
    apu_test_01_length_counter: (0x1551B9 * 3 as usize, 1300901188, Path::new("..").join("roms").join("test").join("blargg_apu_2005.07.30").join("01.len_ctr.nes")),
    apu_test_02_length_table: (0x10C66A * 3 as usize, 1300901188, Path::new("..").join("roms").join("test").join("blargg_apu_2005.07.30").join("02.len_table.nes")),
    apu_test_03_irq_flag: (0x163A61 * 3 as usize, 1300901188, Path::new("..").join("roms").join("test").join("blargg_apu_2005.07.30").join("03.irq_flag.nes")),
    apu_test_04_clock_jitter: (0x163A61 * 3 as usize, 1300901188, Path::new("..").join("roms").join("test").join("blargg_apu_2005.07.30").join("04.clock_jitter.nes")),
    apu_test_05_len_timing_mode0: (0x163A62 * 3 as usize, 1300901188, Path::new("..").join("roms").join("test").join("blargg_apu_2005.07.30").join("05.len_timing_mode0.nes")),
    apu_test_06_len_timing_mode1: (0x163A62 * 3 as usize, 1300901188, Path::new("..").join("roms").join("test").join("blargg_apu_2005.07.30").join("06.len_timing_mode1.nes")),
    apu_test_07_irq_flag_timing: (0x163A62 * 3 as usize, 1300901188, Path::new("..").join("roms").join("test").join("blargg_apu_2005.07.30").join("07.irq_flag_timing.nes")),
    //apu_test_08_irq_timing: (0x163A62 * 3 as usize, 1300901188, Path::new("..").join("roms").join("test").join("blargg_apu_2005.07.30").join("08.irq_timing.nes")), - IRQ happening too soon
    apu_test_09_reset_timing: (0xF696D * 3 as usize, 1300901188, Path::new("..").join("roms").join("test").join("blargg_apu_2005.07.30").join("09.reset_timing.nes")), // Suspect. I haven't even implemented reset anywhere!
    // apu_test_10_len_halt_timing: (0xF696D * 3 as usize, 1300901188, Path::new("..").join("roms").join("test").join("blargg_apu_2005.07.30").join("10.len_halt_timing.nes")), // Failing #03
    // apu_test_11_len_reload_timing: (0xF696D * 3 as usize, 1300901188, Path::new("..").join("roms").join("test").join("blargg_apu_2005.07.30").join("11.len_reload_timing.nes")), // Failing #04
}

const ASCII_GRAYSCALE_ARRAY: [char; 96] = [
    '.', '-', '`', '\'', ',', ':', '_', ';', '~', '\\', '"', '/', '!', '|', '\\', '\\', 'i', '^', 't', 'r', 'c', '*',
    'v', '?', 's', '(', ')', '+', 'l', 'j', '1', '=', 'e', '{', '[', ']', 'z', '}', '<', 'x', 'o', '7', 'f', '>', 'a',
    'J', 'y', '3', 'I', 'u', 'n', '5', '4', '2', 'b', '6', 'L', 'w', '9', 'k', '#', 'd', 'g', 'h', 'q', '8', '0', 'V',
    'p', 'T', '$', 'Y', 'A', 'C', 'S', 'F', 'P', 'U', 'Z', '%', 'm', 'E', 'G', 'X', 'N', 'O', '&', 'D', 'K', 'B', 'R',
    '@', 'H', 'Q', 'W', 'M',
];

fn framebuffer_to_ascii_art(fb: [u8; (256 * 240 * 4) as usize]) -> String {
    fn lookup(greyscale: f32) -> char {
        ASCII_GRAYSCALE_ARRAY[(greyscale * ASCII_GRAYSCALE_ARRAY.len() as f32) as usize]
    }

    fb.chunks(4)
        .map(|pixel| {
            let r = pixel[0] as f32;
            let g = pixel[1] as f32;
            let b = pixel[2] as f32;

            (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255f32
        })
        .map(lookup)
        .collect::<Vec<char>>()
        .chunks(256)
        .map(|char_line| char_line.iter().collect::<String>())
        .fold(String::new(), |a, b| a + "\n" + &b)
}

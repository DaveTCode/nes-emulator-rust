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
            let framebuffer = rust_nes::run_headless_cycles(rom_path.to_str().unwrap(), cycles);
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
    blargg_nes_cpu_test_official: (0x13399B3 * 3 as usize, 2605351162, Path::new(".").join("roms").join("test").join("blargg_nes_cpu_test5").join("official.nes")),
    instr_test_official_only: (0x33B7410 * 3 as usize, 216765697, Path::new(".").join("roms").join("test").join("instr_test-v3").join("official_only.nes")),
    cpu_timing_test: (0x11EB284 * 3 as usize, 377355712, Path::new(".").join("roms").join("test").join("cpu_timing_test6").join("cpu_timing_test.nes")),
    // instr_misc:  (0x11EB284 * 3 as usize, 377355712, Path::new(".").join("roms").join("test").join("instr_misc").join("instr_misc.nes")), - Requires APU length counter (singles up to that pass)
    // instr_timing:  (0x11EB284 * 3 as usize, 377355712, Path::new(".").join("roms").join("test").join("instr_timing").join("instr_timing.nes")), - Requires APU length counter
    cpu_dummy_reads: (0x18F464 * 3 as usize, 2170164011, Path::new(".").join("roms").join("test").join("cpu_dummy_reads").join("cpu_dummy_reads.nes")),
    cpu_dummy_writes_oam: (0xB45D59 * 3 as usize, 3847704951, Path::new(".").join("roms").join("test").join("cpu_dummy_writes").join("cpu_dummy_writes_oam.nes")),
    // cpu_dummy_writes_ppumem: (0xB45D59 * 3 as usize, 3847704951, Path::new(".").join("roms").join("test").join("cpu_dummy_writes").join("cpu_dummy_writes_ppumem.nes")), # Opcodes are fine but open bus behaviour is wrong apparently
    cpu_exec_space_ppuio: (0x2367FD * 3 as usize, 2453696551, Path::new(".").join("roms").join("test").join("cpu_exec_space").join("test_cpu_exec_space_ppuio.nes")),
    branch_timing_basics: (0xCAF7C * 3 as usize, 880592341, Path::new(".").join("roms").join("test").join("branch_timing_tests").join("1.Branch_Basics.nes")),
    branch_timing_backward: (0xCAF7C * 3 as usize, 6166974, Path::new(".").join("roms").join("test").join("branch_timing_tests").join("2.Backward_Branch.nes")),
    branch_timing_forward: (0xCAF7C * 3 as usize, 1293237708, Path::new(".").join("roms").join("test").join("branch_timing_tests").join("3.Forward_Branch.nes")),

    // ----- General PPU Tests -----
    blargg_nes_ppu_test_palette_ram: (0xD23D0 * 3 as usize, 1300901188, Path::new(".").join("roms").join("test").join("blargg_ppu_tests_2005.09.15b").join("palette_ram.nes")),
    blargg_nes_ppu_test_sprite_ram: (0xD23D0 * 3 as usize, 1300901188, Path::new(".").join("roms").join("test").join("blargg_ppu_tests_2005.09.15b").join("sprite_ram.nes")),
    blargg_nes_ppu_test_vbl_clear_time: (0xD23D0 * 3 as usize, 1300901188, Path::new(".").join("roms").join("test").join("blargg_ppu_tests_2005.09.15b").join("vbl_clear_time.nes")),
    blargg_nes_ppu_test_vram_access: (0xD23D0 * 3 as usize, 1300901188, Path::new(".").join("roms").join("test").join("blargg_ppu_tests_2005.09.15b").join("vram_access.nes")),
    // ppu_open_bus: (0x1C22B4 * 3 as usize, 3764449243, Path::new(".").join("roms").join("test").join("ppu_open_bus").join("ppu_open_bus.nes")), - Not working, claims because no decay
    // ppu_read_buffer: (0x1C22B4 * 3 as usize, 3764449243, Path::new(".").join("roms").join("test").join("ppu_read_buffer").join("test_ppu_read_buffer.nes")), - Crashes trying to read invalid address from PPU 2007

    // ----- DMA/DMC Specific Tests -----
    //dma_2007_read: (0xD23D0 * 3 as usize, 1300901188, Path::new(".").join("roms").join("test").join("dmc_dma_during_read4").join("dma_2007_read.nes")), - Fails, unclear why
    dma_2007_write: (0xFDDCD * 3 as usize, 1314372172, Path::new(".").join("roms").join("test").join("dmc_dma_during_read4").join("dma_2007_write.nes")),
    //dma_4016_read: (0xD23D0 * 3 as usize, 1300901188, Path::new(".").join("roms").join("test").join("dmc_dma_during_read4").join("dma_4016_read.nes")), - Fails, unclear why
    //double_2007_read: (0xD23D0 * 3 as usize, 1300901188, Path::new(".").join("roms").join("test").join("dmc_dma_during_read4").join("double_2007_read.nes")), - Fails, unclear why
    read_write_2007: (0xFDDCD * 3 as usize, 2762297165, Path::new(".").join("roms").join("test").join("dmc_dma_during_read4").join("read_write_2007.nes")),

    // ----- OAM Specific Tests -----
    oam_read: (0x1C22B4 * 3 as usize, 3764449243, Path::new(".").join("roms").join("test").join("oam_read").join("oam_read.nes")),
    // oam_stress: (0x1C22B4 * 3 as usize, 3764449243, Path::new(".").join("roms").join("test").join("oam_read").join("oam_stress.nes")), - Not working, not sure why

    // ----- VBL/NMI Timing Tests -----
    ppu_vbl_nmi_01_basics: (0x4FF06A * 3 as usize, 3760518270, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("01-vbl_basics.nes")),
    ppu_vbl_nmi_02_vbl_set_time: (0x5BC105 * 3 as usize, 98639598, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("02-vbl_set_time.nes")),
    ppu_vbl_nmi_03_clear_time: (0x4DAAC6 * 3 as usize, 2257284403, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("03-vbl_clear_time.nes")),
    // ppu_vbl_nmi_04_nmi_control: (0x4FF06A * 3 as usize, 3764449243, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("04-nmi_control.nes")), - Failed #11 - immediate occurrence after next instruction
    // ppu_vbl_nmi_05_nmi_timing: (0x4FF06A * 3 as usize, 3764449243, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("05-nmi_timing.nes")), - Failed
    ppu_vbl_nmi_06_suppression: (0x6ABFF0 * 3 as usize, 3592094813, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("06-suppression.nes")),
    // ppu_vbl_nmi_07_nmi_on_timing: (0x4FF06A * 3 as usize, 3764449243, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("07-nmi_on_timing.nes")), - Failed
    // ppu_vbl_nmi_08_nmi_off_timing: (0x4FF06A * 3 as usize, 3764449243, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("08-nmi_off_timing.nes")), - Failed
    ppu_vbl_nmi_09_even_odd_frames: (0x43AB75 * 3 as usize, 817319831, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("09-even_odd_frames.nes")),
    // ppu_vbl_nmi_10_even_odd_timing: (0x4FF06A * 3 as usize, 3764449243, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("10-even_odd_timing.nes")), - Failed #3 Clock is skipped too late
    vbl_nmi_timing_frame_basics: (0x5CA9A1 * 3 as usize, 3792590752, Path::new(".").join("roms").join("test").join("vbl_nmi_timing").join("1.frame_basics.nes")),
    vbl_nmi_timing_vbl_timing: (0x51C1BF * 3 as usize, 839309104, Path::new(".").join("roms").join("test").join("vbl_nmi_timing").join("2.vbl_timing.nes")),
    vbl_nmi_timing_even_odd_frames: (0x3A94DF * 3 as usize, 3404062440, Path::new(".").join("roms").join("test").join("vbl_nmi_timing").join("3.even_odd_frames.nes")),
    vbl_nmi_timing_vbl_clear_timing: (0x3BF1E1 * 3 as usize, 1325590663, Path::new(".").join("roms").join("test").join("vbl_nmi_timing").join("4.vbl_clear_timing.nes")),
    vbl_nmi_timing_nmi_suppression: (0x539313 * 3 as usize, 670688491, Path::new(".").join("roms").join("test").join("vbl_nmi_timing").join("5.nmi_suppression.nes")),
    // vbl_nmi_timing_nmi_disable: (0x3C6634 * 3 as usize, 1325590663, Path::new(".").join("roms").join("test").join("vbl_nmi_timing").join("6.nmi_disable.nes")), - TODO Failing #2
    // vbl_nmi_timing_nmi_timing: (0x3C6634 * 3 as usize, 1325590663, Path::new(".").join("roms").join("test").join("vbl_nmi_timing").join("7.nmi_timing.nes")), - TODO Failing #3

    // ----- Sprite Zero Hit Tests -----
    sprite_zero_hit_01_basics: (0x1DF406 * 3 as usize, 2445173019, Path::new(".").join("roms").join("test").join("ppu_sprite_hit").join("rom_singles").join("01-basics.nes")),
    sprite_zero_hit_02_alignment: (0x1DF406 * 3 as usize, 901509059, Path::new(".").join("roms").join("test").join("ppu_sprite_hit").join("rom_singles").join("02-alignment.nes")),
    sprite_zero_hit_03_corners: (0x1DF406 * 3 as usize, 218094906, Path::new(".").join("roms").join("test").join("ppu_sprite_hit").join("rom_singles").join("03-corners.nes")),
    sprite_zero_hit_04_flip: (0x1DF406 * 3 as usize, 3268146222, Path::new(".").join("roms").join("test").join("ppu_sprite_hit").join("rom_singles").join("04-flip.nes")),
    //sprite_zero_hit_05_left_clip: (0x1DF406 * 3 as usize, 3268146222, Path::new(".").join("roms").join("test").join("ppu_sprite_hit").join("rom_singles").join("05-left_clip.nes")),
    sprite_zero_hit_06_right_edge: (0x1DF406 * 3 as usize, 2932966414, Path::new(".").join("roms").join("test").join("ppu_sprite_hit").join("rom_singles").join("06-right_edge.nes")),
    //sprite_zero_hit_07_screen_bottom: (0x1DF406 * 3 as usize, 3268146222, Path::new(".").join("roms").join("test").join("ppu_sprite_hit").join("rom_singles").join("07-screen_bottom.nes")),
    sprite_zero_hit_08_double_height: (0x1DF406 * 3 as usize, 3281055842, Path::new(".").join("roms").join("test").join("ppu_sprite_hit").join("rom_singles").join("08-double_height.nes")),
    //sprite_zero_hit_09_timing: (0x1DF406 * 3 as usize, 3268146222, Path::new(".").join("roms").join("test").join("ppu_sprite_hit").join("rom_singles").join("09-timing.nes")),
    //sprite_zero_hit_10_timing_order: (0x1DF406 * 3 as usize, 3268146222, Path::new(".").join("roms").join("test").join("ppu_sprite_hit").join("rom_singles").join("10-timing_order.nes")),

    // ----- Mapper Tests -----
    mapper_0_p32k_c8k_v: (0x56A32 * 3 as usize, 469175584, Path::new(".").join("roms").join("test").join("holy_mapperel").join("M0_P32K_C8K_V.nes")),
    mapper_0_p32k_cr8k_v: (0x270AAB * 3 as usize, 3621921473, Path::new(".").join("roms").join("test").join("holy_mapperel").join("M0_P32K_CR8K_V.nes")),
    mapper_0_p32k_cr32k_v: (0x270AAB * 3 as usize, 3621921473, Path::new(".").join("roms").join("test").join("holy_mapperel").join("M0_P32K_CR32K_V.nes")),
    // mapper_1_no_chrom: (0x56A32 * 3 as usize, 786314361, Path::new(".").join("roms").join("test").join("holy_mapperel").join("M1_P128K.nes")), - 0003 output (bad CHR somehow)
    mapper_1_p128k_c32k: (0x48189 * 3 as usize, 1806907890, Path::new(".").join("roms").join("test").join("holy_mapperel").join("M1_P128K_C32K.nes")),
    mapper_1_p128k_c32k_s8k: (0x48189 * 3 as usize, 1806907890, Path::new(".").join("roms").join("test").join("holy_mapperel").join("M1_P128K_C32K_S8K.nes")),
    mapper_1_p128k_c32k_w8k: (0x48189 * 3 as usize, 1806907890, Path::new(".").join("roms").join("test").join("holy_mapperel").join("M1_P128K_C32K_W8K.nes")),
    mapper_1_p128k_c128k: (0x48189 * 3 as usize, 2153594427, Path::new(".").join("roms").join("test").join("holy_mapperel").join("M1_P128K_C128K.nes")),
    mapper_1_p128k_c128k_s8k: (0x48189 * 3 as usize, 2153594427, Path::new(".").join("roms").join("test").join("holy_mapperel").join("M1_P128K_C128K_S8K.nes")),
    mapper_1_p128k_c128k_w8k: (0x48189 * 3 as usize, 2153594427, Path::new(".").join("roms").join("test").join("holy_mapperel").join("M1_P128K_C128K_W8K.nes")),
    mapper_2_p128k_cr8k_v: (0x253959 * 3 as usize, 1058817094, Path::new(".").join("roms").join("test").join("holy_mapperel").join("M2_P128K_CR8K_V.nes")),
    mapper_2_p128k_v: (0x24C505 * 3 as usize, 3178533875, Path::new(".").join("roms").join("test").join("holy_mapperel").join("M2_P128K_V.nes")),
    mapper_3: (0x90CD6 * 3 as usize, 3952353136, Path::new(".").join("roms").join("test").join("holy_mapperel").join("M3_P32K_C32K_H.nes")),
    // mapper_4_no_chrom: (0x90CD6 * 3 as usize, 3691845950, Path::new(".").join("roms").join("test").join("holy_mapperel").join("M4_P128K.nes")), - 0A13 output (all broken)
    // mapper_4_p256k_c256k: (0x90CD6 * 3 as usize, 3691845950, Path::new(".").join("roms").join("test").join("holy_mapperel").join("M4_P256K_C256K.nes")), - 0A10 output

    // ----- MMC3 IRQ Tests -----
    //mmc3_irq_clocking: (0x90CD6 * 3 as usize, 3691845950, Path::new(".").join("roms").join("test").join("mmc3_test").join("rom_singles").join("1-clocking.nes")), // Failed #3 - Doesn't handle PPUADDR causing changes
    //mmc3_irq_details: (0x90CD6 * 3 as usize, 3691845950, Path::new(".").join("roms").join("test").join("mmc3_test").join("rom_singles").join("2-details.nes")), // Failed #2 - Counter isn't working when reloaded with 255
    //mmc3_irq_a12_clocking: (0x90CD6 * 3 as usize, 3691845950, Path::new(".").join("roms").join("test").join("mmc3_test").join("rom_singles").join("3-A12_clocking.nes")), // Failed #4 - Failure due to PPUADDR changes again
    //mmc3_irq_scanline_timing: (0x90CD6 * 3 as usize, 3691845950, Path::new(".").join("roms").join("test").join("mmc3_test").join("rom_singles").join("4-scanline_timing.nes")), // Failed #14 - IRQ never occurred
    //mmc3_irq_mmc3: (0x90CD6 * 3 as usize, 3691845950, Path::new(".").join("roms").join("test").join("mmc3_test").join("rom_singles").join("5-MMC3.nes")), // Failed #2 - Should reload and set IRQ every clock when reload is 0
    //mmc3_irq_mmc3_alt: (0x90CD6 * 3 as usize, 3691845950, Path::new(".").join("roms").join("test").join("mmc3_test").join("rom_singles").join("6-MMC3_alt.nes")), // Failed #2 - Don't think I support the MMC3 alternate board
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
        .map(|char_line| char_line.into_iter().collect::<String>())
        .fold(String::new(), |a, b| a + "\n" + &b)
}

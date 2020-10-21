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
    cpu_timing_test: (0x11EB284 * 3 as usize, 377355712, Path::new(".").join("roms").join("test").join("cpu_timing_test6").join("cpu_timing_test.nes")),
    // instr_misc:  (0x11EB284 * 3 as usize, 377355712, Path::new(".").join("roms").join("test").join("instr_misc").join("instr_misc.nes")), - Failing due to unimplemented APU length counter (singles up to that pass)
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
    // ppu_read_buffer: (0x1C22B4 * 3 as usize, 3764449243, Path::new(".").join("roms").join("test").join("ppu_read_buffer").join("test_ppu_read_buffer.nes")), - Requires MMC3 support

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
    // ppu_vbl_nmi_02_vbl_set_time: (0x4FF06A * 3 as usize, 3764449243, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("02-vbl_set_time.nes")), - Failing
    ppu_vbl_nmi_03_clear_time: (0x4DAAC6 * 3 as usize, 2257284403, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("03-vbl_clear_time.nes")),
    // ppu_vbl_nmi_04_nmi_control: (0x4FF06A * 3 as usize, 3764449243, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("04-nmi_control.nes")), - Failed #5
    // ppu_vbl_nmi_05_nmi_timing: (0x4FF06A * 3 as usize, 3764449243, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("05-nmi_timing.nes")), - Failed
    // ppu_vbl_nmi_06_suppression: (0x4FF06A * 3 as usize, 3764449243, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("06-suppression.nes")), - Failed
    // ppu_vbl_nmi_07_nmi_on_timing: (0x4FF06A * 3 as usize, 3764449243, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("07-nmi_on_timing.nes")), - Failed
    // ppu_vbl_nmi_08_nmi_off_timing: (0x4FF06A * 3 as usize, 3764449243, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("08-nmi_off_timing.nes")), - Failed
    ppu_vbl_nmi_09_even_odd_frames: (0x43AB75 * 3 as usize, 817319831, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("09-even_odd_frames.nes")),
    // ppu_vbl_nmi_10_even_odd_timing: (0x4FF06A * 3 as usize, 3764449243, Path::new(".").join("roms").join("test").join("ppu_vbl_nmi").join("rom_singles").join("10-even_odd_timing.nes")), - Failed #3 Clock is skipped too late
    vbl_nmi_timing_frame_basics: (0x5CA9A1 * 3 as usize, 3792590752, Path::new(".").join("roms").join("test").join("vbl_nmi_timing").join("1.frame_basics.nes")),
    // vbl_nmi_timing_vbl_timing: (0x5CA9A1 * 3 as usize, 3792590752, Path::new(".").join("roms").join("test").join("vbl_nmi_timing").join("2.vbl_timing.nes")),- TODO - Failing on #8 no suppress
    // vbl_nmi_timing_even_odd_frames: (0x5CA9A1 * 3 as usize, 3792590752, Path::new(".").join("roms").join("test").join("vbl_nmi_timing").join("3.even_odd_frames.nes")), - TODO - Failing on #3
    vbl_nmi_timing_vbl_clear_timing: (0x3C6634 * 3 as usize, 1325590663, Path::new(".").join("roms").join("test").join("vbl_nmi_timing").join("4.vbl_clear_timing.nes")),
    // vbl_nmi_timing_nmi_suppression: (0x3C6634 * 3 as usize, 1325590663, Path::new(".").join("roms").join("test").join("vbl_nmi_timing").join("5.nmi_suppression.nes")), - TODO - Failing #3
    // vbl_nmi_timing_nmi_disable: (0x3C6634 * 3 as usize, 1325590663, Path::new(".").join("roms").join("test").join("vbl_nmi_timing").join("6.nmi_disable.nes")), - TODO
    // vbl_nmi_timing_nmi_timing: (0x3C6634 * 3 as usize, 1325590663, Path::new(".").join("roms").join("test").join("vbl_nmi_timing").join("7.nmi_timing.nes")), - TODO

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
        .map(|greyscale| lookup(greyscale))
        .collect::<Vec<char>>()
        .chunks(256)
        .map(|char_line| char_line.into_iter().collect::<String>())
        .fold(String::new(), |a, b| a + "\n" + &b)
}

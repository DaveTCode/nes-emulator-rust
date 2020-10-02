use apu::Apu;
use cartridge::CartridgeAddressBus;
use cartridge::CartridgeHeader;
use cpu::Cpu;
use io::Io;
use io::{Button, Controller};
use log::info;
use ppu::Ppu;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::fs::File;
use std::io::Write;
use std::{thread, time};

pub(crate) fn run(
    screen_width: u32,
    screen_height: u32,
    prg_address_bus: Box<dyn CartridgeAddressBus>,
    chr_address_bus: Box<dyn CartridgeAddressBus>,
    cartridge_header: CartridgeHeader,
) {
    let sdl = sdl2::init().unwrap();
    let video_subsystem = sdl.video().unwrap();
    let window = video_subsystem
        .window("NES Emulator", screen_width * 2, screen_height * 2)
        .build()
        .unwrap();

    let mut canvas = window
        .into_canvas()
        .build()
        .map_err(|e| e.to_string())
        .unwrap();
    let texture_creator = canvas.texture_creator();

    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::ARGB8888, screen_width, screen_height)
        .map_err(|e| e.to_string())
        .unwrap();

    let mut event_pump = sdl.event_pump().unwrap();

    let mut apu = Apu::new();
    let mut io = Io::new();
    let mut ppu = Ppu::new(chr_address_bus);
    let mut cpu = Cpu::new(prg_address_bus, &mut apu, &mut io, &mut ppu);
    let mut time_of_last_render = time::Instant::now();
    let frame_duration = time::Duration::from_millis(16);

    'main: loop {
        cpu.next();

        // Optionally re-render & poll for events this frame
        if cpu.is_frame_complete_cycle() {
            info!("Frame complete, polling for events and rendering");

            let framebuffer = cpu.get_framebuffer();
            texture
                .update(None, framebuffer, screen_width as usize * 4)
                .unwrap();
            // texture.with_lock(None, |buffer: &mut [u8], _: usize| {
            //     buffer.copy_from_slice(framebuffer);
            // }).unwrap();

            canvas.clear();
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();

            for event in event_pump.poll_iter() {
                info!("{:?}", event);
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => {
                        info!("Quitting emulation");
                        break 'main;
                    }
                    Event::KeyDown {
                        keycode: Some(keycode),
                        ..
                    } => match keycode {
                        Keycode::Z => cpu.button_down(Controller::One, Button::A),
                        Keycode::X => cpu.button_down(Controller::One, Button::B),
                        Keycode::Return => cpu.button_down(Controller::One, Button::Start),
                        Keycode::Tab => cpu.button_down(Controller::One, Button::Select),
                        Keycode::Left => cpu.button_down(Controller::One, Button::Left),
                        Keycode::Right => cpu.button_down(Controller::One, Button::Right),
                        Keycode::Up => cpu.button_down(Controller::One, Button::Up),
                        Keycode::Down => cpu.button_down(Controller::One, Button::Down),
                        Keycode::D => {
                            // Dump contents of PPU
                            let mut vram = [0; 0x4000];
                            let (oam_ram, palette_ram) = cpu.dump_ppu_state(&mut vram);
                            let mut vram_file = File::create("vram.csv").unwrap();
                            let mut oam_ram_file = File::create("oam_ram.csv").unwrap();
                            let mut palette_ram_file = File::create("palette_ram.csv").unwrap();

                            for i in 0..=0x3FFF {
                                write!(vram_file, "{:02X}\n", vram[i]);
                            }
                        }
                        _ => (),
                    },
                    Event::KeyUp {
                        keycode: Some(keycode),
                        ..
                    } => match keycode {
                        Keycode::Z => cpu.button_up(Controller::One, Button::A),
                        Keycode::X => cpu.button_up(Controller::One, Button::B),
                        Keycode::Return => cpu.button_up(Controller::One, Button::Start),
                        Keycode::Tab => cpu.button_up(Controller::One, Button::Select),
                        Keycode::Left => cpu.button_up(Controller::One, Button::Left),
                        Keycode::Right => cpu.button_up(Controller::One, Button::Right),
                        Keycode::Up => cpu.button_up(Controller::One, Button::Up),
                        Keycode::Down => cpu.button_up(Controller::One, Button::Down),
                        _ => (),
                    },
                    _ => (),
                };
            }

            // Wait so that we render at 60fps
            let current_time = time::Instant::now();
            let diff = current_time - time_of_last_render;
            time_of_last_render = current_time;
            if diff < frame_duration {
                info!("Sleeping {:?}", frame_duration - diff);
                thread::sleep(frame_duration - diff);
            }
        }
    }
}

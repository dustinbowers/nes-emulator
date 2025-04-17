mod bus;
mod cartridge;
mod controller;
mod cpu;
mod display;
mod nes;
mod ppu;
mod rom;

use crate::bus::nes_bus::NesBus;
use crate::nes::NES;
use cartridge::nrom::NromCart;
use cartridge::Cartridge;
use controller::joypad::JoypadButtons;
use cpu::processor::CPU;
use rom::Rom;
use rom::RomError;

use display::color_map::COLOR_MAP;
use display::consts::{PIXEL_HEIGHT, PIXEL_WIDTH};
use display::consts::{WINDOW_HEIGHT, WINDOW_WIDTH};
use display::draw_frame;
use display::frame::Frame;
use macroquad::prelude::*;
use std::{env, process};

fn window_conf() -> Conf {
    Conf {
        window_title: "NES".to_owned(),
        fullscreen: false,
        window_height: WINDOW_HEIGHT as i32,
        window_width: WINDOW_WIDTH as i32,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let args: Vec<String> = env::args().collect();

    // Ensure correct number of arguments
    if args.len() != 2 {
        eprintln!("Usage: {} <iNES 1.0 ROM path>", args[0]);
        process::exit(1);
    }
    let rom_path = &args[1];

    play_rom(rom_path).await;
    // render_sprite_banks(rom_path).await;
}

async fn play_rom(rom_path: &str) {
    let rom_data = std::fs::read(rom_path).expect("Error reading ROM file.");
    let rom = match Rom::new(&rom_data) {
        Ok(rom) => rom,
        Err(rom_error) => {
            println!("Error parsing rom: {:}", rom_error);
            return;
        }
    };

    println!("Making NES...");
    let cart = rom.into_cartridge();
    let mut nes = NES::new(cart);

    let stop_after_frames = 10;
    let mut frames = 0;
    loop {
        while !nes.tick() {}
        frames += 1;
        println!("== Frame: {}", frames);
        if frames == stop_after_frames {
            // break;
        }

        clear_background(RED);
        let frame = nes.get_frame_buffer();
        for (i, c) in frame.iter().enumerate() {
            let x = (i % 256) as f32;
            let y = (i / 256) as f32;
            if y == 0.0 {
                continue;
            } // TODO: fix this is nasty hack
            let color = COLOR_MAP.get_color((*c % 54) as usize);
            draw_rectangle(
                x * PIXEL_WIDTH,
                y * PIXEL_HEIGHT,
                PIXEL_WIDTH,
                PIXEL_HEIGHT,
                *color,
            );
        }

        println!("draw frame");
        nes.clear_frame();

        //
        // Handle user input
        //
        let key_map: &[(Vec<KeyCode>, JoypadButtons)] = &[
            (vec![KeyCode::K], JoypadButtons::BUTTON_A),
            (vec![KeyCode::J], JoypadButtons::BUTTON_B),
            (vec![KeyCode::Enter], JoypadButtons::START),
            (vec![KeyCode::RightShift], JoypadButtons::SELECT),
            (vec![KeyCode::W], JoypadButtons::UP),
            (vec![KeyCode::S], JoypadButtons::DOWN),
            (vec![KeyCode::A], JoypadButtons::LEFT),
            (vec![KeyCode::D], JoypadButtons::RIGHT),
        ];
        // Handle user input
        let keys_down = get_keys_down();
        for (keys, button) in key_map.iter() {
            let mut pressed = false;
            for key in keys.iter() {
                if keys_down.contains(&key) {
                    pressed = true;
                    break;
                }
            }
            nes.bus.controller1.set_button_status(button, pressed);
        }

        // Draw some info
        let status_str = format!(
            "PC: ${:04X} SP: ${:02X} A:${:02X} X:${:02X} Y:${:02X}",
            nes.bus.cpu.program_counter,
            nes.bus.cpu.stack_pointer,
            nes.bus.cpu.register_a,
            nes.bus.cpu.register_x,
            nes.bus.cpu.register_y
        );
        draw_text(&status_str, 5.0, 24.0, 24.0, Color::new(1.0, 1.0, 0.0, 1.0));

        let status_str = format!(
            "addr:{:04X} bus_cycles:{} ppu_cycles:{}",
            nes.bus.ppu.scroll_register.get_addr(),
            nes.bus.cycles,
            nes.bus.ppu.cycles
        );
        draw_text(&status_str, 5.0, 48.0, 24.0, Color::new(1.0, 1.0, 0.0, 1.0));

        //
        // DEBUG RENDERING
        //
        let mut debug_rendering = false;
        if keys_down.contains(&KeyCode::Key0) {
            debug_rendering = !debug_rendering;
        }
        if debug_rendering {
            let palette_table_px_size = 5;
            for (i, v) in nes.bus.ppu.palette_table.iter().enumerate() {
                let x = i % 32 * palette_table_px_size + 300;
                let y = i / 32 * palette_table_px_size + 32;
                draw_rectangle(
                    x as f32,
                    y as f32,
                    palette_table_px_size as f32,
                    palette_table_px_size as f32,
                    *COLOR_MAP.get_color((v % 53) as usize),
                );
            }

            let ram_px_size = 2;
            for (i, v) in nes.bus.ppu.ram.iter().enumerate() {
                let x = i % 32 * ram_px_size + 300;
                let y = i / 32 * ram_px_size + 60;
                draw_rectangle(
                    x as f32,
                    y as f32,
                    ram_px_size as f32,
                    ram_px_size as f32,
                    *COLOR_MAP.get_color((v % 53) as usize),
                );
            }

            let oam_px_size = 3;
            for (i, v) in nes.bus.ppu.oam_data.iter().enumerate() {
                let x = i % 32 * oam_px_size + 300;
                let y = i / 32 * oam_px_size + 300;
                draw_rectangle(
                    x as f32,
                    y as f32,
                    oam_px_size as f32,
                    oam_px_size as f32,
                    *COLOR_MAP.get_color(((v) % 53) as usize),
                );
            }
        }

        next_frame().await;
    }
}

#[allow(dead_code)]
async fn render_sprite_banks(rom_path: &str) {
    // Load and parse ROM
    let rom_data = std::fs::read(rom_path).expect("Error reading ROM file");
    let rom = Rom::new(&rom_data).unwrap();

    let mut f: Frame = Frame::new();
    for i in 0..256 {
        f.show_tile(&rom.chr_rom, 0, i);
    }
    for i in 0..256 {
        f.show_tile(&rom.chr_rom, 1, i);
    }

    loop {
        clear_background(BLACK);
        // Handle user input
        let keys_pressed = get_keys_down();
        if keys_pressed.contains(&KeyCode::Escape) {
            break;
        }
        draw_frame(&f);
        next_frame().await;
    }
}

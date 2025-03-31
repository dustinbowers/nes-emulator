mod bus;
mod cpu;
mod display;
mod memory;
mod ppu;
mod rom;

use display::render::render;
use std::{env, process};
use bus::BusMemory;
use display::consts::{PIXEL_HEIGHT, PIXEL_WIDTH, WINDOW_HEIGHT, WINDOW_WIDTH};
use cpu::processor::CPU;
use display::color_map::ColorMap;
use display::draw_frame;
use rom::Rom;
use bus::Bus;
use futures::executor;
use macroquad::prelude::*;
use std::ops::Rem;
use crate::display::frame::Frame;

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
    // run_snake_game().await;
}

async fn play_rom(rom_path: &str) {

    let rom_data = std::fs::read(rom_path).expect("Error reading ROM file.");
    let rom = Rom::new(&rom_data).expect("Error parsing ROM file.");
    let bus = Bus::new(rom);
    let mut cpu = CPU::new(bus);

    let mut frame = Frame::new();
    loop {
        // clear_background(LIGHTGRAY);
        for i in 0..29_830 {
            let (_, _, is_breaking) = cpu.tick();
            if is_breaking {
                break
            }
        }

        if cpu.bus.ready_to_render {
            render(&cpu.bus.ppu, &mut frame);
            cpu.bus.ready_to_render = false;
            draw_frame(&frame);
            // println!("draw frame!");
        }
        let status_str = format!(
            "PC: ${:04X} SP: ${:02X} A:${:02X} X:${:02X} Y:${:02X}",
            cpu.program_counter, cpu.stack_pointer, cpu.register_a, cpu.register_x, cpu.register_y
        );
        draw_text(&status_str, 5.0, 24.0, 18.0, Color::new(1.0, 0.0, 0.0, 1.0));
        let status_str = format!(
            "bus_cycles: {} ppu_cycles: {}",
            cpu.bus.cycles, cpu.bus.ppu.cycles
        );
        draw_text(&status_str, 5.0, 48.0, 18.0, Color::new(1.0, 0.0, 0.0, 1.0));
        next_frame().await;

    }
}

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
            break
        }
        draw_frame(&f);
        next_frame().await;
    }
}

async fn run_snake_game() {
    // Load and parse ROM
    let snake_rom = std::fs::read("roms/snake.nes").expect("error reading ROM file");
    let rom = Rom::new(&snake_rom).unwrap();

    // Create the Bus
    let mut bus = Bus::new(rom);

    // Create a CPU
    let mut cpu = CPU::new(bus);

    cpu.reset();

    let color_map = ColorMap::new();

    loop {
        let key_map: &[(Vec<KeyCode>, u8)] = &[
            (vec![KeyCode::W], 0x77),
            (vec![KeyCode::A], 0x61),
            (vec![KeyCode::S], 0x73),
            (vec![KeyCode::D], 0x64),
        ];

        // Handle user input
        let keys_pressed = get_keys_down();
        for (keys, v) in key_map.iter() {
            let mut pressed = false;
            // if keys_pressed.contains(&KeyCode::Space) {
            //     // reset
            //     bus = Bus::new();
            //     cpu = CPU::new(bus);
            //     cpu.load_program_at(program, 0x0600);
            //     cpu.program_counter = 0x0600;
            //     cpu.load_program_at(program, 0x0600);
            // }
            for k in keys.iter() {
                if keys_pressed.contains(k) {
                    pressed = true;
                }
            }
            if pressed {
                cpu.store_byte(0xFF, *v);
            }
        }

        cpu.store_byte(0xFE, rand::gen_range(1, 16));

        for i in 1..150 {
            cpu.tick();
        }

        clear_background(BLACK);
        let output = cpu.bus.fetch_bytes_raw(0x0200, 0x0400);
        for (i, c) in output.iter().enumerate() {
            let row = i / 32;
            let col = i % 32;
            let color = color_map.get_color(*c as usize);

            draw_rectangle(
                col as f32 * PIXEL_WIDTH,
                row as f32 * PIXEL_HEIGHT,
                PIXEL_WIDTH,
                PIXEL_HEIGHT,
                *color,
            );
        }
        let status_str = format!(
            "PC: ${:04X} SP: ${:02X} A:${:02X} X:${:02X} Y:${:02X}",
            cpu.program_counter, cpu.stack_pointer, cpu.register_a, cpu.register_x, cpu.register_y
        );
        draw_text(&status_str, 5.0, 24.0, 18.0, Color::new(1.0, 0.0, 0.0, 1.0));

        next_frame().await;
    }
}

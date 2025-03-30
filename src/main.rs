mod bus;
mod consts;
mod cpu;
mod display;
mod memory;

mod rom;
mod ppu;

use crate::bus::BusMemory;
use crate::consts::{PIXEL_HEIGHT, PIXEL_WIDTH, WINDOW_HEIGHT, WINDOW_WIDTH};
use crate::display::color_map::ColorMap;
use crate::display::draw_screen;
use crate::rom::Rom;
use bus::Bus;
use futures::executor;
use macroquad::prelude::*;
use std::ops::Rem;
use crate::cpu::processor::CPU;

fn window_conf() -> Conf {
    Conf {
        window_title: "NES".to_owned(),
        fullscreen: false,
        window_height: WINDOW_HEIGHT,
        window_width: WINDOW_WIDTH,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    run_snake_game().await;
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

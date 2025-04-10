mod bus;
mod controller;
mod cpu;
mod display;
mod memory;
mod ppu;
mod rom;

mod cartridge;

mod nes;

use crate::cartridge::nrom::NromCart;
use crate::cartridge::Cartridge;
use crate::controller::joypad::JoypadButtons;
use crate::display::color_map::COLOR_MAP;
use crate::display::frame::Frame;
use crate::nes::NES;
use bus::Bus;
use cpu::processor::CPU;
use display::consts::{WINDOW_HEIGHT, WINDOW_WIDTH};
use display::draw_frame;
use macroquad::prelude::*;
use rom::Rom;
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
        Err(msg) => {
            println!("Error parsing rom: {:?}", msg);
            return;
        }
    };

    println!("making NES...");
    let mut cart = rom.into_cartridge();
    println!("cart at 0x8000 = ${:02X}", cart.prg_read(0x8000));

    println!("cart ptr: {:?}", &cart as *const _);
    let mut nes = NES::new(cart);
    loop {
        println!("main loop tick...");
        nes.tick();
        next_frame().await;
    }
}

// async fn play_rom(rom_path: &str) {
//     let key_map: &[(Vec<KeyCode>, JoypadButtons)] = &[
//         (vec![KeyCode::K], JoypadButtons::BUTTON_A),
//         (vec![KeyCode::J], JoypadButtons::BUTTON_B),
//         (vec![KeyCode::Enter], JoypadButtons::START),
//         (vec![KeyCode::RightShift], JoypadButtons::SELECT),
//         (vec![KeyCode::W], JoypadButtons::UP),
//         (vec![KeyCode::S], JoypadButtons::DOWN),
//         (vec![KeyCode::A], JoypadButtons::LEFT),
//         (vec![KeyCode::D], JoypadButtons::RIGHT),
//     ];
//
//     let frame = Rc::new(RefCell::new(Frame::new()));
//     let rom_data = std::fs::read(rom_path).expect("Error reading ROM file.");
//     let rom = match Rom::new(&rom_data) {
//         Ok(rom) => rom,
//         Err(msg) => {
//             println!("Error parsing rom: {:?}", msg);
//             return;
//         }
//     };
//
//     let bus = Bus::new(rom.into(), |ppu, joypad| {
//         // render(ppu, Rc::clone(&frame));
//
//         // Handle user input
//         let keys_pressed = get_keys_down();
//         for (keys, button) in key_map.iter() {
//             let mut pressed = false;
//             for key in keys.iter() {
//                 if keys_pressed.contains(&key) {
//                     pressed = true;
//                     break;
//                 }
//             }
//             joypad.set_button_status(button, pressed);
//         }
//     });
//     let mut cpu = CPU::new(bus);
//
//     // if rom_path.contains("nestest.nes") {
//     //     cpu.program_counter = 0xC000;
//     // }
//
//     loop {
//         let mut break_loop = false;
//         loop {
//             let (_, _, is_breaking) = cpu.tick();
//             if is_breaking {
//                 break_loop = true;
//                 break;
//             }
//             if cpu.bus.cycles >= 29_830 {
//                 cpu.bus.cycles -= 29_830;
//                 break;
//             }
//             if cpu.bus.poll_frame_complete() {
//                 render(&cpu.bus.ppu, Rc::clone(&frame));
//             }
//         }
//         if break_loop {
//             break;
//         }
//         clear_background(RED);
//         draw_frame(&frame.borrow());
//
//         // Debug Overlays
//         draw_debug_overlays(&cpu);
//
//         // Draw some states
//         let status_str = format!(
//             "PC: ${:04X} SP: ${:02X} A:${:02X} X:${:02X} Y:${:02X}",
//             cpu.program_counter, cpu.stack_pointer, cpu.register_a, cpu.register_x, cpu.register_y
//         );
//         draw_text(&status_str, 5.0, 24.0, 24.0, Color::new(1.0, 1.0, 0.0, 1.0));
//         let status_str = format!(
//             "addr:{:04X} bus_cycles:{} ppu_cycles:{}",
//             cpu.bus.ppu.addr_register.get(),
//             cpu.bus.cycles,
//             cpu.bus.ppu.cycles
//         );
//         draw_text(&status_str, 5.0, 48.0, 24.0, Color::new(1.0, 1.0, 0.0, 1.0));
//         next_frame().await;
//     }
// }

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

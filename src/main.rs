mod bus;
mod cartridge;
mod controller;
mod cpu;
mod display;
mod nes;
mod ppu;
mod rom;

use crate::nes::NES;
use controller::joypad::JoypadButtons;
use rom::Rom;

use display::color_map::COLOR_MAP;
use display::consts::{PIXEL_HEIGHT, PIXEL_WIDTH};
use display::consts::{WINDOW_HEIGHT, WINDOW_WIDTH};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, TextureAccess, TextureCreator, WindowCanvas};
use sdl2::rwops::RWops;
use sdl2::ttf::{Font, Sdl2TtfContext};
use sdl2::video::{Window, WindowContext};
use sdl2::Sdl;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::{env, process};

const TARGET_FPS: u64 = 60;
const FRAME_DURATION: Duration = Duration::from_nanos(1_000_000_000 / TARGET_FPS);

// handle the annoying Rect i32
macro_rules! rect(
    ($x:expr, $y:expr, $w:expr, $h:expr) => (
        Rect::new($x as i32, $y as i32, $w as u32, $h as u32)
    )
);

fn init_sdl() -> (Sdl, Sdl2TtfContext, Window) {
    let sdl_context = sdl2::init().expect("SDL Init failed!");
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string()).unwrap();
    let video_subsystem = sdl_context
        .video()
        .expect("SDL Video Subsystem failed to init!");

    let window: Window = video_subsystem
        .window("NES", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()
        .expect("could not initialize video subsystem");

    (sdl_context, ttf_context, window)
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    // Ensure correct number of arguments
    if args.len() != 2 {
        eprintln!("Usage: {} <iNES 1.0 ROM path>", args[0]);
        process::exit(1);
    }
    let rom_path = &args[1];
    let rom_data = std::fs::read(rom_path).expect("Error reading ROM file.");
    let rom = match Rom::new(&rom_data) {
        Ok(rom) => rom,
        Err(rom_error) => {
            println!("Error parsing rom: {:}", rom_error);
            return Ok(());
        }
    };

    let (sdl_context, ttf_context, window) = init_sdl();

    let font_bytes: &[u8] = include_bytes!("../assets/JetBrainsMono-Bold.ttf");
    let font: &Font = &ttf_context.load_font_from_rwops(RWops::from_bytes(font_bytes)?, 16)?;
    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let texture_creator: TextureCreator<WindowContext> = canvas.texture_creator();

    // Create a streaming texture for pixel data
    let mut texture = texture_creator
        .create_texture(
            PixelFormatEnum::ARGB8888,
            TextureAccess::Streaming, // allows frequent updates
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
        )
        .unwrap();

    println!("Making NES...");
    let cart = rom.into_cartridge();
    let mut nes = NES::new(cart);

    let stop_after_frames = 10;
    let mut frames = 0;

    let key_map_data: &[(Vec<Keycode>, JoypadButtons)] = &[
        (vec![Keycode::K], JoypadButtons::BUTTON_A),
        (vec![Keycode::J], JoypadButtons::BUTTON_B),
        (vec![Keycode::RETURN], JoypadButtons::START),
        (vec![Keycode::RSHIFT], JoypadButtons::SELECT),
        (vec![Keycode::W], JoypadButtons::UP),
        (vec![Keycode::S], JoypadButtons::DOWN),
        (vec![Keycode::A], JoypadButtons::LEFT),
        (vec![Keycode::D], JoypadButtons::RIGHT),
    ];

    let mut keycode_to_joypad: HashMap<Keycode, JoypadButtons> = HashMap::new();
    for (keycodes, button) in key_map_data.iter() {
        for keycode in keycodes.iter() {
            keycode_to_joypad.insert(*keycode, *button);
        }
    }

    let mut event_pump = sdl_context.event_pump()?;
    let mut pixel_buffer: Vec<u8> = vec![0; (WINDOW_WIDTH * WINDOW_HEIGHT * 4) as usize]; // 4 bytes per pixel (A,R,G,B)

    let mut debug_rendering = false;

    canvas.set_draw_color(Color::RGB(0, 255, 0));
    canvas.clear();
    canvas.present();

    // Start Game Loop
    ////////////////////////////
    'running: loop {
        let frame_start_time = Instant::now();

        while !nes.tick() {}
        frames += 1;
        if frames == stop_after_frames {
            // break;
        }

        let frame = nes.get_frame_buffer();
        for (i, c) in frame.iter().enumerate() {
            let x = i % 256;
            let y = i / 256;
            // if y == 0 {
            //     continue;
            // } // TODO: fix this nasty hack

            let color = COLOR_MAP.get_color((*c) as usize);
            for py in 0..PIXEL_HEIGHT as usize {
                for px in 0..PIXEL_WIDTH as usize {
                    let mut ind = ((y * PIXEL_HEIGHT as usize) + py) * (WINDOW_WIDTH as usize) * 4;
                    ind += ((x * PIXEL_WIDTH as usize) + px) * 4;
                    pixel_buffer[ind + 0] = color.b; // This BGRA ordering is unexpected...
                    pixel_buffer[ind + 1] = color.g;
                    pixel_buffer[ind + 2] = color.r;
                    pixel_buffer[ind + 3] = 255;
                }
            }
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    break 'running;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::P),
                    ..
                } => {
                    debug_rendering = !debug_rendering;
                }
                Event::KeyDown { keycode, .. } | Event::KeyUp { keycode, .. } => {
                    let pressed = match event {
                        Event::KeyDown { .. } => true,
                        _ => false,
                    };

                    match keycode {
                        Some(kc) => {
                            if let Some(button) = keycode_to_joypad.get(&kc) {
                                nes.bus.controller1.set_button_status(button, pressed);
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        canvas.set_draw_color(Color::RGB(0, 255, 0));
        canvas.clear();

        texture
            .update(None, &pixel_buffer, (WINDOW_WIDTH * 4) as usize)
            .unwrap();
        canvas.copy(&texture, None, None).unwrap(); // copy texture to the entire canvas

        let elapsed_time = Instant::now().duration_since(frame_start_time);
        if elapsed_time < FRAME_DURATION {
            let sleep_duration = FRAME_DURATION - elapsed_time;
            std::thread::sleep(sleep_duration);
        }

        // nes.clear_frame();

        // Draw some info
        let status_str = format!(
            "PC: ${:04X} SP: ${:02X} A:${:02X} X:${:02X} Y:${:02X}",
            nes.bus.cpu.program_counter,
            nes.bus.cpu.stack_pointer,
            nes.bus.cpu.register_a,
            nes.bus.cpu.register_x,
            nes.bus.cpu.register_y
        );
        draw_text(&status_str, &mut canvas, &font, 5, 5 + 22 * 0);

        let status_str = format!(
            "addr:{:04X} bus_cycles:{} ppu_cycles:{}",
            nes.bus.ppu.scroll_register.get_addr(),
            nes.bus.cycles,
            nes.bus.ppu.cycles
        );
        draw_text(&status_str, &mut canvas, &font, 5, 5 + 22 * 1);

        // let ppu_stats = format!("sprite_count: {}", nes.bus.ppu.sprite_count);
        // draw_text(&ppu_stats, &mut canvas, &font, 5, 5 + 22 * 2);

        // let cpu_mode = format!(
        //     "cpu_mode: {:?}",
        //     nes.bus.cpu.cpu_mode
        // );
        // draw_text(&cpu_mode, &mut canvas, &font, 5, 5+22*3);

        //
        // DEBUG RENDERING
        //
        if debug_rendering {
            debug_render_data(&nes.bus.ppu.palette_table, &mut canvas, 300, 32, 32, 5);
            debug_render_data(&nes.bus.ppu.ram, &mut canvas, 300, 60, 32, 2);

            debug_render_data(&nes.bus.ppu.oam_data, &mut canvas, 450, 350, 8, 2);
            debug_render_data(&nes.bus.ppu.oam_data, &mut canvas, 10, 400, 32, 3);
            debug_render_data(&nes.bus.ppu.sprite_pattern_low, &mut canvas, 10, 405, 32, 3);
            debug_render_data(&nes.bus.ppu.sprite_pattern_low, &mut canvas, 10, 410, 32, 3);
            debug_render_data(&nes.bus.ppu.sprite_x_counter, &mut canvas, 10, 415, 32, 3);
            debug_render_data(&nes.bus.ppu.sprite_x_counter, &mut canvas, 10, 420, 32, 3);
        }

        canvas.present();
    }

    Ok(())
}

fn draw_text(text: &str, canvas: &mut WindowCanvas, font: &Font, x: i32, y: i32) {
    let text_color = Color::RGBA(255, 255, 0, 255); // White text
    let texture_creator: TextureCreator<WindowContext> = canvas.texture_creator();

    let main_text = text;
    let surface = font.render(main_text).blended(text_color).unwrap();

    let texture = texture_creator
        .create_texture_from_surface(&surface)
        .unwrap();

    // get the dimensions of the rendered text
    let (width, height) = surface.size();
    let target_rect = Rect::new(x, y, width, height);

    canvas.set_blend_mode(BlendMode::Blend);
    canvas.set_draw_color(Color::RGBA(0, 0, 0, 128));
    canvas.fill_rect(rect!(x, y, width, height)).unwrap();
    canvas.copy(&texture, None, Some(target_rect)).unwrap();
}

fn debug_render_data(
    data: &[u8],
    canvas: &mut WindowCanvas,
    x: usize,
    y: usize,
    width: usize,
    pixel_size: usize,
) {
    for (i, v) in data.iter().enumerate() {
        let x = i % width * pixel_size + x;
        let y = i / width * pixel_size + y;
        canvas.set_draw_color(*COLOR_MAP.get_color(((v) % 53) as usize));
        canvas
            .fill_rect(rect!(x, y, pixel_size, pixel_size))
            .unwrap();
    }
}

// #[allow(dead_code)]
// async fn render_sprite_banks(rom_path: &str) {
//     // Load and parse ROM
//     let rom_data = std::fs::read(rom_path).expect("Error reading ROM file");
//     let rom = Rom::new(&rom_data).unwrap();
//
//     let mut f: Frame = Frame::new();
//     for i in 0..256 {
//         f.show_tile(&rom.chr_rom, 0, i);
//     }
//     for i in 0..256 {
//         f.show_tile(&rom.chr_rom, 1, i);
//     }
//
//     loop {
//         clear_background(BLACK);
//         // Handle user input
//         let keys_pressed = get_keys_down();
//         if keys_pressed.contains(&KeyCode::Escape) {
//             break;
//         }
//         draw_frame(&f);
//         next_frame().await;
//     }
// }

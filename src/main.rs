mod display;
mod nes;

use nes::NES;
use sdl3::audio::{AudioCallback, AudioFormat, AudioSpec, AudioStream};

use crate::nes::cartridge::rom::Rom;
use crate::nes::controller::joypad::JoypadButtons;
use display::color_map::COLOR_MAP;
use display::consts::{PIXEL_HEIGHT, PIXEL_WIDTH};
use display::consts::{WINDOW_HEIGHT, WINDOW_WIDTH};
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::pixels::{Color, PixelFormat};
use sdl3::rect::Rect;
use sdl3::render::{BlendMode, FRect, TextureAccess, TextureCreator, WindowCanvas};
use sdl3::ttf::{Font, Sdl3TtfContext};
use sdl3::video::{Window, WindowContext};
use sdl3::Sdl;
use std::collections::HashMap;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::{Duration, Instant};
use std::{env, process};

#[cfg(feature = "tracing")]
use crate::nes::tracer::TRACER;

const TARGET_FPS: u64 = 60;
const FRAME_DURATION: Duration = Duration::from_nanos(1_000_000_000 / TARGET_FPS);

// handle the annoying Rect i32
macro_rules! rect(
    ($x:expr, $y:expr, $w:expr, $h:expr) => (
        Rect::new($x as i32, $y as i32, $w as u32, $h as u32)
    )
);

struct NesAudioCallback {
    nes_ptr: NonNull<NES>,
    cycle_acc: f32,
}

// SAFETY: NES is pinned on heap and only mutated from audio thread
//         This is necessary because AudioCallback requires Send but
//         Box<dyn Cartridge>, CpuBusInterface and PpuBusInterface are not.
unsafe impl Send for NesAudioCallback {}

impl AudioCallback<i16> for NesAudioCallback {
    // type Channel = i16;

    fn callback(&mut self, stream: &mut AudioStream, requested: i32) {
        // During shutdown, emit silence and avoid touching NES state
        if STOP_AUDIO.load(Ordering::Relaxed) {
            let silence = vec![0i16; requested as usize];
            let _ = stream.put_data_i16(&silence);
            return;
        }
        let nes: &mut NES = unsafe { self.nes_ptr.as_mut() };

        nes.bus
            .controller1
            .set_buttons(CONTROLLER1.buttons.load(Ordering::Relaxed));

        // PPU cycles per audio sample (5.369318 MHz / 44.1 kHz)
        let ppu_cycles_per_sample = 5369318.0 / 44100.0; // ~121.7 PPU cycles per sample
        let mut cycle_acc = self.cycle_acc;

        let mut out = Vec::<i16>::with_capacity(requested as usize);
        for _ in 0..requested {
            cycle_acc += ppu_cycles_per_sample;

            while cycle_acc >= 1.0 {
                nes.tick(); // tick at PPU frequency
                cycle_acc -= 1.0;
            }

            let raw = nes.bus.apu.sample();
            let scaled = (raw * 32767.0) as i16;
            out.push(scaled);
        }
        let _ = stream.put_data_i16(&out);

        self.cycle_acc = cycle_acc;
    }
}

struct SharedInput {
    buttons: AtomicU8, // bitmask of controller1 buttons
}
static CONTROLLER1: SharedInput = SharedInput {
    buttons: AtomicU8::new(0),
};

// Signal used to tell the audio callback to stop touching emulator state during teardown
static STOP_AUDIO: AtomicBool = AtomicBool::new(false);

fn init_sdl() -> (Sdl, Sdl3TtfContext, Window) {
    let sdl_context = sdl3::init().expect("SDL Init failed!");
    let ttf_context = sdl3::ttf::init().map_err(|e| e.to_string()).unwrap();
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
    let audio_subsystem = sdl_context
        .audio()
        .expect("SDL Audio Subsystem failed to init!");

    let font_bytes: &[u8] = include_bytes!("../assets/JetBrainsMono-Bold.ttf");
    let tmp_path = std::env::temp_dir().join("JetBrainsMono-Bold.ttf");
    if !tmp_path.exists() {
        std::fs::write(&tmp_path, font_bytes).map_err(|e| e.to_string())?;
    }
    let font: Font = ttf_context
        .load_font(&tmp_path, 16.0)
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas();
    let texture_creator: TextureCreator<WindowContext> = canvas.texture_creator();

    // Create a streaming texture for pixel data
    let mut texture = texture_creator
        .create_texture(
            PixelFormat::ARGB8888,
            TextureAccess::Streaming, // allows frequent updates
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
        )
        .unwrap();

    println!("Making NES...");
    let cart = rom.into_cartridge();

    // Pin NES so it can never move
    let mut new_nes = NES::new(cart);
    new_nes.set_sample_frequency(44_100);
    let mut nes_box: Pin<Box<NES>> = Box::pin(new_nes);
    let nes_ptr: NonNull<NES> = NonNull::from_mut(nes_box.as_mut().get_mut());

    let desired_audio_spec = AudioSpec {
        freq: Some(44_100),
        channels: Some(1), // mono
        format: Some(AudioFormat::s16_sys()),
    };
    let device = audio_subsystem
        .open_playback_stream(
            &desired_audio_spec,
            NesAudioCallback {
                nes_ptr,
                cycle_acc: 0.0,
            },
        )
        .unwrap();

    device.resume().expect("TODO: panic message");

    let key_map_data: &[(Vec<Keycode>, JoypadButtons)] = &[
        (vec![Keycode::K], JoypadButtons::BUTTON_A),
        (vec![Keycode::J], JoypadButtons::BUTTON_B),
        (vec![Keycode::Return], JoypadButtons::START),
        (vec![Keycode::RShift], JoypadButtons::SELECT),
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

    let mut event_pump = sdl_context.event_pump().map_err(|e| e.to_string())?;
    let mut pixel_buffer: Vec<u8> = vec![0; (WINDOW_WIDTH * WINDOW_HEIGHT * 4) as usize]; // 4 bytes per pixel (A,R,G,B)

    let mut debug_rendering = false;

    canvas.set_draw_color(Color::RGB(0, 255, 0));
    canvas.clear();
    canvas.present();

    // Start Game Loop
    ////////////////////////////
    'running: loop {
        let frame_start_time = Instant::now();

        let nes: &NES = unsafe { nes_ptr.as_ref() };
        let frame = nes.get_frame_buffer();
        for (i, c) in frame.iter().enumerate() {
            let x = i % 256;
            let y = i / 256;

            let color = COLOR_MAP.get_color((*c) as usize);
            for py in 0..PIXEL_HEIGHT as usize {
                for px in 0..PIXEL_WIDTH as usize {
                    let mut ind = ((y * PIXEL_HEIGHT as usize) + py) * (WINDOW_WIDTH as usize) * 4;
                    ind += ((x * PIXEL_WIDTH as usize) + px) * 4;
                    pixel_buffer[ind + 0] = color.b;
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
                    // Stop audio immediately to avoid races during teardown
                    STOP_AUDIO.store(true, Ordering::Relaxed);
                    let _ = device.pause();
                    // Give the audio thread a moment to observe the flag
                    std::thread::sleep(Duration::from_millis(10));
                    #[cfg(feature = "tracing")]
                    {
                        println!("==== DUMPING TRACE ====");
                        TRACER.lock().unwrap().print();
                    }
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
                                if pressed {
                                    CONTROLLER1
                                        .buttons
                                        .fetch_or((*button).bits(), Ordering::Relaxed);
                                } else {
                                    CONTROLLER1
                                        .buttons
                                        .fetch_and(!(*button).bits(), Ordering::Relaxed);
                                }
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
        // copy texture to the entire canvas
        canvas
            .copy(
                &texture,
                None::<FRect>,
                Rect::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT),
            )
            .unwrap();

        let elapsed_time = Instant::now().duration_since(frame_start_time);
        if elapsed_time < FRAME_DURATION {
            let sleep_duration = FRAME_DURATION - elapsed_time;
            std::thread::sleep(sleep_duration);
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
        draw_text(&status_str, &mut canvas, &font, 5, 5 + 22 * 0);

        let status_str = format!("addr:{:04X}", nes.bus.ppu.scroll_register.get_addr(),);
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
            debug_render_data(&nes.bus.ppu.v_ram, &mut canvas, 300, 60, 32, 2);

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
    canvas.copy(&texture, None, target_rect).unwrap();
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

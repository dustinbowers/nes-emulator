#[path = "../nes/mod.rs"]
mod nes;

use nes::NES;
use std::env;
use std::fs;
use std::process;

enum RunMode {
    Frames { frames: usize, buffer: usize },
    Ticks { ticks: usize },
}

struct Options {
    rom_path: String,
    run_mode: RunMode,
    result_addr: usize,
    verbose: bool,
}

fn parse_args() -> Options {
    let mut args = env::args().skip(1);
    let mut rom_path: Option<String> = None;
    let mut frames: Option<usize> = None;
    let mut ticks: Option<usize> = None;
    let mut buffer: usize = 0;
    let mut result_addr: usize = 0x00F8;
    let mut verbose = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-f" | "--frames" => {
                let val = args.next().unwrap_or_default();
                frames = Some(parse_usize(&val, "frames"));
            }
            "-t" | "--ticks" => {
                let val = args.next().unwrap_or_default();
                ticks = Some(parse_usize(&val, "ticks"));
            }
            "-b" | "--buffer" => {
                let val = args.next().unwrap_or_default();
                buffer = parse_usize(&val, "buffer");
            }
            "-r" | "--result-addr" => {
                let val = args.next().unwrap_or_default();
                result_addr = parse_usize(&val, "result-addr");
            }
            "-v" | "--verbose" => {
                verbose = true;
            }
            _ => {
                if rom_path.is_none() {
                    rom_path = Some(arg);
                } else {
                    eprintln!("Unexpected argument: {arg}");
                    print_usage_and_exit();
                }
            }
        }
    }

    let rom_path = rom_path.unwrap_or_else(|| {
        eprintln!("Missing ROM path.");
        print_usage_and_exit();
    });
    if frames.is_some() && ticks.is_some() {
        eprintln!("Provide either --frames or --ticks, not both.");
        print_usage_and_exit();
    }
    let run_mode = if let Some(ticks) = ticks {
        RunMode::Ticks { ticks }
    } else {
        let frames = frames.unwrap_or_else(|| {
            eprintln!("Missing required --frames (or use --ticks).");
            print_usage_and_exit();
        });
        RunMode::Frames { frames, buffer }
    };

    Options {
        rom_path,
        run_mode,
        result_addr,
        verbose,
    }
}

fn parse_usize(value: &str, name: &str) -> usize {
    if value.starts_with("0x") || value.starts_with("0X") {
        usize::from_str_radix(&value[2..], 16).unwrap_or_else(|_| {
            eprintln!("Invalid hex {name}: {value}");
            print_usage_and_exit();
        })
    } else {
        value.parse::<usize>().unwrap_or_else(|_| {
            eprintln!("Invalid {name}: {value}");
            print_usage_and_exit();
        })
    }
}

fn print_usage_and_exit() -> ! {
    eprintln!("Usage: rom-test-runner <rom_path> --frames <count> [options]");
    eprintln!("   or: rom-test-runner <rom_path> --ticks <count> [options]");
    eprintln!("Options:");
    eprintln!("  -f, --frames <count>        Number of frames to run (required)");
    eprintln!("  -t, --ticks <count>         Number of PPU ticks to run");
    eprintln!("  -b, --buffer <count>        Extra frames to add (default: 0)");
    eprintln!("  -r, --result-addr <addr>    Result RAM address (default: 0x00F8)");
    eprintln!("  -v, --verbose               Print extra diagnostics");
    process::exit(2);
}

fn main() {
    let opts = parse_args();
    let rom_data = fs::read(&opts.rom_path).unwrap_or_else(|err| {
        eprintln!("Failed to read ROM '{}': {err}", opts.rom_path);
        process::exit(2);
    });

    let cart = match NES::parse_rom_bytes(&rom_data) {
        Ok(cart) => cart,
        Err(err) => {
            eprintln!("ROM parse error: {err}");
            process::exit(2);
        }
    };

    let mut nes = NES::new();
    nes.insert_cartridge(cart);

    let mut frames = 0usize;
    let mut ticks = 0usize;

    match opts.run_mode {
        RunMode::Frames { frames: target, buffer } => {
            let target_frames = target + buffer;
            while frames < target_frames {
                if nes.tick() {
                    frames += 1;
                }
                ticks += 1;
            }
        }
        RunMode::Ticks { ticks: target } => {
            while ticks < target {
                nes.tick();
                ticks += 1;
            }
        }
    }

    let result = nes.bus.cpu_ram[opts.result_addr];
    if opts.verbose {
        println!("Ticks: {ticks}");
        println!("Frames: {frames}");
        println!("Result addr: 0x{:04X}", opts.result_addr);
        println!("Result byte: 0x{result:02X}");
    }

    if result == 1 {
        println!("PASS");
        process::exit(0);
    }

    if result >= 2 {
        println!("FAIL #{}", result);
        process::exit(1);
    }

    println!("UNKNOWN (result=0x{result:02X})");
    process::exit(2);
}

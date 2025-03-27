/*
   Test runner is meant to run nes6502 single-step opcode tests from
   https://github.com/SingleStepTests/65x02/tree/main/nes6502
*/

pub mod bus;
pub mod cpu;
pub mod memory;
pub mod opcodes;

use crate::bus::Bus;
use crate::cpu::{Flags, CPU};
use serde::Deserialize;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process;

#[derive(Debug, Deserialize)]
struct OpcodeTest {
    name: String,

    #[serde(rename = "initial")]
    initial_state: CPUState,

    #[serde(rename = "final")]
    final_state: CPUState,
    cycles: Vec<MemoryCycle>,
}

#[derive(Debug, Deserialize)]
struct CPUState {
    pc: u16,
    s: u8,
    a: u8,
    x: u8,
    y: u8,
    p: u8,
    ram: Vec<(u16, u8)>, // Deserialize JSON arrays as tuples
}

#[derive(Debug, Deserialize)]
struct MemoryCycle {
    #[serde(rename = "0")]
    address: u16,
    #[serde(rename = "1")]
    value: u8,
    #[serde(rename = "2")]
    operation: String, // "read" or "write" operations
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Ensure correct number of arguments
    if args.len() != 3 {
        eprintln!("Usage: {} <directory_path> <hex_opcode>", args[0]);
        process::exit(1);
    }

    println!(
        "Currently running at: {}",
        env::current_dir().unwrap().display()
    );

    // Get directory path and resolve to absolute path
    let dir_path = match fs::canonicalize(Path::new(&args[1])) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error: Could not resolve path '{}': {}", args[1], e);
            process::exit(1);
        }
    };

    // Parse the opcode argument as u8 hex value
    let opcode = match u8::from_str_radix(&args[2], 16) {
        Ok(value) => value,
        Err(_) => {
            eprintln!(
                "Error: Invalid opcode '{}'. Expected a valid hex value (e.g., 'ff').",
                args[2]
            );
            process::exit(1);
        }
    };

    println!("Running tests for opcode: {:02X}", opcode);
    // Read, parse, and run tests
    match read_opcode_tests(&dir_path, opcode) {
        Ok(tests) => {
            for (i, opcode_test) in tests.iter().enumerate() {
                println!("\n====== Running 0x{:02X} test #{} ====== ", opcode, i + 1);
                run_opcode_test(opcode_test);
                println!(" Pass!");
            }
        }
        Err(e) => eprintln!("Failed to load tests: {}", e),
    }
}

fn read_opcode_tests(
    dir_path: &PathBuf,
    opcode: u8,
) -> Result<Vec<OpcodeTest>, Box<dyn std::error::Error>> {
    let hex_string = format!("{:02x}", opcode); // Convert to lowercase hex
    let file_path = dir_path.join(format!("{}.json", hex_string)); // Use PathBuf.join()

    // Verify file existence
    if !file_path.exists() {
        return Err(format!("Error: File '{}' not found.", file_path.display()).into());
    }

    // Read file contents
    let mut file = fs::File::open(&file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // Parse JSON into a vector of OpcodeTest
    let tests: Vec<OpcodeTest> = serde_json::from_str(&contents)?;

    Ok(tests)
}

fn run_opcode_test(test: &OpcodeTest) {
    // Create CPU
    let bus = Bus::new();
    let mut cpu = CPU::new(bus);
    cpu.reset();

    // Set initial state of CPU and memory
    let start = &test.initial_state;
    cpu.program_counter = start.pc;
    cpu.stack_pointer = start.s;
    cpu.register_a = start.a;
    cpu.register_x = start.x;
    cpu.register_y = start.y;
    cpu.status = Flags::from_bits_truncate(start.p);
    println!("RAM data:");
    for (address, value) in start.ram.iter() {
        cpu.store_byte(*address, *value);
        println!("\t${:04X} = ${:02X} (0b{:08b})", *address, *value, *value);
    }

    // Single-step
    cpu.tick();

    // Confirm final state is correct
    let end = &test.final_state;
    let expected_cycles = test.cycles.len();
    assert_eq!(
        cpu.program_counter,
        end.pc,
        "{}",
        format!(
            "program_counter mismatch - Got: ${:02X} Want: ${:02X}",
            cpu.program_counter, end.pc
        )
    );
    assert_eq!(
        cpu.stack_pointer,
        end.s,
        "{}",
        format!(
            "stack_pointer mismatch - Got: ${:02X} Want: ${:02X}",
            cpu.stack_pointer, end.s
        )
    );
    assert_eq!(
        cpu.register_a,
        end.a,
        "{}",
        format!(
            "register_a mismatch - Got: ${:02X} Want: ${:02X}",
            cpu.register_a, end.a
        )
    );
    assert_eq!(
        cpu.register_x,
        end.x,
        "{}",
        format!(
            "register_x mismatch - Got: ${:02X} Want: ${:02X}",
            cpu.register_x, end.x
        )
    );
    assert_eq!(
        cpu.register_y,
        end.y,
        "{}",
        format!(
            "register_y mismatch - Got: ${:02X} Want: ${:02X}",
            cpu.register_y, end.y
        )
    );
    assert_eq!(
        cpu.status.bits(),
        end.p,
        "{}",
        format!(
            "status flag mismatch.\n\tGot:  {:08b}\n\tWant: {:08b}",
            cpu.status.bits(),
            end.p
        )
    );
    for (address, value) in end.ram.iter() {
        assert_eq!(cpu.fetch_byte(*address), *value);
    }
    assert_eq!(
        cpu.bus.cycles,
        expected_cycles,
        "{}",
        format!(
            "cycle count mismatch - Got: {} Want: {}",
            cpu.bus.cycles, expected_cycles
        )
    );
}

# NES Emulator

A Nintendo Entertainment System (NES) emulator written in Rust, supporting both native and WebAssembly targets.

> **Note**: This project is currently under active development. (There's still quite a few timing bugs to squash)

## Live Demo

A Live Demo of the current progress is available here (Note there's limited mapper support):  

https://dustinbowers.com/demos/nes-emulator

## Features

- 6502 CPU emulation with memory-cycle-accurate timing
- Dot-based microcoded PPU implementation
- Limited APU support
- Small collection of supported mappers (currently NROM, MMC1, UxROM, CNROM)
- Native desktop application
- WebAssembly browser version
- Test suite for CPU opcodes (single-step tests) and `nes-test-roms`

## Project Structure

This project uses Cargo workspaces:

- `nes-core` - Core emulation library
- `nes-app` - The emulator application (Uses [Eframe](https://github.com/emilk/egui/tree/main/crates/eframe) & [Cpal](https://github.com/RustAudio/cpal))
- `nes-native` - Native application build
- `nes-wasm` - WebAssembly browser build
- `nes-romtest` - Headless ROM testing utility
- `nes-step` - Single-step opcode testing tool

<img src="https://github.com/dustinbowers/nes-emulator/blob/main/imgs/workspace_hierarchy.png" width="60%">

## Quick Start

### Native Build
```bash
# Build and run
make run rom=path/to/rom.nes

# Or build separately
make release
./target/release/nes-native path/to/rom.nes
```

### WebAssembly Build
```bash
# Build for web
make wasm-release

# Serve locally
make wasm-serve
# Navigate to http://localhost:8080
```

## Development

### Building

| Target | Description | Command |
|--------|-------------|---------|
| Debug | Build with debug symbols | `make debug` |
| Release | Optimized release build | `make release` |
| Release + Tracing | Release build with tracing enabled | `make release-tracing` |
| WASM Debug | WebAssembly debug build | `make wasm-debug` |
| WASM Release | WebAssembly release build | `make wasm-release` |
| WASM Release | WebAssembly + Serve | `make wasm-serve` |

### Testing

| Command | Description |
|---------|-------------|
| `make singlestep-op op=A9` | Test a specific CPU opcode (hex value) |
| `make singlestep-all` | Run all CPU opcode tests (00-FF) |
| `make romtest rom=<path> frames=120` | Run headless ROM test for specified frames |
| `make romtest rom=<path> ticks=89342 buffer=30` | Run headless ROM test for specified CPU ticks |

### Utility Commands

| Command | Description |
|---------|-------------|
| `make help` | Display all available targets |
| `make clean-wasm-dist` | Clean WebAssembly distribution files |

## ROM Compatibility

Supports iNES 1.0 format ROMs with the following mappers:
- Mapper 0 (NROM)
- Mapper 1 (MMC1)
- Mapper 2 (UxROM)
- Mapper 3 (CNROM)

## Automated ROM Testing

The project includes a Python script for running batches of ROM tests defined in `nes-test-roms/test_roms.xml`.

### Basic Usage
```bash
# Run all tests
./scripts/run_romtests.py

# Run tests matching a filter
./scripts/run_romtests.py --filter "nestest"

# Run a specific test
./scripts/run_romtests.py --test "nes-test-roms/nestest.nes"

# Dry run (show commands without executing)
./scripts/run_romtests.py --dry-run

# Limit number of tests
./scripts/run_romtests.py --limit 5
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--xml` | `nes-test-roms/test_roms.xml` | Path to test configuration XML |
| `--rom-root` | `nes-test-roms` | Root directory for ROM paths |
| `--buffer` | `30` | Extra frames to add to test duration |
| `--frames` | `0` | Override frame count from XML (0 = use XML value) |
| `--filter` | `""` | Substring filter for ROM filenames |
| `--test` | `""` | Run single test by exact filename |
| `--limit` | `0` | Maximum number of tests to run (0 = unlimited) |
| `--dry-run` | `false` | Print commands without executing |

### Examples
```bash
# Run CPU tests only
./scripts/run_romtests.py --filter "cpu"

# Run first 10 tests with verbose output
./scripts/run_romtests.py --limit 10

# Override frame count for all tests
./scripts/run_romtests.py --frames 200 --buffer 50

# Check what would run without executing
./scripts/run_romtests.py --filter "ppu" --dry-run
```

### Test Configuration

Tests are defined in `nes-test-roms/test_roms.xml` with the following format:
```xml
<test filename="path/to/rom.nes" runframes="120"/>
```

The script automatically adds the buffer frames to each test's duration to account for initialization overhead.

## TODO

- ✅ Implement 6502 CPU
  - ✅ Official opcodes
  - ✅ Unofficial opcodes
  - ✅ Ensure memory-cycle accuracy
- ✅ Create 6502 CPU test runner for single-step tests
- ✅ Implement iNES 1.0 ROM parsing
- ✅ Implement Bus-centric architecture (NES wrapper handles orchestration)
- ✅ Implement interrupt handling
  - ✅ Non-maskable-interrupts
  - ✅ Software-defined interrupts
- ✅ Implement user-input via Joypad 1
- ✅ Implement PPU
  - ✅ Background nametable rendering
  - ✅ 8x8 sprite rendering (with horiz/vert flips)
  - ✅ 8x16 sprite rendering
  - ✅ Sprite collision detection
  - ✅ Detect sprite-overflow
- ✅ Encapsulate mapper logic behind Cartridge trait
- ✅ Implement cycle-accurate DMA transfer through Bus
- ⬜ Implement APU
  - ✅ Pulse Channels
  - ✅ Noise Channel
  - ✅ Triangle Channel
  - ⬜ DMC DMA
  - ⬜ DPCM Channel
- ⬜ Implement mappers
  - ✅ iNES 1.0 Mapper 000 - NROM
  - ✅ iNES 1.0 Mapper 001 - MMC1
  - ✅ iNES 1.0 Mapper 002 - UxROM
  - ✅ iNES 1.0 Mapper 003 - CNROM
  - ✅ iNES 1.0 Mapper 004 - MMC3
  - ⬜ iNES 1.0 Mapper 005 - MMC5
  - ⬜ ...plus more...
- ⬜ Fix CPU<->PPU timing/sync bugs 


## Resources

- 6502 opcode references:
  - https://www.nesdev.org/obelisk-6502-guide/reference.html
  - http://www.6502.org/tutorials/6502opcodes.html
- Unofficial opcodes - https://www.masswerk.at/nowgobang/2021/6502-illegal-opcodes
- Single-step opcode tests - https://github.com/SingleStepTests/65x02/tree/main/nes6502/v1
- iNES file format spec - https://formats.kaitai.io/ines/index.html
- PPU timing chart - https://www.nesdev.org/w/images/default/4/4f/Ppu.svg
- PPU timing details - https://www.nesdev.org/wiki/PPU_rendering
- APU details - https://www.nesdev.org/wiki/APU
- NES APU Sound Hardware Reference - https://www.nesdev.org/apu_ref.txt
- NES Test Roms - https://github.com/christopherpow/nes-test-roms

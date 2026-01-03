# NES Emulator

***NOTE**: This is still very much a work in progress.*

## TODO

- ✅ Implement 6502 CPU (minus APU)
  - ✅ Official opcodes
  - ✅ Unofficial opcodes
  - ✅ Ensure cycle accuracy
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
  - ⬜ iNES 1.0 Mapper 004 - MMC3
  - ⬜ iNES 1.0 Mapper 005 - MMC5
  - ⬜ ...plus more...


## Building

| Command                | Command Description                                |
|------------------------|----------------------------------------------------|
| `make debug`           | Build debug binary                                 |
| `make release`         | Build release binary                               |
| `make release-tracing` | Build release with tracing enabled (*much* slower) |
| `make wasm-debug`      | Build WASM debug to `dist/` (*much* slower)        |
| `make wasm-release`    | Build WASM release to `dist/`                      |

## Running

- Native usage: `./nes-emulator <iNES 1.0 ROM path>`
- For wasm usage: serve `dist/` locally with `python -m http.server 8080` or similar

## Testing

| Command                    | Command Description                                                 |
|----------------------------|---------------------------------------------------------------------|
| `make testop op=<opcode>`  | Test 6502 opcodes using single-step-tests (e.g. `make testop op=a9` |

## Resources

- 6502 opcode references:
  - https://www.nesdev.org/obelisk-6502-guide/reference.html
  - http://www.6502.org/tutorials/6502opcodes.html
- Illegal opcodes - https://www.masswerk.at/nowgobang/2021/6502-illegal-opcodes
- Single-step opcode tests - https://github.com/SingleStepTests/65x02/tree/main/nes6502/v1
- iNES file format spec - https://formats.kaitai.io/ines/index.html
- snake.nes - https://skilldrick.github.io/easy6502/#snake
- PPU timing chart - https://www.nesdev.org/w/images/default/4/4f/Ppu.svg
- PPU timing details - https://www.nesdev.org/wiki/PPU_rendering
- APU details - https://www.nesdev.org/wiki/APU
- NES APU Sound Hardware Reference - https://www.nesdev.org/apu_ref.txt

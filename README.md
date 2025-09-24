# NES Emulator

***NOTE**: This is still very much a work in progress.*

### TODO

- [x] Implement 6502 CPU (minus APU)
    - [x] Official opcodes
    - [x] Unofficial opcodes
    - [x] Ensure cycle accuracy
- [x] Create 6502 CPU test runner for single-step tests
- [x] Implement iNES 1.0 ROM parsing
- [x] Implement Bus-centric architecture (NES wrapper handles orchestration)
- [x] Implement interrupt handling
    - [x] Non-maskable-interrupts
    - [x] Software-defined interrupts
- [x] Implement user-input via Joypad 1
- [x] Implement PPU
    - [x] Background nametable rendering
    - [x] 8x8 sprite rendering (with horiz/vert flips)
    - [x] 8x16 sprite rendering
    - [x] Sprite collision detection
    - [ ] Detect sprite-overflow
- [x] Encapsulate mapper logic behind Cartridge trait
- [x] Implement cycle-accurate DMA transfer through Bus
- [ ] Implement APU

### Resources

- 6502 opcode references:
  - https://www.nesdev.org/obelisk-6502-guide/reference.html
  - http://www.6502.org/tutorials/6502opcodes.html
- Illegal opcodes: https://www.masswerk.at/nowgobang/2021/6502-illegal-opcodes
- Single-step opcode tests: https://github.com/SingleStepTests/65x02/tree/main/nes6502/v1
- iNES file format spec: https://formats.kaitai.io/ines/index.html
- snake.nes: https://skilldrick.github.io/easy6502/#snake
- PPU timing chart: https://www.nesdev.org/w/images/default/4/4f/Ppu.svg
- PPU timing details: https://www.nesdev.org/wiki/PPU_rendering

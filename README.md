# NES Emulator

***NOTE**: This is still very much a work in progress.*

### TODO

- [x] Implement 6502 CPU (minus APU)
    - [x] Official opcodes
    - [x] Unofficial opcodes
    - [x] Ensure cycle accuracy
- [x] Create 6502 CPU test runner for single-step tests
- [x] Implement iNES 1.0 ROM parsing
- [x] Implement interrupt handling
    - [x] Non-maskable-interrupts
    - [x] Software-defined interrupts
- [x] Implement user-input via Joypad 1
- [ ] Implement PPU
    - [x] Background nametable rendering
    - [x] 8x8 sprite rendering (with horiz/vert flips)
    - [ ] 8x16 sprite rendering
    - [ ] Detect sprite-overflow
    - [ ] Sprite collision detection
- [x] Encapsulate mapper logic behind Cartridge trait
- [ ] Implement APU

### Resources

- 6502 opcode references:
  - https://www.nesdev.org/obelisk-6502-guide/reference.html
  - http://www.6502.org/tutorials/6502opcodes.html
- Illegal opcodes: https://www.masswerk.at/nowgobang/2021/6502-illegal-opcodes
- Single-step opcode tests: https://github.com/SingleStepTests/65x02/tree/main/nes6502/v1
- iNES file format spec: https://formats.kaitai.io/ines/index.html
- snake.nes: https://skilldrick.github.io/easy6502/#snake

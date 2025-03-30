# NES Emulator

***NOTE**: This is still very much a work in progress.*

### TODO

- [x] Implement 6502 CPU (minus APU)
- [x] Create 6502 CPU test runner for single-step tests
- [x] Implement iNES 1.0 ROM parsing
- [ ] Implement PPU
- [ ] Implement interrupt handling
- [ ] Implement APU

### Resources

- 6502 opcode references:
  - https://www.nesdev.org/obelisk-6502-guide/reference.html
  - http://www.6502.org/tutorials/6502opcodes.html
- Illegal opcodes: https://www.masswerk.at/nowgobang/2021/6502-illegal-opcodes
- Single-step opcode tests: https://github.com/SingleStepTests/65x02/tree/main/nes6502/v1
- iNES file format spec: https://formats.kaitai.io/ines/index.html
- snake.nes: https://skilldrick.github.io/easy6502/#snake

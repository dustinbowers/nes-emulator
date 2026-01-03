.PHONY: all debug release release-tracing clean copy-assets wasm-debug wasm-release singlestep-op logs singlestep-all romtest help

all: release

help:
	@echo "Targets:"
	@echo "  debug            Build debug"
	@echo "  release          Build release"
	@echo "  release-tracing  Build release with tracing"
	@echo "  wasm-debug       Build wasm debug"
	@echo "  wasm-release     Build wasm release"
	@echo "  singlestep-op    Run single-step opcode test (op=XX)"
	@echo "  singlestep-all   Run all single-step opcode tests"
	@echo "  romtest          Run headless ROM test (rom=..., frames=... or ticks=...)"
	@echo "  clean            Clean dist outputs"

# Build targets
debug:
	cargo build

release:
	cargo build --release --verbose

release-tracing:
	cargo build --release --features "tracing"

# Clean distribution directory
clean:
	rm -rf ./dist/*.html ./dist/*.js ./dist/nes-test-roms

# WebAssembly build targets
wasm-debug: clean
	./wasm-bindgen-macroquad.sh nes-emulator
	$(MAKE) copy-assets

wasm-release: clean
	./wasm-bindgen-macroquad.sh nes-emulator --release
	$(MAKE) copy-assets

# Copy assets to distribution
copy-assets:
	mkdir -p ./dist
	cp -r ./assets/nes-test-roms ./dist/
	cp -f  ./assets/index.html ./dist/
	cp -f  ./assets/emulator.html ./dist/
	cp -f  ./assets/macroquad.js ./dist/

# Create logs directory
logs:
	mkdir -p logs

# Test specific opcode again single-step tests
singlestep-op: # Usage: make singlestep-op op=a9
	@echo "Running tests for operation: $(op)"
	cargo run --quiet --package nes-emulator --bin single-step-runner --features single-step-runner -- "nes6502-tests/" "$(op)"

# Run all opcode tests ranging from 00 to FF
singlestep-all: | logs
	@for i in $$(seq 0 255); do \
		hex=$$(printf "%02X" $$i); \
		echo "Running test with op=0x$$hex"; \
		make singlestep-op op=$$hex > logs/$$hex.log 2>&1; \
	done

# Headless ROM test runner
romtest: # Usage: make romtest rom=path/to/test.nes frames=120 buffer=30 (or ticks=89342)
	@echo "Running ROM test: $(rom)"
	@if [ -z "$(rom)" ]; then echo "Missing rom=..."; exit 2; fi
	@if [ -z "$(ticks)" ] && [ -z "$(frames)" ]; then echo "Missing frames=... or ticks=..."; exit 2; fi
	@if [ ! -x target/debug/rom_test_runner ]; then \
		echo "Building rom_test_runner (first run can take a while)..."; \
		RUSTFLAGS="-Awarnings" cargo build --quiet --bin rom_test_runner; \
	fi
	./target/debug/rom_test_runner "$(rom)" $(if $(ticks),--ticks "$(ticks)",--frames "$(frames)") --buffer "$(if $(buffer),$(buffer),0)"

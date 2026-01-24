.PHONY: all debug release release-tracing clean-wasm-dist copy-assets wasm-debug wasm-release singlestep-op logs singlestep-all romtest help run

all: release

help:
	@echo "Targets:"
	@echo "  debug            Build debug"
	@echo "  release          Build release"
	@echo "  release-tracing  Build release with tracing"
	@echo "  run              Run release with a specified (rom=path/to/rom.nes)"
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
clean-wasm-dist:
	mkdir -p ./dist
	rm -rf ./dist/*.html ./dist/*.js ./dist/*.wasm ./dist/nes-test-roms

# WebAssembly build targets
wasm-debug: clean-wasm-dist
	cd crates/nes-wasm; wasm-pack build --target web --out-dir --out-name nes-emulator dist
	$(MAKE) copy-assets

wasm-release: clean-wasm-dist
	cd crates/nes-wasm; wasm-pack build --target web --out-dir dist --out-name nes-emulator --release
	$(MAKE) copy-assets

# Copy assets to distribution
copy-assets:
	cd crates/nes-wasm; mkdir -p ./dist
	cd crates/nes-wasm; cp index.html ./dist/

# WebAssembly local testing
wasm-serve: clean-wasm-dist | wasm-release
	python3 -m http.server 8080 --directory crates/nes-wasm/dist

# Create logs directory
logs:
	mkdir -p logs

# Test specific opcode again single-step tests
singlestep-op: # Usage: make singlestep-op op=a9
	@echo "Running tests for operation: $(op)"
	cargo run --quiet --package nes-step -- "external/nes6502-single-step-tests/v1/" "$(op)"

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
		echo "Building rom_test_runner..."; \
		RUSTFLAGS="-Awarnings" cargo build --package nes-romtest --quiet --release; \
	fi
	./target/release/nes-romtest "$(rom)" $(if $(ticks),--ticks "$(ticks)",--frames "$(frames)") --buffer "$(if $(buffer),$(buffer),0)"

# Run the emulator with a specified ROM
run: # Usage: make rom=path/to/rom.nes
	cargo run --package nes-native --release -- $(rom)
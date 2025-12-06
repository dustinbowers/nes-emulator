.PHONY: all debug release release-tracing clean copy-assets wasm-debug wasm-release testop logs testall

all: release

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

# Test specific operation
testop: # Usage: make testop op=a9
	@echo "Running tests for operation: $(op)"
	cargo run --quiet --package nes-emulator --bin test-runner --features test-runner -- "nes6502-tests/" "$(op)"
	
# Create logs directory
logs:
	mkdir -p logs

# Run all tests ranging from 00 to FF
testall: | logs
	@for i in $$(seq 0 255); do \
		hex=$$(printf "%02X" $$i); \
		echo "Running test with op=0x$$hex"; \
		make testop op=$$hex > logs/$$hex.log 2>&1; \
	done


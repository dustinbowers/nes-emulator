.PHONY: debug release release-tracing clean wasm-debug wasm-release testop logs testall

all: release

debug:
	cargo build

release:
	cargo build --release --verbose

release-tracing:
	cargo build --release --features "tracing"

clean:
	rm ./dist/*.html ./dist/*.js
	
wasm-debug: clean
	mkdir -p ./dist
	cp -f ./assets/index.html ./dist/
	cp -f ./assets/macroquad.js ./dist/
	./wasm-bindgen-macroquad.sh nes-emulator --release
	
wasm-release: clean
	mkdir -p ./dist
	cp -f ./assets/index.html ./dist/
	cp -f ./assets/macroquad.js ./dist/
	./wasm-bindgen-macroquad.sh nes-emulator --release

testop: # run with: make test op=a9
	# NOTE: The single-step tests expected can be found here: 
	#	   https://github.com/SingleStepTests/65x02/tree/main/nes6502/v1
	#	   The JSON files need to be moved to ./nes6502-tests/
	cargo build --package nes-emulator --bin test-runner
	cargo run --quiet --package nes-emulator --bin test-runner -- "nes6502-tests/" "$(op)"

logs:
	mkdir -p logs

testall: | logs  # run all tests from 00 to FF
	@for i in $$(seq 0 255); do \
		hex=$$(printf "%02X" $$i); \
		echo "Running test with op=0x$$hex"; \
		make testop op=$$hex > logs/$$hex.log 2>&1; \
	done


.PHONY: testop testall logs debug release

testop: # run with $make test op=a9
	cargo build --package nes-emulator --bin test-runner
	cargo run --quiet --package nes-emulator --bin test-runner -- "nes6502-tests/" "$(op)"

testall: | logs  # run all tests from 00 to FF
	@for i in $$(seq 0 255); do \
		hex=$$(printf "%02X" $$i); \
		echo "Running test with op=0x$$hex"; \
		make testop op=$$hex > logs/$$hex.log 2>&1; \
	done

logs:
	mkdir -p logs

debug:
	cargo build

release:
	cargo build --release

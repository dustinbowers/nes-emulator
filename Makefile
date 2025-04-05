.PHONY: testop testall

testop: # run with $make test op=a9
	cargo run --package nes-emulator --bin test-runner  -- "nes6502-tests/" "$(op)"

testall: # run all tests from 00 to FF
	@for i in $$(seq 1 255); do \
		printf -v hex "%02X" $$i; \
		echo "Running test with op=0x$$hex"; \
		make testop op=$$hex; \
	done

debug:
	cargo build

release:
	cargo build --release

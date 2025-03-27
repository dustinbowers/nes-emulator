.PHONY: testop

testop: # run with $make test op=a9
	cargo run --package nes-emulator --bin test-runner  -- "nes6502-tests/" "$(op)"

.PHONY: testop testall

testop: # run with $make test op=a9
	cargo run --package nes-emulator --bin test-runner  -- "nes6502-tests/" "$(op)"

testall: # run all tests from 00 to FF
	@for i in $$(seq 1 255); do \
		printf -v hex "%02X" $$i; \
		echo "Running test with op=0x$$hex"; \
		make testop op=$$hex; \
	done

testalluntilfail: # run all tests from 01 to FF, break on error
	@for i in $(seq 1 255); do \
		printf -v hex "%02X" $i; \
		echo "Running test with op=0x$hex"; \
		if ! make testop op=$hex; then \
			echo "Test with op=0x$hex failed. Stopping tests."; \
			exit 1; \
		fi; \
	done

.PHONY: all build build-rust test clean

all: build-rust test

build-rust:
	cd libnoir_ffi && cargo build --release

test: build-rust
	# Compile Noir circuit
	cd circuit && nargo compile
	# Run Go tests (static linking means no LD_LIBRARY_PATH needed)
	go test -v .

clean:
	cd libnoir_ffi && cargo clean
	rm -rf circuit/target

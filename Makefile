.PHONY: all build build-rust build-rust-native test clean dist download-lib

# Default version for downloads
VERSION ?= latest
ARCH := $(shell uname -m)
OS := $(shell uname -s | tr '[:upper:]' '[:lower:]')

all: build-rust test

build-rust:
	cd libnoir_ffi && cargo build --release

build-rust-native:
	cd libnoir_ffi && cargo build --release --features native-backend

test:
	# Compile Noir circuit
	cd testdata/circuit && nargo compile
	# Run Go tests
	CGO_LDFLAGS="-L$(PWD)/libnoir_ffi/target/release" go test -v .

clean:
	cd libnoir_ffi && cargo clean
	rm -rf testdata/circuit/target dist/

# Prepare artifacts for GitHub Release
dist: build-rust
	mkdir -p dist
	cp libnoir_ffi/target/release/libbarretenberg_ffi.a dist/libbarretenberg_ffi-$(OS)-$(ARCH).a
	cp libnoir_ffi/barretenberg_ffi.h dist/
	tar -czvf dist/go-barretenberg-lib-$(OS)-$(ARCH).tar.gz -C dist libbarretenberg_ffi-$(OS)-$(ARCH).a barretenberg_ffi.h

# Download precompiled static library from GitHub
download-lib:
	@mkdir -p libnoir_ffi/target/release
	@echo "Downloading precompiled library ($(VERSION)) for $(OS)-$(ARCH)..."
	@if [ "$(VERSION)" = "latest" ]; then \
		URL="https://github.com/p4u/go-barretenberg/releases/latest/download/libbarretenberg_ffi-$(OS)-$(ARCH).a"; \
	else \
		URL="https://github.com/p4u/go-barretenberg/releases/download/$(VERSION)/libbarretenberg_ffi-$(OS)-$(ARCH).a"; \
	fi; \
	curl -L $$URL -o libnoir_ffi/target/release/libbarretenberg_ffi.a
	@echo "Download complete. You can now run 'go build' or 'go test'."

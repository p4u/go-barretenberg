# Go-Noir

Go bindings for Noir (v1.0.0-beta.19) using the official Aztec Barretenberg backend.

This project allows Go developers to generate and verify ZK proofs (UltraHonk) for Noir circuits. It uses a **statically linked** Rust FFI bridge, making the resulting Go binaries highly portable.

## 1. Prerequisites

- **Go**: `1.21+`
- **Rust**: `1.85.1+` (to build the FFI bridge)
- **Noir/Nargo**: `v1.0.0-beta.19`

## 2. Install the `bb` Binary

The library requires the `bb` (Barretenberg) binary at runtime. 

### Option A: Official Installer (Recommended)
```bash
curl -L https://raw.githubusercontent.com/AztecProtocol/aztec-up/main/install | bash
bbup -v 3.0.0-nightly.20260102
```

### Option B: From Source
```bash
git clone https://github.com/AztecProtocol/aztec-packages
cd aztec-packages/barretenberg/cpp
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
make bb -j$(nproc)
sudo cp bin/bb /usr/local/bin/
```

## 3. Setup and Build

```bash
git clone https://github.com/vocdoni/go-noir
cd go-noir
make build-rust
```

## 4. API Reference

### Types

#### `ProofSystemSettings`
Configures the UltraHonk proving system.
```go
type ProofSystemSettings struct {
	IpaAccumulation           bool   `json:"ipa_accumulation"`           // true for recursive/rollup proofs
	OracleHashType            string `json:"oracle_hash_type"`            // "poseidon2" (default), "keccak", "blake2s"
	DisableZk                 bool   `json:"disable_zk"`                 // true for faster, non-private proofs
	OptimizedSolidityVerifier bool   `json:"optimized_solidity_verifier"` // true for gas-optimized EVM verification
}
```

### Functions

#### `DefaultSettings() ProofSystemSettings`
Returns the standard settings for UltraHonk (Poseidon2, ZK enabled).

#### `ProveUltraHonk(bytecode string, witnessJson string, settings ProofSystemSettings) ([]byte, error)`
Generates an UltraHonk proof.
- `bytecode`: Base64-encoded gzipped ACIR bytecode.
- `witnessJson`: JSON string containing the witness (e.g., `{"witness": ["0x01", "0x02"]}`).

#### `ProveUltraHonkPoseidon(bytecode string, witnessJson string) ([]byte, error)`
Convenience wrapper for `ProveUltraHonk` using `DefaultSettings()`.

#### `GetVkUltraHonk(bytecode string, settings ProofSystemSettings) ([]byte, error)`
Returns the Verification Key (VK) for the circuit and settings.

#### `VerifyUltraHonk(proof []byte, vk []byte, settings ProofSystemSettings) bool`
Verifies an UltraHonk proof using the provided VK and settings.

#### `InitSRS(bytecode string) error`
Initializes the Structured Reference String. *Note: Usually handled automatically by the backend.*

## 5. Usage Example

```go
package main

import (
	"fmt"
	"github.com/vocdoni/go-noir"
)

func main() {
	// 1. Prepare inputs
	bytecode := "H4sIAAAAAAAA..." 
	witnessJson := `{"witness": ["0x03", "0x09"]}`

	// 2. Configure Settings (or use DefaultSettings())
	settings := noir.DefaultSettings()
	settings.OracleHashType = "keccak" // Example: use Keccak for EVM compatibility

	// 3. Generate Proof
	proof, err := noir.ProveUltraHonk(bytecode, witnessJson, settings)
	if err != nil {
		panic(err)
	}

	// 4. Get VK
	vk, err := noir.GetVkUltraHonk(bytecode, settings)
	if err != nil {
		panic(err)
	}

	// 5. Verify
	if noir.VerifyUltraHonk(proof, vk, settings) {
		fmt.Println("Verification Successful!")
	}
}
```

## 6. Project Integration

1.  **Add Module**: `go get github.com/vocdoni/go-noir`
2.  **Linking**: Ensure `libnoir_ffi.a` is built. Set `CGO_LDFLAGS="-L/path/to/libnoir_ffi/target/release"` when building your app.
3.  **Runtime**: Ensure `bb` is in your `$PATH` or set `BB_BINARY_PATH`.

## Architecture

Proving logic is delegated to a persistent `bb` worker process via high-performance msgpack pipes. This ensures memory isolation (C++ crashes won't kill your Go app) and simplifies distribution through static linking of the FFI bridge.

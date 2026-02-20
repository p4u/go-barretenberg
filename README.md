# go-barretenberg

Go bindings for Barretenberg (Noir's backend) using official Aztec libraries.

This project allows Go developers to generate and verify ZK proofs (UltraHonk) for Noir circuits with **maximum performance** using a statically linked Native backend.

## 1. Prerequisites

- **Go** `1.25+`
- **Rust**
- **Noir/Nargo**

## 2. Quick Start (Native Mode)

In Native mode, the proving logic is compiled into your Go binary. **No external binaries (like `bb`) are required at runtime.**

### Step 1: Install the Go Module
```bash
go get github.com/p4u/go-barretenberg
```

### Step 2: Download the Precompiled Static Library
Download the library for your architecture (Linux x86_64) into your project directory:
```bash
curl -L https://github.com/p4u/go-barretenberg/releases/latest/download/libbarretenberg_ffi-linux-x86_64.a -o libbarretenberg_ffi.a
```

### Step 3: Build your project
Link against the downloaded library:
```bash
export CGO_LDFLAGS="-L$(pwd)"
go build .
```

---

## 2. API Usage

By default, the library uses the **Native** backend for best performance.

```go
package main

import (
	"fmt"
	"github.com/p4u/go-barretenberg"
)

func main() {
	// 1. Prepare your circuit bytecode (gzipped base64) and witness JSON
	bytecode := "H4sIAAAAAAA..." 
	witnessJson := `{"witness": ["0x03", "0x09"]}`

	// 2. Configure Proof System Settings
	settings := barretenberg.DefaultSettings()
	settings.OracleHashType = barretenberg.HashKeccak // Example: use Keccak for EVM compatibility
    settings.DisableZk = false

	// 3. Generate Proof
	proof, err := barretenberg.ProveUltraHonk(bytecode, witnessJson, settings)
	if err != nil {
		panic(err)
	}

	// 4. Get Verification Key
	vk, _ := barretenberg.GetVkUltraHonk(bytecode, settings)

	// 5. Verify
	if barretenberg.VerifyUltraHonk(proof, vk, settings) {
		fmt.Println("Proof is valid!")
	}
}
```

---

## 3. Proof System Settings

The `ProofSystemSettings` struct allows you to configure every aspect of the UltraHonk proving system.

| Field | Type | Description |
| :--- | :--- | :--- |
| `IpaAccumulation` | `bool` | Set to `true` for recursive/rollup-compatible proofs. This uses the IPA accumulation scheme. |
| `OracleHashType` | `OracleHashType` | The hash function used by the prover's oracle. Use the predefined constants: `HashPoseidon2`, `HashKeccak`, or `HashBlake2s`. |
| `DisableZk` | `bool` | If `true`, Zero-Knowledge is disabled. Proving is faster and uses less memory, but the proof reveals the witness. |
| `OptimizedSolidityVerifier`| `bool` | If `true`, the verification key and proof are optimized for deployment on the EVM. |

### Oracle Hash Constants
- `barretenberg.HashPoseidon2` (Default)
- `barretenberg.HashKeccak` (EVM compatible)
- `barretenberg.HashBlake2s`

---

## 4. Alternative: Pipe Mode (Binary Worker)

If you prefer to use the `bb` binary as a separate worker process (for memory isolation), install `bb` via `bbup` and switch modes:

```go
import "github.com/p4u/go-barretenberg"

func init() {
    barretenberg.SetBackendType(barretenberg.BackendPipe)
}
```

## 5. Building from Source (Advanced)

If you need to build the Rust bridge yourself (requires Rust 1.85.1+):

```bash
git clone https://github.com/p4u/go-barretenberg
cd go-barretenberg
make build-rust-native # For Native support
```

## Architecture

This library bridges Go to Aztec's `barretenberg-rs`. 
- **Native Backend**: Links the Barretenberg C++ engine directly into your Go app via a static Rust shim. Highest speed, lowest latency.
- **Pipe Backend**: Spawns a `bb` subprocess. Best for stability if you are worried about C++ memory usage affecting your main Go process.

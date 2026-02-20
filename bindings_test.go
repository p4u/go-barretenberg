package noir

import (
	"encoding/base64"
	"encoding/json"
	"os"
	"testing"
)

func TestProveVerify(t *testing.T) {
	// Read bytecode from circuit/target/circuit.json
	data, err := os.ReadFile("circuit/target/circuit.json")
	if err != nil {
		t.Fatalf("failed to read circuit.json: %v", err)
	}

	var circuit struct {
		Bytecode string `json:"bytecode"`
	}
	if err := json.Unmarshal(data, &circuit); err != nil {
		t.Fatalf("failed to unmarshal circuit.json: %v", err)
	}

	witness := struct {
		Witness []string `json:"witness"`
	}{
		Witness: []string{
			"0x0000000000000000000000000000000000000000000000000000000000000003",
			"0x0000000000000000000000000000000000000000000000000000000000000009",
		},
	}
	witnessJSON, _ := json.Marshal(witness)

	settings := DefaultSettings()

	// 1. Prove
	proof, err := ProveUltraHonk(circuit.Bytecode, string(witnessJSON), settings)
	if err != nil {
		t.Fatalf("failed to prove: %v", err)
	}
	t.Logf("Proof length: %d", len(proof))

	// 2. Get VK
	vk, err := GetVkUltraHonk(circuit.Bytecode, settings)
	if err != nil {
		t.Fatalf("failed to get VK: %v", err)
	}
	t.Logf("VK length: %d", len(vk))

	// 3. Verify
	success := VerifyUltraHonk(proof, vk, settings)
	if !success {
		t.Fatalf("Verification failed")
	}
	t.Logf("Verification success!")
}

func TestBase64(t *testing.T) {
	s := "H4sIAAAAAAAA/4XMPQ5AMBCF4atMvYVIs9No9S6Gv0SjUInG7S080Ssq3reYDxSlRE9t"
	_, err := base64.StdEncoding.DecodeString(s)
	if err != nil {
		t.Fatal(err)
	}
}

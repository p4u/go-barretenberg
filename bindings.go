package noir

/*
#cgo LDFLAGS: ${SRCDIR}/libnoir_ffi/target/release/libnoir_ffi.a -lm -ldl -lpthread
#include <stdlib.h>
#include "libnoir_ffi/noir_ffi.h"
*/
import "C"
import (
	"encoding/json"
	"errors"
	"unsafe"
)

// ProofSystemSettings defines the settings for the UltraHonk proof system.
type ProofSystemSettings struct {
	IpaAccumulation           bool   `json:"ipa_accumulation"`
	OracleHashType            string `json:"oracle_hash_type"`
	DisableZk                 bool   `json:"disable_zk"`
	OptimizedSolidityVerifier bool   `json:"optimized_solidity_verifier"`
}

// DefaultSettings returns the default settings for UltraHonk (Poseidon2).
func DefaultSettings() ProofSystemSettings {
	return ProofSystemSettings{
		IpaAccumulation:           false,
		OracleHashType:            "poseidon2",
		DisableZk:                 false,
		OptimizedSolidityVerifier: false,
	}
}

// Result is a helper to convert C.BBResult to Go types
func resultToBytes(r C.BBResult) ([]byte, error) {
	if !bool(r.ok) {
		if r.err == nil {
			return nil, errors.New("unknown error from backend")
		}
		msg := C.GoString(r.err)
		C.bb_free_err(r.err)
		return nil, errors.New(msg)
	}
	defer C.bb_free_bytes(r.data)
	if r.data.ptr == nil || r.data.len == 0 {
		return nil, nil
	}
	return C.GoBytes(unsafe.Pointer(r.data.ptr), C.int(r.data.len)), nil
}

// InitSRS initializes the SRS from the bytecode
func InitSRS(bytecode string) error {
	cBytecode := C.CString(bytecode)
	defer C.free(unsafe.Pointer(cBytecode))

	r := C.bb_init_srs_from_bytecode(cBytecode)
	_, err := resultToBytes(r)
	return err
}

// ProveUltraHonkPoseidon is a convenience wrapper for ProveUltraHonk with default settings.
func ProveUltraHonkPoseidon(bytecode string, witnessJson string) ([]byte, error) {
	return ProveUltraHonk(bytecode, witnessJson, DefaultSettings())
}

// ProveUltraHonk generates an UltraHonk proof for the given bytecode, witness JSON, and settings.
func ProveUltraHonk(bytecode string, witnessJson string, settings ProofSystemSettings) ([]byte, error) {
	cBytecode := C.CString(bytecode)
	defer C.free(unsafe.Pointer(cBytecode))

	cWJSON := C.CString(witnessJson)
	defer C.free(unsafe.Pointer(cWJSON))

	settingsData, err := json.Marshal(settings)
	if err != nil {
		return nil, err
	}
	cSettings := C.CString(string(settingsData))
	defer C.free(unsafe.Pointer(cSettings))

	r := C.bb_prove_ultrahonk(cBytecode, cWJSON, cSettings)
	return resultToBytes(r)
}

// GetVkUltraHonk returns the verification key for the given bytecode and settings.
func GetVkUltraHonk(bytecode string, settings ProofSystemSettings) ([]byte, error) {
	cBytecode := C.CString(bytecode)
	defer C.free(unsafe.Pointer(cBytecode))

	settingsData, err := json.Marshal(settings)
	if err != nil {
		return nil, err
	}
	cSettings := C.CString(string(settingsData))
	defer C.free(unsafe.Pointer(cSettings))

	r := C.bb_get_vk_ultrahonk(cBytecode, cSettings)
	return resultToBytes(r)
}

// VerifyUltraHonk verifies a proof using the verification key and settings.
func VerifyUltraHonk(proof []byte, vk []byte, settings ProofSystemSettings) bool {
	if len(proof) == 0 || len(vk) == 0 {
		return false
	}

	settingsData, err := json.Marshal(settings)
	if err != nil {
		return false
	}
	cSettings := C.CString(string(settingsData))
	defer C.free(unsafe.Pointer(cSettings))
	
	return bool(C.bb_verify_ultrahonk(
		(*C.uint8_t)(unsafe.Pointer(&proof[0])),
		C.uintptr_t(len(proof)),
		(*C.uint8_t)(unsafe.Pointer(&vk[0])),
		C.uintptr_t(len(vk)),
		cSettings,
	))
}

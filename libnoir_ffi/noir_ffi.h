#ifndef NOIR_FFI_H
#define NOIR_FFI_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

typedef struct {
    uint8_t *ptr;
    size_t len;
    size_t cap;
} ByteBuffer;

typedef struct {
    bool ok;
    char *err;
    ByteBuffer data;
} BBResult;

void bb_free_bytes(ByteBuffer buf);
void bb_free_err(char *s);

BBResult bb_init_srs_from_bytecode(const char *bytecode_b64_gz);

BBResult bb_prove_ultrahonk(
    const char *bytecode_b64_gz,
    const char *witness_json,
    const char *settings_json
);

BBResult bb_get_vk_ultrahonk(
    const char *bytecode_b64_gz,
    const char *settings_json
);

bool bb_verify_ultrahonk(
    const uint8_t *proof_msgpack_ptr,
    size_t proof_msgpack_len,
    const uint8_t *vk_ptr,
    size_t vk_len,
    const char *settings_json
);

#endif /* NOIR_FFI_H */

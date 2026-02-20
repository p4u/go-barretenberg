use std::{ffi::{CStr, CString}, os::raw::c_char, ptr::null_mut};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use barretenberg_rs::{BarretenbergApi, backends::PipeBackend};
use barretenberg_rs::generated_types::{CircuitInput, CircuitInputNoVK, ProofSystemSettings, CircuitProveResponse, Command};
use base64::{Engine as _, engine::general_purpose};
use std::io::Read;
use flate2::read::GzDecoder;
use std::collections::BTreeMap;

static BB_API: OnceCell<std::sync::Mutex<BarretenbergApi<PipeBackend>>> = OnceCell::new();

fn get_api() -> Result<std::sync::MutexGuard<'static, BarretenbergApi<PipeBackend>>, String> {
    let api_mutex = BB_API.get_or_init(|| {
        let bb_path = std::env::var("BB_BINARY_PATH").unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            format!("{}/.bb/bb", home)
        });
        
        let backend = match PipeBackend::new(&bb_path, Some(16)) {
            Ok(b) => b,
            Err(e) => {
                // We return a dummy Mutex if initialization fails, 
                // and we'll handle the error during locking if needed.
                // However, PipeBackend::new currently returns Result.
                // Since OnceCell init cannot easily return Result, 
                // we'll let it panic if absolutely critical, or handle it better.
                panic!("Failed to initialize Barretenberg backend at {}: {}", bb_path, e);
            }
        };
        std::sync::Mutex::new(BarretenbergApi::new(backend))
    });
    
    api_mutex.lock().map_err(|e| format!("Mutex lock failed: {}", e))
}

#[repr(C)]
pub struct ByteBuffer {
    pub ptr: *mut u8,
    pub len: usize,
    pub cap: usize,
}

#[repr(C)]
pub struct BBResult {
    pub ok: bool,
    pub err: *mut c_char,
    pub data: ByteBuffer,
}

fn ok(mut data: Vec<u8>) -> BBResult {
    let len = data.len();
    let cap = data.capacity();
    let ptr = data.as_mut_ptr();
    std::mem::forget(data);
    BBResult {
        ok: true,
        err: null_mut(),
        data: ByteBuffer { ptr, len, cap },
    }
}

fn err(msg: String) -> BBResult {
    let c = CString::new(msg).unwrap_or_else(|_| CString::new("Unknown error").unwrap());
    BBResult {
        ok: false,
        err: c.into_raw(),
        data: ByteBuffer {
            ptr: null_mut(),
            len: 0,
            cap: 0,
        },
    }
}

#[no_mangle]
pub extern "C" fn bb_free_bytes(buf: ByteBuffer) {
    if !buf.ptr.is_null() {
        unsafe {
            drop(Vec::from_raw_parts(buf.ptr, buf.len, buf.cap));
        }
    }
}

#[no_mangle]
pub extern "C" fn bb_free_err(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

unsafe fn cstr_to_string(p: *const c_char) -> Result<String, String> {
    if p.is_null() {
        return Err("null pointer".into());
    }
    CStr::from_ptr(p)
        .to_str()
        .map(|s| s.to_owned())
        .map_err(|e| e.to_string())
}

fn decode_bytecode(bytecode_b64_gz: &str) -> Result<Vec<u8>, String> {
    let compressed = general_purpose::STANDARD
        .decode(bytecode_b64_gz)
        .map_err(|e| e.to_string())?;
    let mut decoder = GzDecoder::new(&compressed[..]);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).map_err(|e| e.to_string())?;
    Ok(decompressed)
}

#[no_mangle]
pub extern "C" fn bb_init_srs_from_bytecode(_bytecode_b64_gz: *const c_char) -> BBResult {
    ok(vec![])
}

#[derive(Deserialize)]
struct WitnessJson {
    witness: Vec<String>,
}

fn parse_field(s: &str) -> Result<[u8; 32], String> {
    let bytes = if s.starts_with("0x") {
        let hex_str = &s[2..];
        let mut decoded = vec![0u8; 32];
        let h = hex::decode(hex_str).map_err(|e| e.to_string())?;
        if h.len() > 32 {
            return Err("Hex string too long for field element".into());
        }
        let offset = 32 - h.len();
        decoded[offset..].copy_from_slice(&h);
        decoded
    } else {
        let val = s.parse::<u128>().map_err(|e| e.to_string())?;
        let mut decoded = [0u8; 32];
        let b = val.to_be_bytes();
        decoded[32-16..].copy_from_slice(&b);
        decoded.to_vec()
    };
    
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

#[derive(Serialize)]
struct WitnessMapWrapper(BTreeMap<u32, serde_bytes::ByteBuf>);

#[derive(Serialize)]
struct StackItemWrapper(u32, WitnessMapWrapper);

fn call_bb(cmd: Command) -> Result<barretenberg_rs::generated_types::Response, String> {
    let mut api = get_api()?;
    
    match cmd {
        Command::CircuitComputeVk(data) => {
            api.circuit_compute_vk(data.circuit, data.settings)
                .map(barretenberg_rs::generated_types::Response::CircuitComputeVkResponse)
                .map_err(|e| e.to_string())
        }
        Command::CircuitProve(data) => {
            api.circuit_prove(data.circuit, &data.witness, data.settings)
                .map(barretenberg_rs::generated_types::Response::CircuitProveResponse)
                .map_err(|e| e.to_string())
        }
        Command::CircuitVerify(data) => {
            api.circuit_verify(&data.verification_key, data.public_inputs, data.proof, data.settings)
                .map(barretenberg_rs::generated_types::Response::CircuitVerifyResponse)
                .map_err(|e| e.to_string())
        }
        _ => Err("Unsupported command".to_string())
    }
}

#[no_mangle]
pub extern "C" fn bb_prove_ultrahonk(
    bytecode_b64_gz: *const c_char,
    witness_json: *const c_char,
    settings_json: *const c_char,
) -> BBResult {
    let res: Result<Vec<u8>, String> = (|| {
        let bytecode_str = unsafe { cstr_to_string(bytecode_b64_gz) }?;
        let bytecode = decode_bytecode(&bytecode_str)?;
        
        let wj_str = unsafe { cstr_to_string(witness_json) }?;
        let parsed: WitnessJson = serde_json::from_str(&wj_str).map_err(|e| e.to_string())?;

        let settings_str = unsafe { cstr_to_string(settings_json) }?;
        let settings: ProofSystemSettings = serde_json::from_str(&settings_str).map_err(|e| e.to_string())?;

        let mut witness_map = BTreeMap::new();
        for (i, val_str) in parsed.witness.into_iter().enumerate() {
            let field_bytes = parse_field(&val_str)?;
            witness_map.insert(i as u32, serde_bytes::ByteBuf::from(field_bytes.to_vec()));
        }

        let stack_item = StackItemWrapper(0, WitnessMapWrapper(witness_map));
        
        #[derive(Serialize)]
        struct FinalWitnessStack {
            stack: Vec<StackItemWrapper>,
        }
        let final_stack = FinalWitnessStack { stack: vec![stack_item] };

        let encoded = rmp_serde::to_vec(&final_stack)
            .map_err(|e| format!("Failed to serialize witness stack: {}", e))?;
        let mut witness_bytes = vec![2u8]; 
        witness_bytes.extend(encoded);

        let circuit_input_no_vk = CircuitInputNoVK {
            name: "circuit".to_string(),
            bytecode: bytecode.clone(), 
        };

        let vk_resp = match call_bb(Command::CircuitComputeVk(barretenberg_rs::generated_types::CircuitComputeVk::new(circuit_input_no_vk, settings.clone())))? {
            barretenberg_rs::generated_types::Response::CircuitComputeVkResponse(r) => r,
            _ => return Err("Unexpected response".to_string()),
        };

        let circuit_input = CircuitInput {
            name: "circuit".to_string(),
            bytecode,
            verification_key: vk_resp.bytes,
        };

        let prove_resp = match call_bb(Command::CircuitProve(barretenberg_rs::generated_types::CircuitProve::new(circuit_input, witness_bytes, settings)))? {
            barretenberg_rs::generated_types::Response::CircuitProveResponse(r) => r,
            _ => return Err("Unexpected response".to_string()),
        };

        let resp_bytes = rmp_serde::to_vec_named(&prove_resp)
            .map_err(|e| format!("Failed to serialize response: {}", e))?;
        
        Ok(resp_bytes)
    })();

    match res {
        Ok(p) => ok(p),
        Err(e) => err(e),
    }
}

#[no_mangle]
pub extern "C" fn bb_get_vk_ultrahonk(
    bytecode_b64_gz: *const c_char,
    settings_json: *const c_char,
) -> BBResult {
    let res = (|| {
        let bytecode_str = unsafe { cstr_to_string(bytecode_b64_gz) }?;
        let bytecode = decode_bytecode(&bytecode_str)?;
        
        let settings_str = unsafe { cstr_to_string(settings_json) }?;
        let settings: ProofSystemSettings = serde_json::from_str(&settings_str).map_err(|e| e.to_string())?;

        let circuit_input = CircuitInputNoVK {
            name: "circuit".to_string(),
            bytecode,
        };

        let vk_resp = match call_bb(Command::CircuitComputeVk(barretenberg_rs::generated_types::CircuitComputeVk::new(circuit_input, settings)))? {
            barretenberg_rs::generated_types::Response::CircuitComputeVkResponse(r) => r,
            _ => return Err("Unexpected response".to_string()),
        };
            
        Ok(vk_resp.bytes)
    })();

    match res {
        Ok(v) => ok(v),
        Err(e) => err(e),
    }
}

#[no_mangle]
pub extern "C" fn bb_verify_ultrahonk(
    proof_msgpack_ptr: *const u8,
    proof_msgpack_len: usize,
    vk_ptr: *const u8,
    vk_len: usize,
    settings_json: *const c_char,
) -> bool {
    let res: Result<bool, String> = (|| {
        if proof_msgpack_ptr.is_null() || vk_ptr.is_null() {
            return Err("null pointer".into());
        }
        let proof_msgpack = unsafe { std::slice::from_raw_parts(proof_msgpack_ptr, proof_msgpack_len) };
        let vk_bytes = unsafe { std::slice::from_raw_parts(vk_ptr, vk_len) }.to_vec();
        
        let settings_str = unsafe { cstr_to_string(settings_json) }?;
        let settings: ProofSystemSettings = serde_json::from_str(&settings_str).map_err(|e| e.to_string())?;

        let prove_resp: CircuitProveResponse = rmp_serde::from_slice(proof_msgpack)
            .map_err(|e| format!("Failed to deserialize proof response: {}", e))?;

        let mut api = get_api()?;
        
        let verified = api.circuit_verify(&vk_bytes, prove_resp.public_inputs, prove_resp.proof, settings)
            .map_err(|e| format!("Failed to verify: {}", e))?;
            
        Ok(verified.verified)
    })();

    res.unwrap_or(false)
}

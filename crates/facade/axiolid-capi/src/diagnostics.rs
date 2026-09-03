use super::*;

/// Copy structured metadata for the context's most recent error.
/// # Safety
/// Every non-null pointer must be aligned and valid for the documented read or write extent.
#[no_mangle]
pub unsafe extern "C" fn axiolid_v0_4_context_last_error_info(
    context: AxiolidContextHandle,
    out_info: *mut AxiolidErrorInfo,
) -> AxiolidStatus {
    boundary(|| {
        if out_info.is_null() {
            return AxiolidStatus::NullPointer;
        }
        let Some(context) = context_entry(context.0) else {
            return AxiolidStatus::InvalidHandle;
        };
        let context = lock_unpoisoned(&context);
        let Some(error) = &context.last_error else {
            return AxiolidStatus::NoError;
        };
        let mut info = AxiolidErrorInfo {
            status: error.status,
            operation: error.operation,
            tolerance: error.tolerance,
            provider: [0; 64],
            message_len: error.message.len(),
            provider_len: 0,
            reserved: [0; 7],
        };
        info.provider_len = copy_text(&error.provider, &mut info.provider);
        // SAFETY: null was rejected; caller provides one writable error record.
        unsafe { out_info.write(info) };
        AxiolidStatus::Ok
    })
}

/// Copy the last error message into a caller-owned UTF-8 buffer.
///
/// `out_required` receives the byte count including the trailing NUL. Passing a
/// null buffer with zero capacity is the supported sizing query.
/// # Safety
/// Every non-null pointer must be aligned and valid for the documented read or write extent.
#[no_mangle]
pub unsafe extern "C" fn axiolid_v0_4_context_last_error_message(
    context: AxiolidContextHandle,
    buffer: *mut u8,
    capacity: usize,
    out_required: *mut usize,
) -> AxiolidStatus {
    boundary(|| {
        if out_required.is_null() || (capacity != 0 && buffer.is_null()) {
            return AxiolidStatus::NullPointer;
        }
        let Some(context) = context_entry(context.0) else {
            return AxiolidStatus::InvalidHandle;
        };
        let context = lock_unpoisoned(&context);
        let Some(error) = &context.last_error else {
            return AxiolidStatus::NoError;
        };
        let required = error.message.len().saturating_add(1);
        // SAFETY: null was rejected; caller provides one writable size value.
        unsafe { out_required.write(required) };
        if capacity < required {
            return AxiolidStatus::BufferTooSmall;
        }
        // SAFETY: capacity was checked; caller promises a writable buffer of that size.
        let destination = unsafe { std::slice::from_raw_parts_mut(buffer, required) };
        destination[..error.message.len()].copy_from_slice(error.message.as_bytes());
        destination[required - 1] = 0;
        AxiolidStatus::Ok
    })
}

/// Query live child-object counts for leak checks and shutdown assertions.
/// # Safety
/// Every non-null pointer must be aligned and valid for the documented read or write extent.
#[no_mangle]
pub unsafe extern "C" fn axiolid_v0_4_context_live_object_counts(
    context: AxiolidContextHandle,
    out_meshes: *mut usize,
    out_results: *mut usize,
) -> AxiolidStatus {
    boundary(|| {
        if out_meshes.is_null() || out_results.is_null() {
            return AxiolidStatus::NullPointer;
        }
        let Some(context) = context_entry(context.0) else {
            return AxiolidStatus::InvalidHandle;
        };
        let context = lock_unpoisoned(&context);
        // SAFETY: null was rejected; caller provides two writable count values.
        unsafe {
            out_meshes.write(context.meshes.live_count());
            out_results.write(context.results.live_count());
        }
        AxiolidStatus::Ok
    })
}

// Fragments of UCKeyTranslate

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    fn UCKeyTranslate(
        key_layout_ptr: *mut std::ffi::c_void,
        virtual_key_code: u16,
        key_action: u16,
        modifier_key_state: u32,
        keyboard_type: u32,
        key_translate_options: u32,
        dead_key_state: *mut u32,
        // These should be c_ulong
        max_string_length: u32,
        actual_string_length: *mut u32,
        unicode_string: *mut u16,
    ) -> i32;
}

let mut dead_key_state = 0;
let mut unicode_str = [0u16; 256];
let mut actual_string_length = 0;
let code = UCKeyTranslate(
    std::ptr::null_mut(),
    0,
    0,
    0,
    0,
    0,
    &mut dead_key_state,
    256,
    &mut actual_string_length,
    unicode_str[..].as_mut_ptr(),
);
println!("code = {}", code);

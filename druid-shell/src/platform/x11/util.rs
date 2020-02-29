use xcb::{Connection, Window};

// See: https://github.com/rtbo/rust-xcb/blob/master/examples/randr_screen_modes.rs
pub fn refresh_rate(conn: &xcb::Connection, window_id: Window) -> Option<f64> {
    let cookie = xcb::randr::get_screen_resources(conn, window_id);
    let reply = cookie.get_reply().unwrap();
    let mut modes = reply.modes();

    // TODO: Assuming the first mode is the one we want to use. This is probably a bug on some
    //       setups. Any better way to find the correct one?
    let mut refresh_rate = modes.nth(0).and_then(|mode_info| {
        let flags = mode_info.mode_flags();
        let vtotal = {
            let mut val = mode_info.vtotal();
            if (flags & xcb::randr::MODE_FLAG_DOUBLE_SCAN) != 0 {
                val *= 2;
            }
            if (flags & xcb::randr::MODE_FLAG_INTERLACE) != 0 {
                val /= 2;
            }
            val
        };

        if vtotal != 0 && mode_info.htotal() != 0 {
            Some((mode_info.dot_clock() as f64) / (vtotal as f64 * mode_info.htotal() as f64))
        } else {
            None
        }
    })?;

    Some(refresh_rate)
}
//! Refresh rate: run every monitor at its highest refresh rate, at its native resolution.
//!
//! Monitors often report 60 Hz as their preferred mode even when they can do more, and Pinnacle
//! uses the preferred mode. The LG 24GL600F (144 Hz) flickers on dark colours at 60 Hz.
//! Applies to monitors present at startup and to ones connected later. Skips the mode change
//! when the monitor already runs at that mode, so a config reload does not blank the screen.
//!
//! No binds. Try other modes live with `pctl mode <output> <w>x<h>@<hz>`.

use pinnacle_api::output::{self, OutputHandle};

use crate::settings::Settings;

pub fn setup(_settings: &Settings) {
    output::for_each_output(max_refresh_rate);
}

fn max_refresh_rate(out: &OutputHandle) {
    let Some(native) = out.preferred_mode().or_else(|| out.current_mode()) else {
        return; // disabled output
    };
    let Some(best) = out
        .modes()
        .filter(|m| m.size == native.size)
        .max_by_key(|m| m.refresh_rate_mhz)
    else {
        return;
    };
    if out.current_mode().as_ref() != Some(&best) {
        out.set_mode(best.size.w, best.size.h, best.refresh_rate_mhz);
    }
}

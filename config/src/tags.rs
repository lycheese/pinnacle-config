//! Tags 1–9 on every output (AwesomeWM-style: a window can be on several tags,
//! and several tags can be shown at once).

use pinnacle_api::input::{self, Bind, Mod};
use pinnacle_api::output;
use pinnacle_api::tag::{self, TagHandle};
use pinnacle_api::window;

use crate::settings::Settings;

pub fn setup(s: &Settings) {
    let tag_names = s.tag_names;
    let m = s.mod_key;

    output::for_each_output(move |output| {
        let mut tags = tag::add(output, tag_names);
        if let Some(first) = tags.next() {
            first.set_active(true);
        }
    });

    // On Neo Bone, l and m sit side by side; maximize moved to the menu (SPC w m).
    let steps = [
        ('l', Step::NextOccupied, "Next tag with windows"),
        ('m', Step::PrevOccupied, "Previous tag with windows"),
        ('h', Step::NextUnused, "Next empty tag"),
    ];
    for (key, step, description) in steps {
        input::keybind(m, key)
            .on_press(move || go(step))
            .group("Tag")
            .description(description);
    }

    for name in tag_names {
        input::keybind(m, name)
            .on_press(move || {
                if let Some(tag) = tag::get(name) {
                    tag.switch_to();
                }
            })
            .group("Tag")
            .description(format!("Switch to tag {name}"));

        input::keybind(m | Mod::CTRL, name)
            .on_press(move || {
                if let Some(tag) = tag::get(name) {
                    tag.toggle_active();
                }
            })
            .group("Tag")
            .description(format!("Also show tag {name}"));

        input::keybind(m | Mod::SHIFT, name)
            .on_press(move || {
                if let Some(tag) = tag::get(name)
                    && let Some(win) = window::get_focused()
                {
                    win.move_to_tag(&tag);
                }
            })
            .group("Tag")
            .description(format!("Move window to tag {name}"));

        input::keybind(m | Mod::CTRL | Mod::SHIFT, name)
            .on_press(move || {
                if let Some(tag) = tag::get(name)
                    && let Some(win) = window::get_focused()
                {
                    win.toggle_tag(&tag);
                }
            })
            .group("Tag")
            .description(format!("Toggle tag {name} on window"));
    }
}

/// Where Super+L / Super+M / Super+H go from the current tag.
#[derive(Clone, Copy, Debug)]
pub enum Step {
    /// The next tag with windows (wrapping).
    NextOccupied,
    /// The previous tag with windows (wrapping).
    PrevOccupied,
    /// The next tag without windows (wrapping).
    NextUnused,
}

/// Picks the target tag's index from each tag's "has windows" flag and the current index.
/// `None` if no other tag qualifies.
pub fn pick(occupied: &[bool], current: usize, step: Step) -> Option<usize> {
    let len = occupied.len();
    let (forward, want_windows) = match step {
        Step::NextOccupied => (true, true),
        Step::PrevOccupied => (false, true),
        Step::NextUnused => (true, false),
    };
    (1..len)
        .map(|k| {
            if forward {
                (current + k) % len
            } else {
                (current + len - k) % len
            }
        })
        .find(|&i| occupied[i] == want_windows)
}

/// Switches the focused output to the tag `step` picks; the current tag is its first active one.
fn go(step: Step) {
    let Some(out) = output::get_focused() else {
        return;
    };
    let tags: Vec<TagHandle> = out.tags().collect();
    let Some(current) = tags.iter().position(|t| t.active()) else {
        return;
    };
    let occupied: Vec<bool> = tags.iter().map(|t| t.windows().next().is_some()).collect();
    if let Some(i) = pick(&occupied, current, step) {
        tags[i].switch_to();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //          tag:  1      2     3      4     5
    const TAGS: [bool; 5] = [true, false, true, false, true];

    #[test]
    fn next_occupied_skips_empty_tags_and_wraps() {
        assert_eq!(pick(&TAGS, 0, Step::NextOccupied), Some(2));
        assert_eq!(pick(&TAGS, 2, Step::NextOccupied), Some(4));
        assert_eq!(pick(&TAGS, 4, Step::NextOccupied), Some(0));
        // From an empty tag too.
        assert_eq!(pick(&TAGS, 1, Step::NextOccupied), Some(2));
    }

    #[test]
    fn prev_occupied_skips_empty_tags_and_wraps() {
        assert_eq!(pick(&TAGS, 4, Step::PrevOccupied), Some(2));
        assert_eq!(pick(&TAGS, 0, Step::PrevOccupied), Some(4));
        assert_eq!(pick(&TAGS, 3, Step::PrevOccupied), Some(2));
    }

    #[test]
    fn next_unused_finds_the_next_empty_tag() {
        assert_eq!(pick(&TAGS, 0, Step::NextUnused), Some(1));
        assert_eq!(pick(&TAGS, 1, Step::NextUnused), Some(3));
        assert_eq!(pick(&TAGS, 4, Step::NextUnused), Some(1));
    }

    #[test]
    fn nothing_to_go_to() {
        // Only the current tag has windows.
        assert_eq!(pick(&[false, true, false], 1, Step::NextOccupied), None);
        assert_eq!(pick(&[false, true, false], 1, Step::PrevOccupied), None);
        // Every other tag is in use.
        assert_eq!(pick(&[true, false, true], 1, Step::NextUnused), None);
        assert_eq!(pick(&[], 0, Step::NextOccupied), None);
    }
}

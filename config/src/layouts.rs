//! Dynamic tiling layouts.
//!
//! Pinnacle asks the config for a layout tree whenever windows change. A *layout generator* turns
//! a window count into that tree. Each tag remembers its own layout. To write your own layout,
//! implement `LayoutGenerator` for a struct and add it to `LAYOUTS` below.
//!
//! The leader menu (Mod+Space → l) calls `cycle` and `set`. Builds without Snowcap have no
//! leader menu and bind Mod+Space / Mod+Shift+Space to `cycle` instead.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use pinnacle_api::layout::generators::{Dwindle, Fair, MasterSide, MasterStack};
use pinnacle_api::layout::{self, LayoutGenerator, LayoutNode, LayoutRequester, LayoutResponse};
use pinnacle_api::output;
use pinnacle_api::tag::TagHandle;
use pinnacle_api::util::Batch;

use crate::settings::Settings;

type BoxedGenerator = Box<dyn LayoutGenerator + Send>;

/// Layout names, in cycling order. The first one is the default for every tag.
pub const LAYOUTS: [&str; 4] = ["master-left", "master-right", "dwindle", "fair"];

struct Layouts {
    generators: Vec<BoxedGenerator>,
    /// Index into `generators` per tag id; missing means 0.
    per_tag: HashMap<u32, usize>,
}

struct State {
    layouts: Mutex<Layouts>,
    requester: LayoutRequester,
}

static STATE: OnceLock<State> = OnceLock::new();

pub fn setup(s: &Settings) {
    let gaps = s.gaps;
    let generators: Vec<BoxedGenerator> = vec![
        Box::new(MasterStack {
            inner_gaps: gaps.into(),
            outer_gaps: gaps.into(),
            ..Default::default()
        }),
        Box::new(MasterStack {
            master_side: MasterSide::Right,
            inner_gaps: gaps.into(),
            outer_gaps: gaps.into(),
            ..Default::default()
        }),
        Box::new(Dwindle {
            inner_gaps: gaps.into(),
            outer_gaps: gaps.into(),
        }),
        Box::new(Fair {
            inner_gaps: gaps.into(),
            outer_gaps: gaps.into(),
            ..Default::default()
        }),
    ];
    debug_assert_eq!(generators.len(), LAYOUTS.len());

    let requester = layout::manage(|args| {
        let Some(tag) = args.tags.first() else {
            return LayoutResponse {
                root_node: LayoutNode::new(),
                tree_id: 0,
            };
        };
        let Some(state) = STATE.get() else {
            return LayoutResponse {
                root_node: LayoutNode::new(),
                tree_id: 0,
            };
        };
        let layouts = state.layouts.lock().unwrap();
        let index = layouts.per_tag.get(&tag.id()).copied().unwrap_or(0);
        LayoutResponse {
            root_node: layouts.generators[index].layout(args.window_count),
            // Unique per tag and layout, so Pinnacle animates between different trees.
            tree_id: ((tag.id() & 0xffff) | ((index as u32) << 16)) + 1,
        }
    });

    let _ = STATE.set(State {
        layouts: Mutex::new(Layouts {
            generators,
            per_tag: HashMap::new(),
        }),
        requester,
    });

    // Without Snowcap there is no leader menu: keep the old direct binds.
    #[cfg(not(feature = "snowcap"))]
    {
        use pinnacle_api::input::{self, Bind, Keysym, Mod};
        input::keybind(s.mod_key, Keysym::space)
            .on_press(|| cycle(true))
            .group("Layout")
            .description("Next layout");
        input::keybind(s.mod_key | Mod::SHIFT, Keysym::space)
            .on_press(|| cycle(false))
            .group("Layout")
            .description("Previous layout");
    }
}

/// Switches the active tag on the focused output to the next (or previous) layout.
pub fn cycle(forward: bool) {
    change(|current, len| {
        if forward {
            (current + 1) % len
        } else {
            (current + len - 1) % len
        }
    });
}

/// Switches the active tag on the focused output to the layout with this name (see `LAYOUTS`).
#[cfg_attr(not(feature = "snowcap"), allow(dead_code))]
pub fn set(name: &str) {
    if let Some(index) = LAYOUTS.iter().position(|l| *l == name) {
        change(|_, _| index);
    }
}

fn change(new_index: impl FnOnce(usize, usize) -> usize) {
    let Some(state) = STATE.get() else { return };
    let Some(output) = output::get_focused() else {
        return;
    };
    let Some(tag) = active_tag(&output) else {
        return;
    };
    {
        let mut layouts = state.layouts.lock().unwrap();
        let len = layouts.generators.len();
        let current = layouts.per_tag.get(&tag.id()).copied().unwrap_or(0);
        layouts.per_tag.insert(tag.id(), new_index(current, len));
    }
    state.requester.request_layout_on_output(&output);
}

fn active_tag(output: &output::OutputHandle) -> Option<TagHandle> {
    output
        .tags()
        .batch_find(|tag| Box::pin(tag.active_async()), |active| *active)
}

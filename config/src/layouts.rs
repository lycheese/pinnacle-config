//! Dynamic tiling layouts.
//!
//! Pinnacle asks the config for a layout tree whenever windows change. A *layout generator* turns
//! a window count into that tree. `Cycle` keeps one current generator per tag, so each tag can
//! use a different layout. To write your own layout, implement `LayoutGenerator` for a struct and
//! add it to the list below.

use std::sync::{Arc, Mutex};

use pinnacle_api::input::{self, Bind, Keysym, Mod};
use pinnacle_api::layout::generators::{Cycle, Dwindle, Fair, MasterSide, MasterStack};
use pinnacle_api::layout::{self, LayoutGenerator, LayoutNode, LayoutResponse};
use pinnacle_api::output;
use pinnacle_api::util::Batch;

use crate::settings::Settings;

type BoxedGenerator = Box<dyn LayoutGenerator + Send>;

fn boxed<T: LayoutGenerator + Send + 'static>(generator: T) -> BoxedGenerator {
    Box::new(generator)
}

pub fn setup(s: &Settings) {
    let gaps = s.gaps;

    // The first entry is the default layout for every tag.
    let cycler = Arc::new(Mutex::new(Cycle::new([
        boxed(MasterStack {
            inner_gaps: gaps.into(),
            outer_gaps: gaps.into(),
            ..Default::default()
        }),
        boxed(MasterStack {
            master_side: MasterSide::Right,
            inner_gaps: gaps.into(),
            outer_gaps: gaps.into(),
            ..Default::default()
        }),
        boxed(Dwindle {
            inner_gaps: gaps.into(),
            outer_gaps: gaps.into(),
        }),
        boxed(Fair {
            inner_gaps: gaps.into(),
            outer_gaps: gaps.into(),
            ..Default::default()
        }),
    ])));

    let requester = layout::manage({
        let cycler = cycler.clone();
        move |args| {
            let Some(tag) = args.tags.first() else {
                return LayoutResponse {
                    root_node: LayoutNode::new(),
                    tree_id: 0,
                };
            };

            let mut cycler = cycler.lock().unwrap();
            cycler.set_current_tag(tag.clone());

            let root_node = cycler.layout(args.window_count);
            let tree_id = cycler.current_tree_id();
            LayoutResponse { root_node, tree_id }
        }
    });

    for (mods, forward, desc) in [
        (Mod::empty(), true, "Next layout"),
        (Mod::SHIFT, false, "Previous layout"),
    ] {
        let cycler = cycler.clone();
        let requester = requester.clone();

        input::keybind(s.mod_key | mods, Keysym::space)
            .on_press(move || {
                let Some(focused_output) = output::get_focused() else {
                    return;
                };
                let Some(tag) = focused_output
                    .tags()
                    .batch_find(|tag| Box::pin(tag.active_async()), |active| *active)
                else {
                    return;
                };

                let mut cycler = cycler.lock().unwrap();
                if forward {
                    cycler.cycle_layout_forward(&tag);
                } else {
                    cycler.cycle_layout_backward(&tag);
                }
                drop(cycler);

                requester.request_layout_on_output(&focused_output);
            })
            .group("Layout")
            .description(desc);
    }
}

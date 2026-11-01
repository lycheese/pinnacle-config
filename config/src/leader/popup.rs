//! The which-key panel: a full-width Snowcap layer at the bottom of the focused monitor.
//!
//! The panel only draws; the key handler in `mod.rs` changes `Nav` and asks for a redraw.

use std::sync::{Arc, Mutex};

use snowcap_api::widget::column::Column;
use snowcap_api::widget::container::{self, Container};
use snowcap_api::widget::font::{Family, Font, Weight};
use snowcap_api::widget::row::Row;
use snowcap_api::widget::text::{self, Text};
use snowcap_api::widget::{Border, Length, Padding, Program, WidgetDef};

use super::tree::{Entry, children_at};

/// Entries per column before a new column starts.
const ROWS: usize = 6;
const COLUMN_WIDTH: f32 = 260.0;
const FONT_PX: f32 = 15.0;

/// Where the user is in the tree.
#[derive(Default)]
pub struct Nav {
    pub path: Vec<char>,
    /// Shown after the breadcrumb, e.g. "no entry for x".
    pub note: Option<String>,
}

pub struct Popup {
    pub nav: Arc<Mutex<Nav>>,
    pub tree: Arc<Vec<Entry>>,
    pub width: f32,
}

impl Program for Popup {
    type Message = ();

    fn update(&mut self, _msg: ()) {}

    fn view(&self) -> WidgetDef<()> {
        let nav = self.nav.lock().unwrap();
        let mono = Font::new_with_family(Family::Monospace);
        let style = |color: [f32; 3]| {
            text::Style::new()
                .font(mono.clone())
                .pixels(FONT_PX)
                .color(color.into())
        };

        let mut crumb = String::from("SPC ›");
        for key in &nav.path {
            crumb.push_str(&format!(" {} ›", key_name(*key)));
        }
        if let Some(note) = &nav.note {
            crumb.push_str(&format!("   {note}"));
        }
        let header = Text::new(crumb).style(
            text::Style::new()
                .font(mono.clone().weight(Weight::Bold))
                .pixels(FONT_PX)
                .color([0.65, 0.75, 1.0].into()),
        );

        let entries = children_at(&self.tree, &nav.path).unwrap_or_default();
        let mut columns = Row::new().spacing(24.0);
        for chunk in entries.chunks(ROWS) {
            let mut column = Column::new()
                .spacing(4.0)
                .width(Length::Fixed(COLUMN_WIDTH));
            for entry in chunk {
                let (label, color) = match entry {
                    Entry::Menu { label, .. } => (format!("+{label}"), [0.95, 0.75, 0.45]),
                    Entry::Action { label, .. } => (label.clone(), [0.9, 0.9, 0.9]),
                };
                let line = Row::new()
                    .spacing(10.0)
                    .push(
                        Text::new(key_name(entry.key()))
                            .width(Length::Fixed(36.0))
                            .style(style([0.55, 0.85, 0.6])),
                    )
                    .push(Text::new(label).style(style(color)));
                column = column.push(line);
            }
            columns = columns.push(column);
        }

        let body = Column::new().spacing(10.0).push(header).push(columns).push(
            Text::new("Esc close · Backspace up").style(style([0.5, 0.5, 0.55]).pixels(12.0)),
        );

        Container::new(body)
            .width(Length::Fixed(self.width))
            .padding(Padding {
                top: 12.0,
                right: 20.0,
                bottom: 12.0,
                left: 20.0,
            })
            .style(
                container::Style::new()
                    .background_color([0.08, 0.08, 0.12, 0.94].into())
                    .border(Border {
                        color: Some([0.4, 0.4, 0.7].into()),
                        width: Some(2.0),
                        radius: Some(0.0.into()),
                    }),
            )
            .into()
    }
}

pub fn key_name(key: char) -> String {
    match key {
        ' ' => "SPC".to_string(),
        other => other.to_string(),
    }
}

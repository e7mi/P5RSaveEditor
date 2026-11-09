//! Page and modal views.

mod backups;
mod basic;
mod compendium;
mod confidants;
mod items;
mod members;
mod modals;
mod settings;

pub use modals::modal;

use rune::{col, row, text, El, TextEdit};

use crate::app::{Editor, Page};
use crate::ui::{self, Metrics};

pub fn page(s: &Editor, m: &Metrics) -> El {
    match s.page {
        Page::Basic => basic::view(s, m),
        Page::Confidants => confidants::view(s, m),
        Page::Members => members::view(s, m),
        Page::Inventory => items::view(s, m),
        Page::Compendium => compendium::view(s, m),
        Page::Backups => backups::view(s, m),
        Page::Settings => settings::view(s, m),
    }
}

fn content_pad(m: &Metrics) -> f32 {
    m.content_pad()
}

fn page_shell(m: &Metrics) -> El {
    col().w_full().padding(content_pad(m)).gap(m.x(12.0))
}

/// A single-line text field with live validation and scaled geometry.
pub fn field_input<S: 'static>(
    m: &Metrics,
    field: &crate::app::Field,
    width: f32,
    valid: bool,
    edit: impl for<'a> Fn(&'a mut S) -> &'a mut TextEdit + 'static,
    change: impl Fn(&mut S) + 'static,
) -> El {
    let input = rune::text_input(&field.edit, m.body())
        .single_line()
        .grow(1.0)
        .h(m.x(24.0))
        .on_edit(edit)
        .on_input_change(move |s: &mut S, _| change(s));
    ui::input_frame(m, field.is_focused(), valid, input).w(width)
}

pub fn labelled_field<S: 'static>(
    m: &Metrics,
    label: &str,
    field: &crate::app::Field,
    width: f32,
    valid: bool,
    edit: impl for<'a> Fn(&'a mut S) -> &'a mut TextEdit + 'static,
    change: impl Fn(&mut S) + 'static,
) -> El {
    row()
        .gap(m.x(8.0))
        .center_y()
        .child(ui::field_label(m, label))
        .child(field_input(m, field, width, valid, edit, change))
}

pub fn keyed_field_input<S: 'static>(
    m: &Metrics,
    key: &'static str,
    field: &crate::app::Field,
    width: f32,
    valid: bool,
    edit: impl for<'a> Fn(&'a mut S) -> &'a mut TextEdit + 'static,
    change: impl Fn(&mut S) + 'static,
) -> El {
    let input = rune::text_input(&field.edit, m.body())
        .key(key)
        .single_line()
        .grow(1.0)
        .h(m.x(24.0))
        .on_edit(edit)
        .on_input_change(move |s: &mut S, _| change(s));
    ui::input_frame(m, field.is_focused(), valid, input).w(width)
}

pub fn two_column(m: &Metrics) -> El {
    row().w_full().gap(m.x(12.0)).flex_wrap().items_start()
}

pub fn column_card(m: &Metrics) -> El {
    ui::card(m).w_frac(0.49).min_w(m.x(340.0))
}

pub fn card_header(m: &Metrics, title: &str) -> El {
    row()
        .w_full()
        .center_y()
        .child(ui::section(m, title))
        .child(rune::spacer())
}

pub fn header_with_actions(m: &Metrics, title: &str, subtitle: &str) -> El {
    row()
        .w_full()
        .gap(m.x(12.0))
        .center_y()
        .child(ui::page_header(m, title, subtitle))
        .child(rune::spacer())
}

pub fn warning_strip(m: &Metrics, message: &str) -> El {
    row()
        .w_full()
        .padding(m.x(8.0))
        .gap(m.x(6.0))
        .center_y()
        .radius(m.x(5.0))
        .fill(ui::warning().alpha(0.10))
        .border(1.0, ui::warning().alpha(0.35))
        .child(text(message).font(m.label()).color(ui::warning()).wrap())
}

pub fn danger_strip(m: &Metrics, message: &str) -> El {
    row()
        .w_full()
        .padding(m.x(8.0))
        .gap(m.x(6.0))
        .center_y()
        .radius(m.x(5.0))
        .fill(ui::danger().alpha(0.10))
        .border(1.0, ui::danger().alpha(0.35))
        .child(text(message).font(m.label()).color(ui::danger()).wrap())
}

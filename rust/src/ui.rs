//! Theme, scaled metrics and shared widgets.

use rune::tok::{radius, type_};
use rune::{col, el, row, text, Color, Cursor, El, TextStyle};

pub struct Metrics {
    pub scale: f32,
    pub width: f32,
    pub height: f32,
}

impl Metrics {
    pub const BASE_WIDTH: f32 = 1280.0;
    pub const BASE_HEIGHT: f32 = 820.0;

    pub fn from_size(size: Option<[f32; 2]>) -> Self {
        let [width, height] = size.unwrap_or([Self::BASE_WIDTH, Self::BASE_HEIGHT]);
        let scale = (width / Self::BASE_WIDTH)
            .min(height / Self::BASE_HEIGHT)
            .clamp(0.85, 1.3);
        Self {
            scale,
            width,
            height,
        }
    }

    pub fn x(&self, value: f32) -> f32 {
        value * self.scale
    }

    pub fn font(&self, base: TextStyle) -> TextStyle {
        TextStyle {
            size: base.size * self.scale,
            line_height: base.line_height * self.scale,
            tracking: base.tracking * self.scale,
            ..base
        }
    }

    pub fn body(&self) -> TextStyle {
        self.font(type_::BODY)
    }

    pub fn label(&self) -> TextStyle {
        self.font(type_::LABEL)
    }

    pub fn section(&self) -> TextStyle {
        self.font(type_::SECTION)
    }

    pub fn title(&self) -> TextStyle {
        TextStyle::new(14.0 * self.scale, 600.0)
    }

    pub fn page_title(&self) -> TextStyle {
        TextStyle::new(17.0 * self.scale, 600.0)
    }

    pub fn mono(&self) -> TextStyle {
        TextStyle::new(11.0 * self.scale, 400.0).mono()
    }

    pub fn sidebar(&self) -> f32 {
        self.x(178.0)
    }

    pub fn content_pad(&self) -> f32 {
        self.x(16.0)
    }
}

pub fn accent() -> Color {
    Color {
        r: 0.40,
        g: 0.66,
        b: 0.92,
        a: 1.0,
    }
}

pub fn danger() -> Color {
    Color {
        r: 0.87,
        g: 0.33,
        b: 0.37,
        a: 1.0,
    }
}

pub fn success() -> Color {
    Color {
        r: 0.44,
        g: 0.76,
        b: 0.52,
        a: 1.0,
    }
}

pub fn warning() -> Color {
    Color {
        r: 0.90,
        g: 0.68,
        b: 0.30,
        a: 1.0,
    }
}

pub fn theme() -> rune::tok::Theme {
    let mut theme = rune::tok::theme(true);
    theme.bg = Color {
        r: 0.055,
        g: 0.060,
        b: 0.070,
        a: 1.0,
    };
    theme.card = Color {
        r: 0.090,
        g: 0.096,
        b: 0.110,
        a: 1.0,
    };
    theme.card_border = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 0.075,
    };
    theme.text = Color {
        r: 0.90,
        g: 0.915,
        b: 0.935,
        a: 1.0,
    };
    theme.text_secondary = Color {
        r: 0.61,
        g: 0.64,
        b: 0.69,
        a: 1.0,
    };
    theme.hover = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 0.055,
    };
    theme.press = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 0.095,
    };
    theme.control_track = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 0.075,
    };
    theme
}

pub fn card(m: &Metrics) -> El {
    let t = theme();
    col()
        .w_full()
        .fill(t.card)
        .border(1.0, t.card_border)
        .radius(m.x(radius::MD))
        .padding(m.x(12.0))
        .gap(m.x(8.0))
}

pub fn title(m: &Metrics, label: impl Into<String>) -> El {
    text(label.into()).font(m.title()).color(theme().text)
}

pub fn page_title(m: &Metrics, label: impl Into<String>) -> El {
    text(label.into()).font(m.page_title()).color(theme().text)
}

pub fn section(m: &Metrics, label: impl Into<String>) -> El {
    text(label.into())
        .font(m.section())
        .color(theme().text_secondary)
}

pub fn body(m: &Metrics, label: impl Into<String>) -> El {
    text(label.into()).font(m.body()).color(theme().text)
}

pub fn muted(m: &Metrics, label: impl Into<String>) -> El {
    text(label.into())
        .font(m.label())
        .color(theme().text_secondary)
}

pub fn small(m: &Metrics, label: impl Into<String>) -> El {
    text(label.into())
        .font(small_style(m))
        .color(theme().text_secondary)
}

pub fn small_style(m: &Metrics) -> TextStyle {
    TextStyle::new(10.5 * m.scale, 500.0)
}

pub fn mono(m: &Metrics) -> TextStyle {
    TextStyle::new(11.0 * m.scale, 400.0).mono()
}

pub fn divider() -> El {
    el().w_full().h(1.0).fill(theme().card_border)
}

pub fn button(
    m: &Metrics,
    label: impl Into<String>,
    fill: Color,
    fg: Color,
    border: Option<Color>,
) -> El {
    let mut el = row()
        .h(m.x(26.0))
        .px(m.x(9.0))
        .center()
        .radius(m.x(radius::SM))
        .fill(fill)
        .hover(Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.08,
        })
        .cursor(Cursor::Pointer)
        .child(text(label.into()).font(m.label()).color(fg));
    if let Some(border) = border {
        el = el.border(1.0, border);
    }
    el
}

pub fn primary(m: &Metrics, label: impl Into<String>) -> El {
    button(m, label, accent(), theme().bg, None)
}

pub fn ghost(m: &Metrics, label: impl Into<String>) -> El {
    button(
        m,
        label,
        Color::TRANSPARENT,
        theme().text,
        Some(theme().card_border),
    )
}

pub fn danger_button(m: &Metrics, label: impl Into<String>) -> El {
    button(
        m,
        label,
        Color::TRANSPARENT,
        danger(),
        Some(danger().alpha(0.55)),
    )
}

pub fn link(m: &Metrics, label: impl Into<String>) -> El {
    row()
        .h(m.x(22.0))
        .px(m.x(4.0))
        .center()
        .radius(m.x(4.0))
        .hover(theme().hover)
        .cursor(Cursor::Pointer)
        .child(text(label.into()).font(m.label()).color(accent()))
}

pub fn checkbox(m: &Metrics, checked: bool) -> El {
    let t = theme();
    let size = m.x(14.0);
    let mut box_el = el()
        .size(size, size)
        .radius(m.x(3.0))
        .border(1.0, if checked { accent() } else { t.card_border })
        .center();
    if checked {
        box_el = box_el.child(
            el().size(m.x(6.0), m.x(6.0))
                .radius(m.x(1.5))
                .fill(accent()),
        );
    }
    box_el
}

pub fn chip<S: 'static>(
    m: &Metrics,
    label: impl Into<String>,
    selected: bool,
    on_press: impl Fn(&mut S) + 'static,
) -> El {
    let t = theme();
    row()
        .h(m.x(24.0))
        .px(m.x(9.0))
        .center()
        .radius(m.x(5.0))
        .fill(if selected {
            accent().alpha(0.16)
        } else {
            t.control_track
        })
        .border(
            1.0,
            if selected {
                accent().alpha(0.55)
            } else {
                Color::TRANSPARENT
            },
        )
        .hover(t.hover)
        .cursor(Cursor::Pointer)
        .on_press(on_press)
        .child(
            text(label.into())
                .font(m.label())
                .color(if selected { accent() } else { t.text }),
        )
}

pub fn field_label(m: &Metrics, label: impl Into<String>) -> El {
    text(label.into())
        .font(m.label())
        .color(theme().text_secondary)
        .w(m.x(68.0))
}

pub fn input_frame(m: &Metrics, focused: bool, valid: bool, child: El) -> El {
    let t = theme();
    let border = if !valid {
        danger()
    } else if focused {
        accent()
    } else {
        t.card_border
    };
    row()
        .h(m.x(26.0))
        .px(m.x(7.0))
        .gap(m.x(4.0))
        .center_y()
        .radius(m.x(5.0))
        .fill(t.control_track)
        .border(1.0, border)
        .child(child)
}

pub fn tag(m: &Metrics, label: impl Into<String>, color: Color) -> El {
    row()
        .h(m.x(19.0))
        .px(m.x(6.0))
        .center()
        .radius(m.x(4.0))
        .fill(color.alpha(0.13))
        .child(text(label.into()).font(small_style(m)).color(color))
}

pub fn page_header(m: &Metrics, title_text: &str, subtitle_text: &str) -> El {
    col()
        .grow(1.0)
        .min_w(0.0)
        .gap(m.x(2.0))
        .child(page_title(m, title_text))
        .child(muted(m, subtitle_text))
}

pub fn empty_state(m: &Metrics, label: &str) -> El {
    card(m)
        .child(title(m, "セーブが読み込まれていません"))
        .child(muted(m, label))
}

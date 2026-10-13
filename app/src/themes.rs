//! Color themes 2fip can use instead of the desktop's (Settings → Theme).
//! Each is a small palette; libcosmic's `ThemeBuilder` derives every widget
//! color (buttons, menus, hover and selection states) from it.

use std::sync::Arc;

use cosmic::cosmic_theme::palette::{Srgb, Srgba};
use cosmic::cosmic_theme::{CornerRadii, Roundness, ThemeBuilder};
use cosmic::iced::{Background, Border, Color};
use cosmic::widget;
use fs_ops::settings::{ColorTheme, Corners};

/// The colors a theme is built from, as `0xrrggbb`.
struct Palette {
    /// Window background.
    background: u32,
    /// Panels (primary containers), a step lighter than the background.
    container: u32,
    /// Raised surfaces: hover, selection, secondary containers.
    raised: u32,
    text: u32,
    /// Tint of the neutral greys (usually the theme's "comment" color).
    neutral: u32,
    /// Selected files, the active path bar, progress.
    accent: u32,
    success: u32,
    warning: u32,
    /// Delete buttons, errors.
    destructive: u32,
}

/// The palette of each of 2fip's own themes; `None` for the ones that use
/// libcosmic's (System, Light, Dark).
fn palette(theme: ColorTheme) -> Option<Palette> {
    let palette = match theme {
        ColorTheme::System | ColorTheme::Light | ColorTheme::Dark => return None,
        // https://draculatheme.com
        ColorTheme::Dracula => Palette {
            background: 0x282a36,
            container: 0x343746,
            raised: 0x44475a,
            text: 0xf8f8f2,
            neutral: 0x6272a4,
            accent: 0xbd93f9,
            success: 0x50fa7b,
            warning: 0xffb86c,
            destructive: 0xff5555,
        },
        // Everforest dark, medium contrast.
        ColorTheme::Everforest => Palette {
            background: 0x2d353b,
            container: 0x343f44,
            raised: 0x3d484d,
            text: 0xd3c6aa,
            neutral: 0x859289,
            accent: 0xa7c080,
            success: 0x83c092,
            warning: 0xe69875,
            destructive: 0xe67e80,
        },
        // Gruvbox Material dark, medium contrast.
        ColorTheme::GruvboxMaterial => Palette {
            background: 0x282828,
            container: 0x32302f,
            raised: 0x45403d,
            text: 0xd4be98,
            neutral: 0x928374,
            accent: 0xd8a657,
            success: 0xa9b665,
            warning: 0xe78a4e,
            destructive: 0xea6962,
        },
        // https://www.nordtheme.com
        ColorTheme::Nord => Palette {
            background: 0x2e3440,
            container: 0x3b4252,
            raised: 0x434c5e,
            text: 0xeceff4,
            neutral: 0x4c566a,
            accent: 0x88c0d0,
            success: 0xa3be8c,
            warning: 0xebcb8b,
            destructive: 0xbf616a,
        },
        // Tokyo Night, "storm" variant.
        ColorTheme::TokyoNightStorm => Palette {
            background: 0x24283b,
            container: 0x292e42,
            raised: 0x3b4261,
            text: 0xc0caf5,
            neutral: 0x565f89,
            accent: 0x7aa2f7,
            success: 0x9ece6a,
            warning: 0xff9e64,
            destructive: 0xf7768e,
        },
        // https://catppuccin.com — Mocha flavor, mauve accent.
        ColorTheme::CatppuccinMocha => Palette {
            background: 0x1e1e2e,
            container: 0x313244,
            raised: 0x45475a,
            text: 0xcdd6f4,
            neutral: 0x6c7086,
            accent: 0xcba6f7,
            success: 0xa6e3a1,
            warning: 0xfab387,
            destructive: 0xf38ba8,
        },
        // Catppuccin Macchiato flavor, mauve accent.
        ColorTheme::CatppuccinMacchiato => Palette {
            background: 0x24273a,
            container: 0x363a4f,
            raised: 0x494d64,
            text: 0xcad3f5,
            neutral: 0x6e738d,
            accent: 0xc6a0f6,
            success: 0xa6da95,
            warning: 0xf5a97f,
            destructive: 0xed8796,
        },
        // Ayu, dark variant.
        ColorTheme::AyuDark => Palette {
            background: 0x0d1017,
            container: 0x131721,
            raised: 0x1e232b,
            text: 0xbfbdb6,
            neutral: 0x5a6378,
            accent: 0xe6b450,
            success: 0xaad94c,
            warning: 0xff8f40,
            destructive: 0xd95757,
        },
        // Green-on-black, after the film's "digital rain".
        ColorTheme::Matrix => Palette {
            background: 0x0d0208,
            container: 0x0a1a0d,
            raised: 0x003b00,
            text: 0x00ff41,
            neutral: 0x008f11,
            accent: 0x00ff41,
            success: 0x00ff41,
            warning: 0xd4ff00,
            destructive: 0xff3131,
        },
        // Monokai Pro, default filter.
        ColorTheme::MonokaiPro => Palette {
            background: 0x2d2a2e,
            container: 0x363337,
            raised: 0x403e41,
            text: 0xfcfcfa,
            neutral: 0x727072,
            accent: 0xffd866,
            success: 0xa9dc76,
            warning: 0xfc9867,
            destructive: 0xff6188,
        },
        // https://ethanschoonover.com/solarized — dark (base03 background).
        ColorTheme::SolarizedDark => Palette {
            background: 0x002b36,
            container: 0x073642,
            raised: 0x0e4452,
            text: 0x93a1a1,
            neutral: 0x586e75,
            accent: 0x268bd2,
            success: 0x859900,
            warning: 0xcb4b16,
            destructive: 0xdc322f,
        },
        // Gruvbox (original) dark, medium contrast.
        ColorTheme::GruvboxDark => Palette {
            background: 0x282828,
            container: 0x3c3836,
            raised: 0x504945,
            text: 0xebdbb2,
            neutral: 0x928374,
            accent: 0xfe8019,
            success: 0xb8bb26,
            warning: 0xfabd2f,
            destructive: 0xfb4934,
        },
    };
    Some(palette)
}

/// The libcosmic theme for one of 2fip's own palettes (`None` for System,
/// Light and Dark, which the caller takes from libcosmic).
pub fn build(theme: ColorTheme) -> Option<cosmic::Theme> {
    let p = palette(theme)?;
    let mut builder = ThemeBuilder::dark()
        .bg_color(hex_alpha(p.background))
        .primary_container_bg(hex_alpha(p.container))
        .text_tint(hex(p.text))
        .neutral_tint(hex(p.neutral))
        .accent(hex(p.accent))
        .success(hex(p.success))
        .warning(hex(p.warning))
        .destructive(hex(p.destructive));
    builder.secondary_container_bg = Some(hex_alpha(p.raised));
    Some(cosmic::Theme::custom(Arc::new(builder.build())))
}

/// A flatter, Microsoft-like theme: light-gray surfaces, blue accent, and
/// square-ish controls, used when the user opts into a Windows visual style.
pub fn windows_like() -> cosmic::Theme {
    let mut builder = ThemeBuilder::light()
        .bg_color(Srgba::new(0.96, 0.96, 0.96, 1.0))
        .primary_container_bg(Srgba::new(0.92, 0.92, 0.92, 1.0))
        .text_tint(Srgb::new(0.12, 0.12, 0.12))
        .neutral_tint(Srgb::new(0.52, 0.52, 0.52))
        .accent(Srgb::new(0.0, 0.47, 0.84))
        .success(Srgb::new(0.16, 0.64, 0.32))
        .warning(Srgb::new(0.95, 0.65, 0.18))
        .destructive(Srgb::new(0.79, 0.22, 0.2));
    builder.secondary_container_bg = Some(Srgba::new(0.86, 0.86, 0.86, 1.0));
    cosmic::Theme::custom(Arc::new(builder.build()))
}

/// The corner sizes for a roundness step. Square, Medium and Large are
/// COSMIC's own Square, Slightly round and Round styles.
fn corner_radii(corners: Corners) -> CornerRadii {
    match corners {
        Corners::Square => Roundness::Square.into(),
        Corners::Small => CornerRadii {
            radius_0: [0.0; 4],
            radius_xs: [2.0; 4],
            radius_s: [4.0; 4],
            radius_m: [4.0; 4],
            radius_l: [4.0; 4],
            radius_xl: [4.0; 4],
        },
        Corners::Medium => Roundness::SlightlyRound.into(),
        Corners::Large => Roundness::Round.into(),
    }
}

/// `theme` with the corners of `corners`. A system theme stays one, so
/// libcosmic keeps following the desktop's changes (which 2fip then rounds
/// again, see `system_theme_update`).
pub fn with_corners(theme: &cosmic::Theme, corners: Corners, system: bool) -> cosmic::Theme {
    let mut cosmic = theme.cosmic().clone();
    cosmic.corner_radii = corner_radii(corners);
    if system {
        cosmic::Theme::system(Arc::new(cosmic))
    } else {
        cosmic::Theme::custom(Arc::new(cosmic))
    }
}

/// libcosmic's tooltip look, plus a readable text color. Its own style leaves
/// the text color to the surroundings, and its background isn't tied to the
/// theme's text color: with COSMIC's light theme the background is near-black
/// while the text is dark. So the text color is picked from the background
/// itself.
pub fn tooltip_style(theme: &cosmic::Theme) -> widget::container::Style {
    let cosmic = theme.cosmic();
    let background = cosmic.palette.neutral_2;
    let text = contrasting_text(background.red, background.green, background.blue);
    widget::container::Style {
        icon_color: Some(text),
        text_color: Some(text),
        background: Some(Background::Color(background.into())),
        border: Border {
            radius: cosmic.corner_radii.radius_l.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Near-white text on a dark background, near-black on a light one
/// (by the background's relative luminance; channels are sRGB, 0–1).
fn contrasting_text(red: f32, green: f32, blue: f32) -> Color {
    let linear = |c: f32| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = 0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue);
    // 0.18: where white and black text have equal contrast (WCAG).
    if luminance < 0.18 {
        Color::from_rgb(0.95, 0.95, 0.95)
    } else {
        Color::from_rgb(0.08, 0.08, 0.08)
    }
}

/// A [`cosmic::widget::tooltip`] whose text always contrasts with its
/// background (see [`tooltip_style`]). Use this everywhere instead of
/// libcosmic's default tooltip, which is unreadable in the light theme.
pub fn tooltip<'a, Message>(
    content: impl Into<cosmic::Element<'a, Message>>,
    tooltip: impl Into<cosmic::Element<'a, Message>>,
    position: cosmic::widget::tooltip::Position,
) -> cosmic::widget::Tooltip<'a, Message> {
    cosmic::widget::tooltip(content, tooltip, position)
        .class(cosmic::theme::Container::custom(tooltip_style))
}

/// `0xrrggbb` → color.
fn hex(rgb: u32) -> Srgb {
    Srgb::new(
        ((rgb >> 16) & 0xff) as f32 / 255.0,
        ((rgb >> 8) & 0xff) as f32 / 255.0,
        (rgb & 0xff) as f32 / 255.0,
    )
}

fn hex_alpha(rgb: u32) -> Srgba {
    let c = hex(rgb);
    Srgba::new(c.red, c.green, c.blue, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWN_THEMES: [ColorTheme; 12] = [
        ColorTheme::Dracula,
        ColorTheme::Everforest,
        ColorTheme::GruvboxMaterial,
        ColorTheme::Nord,
        ColorTheme::TokyoNightStorm,
        ColorTheme::CatppuccinMocha,
        ColorTheme::CatppuccinMacchiato,
        ColorTheme::AyuDark,
        ColorTheme::Matrix,
        ColorTheme::MonokaiPro,
        ColorTheme::SolarizedDark,
        ColorTheme::GruvboxDark,
    ];

    #[test]
    fn each_theme_is_dark_and_uses_its_accent() {
        for theme in OWN_THEMES {
            let expected = hex(palette(theme).unwrap().accent);
            let built = build(theme).unwrap();
            let cosmic = built.cosmic();
            let accent = cosmic.accent_color();
            let close = |a: f32, b: f32| (a - b).abs() < 0.01;
            assert!(
                close(accent.red, expected.red)
                    && close(accent.green, expected.green)
                    && close(accent.blue, expected.blue),
                "{theme:?} accent"
            );
            assert!(cosmic.is_dark, "{theme:?} is dark");
        }
    }

    #[test]
    fn libcosmic_themes_have_no_palette() {
        for theme in [ColorTheme::System, ColorTheme::Light, ColorTheme::Dark] {
            assert!(build(theme).is_none());
        }
    }

    #[test]
    fn windows_like_theme_is_light_and_blue_accent() {
        let built = windows_like();
        let cosmic = built.cosmic();
        let accent = cosmic.accent_color();
        assert!(!cosmic.is_dark);
        assert!(accent.red < 0.1);
        assert!(accent.green > 0.3);
        assert!(accent.blue > 0.7);
    }

    #[test]
    fn tooltip_text_contrasts_with_its_background() {
        // Near-black (what the light theme shows) and the dark palette's #161616.
        assert!(contrasting_text(0.1, 0.1, 0.1).r > 0.9);
        assert!(contrasting_text(0.086, 0.086, 0.086).r > 0.9);
        // The light palette's #BEBEBE and white.
        assert!(contrasting_text(0.745, 0.745, 0.745).r < 0.1);
        assert!(contrasting_text(1.0, 1.0, 1.0).r < 0.1);
    }
}

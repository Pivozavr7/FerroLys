//! MD3-палитры: тёмная схема из seed-акцента через `material-colors`
//! (порт official material-color-utilities, план §5.2).
//!
//! Схема считается на лету → смена акцента в настройках не требует
//! перезапуска. Если качество порта не устроит — план допускает фолбэк на
//! предвычисленные палитры; код останется тем же, поменяется источник.

use material_colors::color::Rgb;
use material_colors::theme::ThemeBuilder;

/// Базовый акцент (M3 baseline purple) — значение по умолчанию.
pub const DEFAULT_ACCENT_SEED: u32 = 0x6750A4;

/// Пресет акцента: стабильный id + seed-цвет (0xRRGGBB).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentPreset {
    pub id: &'static str,
    pub seed: u32,
}

/// Пресеты акцента (волна 0: 6; план требовал минимум 2).
pub const ACCENT_PRESETS: &[AccentPreset] = &[
    AccentPreset {
        id: "purple",
        seed: 0x6750A4,
    },
    AccentPreset {
        id: "blue",
        seed: 0x0061A4,
    },
    AccentPreset {
        id: "teal",
        seed: 0x00696D,
    },
    AccentPreset {
        id: "green",
        seed: 0x386A20,
    },
    AccentPreset {
        id: "orange",
        seed: 0x9A4600,
    },
    AccentPreset {
        id: "red",
        seed: 0xB3261E,
    },
];

/// Цветовые роли MD3, которые нужны UI Ferrolys (значения — 0xRRGGBB).
/// Имена повторяют роли MD3; в `.slint` — тот же набор в `ThemeColors`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemeColors {
    pub primary: u32,
    pub on_primary: u32,
    pub primary_container: u32,
    pub on_primary_container: u32,
    pub inverse_primary: u32,
    pub secondary: u32,
    pub on_secondary: u32,
    pub secondary_container: u32,
    pub on_secondary_container: u32,
    pub tertiary: u32,
    pub on_tertiary: u32,
    pub tertiary_container: u32,
    pub on_tertiary_container: u32,
    pub error: u32,
    pub on_error: u32,
    pub error_container: u32,
    pub on_error_container: u32,
    pub surface_dim: u32,
    pub surface: u32,
    pub surface_bright: u32,
    pub surface_container_lowest: u32,
    pub surface_container_low: u32,
    pub surface_container: u32,
    pub surface_container_high: u32,
    pub surface_container_highest: u32,
    pub on_surface: u32,
    pub on_surface_variant: u32,
    pub outline: u32,
    pub outline_variant: u32,
    pub inverse_surface: u32,
    pub inverse_on_surface: u32,
    pub scrim: u32,
}

/// Тёмная MD3-схема для seed-цвета (главная тема Ferrolys — тёмная).
pub fn dark_scheme(seed: u32) -> SchemeColors {
    let theme = ThemeBuilder::with_source(Rgb::from_u32(seed)).build();
    let s = theme.schemes.dark;
    SchemeColors {
        primary: hex(s.primary),
        on_primary: hex(s.on_primary),
        primary_container: hex(s.primary_container),
        on_primary_container: hex(s.on_primary_container),
        inverse_primary: hex(s.inverse_primary),
        secondary: hex(s.secondary),
        on_secondary: hex(s.on_secondary),
        secondary_container: hex(s.secondary_container),
        on_secondary_container: hex(s.on_secondary_container),
        tertiary: hex(s.tertiary),
        on_tertiary: hex(s.on_tertiary),
        tertiary_container: hex(s.tertiary_container),
        on_tertiary_container: hex(s.on_tertiary_container),
        error: hex(s.error),
        on_error: hex(s.on_error),
        error_container: hex(s.error_container),
        on_error_container: hex(s.on_error_container),
        surface_dim: hex(s.surface_dim),
        surface: hex(s.surface),
        surface_bright: hex(s.surface_bright),
        surface_container_lowest: hex(s.surface_container_lowest),
        surface_container_low: hex(s.surface_container_low),
        surface_container: hex(s.surface_container),
        surface_container_high: hex(s.surface_container_high),
        surface_container_highest: hex(s.surface_container_highest),
        on_surface: hex(s.on_surface),
        on_surface_variant: hex(s.on_surface_variant),
        outline: hex(s.outline),
        outline_variant: hex(s.outline_variant),
        inverse_surface: hex(s.inverse_surface),
        inverse_on_surface: hex(s.inverse_on_surface),
        scrim: hex(s.scrim),
    }
}

/// Rgb → 0xRRGGBB.
fn hex(c: Rgb) -> u32 {
    ((c.red as u32) << 16) | ((c.green as u32) << 8) | (c.blue as u32)
}

/// Полная MD3-схема: все роли, включая fixed/background/shadow —
/// их требует `MaterialPalette` библиотеки material-components (§5).
/// Имена повторяют роли MD3 (snake_case, как в `material-colors`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FullScheme {
    pub primary: u32,
    pub on_primary: u32,
    pub primary_container: u32,
    pub on_primary_container: u32,
    pub inverse_primary: u32,
    pub primary_fixed: u32,
    pub primary_fixed_dim: u32,
    pub on_primary_fixed: u32,
    pub on_primary_fixed_variant: u32,
    pub secondary: u32,
    pub on_secondary: u32,
    pub secondary_container: u32,
    pub on_secondary_container: u32,
    pub secondary_fixed: u32,
    pub secondary_fixed_dim: u32,
    pub on_secondary_fixed: u32,
    pub on_secondary_fixed_variant: u32,
    pub tertiary: u32,
    pub on_tertiary: u32,
    pub tertiary_container: u32,
    pub on_tertiary_container: u32,
    pub tertiary_fixed: u32,
    pub tertiary_fixed_dim: u32,
    pub on_tertiary_fixed: u32,
    pub on_tertiary_fixed_variant: u32,
    pub error: u32,
    pub on_error: u32,
    pub error_container: u32,
    pub on_error_container: u32,
    pub surface_dim: u32,
    pub surface: u32,
    pub surface_tint: u32,
    pub surface_bright: u32,
    pub surface_container_lowest: u32,
    pub surface_container_low: u32,
    pub surface_container: u32,
    pub surface_container_high: u32,
    pub surface_container_highest: u32,
    pub on_surface: u32,
    pub on_surface_variant: u32,
    pub outline: u32,
    pub outline_variant: u32,
    pub inverse_surface: u32,
    pub inverse_on_surface: u32,
    pub surface_variant: u32,
    pub background: u32,
    pub on_background: u32,
    pub shadow: u32,
    pub scrim: u32,
}

/// Полная схема из seed: `light` — светлая, иначе тёмная.
pub fn full_scheme(seed: u32, light: bool) -> FullScheme {
    let theme = ThemeBuilder::with_source(Rgb::from_u32(seed)).build();
    let s = if light {
        &theme.schemes.light
    } else {
        &theme.schemes.dark
    };
    FullScheme {
        primary: hex(s.primary),
        on_primary: hex(s.on_primary),
        primary_container: hex(s.primary_container),
        on_primary_container: hex(s.on_primary_container),
        inverse_primary: hex(s.inverse_primary),
        primary_fixed: hex(s.primary_fixed),
        primary_fixed_dim: hex(s.primary_fixed_dim),
        on_primary_fixed: hex(s.on_primary_fixed),
        on_primary_fixed_variant: hex(s.on_primary_fixed_variant),
        secondary: hex(s.secondary),
        on_secondary: hex(s.on_secondary),
        secondary_container: hex(s.secondary_container),
        on_secondary_container: hex(s.on_secondary_container),
        secondary_fixed: hex(s.secondary_fixed),
        secondary_fixed_dim: hex(s.secondary_fixed_dim),
        on_secondary_fixed: hex(s.on_secondary_fixed),
        on_secondary_fixed_variant: hex(s.on_secondary_fixed_variant),
        tertiary: hex(s.tertiary),
        on_tertiary: hex(s.on_tertiary),
        tertiary_container: hex(s.tertiary_container),
        on_tertiary_container: hex(s.on_tertiary_container),
        tertiary_fixed: hex(s.tertiary_fixed),
        tertiary_fixed_dim: hex(s.tertiary_fixed_dim),
        on_tertiary_fixed: hex(s.on_tertiary_fixed),
        on_tertiary_fixed_variant: hex(s.on_tertiary_fixed_variant),
        error: hex(s.error),
        on_error: hex(s.on_error),
        error_container: hex(s.error_container),
        on_error_container: hex(s.on_error_container),
        surface_dim: hex(s.surface_dim),
        surface: hex(s.surface),
        surface_tint: hex(s.surface_tint),
        surface_bright: hex(s.surface_bright),
        surface_container_lowest: hex(s.surface_container_lowest),
        surface_container_low: hex(s.surface_container_low),
        surface_container: hex(s.surface_container),
        surface_container_high: hex(s.surface_container_high),
        surface_container_highest: hex(s.surface_container_highest),
        on_surface: hex(s.on_surface),
        on_surface_variant: hex(s.on_surface_variant),
        outline: hex(s.outline),
        outline_variant: hex(s.outline_variant),
        inverse_surface: hex(s.inverse_surface),
        inverse_on_surface: hex(s.inverse_on_surface),
        surface_variant: hex(s.surface_variant),
        background: hex(s.background),
        on_background: hex(s.on_background),
        shadow: hex(s.shadow),
        scrim: hex(s.scrim),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_have_unique_ids_and_seeds() {
        let mut ids = std::collections::HashSet::new();
        let mut seeds = std::collections::HashSet::new();
        for preset in ACCENT_PRESETS {
            assert!(ids.insert(preset.id), "duplicate preset id {}", preset.id);
            assert!(seeds.insert(preset.seed), "duplicate preset seed");
        }
        assert!(
            ACCENT_PRESETS.len() >= 2,
            "plan requires at least 2 accents"
        );
    }

    #[test]
    fn default_seed_is_a_preset() {
        assert!(ACCENT_PRESETS.iter().any(|p| p.seed == DEFAULT_ACCENT_SEED));
    }

    #[test]
    fn dark_scheme_has_distinguishable_surface_and_on_surface() {
        let s = dark_scheme(DEFAULT_ACCENT_SEED);
        // Тёмная тема: surface (фон) должен быть заметно темнее текста.
        assert_ne!(s.surface, s.on_surface);
        assert_ne!(s.primary, s.on_primary);
        assert_ne!(s.outline, s.outline_variant);
    }

    #[test]
    fn different_seeds_produce_different_primary() {
        let a = dark_scheme(0x0061A4);
        let b = dark_scheme(0x386A20);
        assert_ne!(a.primary, b.primary);
    }

    #[test]
    fn all_colors_are_rgb_triples() {
        let s = dark_scheme(0x6750A4);
        // 0xRRGGBB: ни один канал не вылез (проверка структуры значения)
        let all = [
            s.primary,
            s.surface,
            s.on_surface,
            s.scrim,
            s.outline,
            s.error,
        ];
        for color in all {
            assert!(color <= 0xFF_FFFF);
        }
    }
}

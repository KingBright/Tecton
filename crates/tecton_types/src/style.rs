use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};

use crate::shape::ShapeType;

/// Flex direction for layout
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

impl FlexDirection {
    pub fn is_row(&self) -> bool {
        matches!(self, FlexDirection::Row | FlexDirection::RowReverse)
    }

    pub fn is_column(&self) -> bool {
        matches!(self, FlexDirection::Column | FlexDirection::ColumnReverse)
    }

    pub fn is_reverse(&self) -> bool {
        matches!(
            self,
            FlexDirection::RowReverse | FlexDirection::ColumnReverse
        )
    }
}

/// Color representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn with_alpha(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self::new(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Self::with_alpha(r, g, b, a))
            }
            _ => None,
        }
    }

    pub fn to_hex(&self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }

    pub fn to_rgba_string(&self) -> String {
        format!(
            "rgba({}, {}, {}, {})",
            self.r,
            self.g,
            self.b,
            self.a as f32 / 255.0
        )
    }
}

// Preset colors
impl Color {
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const RED: Self = Self {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const GREEN: Self = Self {
        r: 0,
        g: 255,
        b: 0,
        a: 255,
    };
    pub const BLUE: Self = Self {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    // Common grays
    pub const GRAY_100: Self = Self {
        r: 243,
        g: 244,
        b: 246,
        a: 255,
    };
    pub const GRAY_200: Self = Self {
        r: 229,
        g: 231,
        b: 235,
        a: 255,
    };
    pub const GRAY_300: Self = Self {
        r: 209,
        g: 213,
        b: 219,
        a: 255,
    };
    pub const GRAY_400: Self = Self {
        r: 156,
        g: 163,
        b: 175,
        a: 255,
    };
    pub const GRAY_500: Self = Self {
        r: 107,
        g: 114,
        b: 128,
        a: 255,
    };
    pub const GRAY_600: Self = Self {
        r: 75,
        g: 85,
        b: 99,
        a: 255,
    };
    pub const GRAY_700: Self = Self {
        r: 55,
        g: 65,
        b: 81,
        a: 255,
    };
    pub const GRAY_800: Self = Self {
        r: 31,
        g: 41,
        b: 55,
        a: 255,
    };
    pub const GRAY_900: Self = Self {
        r: 17,
        g: 24,
        b: 39,
        a: 255,
    };
}

/// Style properties for nodes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Style {
    /// Background color
    #[serde(default = "default_background")]
    pub background: Color,
    /// Border color
    #[serde(default = "default_border")]
    pub border_color: Color,
    /// Border width in pixels
    #[serde(default = "default_border_width")]
    pub border_width: OrderedFloat<f32>,
    /// Border radius in pixels
    #[serde(default)]
    pub border_radius: OrderedFloat<f32>,
    /// Text color
    #[serde(default = "default_text")]
    pub text_color: Color,
    /// Font size in pixels
    #[serde(default = "default_font_size")]
    pub font_size: OrderedFloat<f32>,
    /// Padding inside the node
    #[serde(default)]
    pub padding: Padding,
    /// Margin outside the node
    #[serde(default)]
    pub margin: Margin,
    /// Flex direction for children
    #[serde(default)]
    pub flex_direction: FlexDirection,
    /// Gap between children
    #[serde(default)]
    pub gap: OrderedFloat<f32>,
    /// Shape type for the node
    #[serde(default)]
    pub shape: ShapeType,
}

fn default_background() -> Color {
    Color::WHITE
}

fn default_border() -> Color {
    Color::GRAY_300
}

fn default_border_width() -> OrderedFloat<f32> {
    OrderedFloat(1.0)
}

fn default_text() -> Color {
    Color::GRAY_900
}

fn default_font_size() -> OrderedFloat<f32> {
    OrderedFloat(14.0)
}

impl Style {
    pub fn border_width(&self) -> f32 {
        self.border_width.into_inner()
    }

    pub fn border_radius(&self) -> f32 {
        self.border_radius.into_inner()
    }

    pub fn font_size(&self) -> f32 {
        self.font_size.into_inner()
    }

    pub fn gap(&self) -> f32 {
        self.gap.into_inner()
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background: default_background(),
            border_color: default_border(),
            border_width: default_border_width(),
            border_radius: OrderedFloat(0.0),
            text_color: default_text(),
            font_size: default_font_size(),
            padding: Padding::default(),
            margin: Margin::default(),
            flex_direction: FlexDirection::default(),
            gap: OrderedFloat(0.0),
            shape: ShapeType::default(),
        }
    }
}

/// Padding on all sides
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Padding {
    pub top: OrderedFloat<f32>,
    pub right: OrderedFloat<f32>,
    pub bottom: OrderedFloat<f32>,
    pub left: OrderedFloat<f32>,
}

impl Padding {
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top: OrderedFloat(top),
            right: OrderedFloat(right),
            bottom: OrderedFloat(bottom),
            left: OrderedFloat(left),
        }
    }

    pub fn uniform(value: f32) -> Self {
        Self::new(value, value, value, value)
    }

    pub fn horizontal(value: f32) -> Self {
        Self::new(0.0, value, 0.0, value)
    }

    pub fn vertical(value: f32) -> Self {
        Self::new(value, 0.0, value, 0.0)
    }

    pub fn top(&self) -> f32 {
        self.top.into_inner()
    }

    pub fn right(&self) -> f32 {
        self.right.into_inner()
    }

    pub fn bottom(&self) -> f32 {
        self.bottom.into_inner()
    }

    pub fn left(&self) -> f32 {
        self.left.into_inner()
    }

    pub fn horizontal_total(&self) -> f32 {
        self.left() + self.right()
    }

    pub fn vertical_total(&self) -> f32 {
        self.top() + self.bottom()
    }
}

impl Default for Padding {
    fn default() -> Self {
        Self::uniform(0.0)
    }
}

/// Margin on all sides
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Margin {
    pub top: OrderedFloat<f32>,
    pub right: OrderedFloat<f32>,
    pub bottom: OrderedFloat<f32>,
    pub left: OrderedFloat<f32>,
}

impl Margin {
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top: OrderedFloat(top),
            right: OrderedFloat(right),
            bottom: OrderedFloat(bottom),
            left: OrderedFloat(left),
        }
    }

    pub fn uniform(value: f32) -> Self {
        Self::new(value, value, value, value)
    }

    pub fn horizontal(value: f32) -> Self {
        Self::new(0.0, value, 0.0, value)
    }

    pub fn vertical(value: f32) -> Self {
        Self::new(value, 0.0, value, 0.0)
    }

    pub fn top(&self) -> f32 {
        self.top.into_inner()
    }

    pub fn right(&self) -> f32 {
        self.right.into_inner()
    }

    pub fn bottom(&self) -> f32 {
        self.bottom.into_inner()
    }

    pub fn left(&self) -> f32 {
        self.left.into_inner()
    }

    pub fn horizontal_total(&self) -> f32 {
        self.left() + self.right()
    }

    pub fn vertical_total(&self) -> f32 {
        self.top() + self.bottom()
    }
}

impl Default for Margin {
    fn default() -> Self {
        Self::uniform(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_from_hex() {
        let color = Color::from_hex("#FF5733").unwrap();
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 87);
        assert_eq!(color.b, 51);
        assert_eq!(color.a, 255);

        let color = Color::from_hex("FF5733").unwrap();
        assert_eq!(color.r, 255);

        let color = Color::from_hex("#FF5733AA").unwrap();
        assert_eq!(color.a, 170);
    }

    #[test]
    fn test_color_to_hex() {
        let color = Color::new(255, 87, 51);
        assert_eq!(color.to_hex(), "#ff5733");

        let color = Color::with_alpha(255, 87, 51, 170);
        assert_eq!(color.to_hex(), "#ff5733aa");
    }

    #[test]
    fn test_padding() {
        let padding = Padding::uniform(10.0);
        assert_eq!(padding.horizontal_total(), 20.0);
        assert_eq!(padding.vertical_total(), 20.0);
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_color_default_values() {
        let color = Color::new(255, 128, 0);
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 0);
        assert_eq!(color.a, 255); // Default alpha
    }

    #[test]
    fn test_color_with_alpha() {
        let color = Color::with_alpha(255, 128, 0, 128);
        assert_eq!(color.a, 128);
    }

    #[test]
    fn test_color_invalid_hex() {
        assert!(Color::from_hex("#XYZ").is_none());
        assert!(Color::from_hex("invalid").is_none());
        assert!(Color::from_hex("#12345").is_none()); // Wrong length
    }

    #[test]
    fn test_color_to_rgba_string() {
        let color = Color::new(255, 128, 64);
        assert_eq!(color.to_rgba_string(), "rgba(255, 128, 64, 1)");

        let with_alpha = Color::with_alpha(255, 128, 64, 128);
        assert_eq!(with_alpha.to_rgba_string(), "rgba(255, 128, 64, 0.5019608)");
    }

    #[test]
    fn test_color_presets() {
        assert_eq!(Color::BLACK.r, 0);
        assert_eq!(Color::WHITE.r, 255);
        assert_eq!(Color::RED.r, 255);
        assert_eq!(Color::GREEN.g, 255);
        assert_eq!(Color::BLUE.b, 255);
        assert_eq!(Color::TRANSPARENT.a, 0);
    }

    #[test]
    fn test_color_gray_scale() {
        assert_eq!(Color::GRAY_100.r, 243);
        assert_eq!(Color::GRAY_500.r, 107);
        assert_eq!(Color::GRAY_900.r, 17);
    }

    #[test]
    fn test_style_default() {
        let style = Style::default();

        assert_eq!(style.background, Color::WHITE);
        assert_eq!(style.border_color, Color::GRAY_300);
        assert!((style.border_width() - 1.0).abs() < 0.001);
        assert!((style.border_radius() - 0.0).abs() < 0.001);
        assert_eq!(style.text_color, Color::GRAY_900);
        assert!((style.font_size() - 14.0).abs() < 0.001);
    }

    #[test]
    fn test_style_gap() {
        let style = Style::default();
        assert!((style.gap() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_padding_uniform() {
        let padding = Padding::uniform(10.0);

        assert!((padding.top() - 10.0).abs() < 0.001);
        assert!((padding.right() - 10.0).abs() < 0.001);
        assert!((padding.bottom() - 10.0).abs() < 0.001);
        assert!((padding.left() - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_padding_horizontal() {
        let padding = Padding::horizontal(20.0);

        assert!((padding.top() - 0.0).abs() < 0.001);
        assert!((padding.right() - 20.0).abs() < 0.001);
        assert!((padding.bottom() - 0.0).abs() < 0.001);
        assert!((padding.left() - 20.0).abs() < 0.001);
        assert!((padding.horizontal_total() - 40.0).abs() < 0.001);
    }

    #[test]
    fn test_padding_vertical() {
        let padding = Padding::vertical(15.0);

        assert!((padding.top() - 15.0).abs() < 0.001);
        assert!((padding.right() - 0.0).abs() < 0.001);
        assert!((padding.bottom() - 15.0).abs() < 0.001);
        assert!((padding.left() - 0.0).abs() < 0.001);
        assert!((padding.vertical_total() - 30.0).abs() < 0.001);
    }

    #[test]
    fn test_padding_default() {
        let padding = Padding::default();

        assert!((padding.top() - 0.0).abs() < 0.001);
        assert!((padding.right() - 0.0).abs() < 0.001);
        assert!((padding.bottom() - 0.0).abs() < 0.001);
        assert!((padding.left() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_margin_uniform() {
        let margin = Margin::uniform(10.0);

        assert!((margin.top() - 10.0).abs() < 0.001);
        assert!((margin.right() - 10.0).abs() < 0.001);
        assert!((margin.bottom() - 10.0).abs() < 0.001);
        assert!((margin.left() - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_margin_horizontal() {
        let margin = Margin::horizontal(20.0);

        assert!((margin.top() - 0.0).abs() < 0.001);
        assert!((margin.right() - 20.0).abs() < 0.001);
        assert!((margin.horizontal_total() - 40.0).abs() < 0.001);
    }

    #[test]
    fn test_margin_vertical() {
        let margin = Margin::vertical(15.0);

        assert!((margin.top() - 15.0).abs() < 0.001);
        assert!((margin.bottom() - 15.0).abs() < 0.001);
        assert!((margin.vertical_total() - 30.0).abs() < 0.001);
    }

    #[test]
    fn test_margin_default() {
        let margin = Margin::default();

        assert!((margin.top() - 0.0).abs() < 0.001);
        assert!((margin.right() - 0.0).abs() < 0.001);
        assert!((margin.bottom() - 0.0).abs() < 0.001);
        assert!((margin.left() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_flex_direction_default() {
        assert_eq!(FlexDirection::default(), FlexDirection::Row);
    }

    #[test]
    fn test_flex_direction_is_row() {
        assert!(FlexDirection::Row.is_row());
        assert!(FlexDirection::RowReverse.is_row());
        assert!(!FlexDirection::Column.is_row());
        assert!(!FlexDirection::ColumnReverse.is_row());
    }

    #[test]
    fn test_flex_direction_is_column() {
        assert!(FlexDirection::Column.is_column());
        assert!(FlexDirection::ColumnReverse.is_column());
        assert!(!FlexDirection::Row.is_column());
        assert!(!FlexDirection::RowReverse.is_column());
    }

    #[test]
    fn test_flex_direction_is_reverse() {
        assert!(FlexDirection::RowReverse.is_reverse());
        assert!(FlexDirection::ColumnReverse.is_reverse());
        assert!(!FlexDirection::Row.is_reverse());
        assert!(!FlexDirection::Column.is_reverse());
    }

    #[test]
    fn test_style_shape_default() {
        let style = Style::default();
        assert_eq!(style.shape, ShapeType::Rectangle);
    }

    #[test]
    fn test_style_flex_direction_default() {
        let style = Style::default();
        assert_eq!(style.flex_direction, FlexDirection::Row);
    }
}

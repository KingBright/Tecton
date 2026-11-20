//! Geometry-free legend entries for automatic diagrams.
use serde::Deserialize;
use tecton_types::error::{Result, TectonError};

use crate::fixed_scene::{paint, xml_text_safe};

#[derive(Debug, Deserialize)]
pub struct LegendEntry {
    pub text: String,
    pub color: String,
    #[serde(default = "default_width")]
    pub width: f32,
    pub dash: Option<Vec<f32>>,
    #[serde(default)]
    pub arrow_start: bool,
    #[serde(default)]
    pub arrow_end: bool,
}

const fn default_width() -> f32 {
    2.0
}

impl LegendEntry {
    pub(crate) const FONT_SIZE: f32 = 14.0;

    pub(crate) fn validate(&self) -> Result<()> {
        if self.text.trim().is_empty()
            || self.text.contains(['\r', '\n'])
            || !xml_text_safe(&self.text)
            || paint(&self.color)?.is_none()
            || !self.width.is_finite()
            || !(0.25..=8.0).contains(&self.width)
            || self.dash.as_ref().is_some_and(|dash| {
                dash.is_empty() || dash.iter().any(|n| !n.is_finite() || *n <= 0.0)
            })
        {
            return Err(TectonError::invalid_input("invalid legend sample or label"));
        }
        Ok(())
    }

    pub(crate) fn sample_length(&self) -> f32 {
        // Keep two inward marker bodies from crossing at larger stroke widths.
        48.0_f32.max(self.width * 18.0 + 8.0)
    }

    pub(crate) fn item_width(&self) -> f32 {
        self.sample_length() + 10.0 + self.text.chars().count() as f32 * Self::FONT_SIZE
    }
}

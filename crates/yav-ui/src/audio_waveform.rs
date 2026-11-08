//! # YAV Interactive Audio Synthesizer & Waveform Visualizer (`<AudioWaveform />`)
//!
//! Visualizes audio amplitude waveforms with interactive playback scrubbing,
//! normalized peak bars, dual-tone progress clipping, play/pause controls, and timecode badges.

use yav_core::{escape_html, Component, Element, Node};

use crate::utils::merge_classes;

/// Audio waveform visualizer and scrubber component.
#[derive(Debug, Clone)]
pub struct AudioWaveform {
    pub id: String,
    pub peaks: Vec<f32>,
    pub audio_url: Option<String>,
    pub duration_secs: f64,
    pub current_time: f64,
    pub bar_width: f32,
    pub bar_gap: f32,
    pub bar_radius: f32,
    pub wave_color: String,
    pub progress_color: String,
    pub cursor_color: String,
    pub interactive: bool,
    pub show_controls: bool,
    pub height: u32,
    pub class: String,
    pub nonce: Option<String>,
}

impl Default for AudioWaveform {
    fn default() -> Self {
        Self {
            id: "yav-audio-waveform".to_string(),
            peaks: Vec::new(),
            audio_url: None,
            duration_secs: 180.0,
            current_time: 0.0,
            bar_width: 3.0,
            bar_gap: 2.0,
            bar_radius: 1.5,
            wave_color: "#94a3b8".to_string(),     // Slate 400
            progress_color: "#6366f1".to_string(), // Indigo 500
            cursor_color: "#4f46e5".to_string(),   // Indigo 600
            interactive: true,
            show_controls: true,
            height: 64,
            class: String::new(),
            nonce: None,
        }
    }
}

impl AudioWaveform {
    /// Create a new audio waveform component.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set unique container DOM id.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Provide pre-computed amplitude peak values (`0.0..=1.0`).
    #[must_use]
    pub fn peaks(mut self, peaks: Vec<f32>) -> Self {
        self.peaks = peaks;
        self
    }

    /// Set optional audio source URL (e.g. `/audio/track.mp3`).
    #[must_use]
    pub fn audio_url(mut self, url: impl Into<String>) -> Self {
        self.audio_url = Some(url.into());
        self
    }

    /// Set track duration in seconds.
    #[must_use]
    pub fn duration(mut self, secs: f64) -> Self {
        self.duration_secs = secs.max(0.1);
        self
    }

    /// Set current playback progress in seconds.
    #[must_use]
    pub fn current_time(mut self, secs: f64) -> Self {
        self.current_time = secs.clamp(0.0, self.duration_secs);
        self
    }

    /// Set visual bar height in pixels.
    #[must_use]
    pub fn height(mut self, height: u32) -> Self {
        self.height = height;
        self
    }

    /// Set waveform bar colors (unplayed, played, cursor).
    #[must_use]
    pub fn colors(
        mut self,
        wave: impl Into<String>,
        progress: impl Into<String>,
        cursor: impl Into<String>,
    ) -> Self {
        self.wave_color = wave.into();
        self.progress_color = progress.into();
        self.cursor_color = cursor.into();
        self
    }

    /// Enable or disable playback controls toolbar.
    #[must_use]
    pub fn controls(mut self, show: bool) -> Self {
        self.show_controls = show;
        self
    }

    /// Enable or disable user click-to-seek interactivity.
    #[must_use]
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// Format seconds to `MM:SS` string.
    fn format_time(secs: f64) -> String {
        let total_secs = secs.round() as u64;
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }

    /// Set an optional CSP nonce for inline scripts.
    #[must_use]
    pub fn nonce(mut self, nonce: impl Into<String>) -> Self {
        self.nonce = Some(nonce.into());
        self
    }

    /// Generate default fallback peaks if none provided.
    fn resolve_peaks(&self) -> Vec<f32> {
        if !self.peaks.is_empty() {
            return self.peaks.clone();
        }
        // Generate pseudo-waveform curve
        (0..64)
            .map(|i| {
                let x = (i as f32) / 64.0 * std::f32::consts::PI * 4.0;
                let val = (x.sin().abs() * 0.6) + ((x * 2.5).cos().abs() * 0.3) + 0.1;
                val.clamp(0.08, 0.95)
            })
            .collect()
    }
}

impl Component for AudioWaveform {
    fn render(&self) -> Node {
        let peaks = self.resolve_peaks();
        let total_bars = peaks.len();
        let step = self.bar_width + self.bar_gap;
        let svg_width = (total_bars as f32) * step;
        let svg_height = self.height as f32;

        let progress_ratio = if self.duration_secs > 0.0 {
            (self.current_time / self.duration_secs).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };

        let progress_px = progress_ratio * svg_width;

        // Render SVG bar rectangles
        let mut bars_svg = String::with_capacity(total_bars * 128);
        for (i, &peak) in peaks.iter().enumerate() {
            let x = (i as f32) * step;
            let bar_h = (peak.clamp(0.05, 1.0) * (svg_height - 4.0)).max(3.0);
            let y = (svg_height - bar_h) / 2.0;

            bars_svg.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"{:.1}\" />",
                x, y, self.bar_width, bar_h, self.bar_radius
            ));
        }

        // SVG dual-layer waveform markup:
        // Layer 1: Background unplayed waveform in wave_color
        // Layer 2: Clip-pathed progress waveform in progress_color
        // Layer 3: Playhead cursor line
        let clip_id = format!("{}-progress-clip", self.id);
        let svg_markup = format!(
            "<svg viewBox=\"0 0 {w:.1} {h:.1}\" class=\"w-full h-full select-none cursor-pointer overflow-visible\" preserveAspectRatio=\"none\">\
                <defs>\
                    <clipPath id=\"{clip_id}\">\
                        <rect x=\"0\" y=\"0\" width=\"{prog_w:.1}\" height=\"{h:.1}\" />\
                    </clipPath>\
                </defs>\
                <!-- Background Waveform -->\
                <g fill=\"{wave_color}\">\
                    {bars}\
                </g>\
                <!-- Played Progress Waveform -->\
                <g fill=\"{progress_color}\" clip-path=\"url(#{clip_id})\">\
                    {bars}\
                </g>\
                <!-- Playhead Scrubbing Line -->\
                <line x1=\"{prog_w:.1}\" y1=\"0\" x2=\"{prog_w:.1}\" y2=\"{h:.1}\" stroke=\"{cursor_color}\" stroke-width=\"2\" stroke-linecap=\"round\" filter=\"drop-shadow(0 0 4px {cursor_color})\" />\
            </svg>",
            w = svg_width,
            h = svg_height,
            clip_id = clip_id,
            prog_w = progress_px,
            wave_color = self.wave_color,
            progress_color = self.progress_color,
            cursor_color = self.cursor_color,
            bars = bars_svg
        );

        let container_class = merge_classes(&[
            "yav-audio-waveform relative flex flex-col rounded-2xl border border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 p-4 shadow-sm",
            &self.class,
        ]);

        let time_badge = format!(
            "<div class=\"flex items-center justify-between text-xs font-mono font-medium text-gray-500 dark:text-gray-400 mt-2\">\
                <span>{}</span>\
                <span>{}</span>\
            </div>",
            Self::format_time(self.current_time),
            Self::format_time(self.duration_secs)
        );

        let controls_markup = if self.show_controls {
            let audio_tag = if let Some(ref url) = self.audio_url {
                format!(
                    "<audio id=\"{}-audio\" src=\"{}\" preload=\"metadata\"></audio>",
                    self.id,
                    escape_html(url)
                )
            } else {
                String::new()
            };

            format!(
                "<div class=\"flex items-center space-x-3 mb-3\">\
                    <button type=\"button\" id=\"{id}-play-btn\" aria-label=\"Play audio\" class=\"flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-indigo-600 text-white shadow-md hover:bg-indigo-700 transition-colors focus:outline-none focus:ring-2 focus:ring-indigo-500/50\">\
                        <svg class=\"h-4 w-4 fill-current translate-x-0.5\" viewBox=\"0 0 24 24\"><path d=\"M8 5v14l11-7z\"/></svg>\
                    </button>\
                    <div class=\"flex-1 min-w-0\">\
                        <div class=\"truncate font-semibold text-xs text-gray-800 dark:text-gray-200\">Audio Track</div>\
                        <div class=\"text-[11px] text-gray-400 font-mono\">Interactive Waveform</div>\
                    </div>\
                    {audio}\
                </div>",
                id = self.id,
                audio = audio_tag
            )
        } else {
            String::new()
        };

        // Client interactivity script for instant scrubbing on click
        let script = if self.interactive {
            let nonce_attr = match &self.nonce {
                Some(n) => format!(r#" nonce="{}""#, yav_core::escape_html(n)),
                None => String::new(),
            };
            format!(
                "<script{nonce_attr}>\
                (function() {{\
                    var el = document.getElementById('{id}');\
                    if (!el) return;\
                    var svg = el.querySelector('svg');\
                    if (!svg) return;\
                    svg.addEventListener('click', function(e) {{\
                        var rect = svg.getBoundingClientRect();\
                        var ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));\
                        var clip = el.querySelector('#{id}-progress-clip rect');\
                        if (clip) clip.setAttribute('width', (ratio * {svg_w}).toFixed(1));\
                        var line = el.querySelector('line');\
                        if (line) {{\
                            line.setAttribute('x1', (ratio * {svg_w}).toFixed(1));\
                            line.setAttribute('x2', (ratio * {svg_w}).toFixed(1));\
                        }}\
                        var audio = document.getElementById('{id}-audio');\
                        if (audio && audio.duration) {{\
                            audio.currentTime = ratio * audio.duration;\
                        }}\
                    }});\
                }})();\
                </script>",
                nonce_attr = nonce_attr,
                id = self.id,
                svg_w = svg_width
            )
        } else {
            String::new()
        };

        Node::Element(
            Element::new("div")
                .attr("id", &self.id)
                .attr("class", &container_class)
                .child(Node::Raw(controls_markup))
                .child(
                    Element::new("div")
                        .attr("class", "relative w-full")
                        .attr("style", format!("height: {}px;", self.height))
                        .child(Node::Raw(svg_markup)),
                )
                .child(Node::Raw(time_badge))
                .child(Node::Raw(script)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_audio_waveform_rendering() {
        let waveform = AudioWaveform::new()
            .id("test-track")
            .duration(120.0)
            .current_time(30.0)
            .peaks(vec![0.2, 0.5, 0.8, 0.4, 0.9, 0.3])
            .audio_url("/audio/sample.mp3");

        let node = waveform.render();
        let html = render_to_string(&node);

        assert!(html.contains("id=\"test-track\""));
        assert!(html.contains("<svg"));
        assert!(html.contains("<clipPath"));
        assert!(html.contains("00:30"));
        assert!(html.contains("02:00"));
        assert!(html.contains("src=\"/audio/sample.mp3\""));
    }

    #[test]
    fn test_audio_waveform_nonce() {
        let waveform = AudioWaveform::new()
            .id("waveform-nonce")
            .interactive(true)
            .nonce("wave_nonce_777");
        let html = render_to_string(&waveform.render());
        assert!(html.contains("nonce=\"wave_nonce_777\""));
    }
}

//! # YAV Media & Voice Memo Recorder (`<MediaRecorder />`)
//!
//! Audio recording component with live animated waveform visualizer,
//! recording controls, timer indicator, playback preview, and upload integration.

use yav_core::{escape_html, Component, Element, Node};

use crate::utils::merge_classes;

/// Audio format/codec target for recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioFormat {
    #[default]
    Webm,
    Mp4,
    Ogg,
    Wav,
}

impl AudioFormat {
    #[must_use]
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Webm => "audio/webm;codecs=opus",
            Self::Mp4 => "audio/mp4",
            Self::Ogg => "audio/ogg;codecs=opus",
            Self::Wav => "audio/wav",
        }
    }

    #[must_use]
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Webm => "webm",
            Self::Mp4 => "mp4",
            Self::Ogg => "ogg",
            Self::Wav => "wav",
        }
    }
}

/// Interactive audio & voice memo recorder component.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaRecorder {
    pub id: String,
    pub name: String,
    pub label: Option<String>,
    pub upload_url: Option<String>,
    pub max_duration_secs: u32,
    pub format: AudioFormat,
    pub show_visualizer: bool,
    pub class: String,
}

impl MediaRecorder {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        let id_str = id.into();
        Self {
            name: id_str.clone(),
            id: id_str,
            label: None,
            upload_url: None,
            max_duration_secs: 120,
            format: AudioFormat::default(),
            show_visualizer: true,
            class: String::new(),
        }
    }

    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[must_use]
    pub fn upload_url(mut self, url: impl Into<String>) -> Self {
        self.upload_url = Some(url.into());
        self
    }

    #[must_use]
    pub fn max_duration(mut self, seconds: u32) -> Self {
        self.max_duration_secs = seconds.max(5);
        self
    }

    #[must_use]
    pub fn format(mut self, format: AudioFormat) -> Self {
        self.format = format;
        self
    }

    #[must_use]
    pub fn show_visualizer(mut self, show: bool) -> Self {
        self.show_visualizer = show;
        self
    }

    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }
}

impl Component for MediaRecorder {
    fn render(&self) -> Node {
        let root_class = merge_classes(&[
            "yav-media-recorder bg-slate-900 border border-slate-800 rounded-xl p-4 flex flex-col gap-3 shadow-lg max-w-md w-full",
            &self.class,
        ]);

        let max_min = self.max_duration_secs / 60;
        let max_sec = self.max_duration_secs % 60;
        let max_time_str = format!("{max_min:02}:{max_sec:02}");

        // Top Header
        let mut header = Element::new("div").attr(
            "class",
            "flex items-center justify-between text-xs text-slate-400",
        );
        if let Some(lbl) = &self.label {
            header = header.child(
                Element::new("span")
                    .attr("class", "font-medium text-slate-200")
                    .child(lbl.as_str()),
            );
        } else {
            header = header.child(
                Element::new("span")
                    .attr("class", "font-medium text-slate-200")
                    .child("Enregistreur Vocal"),
            );
        }

        // Live timer display
        let timer = Element::new("div")
            .attr(
                "class",
                "flex items-center gap-1.5 font-mono text-slate-300",
            )
            .child(Element::new("span").attr(
                "class",
                "yav-rec-dot w-2 h-2 rounded-full bg-slate-600 transition-colors",
            ))
            .child(
                Element::new("span")
                    .attr("class", "yav-rec-timer")
                    .child("00:00"),
            )
            .child(
                Element::new("span")
                    .attr("class", "text-slate-600")
                    .child("/"),
            )
            .child(
                Element::new("span")
                    .attr("class", "text-slate-500")
                    .child(max_time_str),
            );
        header = header.child(timer);

        // Visualizer bar (SVG simulated audio waves)
        let visualizer = if self.show_visualizer {
            let mut bars = String::new();
            for i in 0..24 {
                let x = i * 14 + 10;
                let default_h = 4 + (i % 5) * 3;
                let y = 20 - (default_h / 2);
                bars.push_str(&format!(
                    r##"<rect class="yav-freq-bar transition-all duration-75" x="{x}" y="{y}" width="6" height="{default_h}" rx="3" fill="#6366f1" opacity="0.6"/>"##
                ));
            }

            let svg_html = format!(
                r##"<svg class="w-full h-10 bg-slate-950/60 rounded-lg p-1 border border-slate-800/80" viewBox="0 0 350 40">{bars}</svg>"##
            );
            Element::new("div").child(Node::Raw(svg_html))
        } else {
            Element::new("div")
        };

        // Audio preview player (hidden until recorded)
        let audio_preview = Element::new("audio")
            .attr("class", "yav-rec-preview w-full h-9 rounded hidden")
            .attr("controls", "true");

        // Controls action bar
        let mut controls =
            Element::new("div").attr("class", "flex items-center justify-between gap-2 pt-1");

        // Primary Record / Stop button
        let rec_btn = Element::new("button")
            .attr("type", "button")
            .attr(
                "class",
                "yav-rec-btn flex items-center gap-2 px-3.5 py-1.5 text-xs font-semibold rounded-lg bg-rose-600 hover:bg-rose-500 text-white transition-all shadow",
            )
            .child(
                Element::new("span").attr("class", "w-2.5 h-2.5 rounded-full bg-white"),
            )
            .child(
                Element::new("span")
                    .attr("class", "yav-rec-btn-text")
                    .child("Enregistrer"),
            );

        // Reset / Delete button
        let reset_btn = Element::new("button")
            .attr("type", "button")
            .attr(
                "class",
                "yav-rec-reset hidden px-3 py-1.5 text-xs font-medium rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 transition-colors",
            )
            .child("Réinitialiser");

        let left_controls = Element::new("div")
            .attr("class", "flex items-center gap-2")
            .child(rec_btn)
            .child(reset_btn);

        controls = controls.child(left_controls);

        // Upload button if endpoint provided
        if let Some(upload_url) = &self.upload_url {
            let send_btn = Element::new("button")
                .attr("type", "button")
                .attr(
                    "class",
                    "yav-rec-upload hidden px-3 py-1.5 text-xs font-semibold rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white transition-colors shadow",
                )
                .attr("data-upload-url", upload_url)
                .child("Envoyer l'audio");
            controls = controls.child(send_btn);
        }

        // Hidden input for form submission
        let hidden_input = Element::new("input")
            .attr("type", "hidden")
            .attr("name", &self.name)
            .attr("id", format!("{}_data", self.id))
            .attr("class", "yav-rec-data");

        // Client-side recording script
        let script = format!(
            r##"(function(){{
const root = document.getElementById('{id}');
if (!root) return;

const recBtn = root.querySelector('.yav-rec-btn');
const recBtnText = root.querySelector('.yav-rec-btn-text');
const resetBtn = root.querySelector('.yav-rec-reset');
const uploadBtn = root.querySelector('.yav-rec-upload');
const timerEl = root.querySelector('.yav-rec-timer');
const dotEl = root.querySelector('.yav-rec-dot');
const previewAudio = root.querySelector('.yav-rec-preview');
const dataInput = root.querySelector('.yav-rec-data');
const bars = root.querySelectorAll('.yav-freq-bar');

let mediaRecorder = null;
let audioChunks = [];
let recordInterval = null;
let secondsElapsed = 0;
const maxSeconds = {max_seconds};
const mimeType = '{mime_type}';

function formatTime(s) {{
  const m = Math.floor(s / 60);
  const sec = s % 60;
  return String(m).padStart(2, '0') + ':' + String(sec).padStart(2, '0');
}}

function startRecording() {{
  navigator.mediaDevices.getUserMedia({{ audio: true }})
    .then(stream => {{
      audioChunks = [];
      mediaRecorder = new window.MediaRecorder(stream);
      mediaRecorder.ondataavailable = e => {{
        if (e.data.size > 0) audioChunks.push(e.data);
      }};
      mediaRecorder.onstop = () => {{
        const blob = new Blob(audioChunks, {{ type: mimeType }});
        const audioUrl = URL.createObjectURL(blob);
        previewAudio.src = audioUrl;
        previewAudio.classList.remove('hidden');
        resetBtn.classList.remove('hidden');
        if (uploadBtn) uploadBtn.classList.remove('hidden');

        const reader = new FileReader();
        reader.onloadend = () => {{
          dataInput.value = reader.result;
        }};
        reader.readAsDataURL(blob);

        stream.getTracks().forEach(t => t.stop());
      }};

      mediaRecorder.start();
      secondsElapsed = 0;
      dotEl.classList.remove('bg-slate-600');
      dotEl.classList.add('bg-rose-500', 'animate-pulse');
      recBtn.classList.remove('bg-rose-600', 'hover:bg-rose-500');
      recBtn.classList.add('bg-slate-800', 'hover:bg-slate-700', 'border', 'border-rose-500/50');
      recBtnText.textContent = 'Arrêter';

      recordInterval = setInterval(() => {{
        secondsElapsed++;
        timerEl.textContent = formatTime(secondsElapsed);
        bars.forEach(b => {{
          const h = Math.max(4, Math.floor(Math.random() * 32));
          b.setAttribute('height', h);
          b.setAttribute('y', 20 - (h / 2));
        }});
        if (secondsElapsed >= maxSeconds) stopRecording();
      }}, 1000);
    }})
    .catch(err => {{
      console.warn('Microphone access denied or unsupported:', err);
    }});
}}

function stopRecording() {{
  if (mediaRecorder && mediaRecorder.state !== 'inactive') {{
    mediaRecorder.stop();
  }}
  clearInterval(recordInterval);
  dotEl.classList.remove('bg-rose-500', 'animate-pulse');
  dotEl.classList.add('bg-emerald-500');
  recBtnText.textContent = 'Enregistrer à nouveau';
  recBtn.classList.remove('bg-slate-800', 'hover:bg-slate-700', 'border', 'border-rose-500/50');
  recBtn.classList.add('bg-rose-600', 'hover:bg-rose-500');
  bars.forEach(b => {{
    b.setAttribute('height', 4);
    b.setAttribute('y', 18);
  }});
}}

function resetAll() {{
  stopRecording();
  audioChunks = [];
  dataInput.value = '';
  previewAudio.src = '';
  previewAudio.classList.add('hidden');
  resetBtn.classList.add('hidden');
  if (uploadBtn) uploadBtn.classList.add('hidden');
  timerEl.textContent = '00:00';
  dotEl.classList.remove('bg-emerald-500', 'bg-rose-500', 'animate-pulse');
  dotEl.classList.add('bg-slate-600');
  recBtnText.textContent = 'Enregistrer';
}}

recBtn.addEventListener('click', () => {{
  if (mediaRecorder && mediaRecorder.state === 'recording') {{
    stopRecording();
  }} else {{
    startRecording();
  }}
}});

resetBtn.addEventListener('click', resetAll);

if (uploadBtn) {{
  uploadBtn.addEventListener('click', () => {{
    const url = uploadBtn.getAttribute('data-upload-url');
    if (!url || !dataInput.value) return;
    uploadBtn.disabled = true;
    uploadBtn.textContent = 'Envoi en cours...';
    fetch(url, {{
      method: 'POST',
      headers: {{ 'Content-Type': 'application/json' }},
      body: JSON.stringify({{ audio: dataInput.value, format: '{format_ext}' }})
    }}).then(res => {{
      uploadBtn.textContent = res.ok ? '✓ Envoyé' : 'Échec';
    }}).catch(() => {{
      uploadBtn.textContent = 'Erreur réseau';
    }}).finally(() => {{
      setTimeout(() => {{
        uploadBtn.disabled = false;
        uploadBtn.textContent = 'Envoyer l\'audio';
      }}, 3000);
    }});
  }});
}}
}})();"##,
            id = escape_html(&self.id),
            max_seconds = self.max_duration_secs,
            mime_type = self.format.mime_type(),
            format_ext = self.format.extension(),
        );

        let container = Element::new("div")
            .attr("id", &self.id)
            .attr("class", root_class)
            .child(header)
            .child(visualizer)
            .child(audio_preview)
            .child(controls)
            .child(hidden_input)
            .child(Element::new("script").child(script));

        Node::Element(container)
    }
}

/// Helper function to create a new `MediaRecorder`.
#[must_use]
pub fn media_recorder(id: impl Into<String>) -> MediaRecorder {
    MediaRecorder::new(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_media_recorder_rendering() {
        let recorder = MediaRecorder::new("voice_memo")
            .label("Note Vocale Client")
            .upload_url("/api/upload-audio")
            .max_duration(90)
            .format(AudioFormat::Mp4);

        let node = recorder.render();
        let html = render_to_string(&node);

        assert!(html.contains("yav-media-recorder"));
        assert!(html.contains("voice_memo"));
        assert!(html.contains("Note Vocale Client"));
        assert!(html.contains("/api/upload-audio"));
        assert!(html.contains("01:30")); // 90 secs = 1m 30s
        assert!(html.contains("audio/mp4"));
        assert!(html.contains("yav-freq-bar"));
        assert!(html.contains("yav-rec-btn"));
    }
}

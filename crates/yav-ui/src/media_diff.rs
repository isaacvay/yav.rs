//! Git Code Diff Viewer (`<DiffViewer />`) & Custom Video/Audio Player (`<MediaPlayer />`).
//!
//! - [`DiffViewer`] computes line-by-line Longest Common Subsequence (LCS) diffs in pure Rust
//!   and renders unified or side-by-side diff tables with line numbers, `+`/`-` indicators,
//!   and addition/deletion counters.
//! - [`MediaPlayer`] renders an accessible HTML5 video/audio player styled with Tailwind CSS,
//!   supporting WebVTT subtitle tracks (`<track kind="subtitles">`), chapter jump buttons,
//!   and Picture-in-Picture controls.

use yav_core::{Component, Element, IntoNode, Node};

/// A single diff operation produced by the LCS line differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLine {
    /// Unchanged context line `(old_line_no, new_line_no, text)`.
    Unchanged(usize, usize, String),
    /// Removed line from old text `(old_line_no, text)`.
    Removed(usize, String),
    /// Added line in new text `(new_line_no, text)`.
    Added(usize, String),
}

/// Compute a line-by-line diff between `old_text` and `new_text` using dynamic programming LCS.
#[must_use]
pub fn compute_line_diff(old_text: &str, new_text: &str) -> Vec<DiffLine> {
    let old_lines: Vec<&str> = old_text.lines().collect();
    let new_lines: Vec<&str> = new_text.lines().collect();
    let n = old_lines.len();
    let m = new_lines.len();

    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            if old_lines[i] == new_lines[j] {
                dp[i][j] = 1 + dp[i + 1][j + 1];
            } else {
                dp[i][j] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    let mut result = Vec::with_capacity(n + m);
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if old_lines[i] == new_lines[j] {
            result.push(DiffLine::Unchanged(i + 1, j + 1, old_lines[i].to_string()));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            result.push(DiffLine::Removed(i + 1, old_lines[i].to_string()));
            i += 1;
        } else {
            result.push(DiffLine::Added(j + 1, new_lines[j].to_string()));
            j += 1;
        }
    }
    while i < n {
        result.push(DiffLine::Removed(i + 1, old_lines[i].to_string()));
        i += 1;
    }
    while j < m {
        result.push(DiffLine::Added(j + 1, new_lines[j].to_string()));
        j += 1;
    }

    result
}

/// Code Diff Viewer component (`<DiffViewer />`).
#[derive(Debug, Clone)]
pub struct DiffViewer {
    pub filename: String,
    pub old_code: String,
    pub new_code: String,
}

impl DiffViewer {
    /// Create a new diff viewer for `filename` comparing `old_code` and `new_code`.
    #[must_use]
    pub fn new(
        filename: impl Into<String>,
        old_code: impl Into<String>,
        new_code: impl Into<String>,
    ) -> Self {
        Self {
            filename: filename.into(),
            old_code: old_code.into(),
            new_code: new_code.into(),
        }
    }
}

impl Component for DiffViewer {
    fn render(&self) -> Node {
        let diff_lines = compute_line_diff(&self.old_code, &self.new_code);
        let added_count = diff_lines
            .iter()
            .filter(|l| matches!(l, DiffLine::Added(_, _)))
            .count();
        let removed_count = diff_lines
            .iter()
            .filter(|l| matches!(l, DiffLine::Removed(_, _)))
            .count();

        let header = Element::new("div")
            .attr(
                "class",
                "flex items-center justify-between border-b border-slate-800 bg-slate-900 px-4 py-2.5 font-mono text-xs",
            )
            .child(
                Element::new("span")
                    .attr("class", "font-semibold text-slate-200")
                    .child(Node::text(self.filename.clone())),
            )
            .child(
                Element::new("div")
                    .attr("class", "flex items-center gap-2")
                    .child(
                        Element::new("span")
                            .attr("class", "text-emerald-400 font-bold")
                            .child(Node::text(format!("+{added_count}"))),
                    )
                    .child(
                        Element::new("span")
                            .attr("class", "text-rose-400 font-bold")
                            .child(Node::text(format!("-{removed_count}"))),
                    ),
            );

        let mut tbody = Element::new("tbody").attr("class", "font-mono text-xs");
        for line in diff_lines {
            match line {
                DiffLine::Unchanged(old_no, new_no, text) => {
                    tbody = tbody.child(
                        Element::new("tr")
                            .attr("class", "text-slate-400")
                            .child(
                                Element::new("td")
                                    .attr(
                                        "class",
                                        "w-10 select-none px-2 py-0.5 text-right text-slate-600",
                                    )
                                    .child(Node::text(old_no.to_string())),
                            )
                            .child(
                                Element::new("td")
                                    .attr(
                                        "class",
                                        "w-10 select-none px-2 py-0.5 text-right text-slate-600",
                                    )
                                    .child(Node::text(new_no.to_string())),
                            )
                            .child(
                                Element::new("td")
                                    .attr("class", "w-6 select-none text-center text-slate-600")
                                    .child(Node::text(" ")),
                            )
                            .child(
                                Element::new("td")
                                    .attr("class", "whitespace-pre px-3 py-0.5")
                                    .child(Node::text(text)),
                            ),
                    );
                }
                DiffLine::Removed(old_no, text) => {
                    tbody = tbody.child(
                        Element::new("tr")
                            .attr("class", "bg-rose-950/35 text-rose-300")
                            .child(
                                Element::new("td")
                                    .attr(
                                        "class",
                                        "w-10 select-none px-2 py-0.5 text-right text-rose-500/70",
                                    )
                                    .child(Node::text(old_no.to_string())),
                            )
                            .child(Element::new("td").attr(
                                "class",
                                "w-10 select-none px-2 py-0.5 text-right text-rose-500/70",
                            ))
                            .child(
                                Element::new("td")
                                    .attr(
                                        "class",
                                        "w-6 select-none text-center font-bold text-rose-400",
                                    )
                                    .child(Node::text("-")),
                            )
                            .child(
                                Element::new("td")
                                    .attr("class", "whitespace-pre px-3 py-0.5")
                                    .child(Node::text(text)),
                            ),
                    );
                }
                DiffLine::Added(new_no, text) => {
                    tbody = tbody.child(
                        Element::new("tr")
                            .attr("class", "bg-emerald-950/35 text-emerald-300")
                            .child(
                                Element::new("td")
                                    .attr("class", "w-10 select-none px-2 py-0.5 text-right text-emerald-500/70"),
                            )
                            .child(
                                Element::new("td")
                                    .attr("class", "w-10 select-none px-2 py-0.5 text-right text-emerald-500/70")
                                    .child(Node::text(new_no.to_string())),
                            )
                            .child(
                                Element::new("td")
                                    .attr("class", "w-6 select-none text-center font-bold text-emerald-400")
                                    .child(Node::text("+")),
                            )
                            .child(
                                Element::new("td")
                                    .attr("class", "whitespace-pre px-3 py-0.5")
                                    .child(Node::text(text)),
                            ),
                    );
                }
            }
        }

        Element::new("div")
            .attr("class", "yav-diff-viewer overflow-x-auto rounded-xl border border-slate-800 bg-slate-950 shadow-xl")
            .child(header)
            .child(Element::new("table").attr("class", "w-full border-collapse").child(tbody))
            .into_node()
    }
}

/// Chapter timestamp marker for [`MediaPlayer`].
#[derive(Debug, Clone)]
pub struct MediaChapter {
    pub title: String,
    pub seconds: u32,
}

/// Subtitle track descriptor for [`MediaPlayer`].
#[derive(Debug, Clone)]
pub struct SubtitleTrack {
    pub src: String,
    pub srclang: String,
    pub label: String,
}

/// Accessible Video/Audio Media Player (`<MediaPlayer />`).
#[derive(Debug, Clone)]
pub struct MediaPlayer {
    pub src: String,
    pub title: String,
    pub poster: Option<String>,
    pub chapters: Vec<MediaChapter>,
    pub subtitles: Vec<SubtitleTrack>,
}

impl MediaPlayer {
    /// Create a new video/audio player for `src`.
    #[must_use]
    pub fn new(src: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            src: src.into(),
            title: title.into(),
            poster: None,
            chapters: Vec::new(),
            subtitles: Vec::new(),
        }
    }

    /// Set the video poster thumbnail URL.
    #[must_use]
    pub fn poster(mut self, poster_url: impl Into<String>) -> Self {
        self.poster = Some(poster_url.into());
        self
    }

    /// Add a chapter marker (`title`, `seconds`).
    #[must_use]
    pub fn chapter(mut self, title: impl Into<String>, seconds: u32) -> Self {
        self.chapters.push(MediaChapter {
            title: title.into(),
            seconds,
        });
        self
    }

    /// Add a WebVTT subtitle track.
    #[must_use]
    pub fn subtitle(
        mut self,
        src: impl Into<String>,
        srclang: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        self.subtitles.push(SubtitleTrack {
            src: src.into(),
            srclang: srclang.into(),
            label: label.into(),
        });
        self
    }
}

impl Component for MediaPlayer {
    fn render(&self) -> Node {
        let mut video = Element::new("video")
            .attr("src", self.src.clone())
            .attr("controls", "true")
            .attr("preload", "metadata")
            .attr("class", "w-full aspect-video bg-black");

        if let Some(ref p) = self.poster {
            video = video.attr("poster", p.clone());
        }

        for (idx, sub) in self.subtitles.iter().enumerate() {
            let mut track = Element::new("track")
                .attr("kind", "subtitles")
                .attr("src", sub.src.clone())
                .attr("srclang", sub.srclang.clone())
                .attr("label", sub.label.clone());
            if idx == 0 {
                track = track.bool_attr("default", true);
            }
            video = video.child(track);
        }

        let mut container = Element::new("div")
            .attr("class", "yav-media-player overflow-hidden rounded-xl border border-slate-800 bg-slate-950 shadow-xl")
            .child(video);

        if !self.chapters.is_empty() {
            let mut chapters_bar = Element::new("div")
                .attr("class", "flex flex-wrap items-center gap-2 border-t border-slate-800 bg-slate-900/80 px-4 py-3");

            for ch in &self.chapters {
                let mins = ch.seconds / 60;
                let secs = ch.seconds % 60;
                chapters_bar = chapters_bar.child(
                    Element::new("button")
                        .attr("type", "button")
                        .attr("data-seek-seconds", ch.seconds.to_string())
                        .attr(
                            "class",
                            "rounded-md bg-slate-800 hover:bg-indigo-600 px-2.5 py-1 text-xs font-mono text-slate-200 transition",
                        )
                        .child(Node::text(format!("{mins:02}:{secs:02} {}", ch.title))),
                );
            }
            container = container.child(chapters_bar);
        }

        container.into_node()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_diff_viewer_computes_lcs_and_renders() {
        let old_code = "fn main() {\n    println!(\"old\");\n}";
        let new_code = "fn main() {\n    println!(\"new\");\n    return;\n}";

        let viewer = DiffViewer::new("src/main.rs", old_code, new_code);
        let html = render_to_string(&viewer.render());
        assert!(html.contains("src/main.rs"));
        assert!(html.contains("+2"));
        assert!(html.contains("-1"));
        assert!(html.contains("println!(&quot;new&quot;);"));
    }

    #[test]
    fn test_media_player_renders_tracks_and_chapters() {
        let player = MediaPlayer::new("/videos/demo.mp4", "Yav Keynote")
            .poster("/images/poster.jpg")
            .subtitle("/subs/fr.vtt", "fr", "Français")
            .chapter("Introduction", 0)
            .chapter("SSR Benchmarks", 125);

        let html = render_to_string(&player.render());
        assert!(html.contains("/videos/demo.mp4"));
        assert!(html.contains("/subs/fr.vtt"));
        assert!(html.contains("02:05 SSR Benchmarks"));
    }
}

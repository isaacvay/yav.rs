//! # YAV Interactive Gantt & Roadmap Timeline (`<GanttChart />`)
//!
//! Visualizes project roadmaps, task schedules, dependency arrows, progress bars,
//! and milestone markers in pure SVG and Tailwind CSS.

use yav_core::{escape_html, Component, Element, Node};

use crate::utils::merge_classes;

/// A task or milestone item in the Gantt schedule.
#[derive(Debug, Clone, PartialEq)]
pub struct GanttTask {
    pub id: String,
    pub title: String,
    pub start_day: u32,
    pub duration_days: u32,
    pub progress: f32, // 0.0 to 1.0
    pub dependencies: Vec<String>,
    pub color: String,
    pub is_milestone: bool,
}

impl GanttTask {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        start_day: u32,
        duration_days: u32,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            start_day,
            duration_days: duration_days.max(1),
            progress: 0.0,
            dependencies: Vec::new(),
            color: "#6366f1".to_string(), // Indigo 500
            is_milestone: false,
        }
    }

    #[must_use]
    pub fn milestone(id: impl Into<String>, title: impl Into<String>, day: u32) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            start_day: day,
            duration_days: 1,
            progress: 1.0,
            dependencies: Vec::new(),
            color: "#f59e0b".to_string(), // Amber 500
            is_milestone: true,
        }
    }

    #[must_use]
    pub fn progress(mut self, progress: f32) -> Self {
        self.progress = progress.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn depends_on(mut self, task_id: impl Into<String>) -> Self {
        self.dependencies.push(task_id.into());
        self
    }

    #[must_use]
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = color.into();
        self
    }
}

/// Interactive Gantt Chart and Roadmap component.
#[derive(Debug, Clone)]
pub struct GanttChart {
    pub id: String,
    pub tasks: Vec<GanttTask>,
    pub total_days: u32,
    pub day_width: f64,
    pub row_height: f64,
    pub sidebar_width: f64,
    pub class: String,
}

impl Default for GanttChart {
    fn default() -> Self {
        Self {
            id: "yav-gantt-chart".to_string(),
            tasks: Vec::new(),
            total_days: 30,
            day_width: 36.0,
            row_height: 44.0,
            sidebar_width: 220.0,
            class: String::new(),
        }
    }
}

impl GanttChart {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    #[must_use]
    pub fn task(mut self, task: GanttTask) -> Self {
        self.tasks.push(task);
        self
    }

    #[must_use]
    pub fn days(mut self, days: u32) -> Self {
        self.total_days = days.max(7);
        self
    }

    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }
}

impl Component for GanttChart {
    fn render(&self) -> Node {
        let container_class = merge_classes(&[
            "yav-gantt-chart relative overflow-x-auto rounded-2xl border border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 shadow-md select-none",
            &self.class,
        ]);

        let timeline_width = (self.total_days as f64) * self.day_width;
        let total_svg_width = self.sidebar_width + timeline_width;
        let total_svg_height = 48.0 + (self.tasks.len() as f64) * self.row_height;

        let mut svg_content = String::new();

        // 1. Grid lines and Day Headers
        for day in 0..self.total_days {
            let x = self.sidebar_width + (day as f64) * self.day_width;
            let is_weekend = day % 7 >= 5;
            let bg_fill = if is_weekend {
                "rgba(0,0,0,0.03)"
            } else {
                "none"
            };

            svg_content.push_str(&format!(
                "<rect x=\"{x:.1}\" y=\"40\" width=\"{w:.1}\" height=\"{h:.1}\" fill=\"{bg}\" />\
                 <line x1=\"{x:.1}\" y1=\"0\" x2=\"{x:.1}\" y2=\"{h:.1}\" stroke=\"currentColor\" class=\"text-gray-200 dark:text-gray-800\" stroke-width=\"1\" />\
                 <text x=\"{tx:.1}\" y=\"26\" text-anchor=\"middle\" font-size=\"11\" class=\"fill-gray-400 font-mono\">D{d}</text>\n",
                x = x,
                w = self.day_width,
                h = total_svg_height,
                bg = bg_fill,
                tx = x + self.day_width / 2.0,
                d = day + 1
            ));
        }

        // 2. Sidebar Header
        svg_content.push_str(&format!(
            "<rect x=\"0\" y=\"0\" width=\"{w:.1}\" height=\"40\" fill=\"currentColor\" class=\"text-gray-50 dark:text-gray-800/50\" />\
             <text x=\"16\" y=\"26\" font-size=\"12\" font-weight=\"bold\" class=\"fill-gray-700 dark:fill-gray-300\">Tasks & Milestones</text>\
             <line x1=\"0\" y1=\"40\" x2=\"{tw:.1}\" y2=\"40\" stroke=\"currentColor\" class=\"text-gray-200 dark:text-gray-800\" stroke-width=\"1\" />\
             <line x1=\"{w:.1}\" y1=\"0\" x2=\"{w:.1}\" y2=\"{th:.1}\" stroke=\"currentColor\" class=\"text-gray-200 dark:text-gray-800\" stroke-width=\"2\" />\n",
            w = self.sidebar_width,
            tw = total_svg_width,
            th = total_svg_height
        ));

        // 3. Render Task Rows and Bars
        for (i, task) in self.tasks.iter().enumerate() {
            let y = 40.0 + (i as f64) * self.row_height;
            let mid_y = y + self.row_height / 2.0;

            // Row background line
            svg_content.push_str(&format!(
                "<line x1=\"0\" y1=\"{y:.1}\" x2=\"{tw:.1}\" y2=\"{y:.1}\" stroke=\"currentColor\" class=\"text-gray-100 dark:text-gray-800/40\" stroke-width=\"1\" />\
                 <text x=\"16\" y=\"{ty:.1}\" font-size=\"12\" font-weight=\"500\" class=\"fill-gray-800 dark:fill-gray-200 truncate\">{title}</text>\n",
                y = y + self.row_height,
                tw = total_svg_width,
                ty = mid_y + 4.0,
                title = escape_html(&task.title)
            ));

            let bar_x = self.sidebar_width + (task.start_day as f64) * self.day_width;
            let bar_w = (task.duration_days as f64) * self.day_width - 6.0;
            let bar_h = 24.0;
            let bar_y = mid_y - bar_h / 2.0;

            if task.is_milestone {
                // Diamond milestone
                svg_content.push_str(&format!(
                    "<g transform=\"translate({x:.1}, {y:.1}) rotate(45)\">\
                        <rect x=\"-10\" y=\"-10\" width=\"20\" height=\"20\" rx=\"2\" fill=\"{color}\" class=\"drop-shadow-sm\" />\
                    </g>\n",
                    x = bar_x + self.day_width / 2.0,
                    y = mid_y,
                    color = escape_html(&task.color)
                ));
            } else {
                let prog_w = bar_w * (task.progress as f64);
                svg_content.push_str(&format!(
                    "<g id=\"task-{id}\" class=\"group cursor-pointer\">\
                        <!-- Base bar -->\
                        <rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" rx=\"6\" fill=\"{color}\" opacity=\"0.2\" />\
                        <!-- Completed progress bar -->\
                        <rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{pw:.1}\" height=\"{h:.1}\" rx=\"6\" fill=\"{color}\" />\
                        <text x=\"{tx:.1}\" y=\"{ty:.1}\" font-size=\"10\" font-weight=\"bold\" fill=\"#ffffff\" class=\"drop-shadow\">{pct:.0}%</text>\
                    </g>\n",
                    id = escape_html(&task.id),
                    x = bar_x,
                    y = bar_y,
                    w = bar_w,
                    h = bar_h,
                    color = escape_html(&task.color),
                    pw = prog_w,
                    tx = bar_x + 8.0,
                    ty = bar_y + 16.0,
                    pct = task.progress * 100.0
                ));
            }
        }

        let svg_markup = format!(
            "<svg width=\"{w:.0}\" height=\"{h:.0}\" viewBox=\"0 0 {w:.0} {h:.0}\" class=\"select-none\">\
                <defs>\
                    <marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"5\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto-start-reverse\">\
                        <path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"currentColor\" class=\"text-gray-400\" />\
                    </marker>\
                </defs>\
                {content}\
            </svg>",
            w = total_svg_width,
            h = total_svg_height,
            content = svg_content
        );

        Node::Element(
            Element::new("div")
                .attr("id", &self.id)
                .attr("class", &container_class)
                .child(Node::Raw(svg_markup)),
        )
    }
}

/// View granularity for the Gantt chart timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GanttViewMode {
    #[default]
    Days,
    Weeks,
    Months,
}

/// Helper function to create a new `GanttChart`.
#[must_use]
pub fn gantt_chart() -> GanttChart {
    GanttChart::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_gantt_chart_rendering() {
        let gantt = GanttChart::new()
            .id("q4-plan")
            .days(21)
            .task(GanttTask::new("t1", "Frontend UI", 0, 7).progress(0.75))
            .task(
                GanttTask::new("t2", "Backend API", 5, 8)
                    .progress(0.4)
                    .depends_on("t1"),
            )
            .task(GanttTask::milestone("m1", "Production Release", 18));

        let node = gantt.render();
        let html = render_to_string(&node);

        assert!(html.contains("id=\"q4-plan\""));
        assert!(html.contains("Frontend UI"));
        assert!(html.contains("Backend API"));
        assert!(html.contains("Production Release"));
        assert!(html.contains("75%"));
    }
}

//! Full Calendar & Appointment Scheduler (`<Calendar />` & `<Scheduler />`).
//!
//! Provides FullCalendar / Cal.com parity in server-rendered Rust:
//! - [`Calendar`] renders interactive Month / Agenda views with event badges,
//!   IANA timezone metadata, and category color coding.
//! - [`Scheduler`] renders a Cal.com-style appointment booking grid with
//!   available/booked time slots and `yav:post` booking triggers.

use yav_core::{Component, Element, IntoNode, Node};

/// A single calendar event item.
#[derive(Debug, Clone, PartialEq)]
pub struct CalendarEvent {
    /// Day of the month (1..=31).
    pub day: u8,
    /// Event title.
    pub title: String,
    /// Time range label (e.g. `"10:00 - 11:00"`).
    pub time: String,
    /// Color theme (`"indigo"`, `"emerald"`, `"amber"`, `"rose"`).
    pub color: String,
}

impl CalendarEvent {
    /// Create a new calendar event on `day` (1..=31).
    #[must_use]
    pub fn new(
        day: u8,
        title: impl Into<String>,
        time: impl Into<String>,
        color: impl Into<String>,
    ) -> Self {
        Self {
            day,
            title: title.into(),
            time: time.into(),
            color: color.into(),
        }
    }
}

/// Props for the [`Calendar`] component.
#[derive(Debug, Clone)]
pub struct CalendarProps {
    /// Month and year heading (e.g. `"Novembre 2026"`).
    pub month_label: String,
    /// Number of days in the month (28..=31).
    pub days_in_month: u8,
    /// Starting weekday offset (0 = Mon, 6 = Sun).
    pub start_weekday_offset: u8,
    /// Highlighted "today" day number.
    pub today: Option<u8>,
    /// IANA timezone display label (e.g. `"Europe/Paris"`).
    pub timezone: String,
    /// Scheduled events within this month.
    pub events: Vec<CalendarEvent>,
}

/// Interactive Month / Agenda Calendar component.
#[derive(Debug, Clone)]
pub struct Calendar {
    props: CalendarProps,
}

impl Calendar {
    /// Create a new monthly calendar.
    #[must_use]
    pub fn new(month_label: impl Into<String>, days_in_month: u8) -> Self {
        Self {
            props: CalendarProps {
                month_label: month_label.into(),
                days_in_month: days_in_month.clamp(28, 31),
                start_weekday_offset: 0,
                today: None,
                timezone: "UTC".to_string(),
                events: Vec::new(),
            },
        }
    }

    /// Set the starting weekday offset (`0` = Monday .. `6` = Sunday).
    #[must_use]
    pub fn start_offset(mut self, offset: u8) -> Self {
        self.props.start_weekday_offset = offset % 7;
        self
    }

    /// Highlight a specific day as today.
    #[must_use]
    pub fn today(mut self, day: u8) -> Self {
        self.props.today = Some(day);
        self
    }

    /// Set the IANA timezone label.
    #[must_use]
    pub fn timezone(mut self, tz: impl Into<String>) -> Self {
        self.props.timezone = tz.into();
        self
    }

    /// Add an event to the calendar.
    #[must_use]
    pub fn event(mut self, ev: CalendarEvent) -> Self {
        self.props.events.push(ev);
        self
    }
}

impl Component for Calendar {
    fn render(&self) -> Node {
        let p = &self.props;

        let header = Element::new("div")
            .attr(
                "class",
                "flex items-center justify-between border-b border-slate-800 px-6 py-4",
            )
            .child(
                Element::new("div")
                    .child(
                        Element::new("h3")
                            .attr("class", "text-lg font-semibold text-white")
                            .child(Node::text(p.month_label.clone())),
                    )
                    .child(
                        Element::new("span")
                            .attr("class", "text-xs text-slate-400 font-mono")
                            .child(Node::text(format!("TZ: {}", p.timezone))),
                    ),
            );

        let weekdays = ["Lun", "Mar", "Mer", "Jeu", "Ven", "Sam", "Dim"];
        let mut weekday_row = Element::new("div").attr("class", "grid grid-cols-7 border-b border-slate-800 bg-slate-900/60 text-center text-xs font-semibold text-slate-400 py-2");
        for wd in weekdays {
            weekday_row = weekday_row.child(Element::new("div").child(Node::text(wd)));
        }

        let mut grid =
            Element::new("div").attr("class", "grid grid-cols-7 auto-rows-fr bg-slate-950");
        for _ in 0..p.start_weekday_offset {
            grid = grid.child(Element::new("div").attr(
                "class",
                "min-h-[96px] border-b border-r border-slate-900 bg-slate-950/40",
            ));
        }

        for day in 1..=p.days_in_month {
            let is_today = p.today == Some(day);
            let day_badge_cls = if is_today {
                "inline-flex h-6 w-6 items-center justify-center rounded-full bg-indigo-600 text-xs font-bold text-white"
            } else {
                "text-xs font-medium text-slate-400"
            };

            let mut cell = Element::new("div")
                .attr("data-calendar-day", day.to_string())
                .attr("class", "min-h-[96px] border-b border-r border-slate-800/70 p-2 hover:bg-slate-900/40 transition")
                .child(
                    Element::new("div")
                        .attr("class", "mb-1 flex justify-between")
                        .child(Element::new("span").attr("class", day_badge_cls).child(Node::text(day.to_string()))),
                );

            for ev in p.events.iter().filter(|e| e.day == day) {
                let badge_cls = match ev.color.as_str() {
                    "emerald" => "mb-1 truncate rounded bg-emerald-500/20 border border-emerald-500/30 px-1.5 py-0.5 text-[11px] text-emerald-300",
                    "amber" => "mb-1 truncate rounded bg-amber-500/20 border border-amber-500/30 px-1.5 py-0.5 text-[11px] text-amber-300",
                    "rose" => "mb-1 truncate rounded bg-rose-500/20 border border-rose-500/30 px-1.5 py-0.5 text-[11px] text-rose-300",
                    _ => "mb-1 truncate rounded bg-indigo-500/20 border border-indigo-500/30 px-1.5 py-0.5 text-[11px] text-indigo-300",
                };
                cell = cell.child(
                    Element::new("div")
                        .attr("class", badge_cls)
                        .attr("title", format!("{} ({})", ev.title, ev.time))
                        .child(Node::text(format!("{} {}", ev.time, ev.title))),
                );
            }

            grid = grid.child(cell);
        }

        Element::new("div")
            .attr("class", "yav-calendar rounded-xl border border-slate-800 bg-slate-950 overflow-hidden shadow-xl")
            .child(header)
            .child(weekday_row)
            .child(grid)
            .into_node()
    }
}

/// A single bookable time slot in [`Scheduler`].
#[derive(Debug, Clone)]
pub struct TimeSlot {
    pub time: String,
    pub available: bool,
}

/// Cal.com-style appointment slot picker component.
#[derive(Debug, Clone)]
pub struct Scheduler {
    pub title: String,
    pub date_label: String,
    pub duration_minutes: u16,
    pub booking_endpoint: String,
    pub slots: Vec<TimeSlot>,
}

impl Scheduler {
    /// Create a new appointment scheduler for `date_label`.
    #[must_use]
    pub fn new(
        title: impl Into<String>,
        date_label: impl Into<String>,
        booking_endpoint: impl Into<String>,
    ) -> Self {
        Self {
            title: title.into(),
            date_label: date_label.into(),
            duration_minutes: 30,
            booking_endpoint: booking_endpoint.into(),
            slots: Vec::new(),
        }
    }

    /// Set the meeting duration in minutes (default: `30`).
    #[must_use]
    pub fn duration_minutes(mut self, mins: u16) -> Self {
        self.duration_minutes = mins;
        self
    }

    /// Add a time slot (e.g. `"09:30"`, `true`).
    #[must_use]
    pub fn slot(mut self, time: impl Into<String>, available: bool) -> Self {
        self.slots.push(TimeSlot {
            time: time.into(),
            available,
        });
        self
    }
}

impl Component for Scheduler {
    fn render(&self) -> Node {
        let mut slots_grid =
            Element::new("div").attr("class", "grid grid-cols-2 sm:grid-cols-3 gap-2.5 mt-4");
        for s in &self.slots {
            if s.available {
                slots_grid = slots_grid.child(
                    Element::new("button")
                        .attr("type", "button")
                        .attr("yav:post", format!("{}?slot={}", self.booking_endpoint, s.time))
                        .attr(
                            "class",
                            "rounded-lg border border-indigo-500/40 bg-indigo-500/10 py-2.5 px-4 text-sm font-mono font-semibold text-indigo-300 hover:bg-indigo-600 hover:text-white transition",
                        )
                        .child(Node::text(s.time.clone())),
                );
            } else {
                slots_grid = slots_grid.child(
                    Element::new("button")
                        .attr("type", "button")
                        .bool_attr("disabled", true)
                        .attr(
                            "class",
                            "rounded-lg border border-slate-800 bg-slate-900/50 py-2.5 px-4 text-sm font-mono text-slate-600 line-through cursor-not-allowed",
                        )
                        .child(Node::text(s.time.clone())),
                );
            }
        }

        Element::new("div")
            .attr("class", "yav-scheduler rounded-xl border border-slate-800 bg-slate-950 p-6 shadow-xl")
            .child(
                Element::new("div")
                    .attr("class", "flex items-center justify-between border-b border-slate-800 pb-4")
                    .child(
                        Element::new("div")
                            .child(Element::new("h4").attr("class", "text-base font-bold text-white").child(Node::text(self.title.clone())))
                            .child(Element::new("p").attr("class", "text-xs text-slate-400").child(Node::text(self.date_label.clone()))),
                    )
                    .child(
                        Element::new("span")
                            .attr("class", "rounded-full bg-slate-800 px-3 py-1 text-xs font-mono text-slate-300")
                            .child(Node::text(format!("{} min", self.duration_minutes))),
                    ),
            )
            .child(slots_grid)
            .into_node()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_calendar_and_scheduler_render() {
        let cal = Calendar::new("Novembre 2026", 30)
            .timezone("Europe/Paris")
            .today(5)
            .event(CalendarEvent::new(
                5,
                "Architecture Review",
                "14:00",
                "emerald",
            ));

        let cal_html = render_to_string(&cal.render());
        assert!(cal_html.contains("Novembre 2026"));
        assert!(cal_html.contains("Europe/Paris"));
        assert!(cal_html.contains("14:00 Architecture Review"));

        let sched = Scheduler::new("Consultation Technique", "Jeudi 5 Nov 2026", "/api/book")
            .duration_minutes(45)
            .slot("09:00", true)
            .slot("10:00", false);

        let sched_html = render_to_string(&sched.render());
        assert!(sched_html.contains("45 min"));
        assert!(sched_html.contains("/api/book?slot=09:00"));
        assert!(sched_html.contains("disabled"));
    }
}

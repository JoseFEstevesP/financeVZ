use dioxus::prelude::*;
use finance_core::{CategoryTotal, MonthlyTotal};

#[derive(Props, Clone, PartialEq)]
pub struct DonutProps {
    pub items: Vec<CategoryTotal>,
    pub total: i64,
}

struct Segment {
    color: String,
    dash: String,
    offset: String,
}

#[component]
pub fn Donut(props: DonutProps) -> Element {
    let radius = 42.0_f64;
    let circumference = 2.0 * std::f64::consts::PI * radius;
    let mut used = 0.0_f64;
    let mut segments = Vec::new();

    for item in &props.items {
        let fraction = if props.total > 0 {
            item.total.cents() as f64 / props.total as f64
        } else {
            0.0
        };
        let length = (fraction * circumference).max(0.0);
        let gap = circumference - length;
        segments.push(Segment {
            color: item.category.color.clone(),
            dash: format!("{length:.2} {gap:.2}"),
            offset: format!("{:.2}", -used),
        });
        used += length;
    }

    rsx! {
        svg {
            view_box: "0 0 120 120",
            width: "100%",
            height: "100%",
            circle {
                cx: "60", cy: "60", r: "42",
                fill: "none", stroke: "#e8ecf4", stroke_width: "13",
            }
            for segment in segments {
                circle {
                    cx: "60", cy: "60", r: "42",
                    fill: "none",
                    stroke: segment.color,
                    stroke_width: "13",
                    stroke_dasharray: segment.dash,
                    stroke_dashoffset: segment.offset,
                    transform: "rotate(-90 60 60)",
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct BarsProps {
    pub items: Vec<MonthlyTotal>,
}

struct Bar {
    label: String,
    height: String,
    title: String,
}

#[component]
pub fn Bars(props: BarsProps) -> Element {
    let max = props
        .items
        .iter()
        .map(|month| month.total.cents())
        .max()
        .unwrap_or(1)
        .max(1);

    let bars: Vec<Bar> = props
        .items
        .iter()
        .map(|month| Bar {
            label: format!("{:02}", month.month),
            height: format!(
                "{:.3}",
                (month.total.cents() as f64 / max as f64 * 8.125).max(0.125)
            ),
            title: format!("{}-{:02}", month.year, month.month),
        })
        .collect();

    rsx! {
        div { class: "bars",
            for bar in bars {
                div { class: "bar-column",
                    div { class: "bar", style: "height: {bar.height}rem;", title: bar.title }
                    span { class: "bar-label", "{bar.label}" }
                }
            }
        }
    }
}

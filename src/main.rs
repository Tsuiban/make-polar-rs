mod datapoints;

use chrono::{DateTime, TimeDelta};
use clap::Parser;
use slint::SharedString;
slint::include_modules!();

use datapoints::Data;

const GRAPH_IMAGE_WIDTH: u32 = 1000;
const GRAPH_IMAGE_HEIGHT: u32 = 400;

#[derive(Debug, Parser)]
struct Cli {
    filename: Option<String>,
}
fn main() -> Result<(), slint::PlatformError> {
    let cli = Cli::parse();
    let data = Data::load_filename(cli.filename.clone());

    let (data_min_timestamp, data_max_timestamp) = data
        .data
        .iter()
        .map(|a| (a.timestamp, a.timestamp))
        .reduce(|a, b| (a.0.min(b.0), a.1.max(b.1)))
        .unwrap();

    let ui = AppWindow::new()?;

    ui.set_graph_image_height(GRAPH_IMAGE_HEIGHT as f32);
    ui.set_graph_image_width(GRAPH_IMAGE_WIDTH as f32);
    ui.set_graph_image(data.graph(
        GRAPH_IMAGE_WIDTH,
        GRAPH_IMAGE_HEIGHT,
        data_min_timestamp,
        data_max_timestamp,
    ));

    // The absolute minimum and maximum times for the entire data set
    ui.set_data_minimum_time(SharedString::from(data_min_timestamp.to_rfc3339()));
    ui.set_data_maximum_time(SharedString::from(data_max_timestamp.to_rfc3339()));

    // The minimum and maximum displayed times
    ui.set_display_timestamp_min(SharedString::from(data_min_timestamp.to_rfc3339()));
    ui.set_display_timestamp_max(SharedString::from(data_max_timestamp.to_rfc3339()));

    let max_time_interval = data_max_timestamp - data_min_timestamp;

    ui.set_display_scroller_max_value(max_time_interval.num_seconds() as f32);
    ui.set_display_start_scroller_value(0f32);
    ui.set_display_end_scroller_value(max_time_interval.num_seconds() as f32);

    ui.on_redraw_graph({
        let ui_weak = ui.as_weak();
        let data_min_timestamp = data_min_timestamp.clone();
        let data_max_timestamp = data_max_timestamp.clone();
        let data = data.clone();
        move || {
            let ui = ui_weak.unwrap();
            let a = ui.get_display_timestamp_min();
            let a1 = a.as_str();
            let b = ui.get_display_timestamp_max();
            let b1 = b.as_str();
            match DateTime::parse_from_rfc3339(a1) {
                Ok(mut min_timestamp) => {
                    if min_timestamp < data_min_timestamp {
                        min_timestamp = data_min_timestamp.fixed_offset()
                    };
                    match DateTime::parse_from_rfc3339(b1) {
                        Ok(mut max_timestamp) => {
                            if max_timestamp > data_max_timestamp {
                                max_timestamp = data_max_timestamp.fixed_offset()
                            };
                            ui.set_graph_image(data.graph(
                                GRAPH_IMAGE_WIDTH,
                                GRAPH_IMAGE_HEIGHT,
                                min_timestamp.to_utc(),
                                max_timestamp.to_utc(),
                            ));
                        }
                        Err(e) => eprintln!("{e:?}"),
                    }
                }
                Err(e) => eprintln!("{e:?}"),
            }
        }
    });

    ui.on_scroller_changed({
        let ui_weak = ui.as_weak();
        let data_min_timestamp = data_min_timestamp.clone();
        move || {
            let ui = ui_weak.unwrap();
            let start_offset: f32 = ui.get_display_start_scroller_value();
            let end_offset: f32 = ui.get_display_end_scroller_value();
            let start_delta = TimeDelta::seconds(start_offset as i64);
            let end_delta = TimeDelta::seconds(end_offset as i64);
            let start_time = data_min_timestamp + start_delta;
            let end_time = data_min_timestamp + end_delta;
            ui.set_display_timestamp_min(SharedString::from(start_time.to_rfc3339()));
            ui.set_display_timestamp_max(SharedString::from(end_time.to_rfc3339()));
        }
    });

    ui.run()
}

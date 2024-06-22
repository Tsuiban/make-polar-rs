use chrono::{DateTime, NaiveDateTime, TimeDelta, Utc};
use libgraphicimage_slint::GraphicImage;
use libnmea0183::base::{DateTimeError, Nmea0183Base};
use libnmea0183::classify;
use libnmea0183::Nmea0183::{BWC, BWR, GGA, GRS, GST, GXA, MWV, RMC, TRF, VBW, VHW, ZDA, ZFO, ZTG};
use slint::private_unstable_api::re_exports::euclid::approxeq::ApproxEq;
use slint::{Image, Rgb8Pixel};
use std::cmp::Ordering;
use std::fs;
use std::io::{stdin, BufRead, BufReader};
use std::process::exit;

const BOAT_SPEED_COLOUR: Rgb8Pixel = Rgb8Pixel {
    r: 0,
    g: 0xff,
    b: 0,
};
const WIND_SPEED_COLOUR: Rgb8Pixel = Rgb8Pixel {
    r: 0xff,
    g: 0xff,
    b: 0xff,
};
const WIND_DIRECTION_COLOUR: Rgb8Pixel = Rgb8Pixel {
    r: 0xff,
    g: 0,
    b: 0,
};

#[derive(Debug, Clone)]
pub struct DataPoint {
    pub timestamp: DateTime<Utc>,
    pub boatspeed: f32,
    pub windspeed: f32,
    pub winddirection: f32,
}

impl DataPoint {
    pub fn new() -> DataPoint {
        DataPoint {
            timestamp: DateTime::default(),
            boatspeed: 0.,
            windspeed: 0.,
            winddirection: 0.,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Data {
    pub data: Vec<DataPoint>,
}

impl Data {
    pub fn new() -> Data {
        Data { data: Vec::new() }
    }

    pub fn load_filename(filename: Option<String>) -> Data {
        let reader: Box<dyn BufRead> = match filename {
            None => {
                println!("Loading from stdin.");
                Box::new(BufReader::new(stdin()))
            }
            Some(filename) => match fs::File::open(filename.clone()) {
                Ok(file) => {
                    println!("Loading from {filename}");
                    Box::new(BufReader::new(file))
                }
                Err(e) => {
                    eprintln!("{e:?}");
                    exit(-1);
                }
            },
        };

        let mut data = Data::new();
        data.load_reader(reader);
        data
    }

    pub fn load_reader(&mut self, reader: Box<dyn BufRead>) {
        let mut dp = DataPoint::new();

        for line in reader.lines() {
            match line {
                Err(e) => {
                    eprintln!("{e:?}");
                    exit(-1);
                }
                Ok(line) => match Nmea0183Base::from_string(&line) {
                    Err(e) => {
                        eprintln!("{e:?}");
                        exit(-1);
                    }
                    Ok(base) => {
                        self.process_nmea(&mut dp, base);
                        if dp.windspeed > 0.
                            && dp.boatspeed > 0.
                            && dp.winddirection != 0.
                            && dp.timestamp != DateTime::<Utc>::default()
                        {
                            let current_date = dp.timestamp.clone();
                            self.data.push(dp);
                            dp = DataPoint {
                                timestamp: current_date,
                                boatspeed: 0.,
                                windspeed: 0.,
                                winddirection: 0.,
                            }
                        }
                    }
                },
            }
        }
    }

    pub fn graph(
        &self,
        width: u32,
        height: u32,
        start_datetime: DateTime<Utc>,
        end_datetime: DateTime<Utc>,
    ) -> Image {
        let mut graphicimage = GraphicImage::new(width, height);
        if self.data.len() >= 2 {
            let (
                earliest_time,
                latest_time,
                _,
                largest_boatspeed,
                _,
                largest_windspeed,
            ) = self
                .data
                .iter()
                .filter(|a| a.timestamp >= start_datetime && a.timestamp <= end_datetime)
                .map(|a| {
                    (
                        a.timestamp,
                        a.timestamp,
                        a.boatspeed,
                        a.boatspeed,
                        a.windspeed,
                        a.windspeed,
                    )
                })
                .reduce(|a, b| {
                    (
                        a.0.min(b.0),
                        a.1.max(b.1),
                        a.2.min(b.2),
                        a.3.max(b.3),
                        a.4.min(b.4),
                        a.5.max(b.5),
                    )
                })
                .unwrap();
            let speed_ratio = (height - 1) as f32 / ((largest_boatspeed.max(largest_windspeed)).floor() + 1f32);
            let direction_ratio = height as f32 / 180f32;

            let time_range_milliseconds = (latest_time.min(end_datetime) - earliest_time).num_milliseconds() as f32;
            let bin_time_range =
                TimeDelta::milliseconds((time_range_milliseconds / width as f32) as i64);
            let mut bin_start_time = earliest_time.max(start_datetime);
            let stop_time = latest_time.min(end_datetime);

            let mut x = 0;

            while bin_start_time <= stop_time {
                let bin_end_time = bin_start_time + bin_time_range;
                // Boat speeds
                let bin_data_set: Vec<&DataPoint> = self
                    .data
                    .iter()
                    .filter(|a| a.timestamp >= bin_start_time && a.timestamp < bin_end_time)
                    .collect();
                let bin_boatspeeds: Vec<f32> = bin_data_set.iter().map(|a| a.boatspeed).collect();
                let bin_windspeeds: Vec<f32> = bin_data_set.iter().map(|a| a.windspeed).collect();
                let bin_winddirections: Vec<f32> = bin_data_set
                    .iter()
                    .map(|a| {
                        if a.winddirection > 180f32 {
                            360f32 - a.winddirection
                        } else {
                            a.winddirection
                        }
                    })
                    .collect();

                let (bin_low_boatspeed, bin_high_boatspeed) = calculate_bin_values(&bin_boatspeeds);
                let (bin_low_windspeed, bin_high_windspeed) = calculate_bin_values(&bin_windspeeds);
                let (bin_low_winddirection, bin_high_winddirection) =
                    calculate_bin_values(&bin_winddirections);

                let bin_boatspeed_high_y = (bin_high_boatspeed * speed_ratio) as u32;
                let bin_windspeed_high_y = (bin_high_windspeed * speed_ratio) as u32;
                let bin_winddirection_high_y =
                    height - (bin_low_winddirection * direction_ratio) as u32;

                let bin_boatspeed_low_y = (bin_low_boatspeed * speed_ratio) as u32;
                let bin_windspeed_low_y = (bin_low_windspeed * speed_ratio) as u32;
                let bin_winddirection_low_y =
                    height - (bin_high_winddirection * direction_ratio) as u32;

                for item in [
                    (bin_boatspeed_low_y, bin_boatspeed_high_y, BOAT_SPEED_COLOUR),
                    (bin_windspeed_low_y, bin_windspeed_high_y, WIND_SPEED_COLOUR),
                    (
                        bin_winddirection_low_y,
                        bin_winddirection_high_y,
                        WIND_DIRECTION_COLOUR,
                    ),
                ] {
                    graphicimage.line_from_to(
                        (x, if item.0 >= 6 { item.0 - 6 } else { 0 }),
                        (x, (item.0 + 6).min(height - 1)),
                        item.2,
                    );
                    graphicimage.line_from_to(
                        (x, if item.1 >= 6 { item.1 - 6 } else { 0 }),
                        (x, (item.1 + 6).min(height - 1)),
                        item.2,
                    );
                    graphicimage.line_from_to((x, item.0), (x, item.1), item.2)
                }

                x += 1;
                bin_start_time += bin_time_range;
            }
        }
        graphicimage.to_image()
    }

    fn process_nmea(&mut self, datapoint: &mut DataPoint, base: Nmea0183Base) {
        match classify(base) {
            // These all contain time stamps of one sort or another
            BWC(sentence) => self.process_utc_time(datapoint, sentence.timestamp()),
            BWR(sentence) => self.process_utc_time(datapoint, sentence.timestamp()),
            GGA(sentence) => self.process_utc_time(datapoint, sentence.timestamp()),
            GRS(sentence) => self.process_utc_time(datapoint, sentence.timestamp()),
            GST(sentence) => self.process_utc_time(datapoint, sentence.timestamp()),
            GXA(sentence) => self.process_utc_time(datapoint, sentence.timestamp()),
            RMC(sentence) => self.process_utc_timestamp(datapoint, sentence.timestamp()),
            TRF(sentence) => self.process_utc_time(datapoint, sentence.timestamp()),
            ZDA(sentence) => self.process_utc_timestamp(datapoint, sentence.timestamp()),
            ZFO(sentence) => self.process_utc_time(datapoint, sentence.timestamp()),
            ZTG(sentence) => self.process_utc_timestamp(datapoint, sentence.timestamp()),

            // These contain wind or boat information
            MWV(sentence) => {
                if let Ok(speed) = sentence.wind_speed() {
                    datapoint.windspeed = speed.as_knots();
                }
                if let Ok(direction) = sentence.angle_true() {
                    datapoint.winddirection = direction;
                }
            }
            VBW(sentence) => {
                if let Ok(speed) = sentence.water_speed() {
                    datapoint.boatspeed = speed.as_knots();
                }
            }
            VHW(sentence) => {
                if let Ok(speed) = sentence.water_speed() {
                    datapoint.boatspeed = speed.as_knots();
                }
            }

            _ => {}
        }
    }

    fn process_utc_time(&mut self, datapoint: &mut DataPoint, time: DateTimeError) {
        if let Ok(t) = time {
            let d = datapoint.timestamp.date_naive();
            let t = t.time();
            let dt = NaiveDateTime::new(d, t);
            datapoint.timestamp = DateTime::from_naive_utc_and_offset(dt, Utc);
        }
    }

    fn process_utc_timestamp(&mut self, datapoint: &mut DataPoint, time: DateTimeError) {
        if let Ok(t) = time {
            datapoint.timestamp = t;
        }
    }
}

fn calculate_bin_values(data: &Vec<f32>) -> (f32, f32) {
    if data.len() == 0 {
        return (0., 0.);
    } else if data.len() == 1 {
        return (data[0], data[0]);
    };

    let mut speed_frequencies: Vec<(f32, i64)> = Vec::new();
    for item in data {
        let mut found = false;
        for entry in &mut speed_frequencies {
            if entry.0.approx_eq(item) {
                entry.1 = entry.1 + 1;
                found = true;
                break;
            }
        }
        if !found {
            speed_frequencies.push((*item, 1));
        }
    }

    speed_frequencies.sort_by(|a, b| {
        let compare = a.1.partial_cmp(&b.1).unwrap();
        if compare == Ordering::Equal {
            a.0.partial_cmp(&b.0).unwrap()
        } else {
            compare
        }
    });
    speed_frequencies.reverse();
    let a = speed_frequencies[0].0;
    let b = if speed_frequencies.len() == 1 {
        speed_frequencies[0].0
    } else {
        speed_frequencies[1].0
    };
    (a.min(b), a.max(b))
}

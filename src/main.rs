mod datapoints;

use clap::Parser;
slint::include_modules!();

use datapoints::Data;

const GRAPH_IMAGE_WIDTH : u32 = 1000;
const GRAPH_IMAGE_HEIGHT : u32 = 600;

#[derive(Debug, Parser)]
struct Cli {
    filename: Option<String>,
}
fn main() -> Result<(), slint::PlatformError> {
    let cli = Cli::parse();
    let data = Data::load_filename(cli.filename.clone());

    let image = data.graph(GRAPH_IMAGE_WIDTH, GRAPH_IMAGE_HEIGHT);
    println!("{cli:?}");

    let ui = AppWindow::new()?;
    ui.set_graph_image(image);
    ui.set_graph_image_height(GRAPH_IMAGE_HEIGHT as f32);
    ui.set_graph_image_width(GRAPH_IMAGE_WIDTH as f32);
    ui.run()
}

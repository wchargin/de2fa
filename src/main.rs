extern crate image;

use image::GenericImage;

fn main() {
    let argv: Vec<_> = std::env::args().collect();
    if argv.len() <= 1 {
        eprintln!("Usage: {} IMAGE_FILENAME", argv[0]);
        std::process::exit(1);
    }
    let filename = &argv[1];
    let img = match image::open(filename) {
        Err(e) => {
            println!("Failed to decode: {}", e);
            std::process::exit(1);
        }
        Ok(img) => img,
    };
    println!("Successfully decoded. Dimensions: {:?}", img.dimensions());
}

extern crate image;
extern crate quirc;

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
    let img = image::imageops::colorops::grayscale(&img);
    let (width, height) = img.dimensions();
    let pixels: Vec<u8> = img.pixels().map(|p| p.data[0]).collect();
    let mut qr_coder = match quirc::QrCoder::new() {
        Err(e) => {
            eprintln!("Failed to create QR code decoder: {:?}", e);
            std::process::exit(1);
        }
        Ok(qr_coder) => qr_coder,
    };
    let qr_codes = match qr_coder.codes(&pixels, width, height) {
        Err(e) => {
            eprintln!("Failed to decode QR codes: {:?}", e);
            std::process::exit(1);
        }
        Ok(qr_codes) => qr_codes,
    };
    println!("QR codes found: {}.", qr_codes.count());
}

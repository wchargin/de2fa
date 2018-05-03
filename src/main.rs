extern crate image;
extern crate quirc;
extern crate url;

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
    let qr_codes: Vec<_> = match qr_coder.codes(&pixels, width, height) {
        Err(e) => {
            eprintln!("Failed to decode QR codes: {:?}", e);
            std::process::exit(1);
        }
        Ok(qr_codes) => qr_codes,
    }.collect();
    println!("QR codes found: {}.", qr_codes.len());
    for (i, result) in qr_codes.iter().enumerate() {
        match result {
            &Err(ref e) => println!("#{}: failure: {:?}", i, e),
            &Ok(ref qr_code) => {
                println!("#{}: success", i);
                println!("{:?}", process_qr_code(&qr_code.payload));
            }
        }
    }
}

fn process_qr_code(payload_raw: &[u8]) -> Result<String, &'static str> {
    println!("Payload size: {}", payload_raw.len());
    println!("Payload bytes:");
    for datum in payload_raw {
        print!("{:x} ", datum);
    }
    println!();
    let payload = match std::str::from_utf8(&payload_raw) {
        Err(_) => {
            return Err("Not valid UTF-8.");
        }
        Ok(payload) => payload,
    };
    let parsed_url = match url::Url::parse(payload) {
        Err(_) => {
            return Err("Not a valid URL.");
        }
        Ok(parsed_url) => parsed_url,
    };
    let hash_query: std::collections::HashMap<_, _> =
        parsed_url.query_pairs().into_owned().collect();
    match hash_query.get("secret") {
        None => {
            return Err("URL query does not contain \"secret\" key.");
        }
        Some(secret) => Ok(secret.clone()),
    }
}

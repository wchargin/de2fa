extern crate base32;
extern crate image;
extern crate oath;
extern crate quirc;
extern crate url;

fn main() {
    let argv: Vec<_> = std::env::args().collect();
    if argv.len() <= 1 {
        eprintln!("Usage: {} IMAGE_FILENAME", argv[0]);
        std::process::exit(1);
    }
    let filename = &argv[1];
    from_image_filename(filename);
}

fn from_image_filename(filename: &str) -> () {
    println!("Got image filename: {}", filename);
    let img = match image::open(filename) {
        Err(e) => {
            println!("Failed to decode: {}", e);
            return;
        }
        Ok(img) => img,
    };
    let img = image::imageops::colorops::grayscale(&img);
    let (width, height) = img.dimensions();
    let pixels: Vec<u8> = img.pixels().map(|p| p.data[0]).collect();

    let mut qr_coder = match quirc::QrCoder::new() {
        Err(e) => {
            println!("Failed to create QR code decoder: {:?}", e);
            return;
        }
        Ok(qr_coder) => qr_coder,
    };
    let qr_codes: Vec<_> = match qr_coder.codes(&pixels, width, height) {
        Err(e) => {
            println!("Failed to decode QR codes: {:?}", e);
            return;
        }
        Ok(qr_codes) => qr_codes,
    }.collect();

    let raw_payloads: Vec<_> = qr_codes
        .into_iter()
        .filter_map(|result| match result {
            Err(_) => None,
            Ok(qr_code) => Some(qr_code.payload),
        })
        .collect();
    from_raw_payloads(&raw_payloads);
}

fn from_raw_payloads(raw_payloads: &Vec<Vec<u8>>) -> () {
    println!("QR codes found: {}", raw_payloads.len());
    for (i, raw_payload) in raw_payloads.iter().enumerate() {
        println!();
        println!("--- #{}", i);
        from_raw_payload(&raw_payload);
    }
}

fn from_raw_payload(raw_payload: &[u8]) -> () {
    println!("Got raw payload ({} bytes):", raw_payload.len());
    for byte in raw_payload {
        print!("{:x} ", byte);
    }
    println!();
    let payload = match std::str::from_utf8(&raw_payload) {
        Err(_) => {
            println!("Failed: Not valid UTF-8.");
            return;
        }
        Ok(payload) => payload,
    };
    from_payload(payload);
}

fn from_payload(payload: &str) -> () {
    println!("Got payload: {}", payload);
    let parsed_url = match url::Url::parse(payload) {
        Err(_) => {
            println!("Failed: Not a valid URL.");
            return;
        }
        Ok(parsed_url) => parsed_url,
    };
    let hash_query: std::collections::HashMap<_, _> =
        parsed_url.query_pairs().into_owned().collect();
    let secret = match hash_query.get("secret") {
        None => {
            println!("Failed: URL query does not contain \"secret\" key.");
            return;
        }
        Some(secret) => secret,
    };
    from_secret(secret);
}

fn from_secret(secret: &str) {
    println!("Got secret: {}", secret);
    let secret_bytes = match base32::decode(base32::Alphabet::RFC4648 { padding: false }, &secret) {
        None => {
            println!("Failed: Secret is not valid base32.");
            return;
        }
        Some(secret_bytes) => secret_bytes,
    };
    let response = oath::totp_raw_now(&secret_bytes, 6, 0, 30, &oath::HashType::SHA1);
    println!("Response: {:06}", response);
}

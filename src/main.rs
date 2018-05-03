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
    match image_to_raw_payloads(filename) {
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
        Ok(raw_payloads) => {
            println!("QR codes found: {}", raw_payloads.len());
            for (i, raw_payload) in raw_payloads.iter().enumerate() {
                println!("--- #{}", i);
                match process_qr_code(&raw_payload) {
                    Err(message) => println!("Failed: {}", message),
                    Ok(response) => println!("Response: {}", response),
                }
            }
        }
    }
}

fn image_to_raw_payloads(filename: &str) -> Result<Vec<Vec<u8>>, String> {
    let img = match image::open(filename) {
        Err(e) => {
            return Err(format!("Failed to decode: {}", e));
        }
        Ok(img) => img,
    };
    let img = image::imageops::colorops::grayscale(&img);
    let (width, height) = img.dimensions();
    let pixels: Vec<u8> = img.pixels().map(|p| p.data[0]).collect();

    let mut qr_coder = match quirc::QrCoder::new() {
        Err(e) => {
            return Err(format!("Failed to create QR code decoder: {:?}", e));
        }
        Ok(qr_coder) => qr_coder,
    };
    let qr_codes: Vec<_> = match qr_coder.codes(&pixels, width, height) {
        Err(e) => return Err(format!("Failed to decode QR codes: {:?}", e)),
        Ok(qr_codes) => qr_codes,
    }.collect();

    Ok(qr_codes
        .into_iter()
        .filter_map(|result| match result {
            Err(_) => None,
            Ok(qr_code) => Some(qr_code.payload),
        })
        .collect())
}

fn process_qr_code(payload_raw: &[u8]) -> Result<String, &'static str> {
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
    let secret = match hash_query.get("secret") {
        None => {
            return Err("URL query does not contain \"secret\" key.");
        }
        Some(secret) => secret,
    };
    let secret_bytes = match base32::decode(base32::Alphabet::RFC4648 { padding: false }, &secret) {
        None => {
            return Err("Secret is not valid base32.");
        }
        Some(secret_bytes) => secret_bytes,
    };
    Ok(format!(
        "{:06}",
        oath::totp_raw_now(&secret_bytes, 6, 0, 30, &oath::HashType::SHA1)
    ))
}

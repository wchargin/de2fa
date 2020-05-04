extern crate base32;
extern crate clap;
extern crate image;
extern crate oath;
extern crate quirc;
extern crate url;

enum Source {
    Image {
        filename: String,
    },
    QrCode {
        image_filename: String,
        index: usize,
        out_of: usize,
    },
    Url(String),
    Secret(String),
}

impl ToString for Source {
    fn to_string(&self) -> String {
        match &self {
            Source::Image { filename } => format!("image {}", filename),
            Source::QrCode {
                image_filename,
                index,
                out_of,
            } => {
                if *out_of == 1 {
                    format!("qr {}", image_filename)
                } else {
                    let fill_width = format!("{}", out_of).len();
                    format!(
                        "qr[{:0width$}/{}] {}",
                        index + 1,
                        out_of,
                        image_filename,
                        width = fill_width
                    )
                }
            }
            Source::Url(url) => format!("url {}", url),
            Source::Secret(secret) => format!("secret {}", secret),
        }
    }
}

struct Output {
    source: Source,
    result: std::result::Result<u64, String>,
}

impl ToString for Output {
    fn to_string(&self) -> String {
        match &self.result {
            Ok(response) => format!("{:06}\t{}", response, &self.source.to_string()),
            Err(e) => format!("------\t{}: {}", &self.source.to_string(), e),
        }
    }
}

fn main() {
    let matches = clap::App::new("de2fa")
        .version("0.1.0")
        .arg(
            clap::Arg::with_name("from")
                .long("--from")
                .possible_values(&["image", "url", "secret"])
                .default_value("image")
                .help("Input source type")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("verbose")
                .long("--verbose")
                .short("-v")
                .help("Display raw data and TOTP secret, not just response"),
        )
        .arg(
            clap::Arg::with_name("SOURCE")
                .required(true)
                .multiple(true)
                .takes_value(true),
        )
        .get_matches();
    let sources = matches.values_of("SOURCE").unwrap();
    let verbose = matches.occurrences_of("verbose") > 0;
    let results: Box<dyn Iterator<Item = Output>> = match matches.value_of("from").unwrap() {
        "image" => Box::new(sources.flat_map(|source| from_image_filename(source, verbose))),
        "url" => Box::new(
            sources.map(|source| from_payload(source, Source::Url(source.to_string()), verbose)),
        ),
        "secret" => Box::new(
            sources.map(|source| from_secret(source, Source::Secret(source.to_string()), verbose)),
        ),
        other => panic!("Unknown source type: {}", other),
    };
    for output in results {
        println!("{}", output.to_string())
    }
}

enum FromImageIterator {
    Err(std::iter::Once<Output>),
    Ok {
        raw_payloads_with_sources: std::vec::IntoIter<(Vec<u8>, Source)>,
        verbose: bool,
    },
}

impl Iterator for FromImageIterator {
    type Item = Output;
    fn next(&mut self) -> Option<Output> {
        match self {
            FromImageIterator::Err(ref mut i) => i.next(),
            FromImageIterator::Ok {
                raw_payloads_with_sources,
                verbose,
            } => raw_payloads_with_sources
                .next()
                .map(|(raw_payload, source)| from_raw_payload(&raw_payload, source, *verbose)),
        }
    }
}

fn from_image_filename(filename: &str, verbose: bool) -> FromImageIterator {
    FromImageIterator::new(filename, verbose)
}

impl FromImageIterator {
    fn new(filename: &str, verbose: bool) -> FromImageIterator {
        if verbose {
            println!("Got image filename: {}", filename);
        }
        let fail = |msg| {
            FromImageIterator::Err(std::iter::once(Output {
                source: Source::Image {
                    filename: filename.to_string(),
                },
                result: std::result::Result::Err(msg),
            }))
        };
        let img = match image::open(filename) {
            Err(e) => return fail(format!("Failed to decode: {}", e)),
            Ok(img) => img,
        };
        let img = image::imageops::colorops::grayscale(&img);
        let (width, height) = img.dimensions();
        let pixels: Vec<u8> = img.pixels().map(|p| p.data[0]).collect();

        let mut qr_coder = match quirc::QrCoder::new() {
            Err(e) => return fail(format!("Failed to create QR code decoder: {:?}", e)),
            Ok(qr_coder) => qr_coder,
        };
        let qr_codes: Vec<_> = match qr_coder.codes(&pixels, width, height) {
            Err(e) => return fail(format!("Failed to decode QR codes: {:?}", e)),
            Ok(qr_codes) => qr_codes,
        }
        .collect();

        let raw_payloads: Vec<Vec<u8>> = qr_codes
            .into_iter()
            .filter_map(|result| match result {
                Err(_) => None,
                Ok(qr_code) => Some(qr_code.payload),
            })
            .collect();
        let count = raw_payloads.len();
        if verbose {
            println!("QR codes found: {}", raw_payloads.len());
        }

        let raw_payloads_with_sources: Vec<(Vec<u8>, Source)> = raw_payloads
            .into_iter()
            .enumerate()
            .map(|(i, raw_payload)| {
                let source = Source::QrCode {
                    image_filename: filename.to_string(),
                    index: i,
                    out_of: count,
                };
                (raw_payload, source)
            })
            .collect();
        FromImageIterator::Ok {
            raw_payloads_with_sources: raw_payloads_with_sources.into_iter(),
            verbose,
        }
    }
}

fn from_raw_payload(raw_payload: &[u8], source: Source, verbose: bool) -> Output {
    if verbose {
        println!("Got raw payload ({} bytes):", raw_payload.len());
        for byte in raw_payload {
            print!("{:x} ", byte);
        }
        println!();
    }
    let payload = match std::str::from_utf8(&raw_payload) {
        Err(_) => {
            return Output {
                source,
                result: std::result::Result::Err("Payload is not valid UTF-8".to_string()),
            }
        }
        Ok(payload) => payload,
    };
    from_payload(payload, source, verbose)
}

fn from_payload(payload: &str, source: Source, verbose: bool) -> Output {
    if verbose {
        println!("Got payload: {}", payload);
    }
    let fail = |source, msg| Output {
        source,
        result: std::result::Result::Err(format!("{}: {}", msg, payload)),
    };
    let parsed_url = match url::Url::parse(payload) {
        Err(_) => return fail(source, "Invalid URL"),
        Ok(parsed_url) => parsed_url,
    };
    let hash_query: std::collections::HashMap<_, _> =
        parsed_url.query_pairs().into_owned().collect();
    let secret = match hash_query.get("secret") {
        None => return fail(source, "URL query does not contain \"secret\" key"),
        Some(secret) => secret,
    };
    from_secret(secret, source, verbose)
}

fn from_secret(secret: &str, source: Source, verbose: bool) -> Output {
    if verbose {
        println!("Got secret: {}", secret);
    }
    let secret_bytes = match base32::decode(base32::Alphabet::RFC4648 { padding: false }, &secret) {
        None => {
            return Output {
                source,
                result: std::result::Result::Err("Secret is not valid base32".to_string()),
            }
        }
        Some(secret_bytes) => secret_bytes,
    };
    let response = oath::totp_raw_now(&secret_bytes, 6, 0, 30, &oath::HashType::SHA1);
    Output {
        source,
        result: std::result::Result::Ok(response),
    }
}

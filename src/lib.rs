use std::io::Cursor;
use symphonia::core::{
    codecs::{DecoderOptions, CODEC_TYPE_NULL},
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};
use worker::{
    console_log, event, Cors, Date, Env, FormEntry, Method, Request, Response, Result, Router,
};

mod utils;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);

    // Optionally, get more helpful error messages written to the console in the case of a panic.
    utils::set_panic_hook();

    // Optionally, use the Router to handle matching endpoints, use ":name" placeholders, or "*name"
    // catch-alls to match on specific patterns. Alternatively, use `Router::with_data(D)` to
    // provide arbitrary data that will be accessible in each route via the `ctx.data()` method.
    let router = Router::new();

    // Add as many routes as your Worker needs! Each route will get a `Request` for handling HTTP
    // functionality and a `RouteContext` which you can use to  and get route parameters and
    // Environment bindings like KV Stores, Durable Objects, Secrets, and Variables.
    router
        .get("/", |_, _| Response::ok("Hello from Workers!"))
        .post_async("/audiowave", |mut req, _ctx| async move {
            match req.form_data().await?.get("file") {
                Some(FormEntry::File(buf)) => {
                    let cursor = Cursor::new(buf.bytes().await?);
                    let media_source_stream =
                        MediaSourceStream::new(Box::new(cursor), Default::default());

                    // Create a probe hint using the file's extension. [Optional]
                    let mut hint = Hint::new();
                    hint.with_extension("mp3");

                    // Use the default options for metadata and format readers.
                    let meta_opts: MetadataOptions = Default::default();
                    let fmt_opts: FormatOptions = Default::default();

                    // Probe the media source.
                    let probed = symphonia::default::get_probe()
                        .format(&hint, media_source_stream, &fmt_opts, &meta_opts)
                        .expect("unsupported format");

                    // Get the instantiated format reader.
                    let format = probed.format;

                    // Find the first audio track with a known (decodeable) codec.
                    let track = format
                        .tracks()
                        .iter()
                        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
                        .expect("no supported audio tracks");

                    // Use the default options for the decoder.
                    let dec_opts: DecoderOptions = Default::default();

                    // Create a decoder for the track.
                    let mut decoder = symphonia::default::get_codecs()
                        .make(&track.codec_params, &dec_opts)
                        .expect("unsupported codec");

                    // Store the track identifier, it will be used to filter packets.
                    let track_id = track.id;
                    dbg!(track_id);
                    return Response::ok("processing");
                }
                Some(_) | None => return Response::error("Bad Request", 400),
            }
        })
        // cors handling
        .options("/", |req, _ctx| {
            let headers = req.headers();
            if let (Some(_), Some(_), Some(_)) = (
                headers.get("Origin").transpose(),
                headers.get("Access-Control-Request-Method").transpose(),
                headers.get("Access-Control-Request-Headers").transpose(),
            ) {
                let cors = Cors::default()
                    .with_max_age(86400)
                    .with_origins(vec!["localhost:3000"])
                    .with_methods(vec![
                        Method::Get,
                        Method::Head,
                        Method::Post,
                        Method::Options,
                    ]);
                Response::empty().and_then(|resp| resp.with_cors(&cors))
            } else {
                Response::empty()
            }
        })
        .get("/worker-version", |_, ctx| {
            let version = ctx.var("WORKERS_RS_VERSION")?.to_string();
            Response::ok(version)
        })
        .run(req, env)
        .await
}

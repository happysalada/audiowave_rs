use lazy_static::lazy_static;
use worker::{
    console_log, event, Cors, Date, Env, FormEntry, Method, Request, Response, Result, Router,
};

mod audio;
mod utils;

lazy_static! {
    static ref CORS: Cors = Cors::default()
        .with_max_age(86400)
        .with_origins(vec!["*"])
        .with_methods(vec![
            Method::Get,
            Method::Head,
            Method::Post,
            Method::Options,
        ]);
}

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
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
                Some(FormEntry::File(file)) => {
                    let name = file.name();
                    let bytes = file.bytes().await?;
                    match audio::get_waveform(name, bytes) {
                        Ok(waveform) => {
                            Response::from_json(&waveform).and_then(|resp| resp.with_cors(&CORS))
                        }
                        Err(err) => Response::error(format!("Internal server error: {}", err), 500),
                    }
                }
                Some(_) | None => Response::error("Bad Request", 400),
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
                Response::empty().and_then(|resp| resp.with_cors(&CORS))
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

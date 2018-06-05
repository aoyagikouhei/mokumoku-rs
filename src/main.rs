extern crate futures;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate hyper;
extern crate mime;
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate r2d2;
extern crate r2d2_redis;
extern crate redis;

use futures::{
    Future, 
    Stream,
};
use gotham::{
    http::response::create_response,
    state::{
        FromState,
        State,
    },
    pipeline::{
        new_pipeline,
        single::single_pipeline,
    },
    router::{
        Router,
        builder::*,
    },
    handler::{
        IntoHandlerError, 
        HandlerFuture,
    },
    middleware::Middleware,
};
use hyper::{
    Body, 
    Headers,
    StatusCode, 
    Uri, 
};
use std::{
    collections::HashMap,
    env,
    error::Error,
    fs::File,
    io::prelude::*,
    ops::Deref,
    sync::Arc,
};
use r2d2_redis::RedisConnectionManager;
use redis::Script;

#[derive(Deserialize)]
struct RedisResponse {
    content_type: String,
    status_code: u16,
    body: String,
}

#[derive(StateData)]
pub struct MiddlewareData {
    pub conn: r2d2::PooledConnection<r2d2_redis::RedisConnectionManager>,
    pub script: Arc<Script>,
}

#[derive(Clone, NewMiddleware)]
pub struct ConfigMiddleware {
    pub pool: r2d2::Pool<r2d2_redis::RedisConnectionManager>,
    pub script: Arc<Script>,
}

impl Middleware for ConfigMiddleware {
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        state.put(MiddlewareData{
            conn: self.pool.get().unwrap(),
            script: self.script.clone(),
        });
        chain(state)
    }
}

impl std::panic::RefUnwindSafe for ConfigMiddleware {}

fn handler_get(state: State) -> Box<HandlerFuture> {
    handler(state, "get")
}

fn handler_post(state: State) -> Box<HandlerFuture> {
    handler(state, "post")
}

fn handler(mut state: State, method: &'static str) -> Box<HandlerFuture> {
    let f = Body::take_from(&mut state)
        .concat2()
        .then(move |full_body| match full_body {
            Ok(valid_body) => {
                let content: RedisResponse = {
                    let body_content = String::from_utf8(valid_body.to_vec()).unwrap();
                    let data = MiddlewareData::borrow_from(&state);
                    let uri = Uri::borrow_from(&state);
                    let headers = Headers::borrow_from(&state);
                    let mut map = HashMap::new();
                    for it in headers.iter() {
                        map.insert(it.name(), it.value_string());
                    }
                    let v = json!({
                        "uri": {
                            "path": uri.path(),
                            "query": uri.query(),
                        },
                        "body": body_content,
                        "headers": map,
                        "method": method,
                    });
                    let res: String = data.script.arg(v.to_string()).invoke(data.conn.deref()).unwrap();
                    serde_json::from_str(&res).unwrap()
                };
                let mime_type: mime::Mime = content.content_type.parse().unwrap();
                let res = create_response(
                    &state, 
                    StatusCode::try_from(content.status_code).unwrap(), 
                    Some((content.body.into_bytes(), mime_type)),
                );
                futures::future::ok((state, res))
            }
            Err(e) => return futures::future::err((state, e.into_handler_error())),
        });
    
    Box::new(f)
}

fn router(uri: &str, path: &str) -> Result<Router, Box<Error>> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let pool = r2d2::Pool::builder()
        //.max_size(2)
        .build(RedisConnectionManager::new(uri)?)?;
    let config_milldeware = ConfigMiddleware {
        pool: pool,
        script: Arc::new(Script::new(&contents)),
    };
    let (chain, pipelines) = single_pipeline(
        new_pipeline()
            .add(config_milldeware)
            .build()
    );

    Ok(build_router(chain, pipelines, |route| {
        route.get("/").to(handler_get);
        route.get("*").to(handler_get);
        route.post("/").to(handler_post);
        route.post("*").to(handler_post);
    }))
}

// cargo run -- redis://localhost:6379/0 samples/helloworld/index.lua
pub fn main() {    
    let args: Vec<String> = env::args().collect();
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);

    gotham::start(addr, router(&args[1], &args[2]).unwrap())
}
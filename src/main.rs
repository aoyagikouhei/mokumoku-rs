extern crate futures;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate hyper;
extern crate mime;
#[macro_use]
extern crate serde_json;
extern crate r2d2;
extern crate r2d2_redis;
extern crate redis;

use futures::{Future, Stream};

use gotham::http::response::create_response;
use gotham::state::State;
use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;
use gotham::router::Router;
use gotham::router::builder::*;
use gotham::handler::{IntoHandlerError, HandlerFuture};
use gotham::middleware::Middleware;
use gotham::state::FromState;

use hyper::{StatusCode, Uri, Body, Headers};

use std::collections::HashMap;
use std::error::Error;
use std::env;

use r2d2_redis::RedisConnectionManager;
use redis::Commands;

#[derive(StateData)]
pub struct MiddlewareData {
    pub conn: r2d2::PooledConnection<r2d2_redis::RedisConnectionManager>
}

#[derive(Clone, NewMiddleware)]
pub struct ConfigMiddleware {
    pub pool: r2d2::Pool<r2d2_redis::RedisConnectionManager>
}

impl Middleware for ConfigMiddleware {
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        state.put(MiddlewareData{
            conn: self.pool.get().unwrap()
        });
        chain(state)
    }
}

impl std::panic::RefUnwindSafe for ConfigMiddleware {}

fn handler(mut state: State) -> Box<HandlerFuture> {
    let f = Body::take_from(&mut state)
        .concat2()
        .then(|full_body| match full_body {
            Ok(valid_body) => {
                let content = {
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
                    });
                    println!("{:?}", v);



                    /*
                    match data.config_ary.iter().find(|&v| v.path == uri.path()) {
                        Some(ref config) => config.content.clone(),
                        None => String::from("xxxxxxx")
                    }*/
                
                    //let a = redis::cmd("KEYS").arg("*").query::<Vec<String>>(data.conn.deref());
                    //println!("{:?}", a);
                    //let res: Option<String> = data.conn.get("aaa").unwrap();
                    let res: String = data.conn.get("予定表〜①ﾊﾝｶｸだ").unwrap();
                    res
                };
                let res = create_response(
                    &state, 
                    StatusCode::Ok, 
                    Some((content.into_bytes(), mime::TEXT_PLAIN_UTF_8)),
                );
                futures::future::ok((state, res))
            }
            Err(e) => return futures::future::err((state, e.into_handler_error())),
        });
    
    Box::new(f)
}

fn router(uri: &str) -> Result<Router, Box<Error>> {
    let pool = r2d2::Pool::builder()
        //.max_size(2)
        .build(RedisConnectionManager::new(uri)?)?;
    let config_milldeware = ConfigMiddleware {
        pool: pool
    };
    let (chain, pipelines) = single_pipeline(
        new_pipeline()
            .add(config_milldeware)
            .build()
    );

    Ok(build_router(chain, pipelines, |route| {
        route.get("/").to(handler);
        route.get("*").to(handler);
        route.post("/").to(handler);
        route.post("*").to(handler);
    }))
}

// cargo run -- redis://localhost:6379/0
pub fn main() {    
    let args: Vec<String> = env::args().collect();
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);

    gotham::start(addr, router(&args[1]).unwrap())
}
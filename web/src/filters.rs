//! Warp filters.
//!
//! Entry point filters will call handlers to execute logic.
//!
//!
use crate::config::*;
use crate::handler;
use crate::model;
use crate::world;
use r2d2_redis::{r2d2, RedisConnectionManager};
use slog::{o, Logger};
use sqlx::postgres::PgPool;
use std::convert::Infallible;
use std::sync::{Arc, RwLock};
use warp::http::StatusCode;
use warp::reply::with_status;
use warp::Filter;

async fn health_check() -> Result<impl warp::Reply, Infallible> {
    let response = with_status(warp::reply(), StatusCode::NO_CONTENT);
    Ok(response)
}

pub fn api(
    logger: Logger,
    conf: Config,
    cache_pool: r2d2::Pool<RedisConnectionManager>,
    db_pool: PgPool,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let conf = std::sync::Arc::new(conf);

    let cache_pool = {
        let filter = warp::any().map(move || cache_pool.clone());
        move || filter.clone()
    };

    let db_pool = {
        let filter = warp::any().map(move || db_pool.clone());
        move || filter.clone()
    };

    let config = {
        let filter = warp::any().map(move || {
            let conf = Arc::clone(&conf);
            conf
        });
        move || filter.clone()
    };

    let jwks_cache = {
        let cache = Arc::new(RwLock::new(std::mem::MaybeUninit::uninit()));
        let filter = warp::any().map(move || Arc::clone(&cache));
        move || filter.clone()
    };

    let jwks = {
        let logger = logger.clone();
        let filter = warp::any()
            .and(warp::any().map(move || logger.clone()))
            .and(jwks_cache())
            .and_then(model::load_jwks);
        move || filter.clone()
    };

    // I used `and + optional` instead of `or` because a lack of `authorization` is not inherently
    // and error, however `or` would return 400 if neither method is used
    let identity = {
        let logger = logger.clone();
        let identity = warp::any()
            .and(config())
            .and(warp::filters::header::optional("Authorization"))
            .and(warp::filters::cookie::optional("Authorization"))
            .and(jwks())
            .map(
                move |config: Arc<Config>,
                      header_id: Option<String>,
                      cookie_id: Option<String>,
                      jwks: &model::JWKS| {
                    header_id
                        .as_ref()
                        .and_then(|id| {
                            const BEARER_PREFIX: &str = "Bearer ";
                            if !id.starts_with(BEARER_PREFIX) {
                                return None;
                            }
                            Some(&id[BEARER_PREFIX.len()..])
                        })
                        .or(cookie_id.as_ref().map(|id| id.as_str()))
                        .and_then(|token: &str| {
                            model::Identity::validated_id(&logger, config.as_ref(), token, jwks)
                        })
                },
            );
        move || identity.clone()
    };

    let current_user = {
        let current_user = warp::any()
            .and(identity())
            .and(db_pool())
            .and_then(model::current_user);
        move || current_user.clone()
    };

    let logger = {
        let filter = warp::any().and(warp::addr::remote()).and(identity()).map(
            move |addr: Option<std::net::SocketAddr>, id: Option<model::Identity>| {
                logger.new(o!(
                    "current_user_id" => id.map(|id|format!("{:?}", id.id)),
                    "address" => addr
                ))
            },
        );
        move || filter.clone()
    };

    let health_check = warp::get().and(warp::path("health")).and_then(health_check);
    let world_stream = warp::get()
        .and(warp::path("world"))
        .and(logger())
        .and(warp::ws())
        .and(current_user())
        .and(cache_pool())
        .map(move |logger: Logger, ws: warp::ws::Ws, user, pool| {
            ws.on_upgrade(move |socket| world::world_stream(logger, socket, user, pool))
        });

    let myself = warp::get()
        .and(warp::path("myself"))
        .and(current_user())
        .and_then(handler::myself);

    let schema = warp::get()
        .and(warp::path("schema"))
        .and(logger())
        .and(cache_pool())
        .and_then(handler::schema);

    let terrain_rooms = warp::get()
        .and(warp::path!("terrain" / "rooms"))
        .and(db_pool())
        .and_then(handler::terrain_rooms);

    let terrain = warp::get()
        .and(warp::path("terrain"))
        .and(logger())
        .and(warp::query())
        .and(db_pool())
        .and_then(handler::terrain);

    let compile = warp::post()
        .and(warp::path("compile"))
        .and(logger())
        .and(warp::filters::body::json())
        .and_then(handler::compile);

    let save_script = warp::post()
        .and(warp::path!("scripts" / "commit"))
        .and(logger())
        .and(current_user())
        .and(warp::filters::body::json())
        .and(db_pool())
        .and(cache_pool())
        .and_then(handler::save_script);

    let register = warp::post()
        .and(warp::path!("user" / "register"))
        .and(logger())
        .and(warp::filters::body::json())
        .and(db_pool())
        .and_then(handler::register);

    let put_user = warp::put()
        .and(warp::path!("user"))
        .and(logger())
        .and(warp::filters::body::json())
        .and(db_pool())
        .and_then(handler::put_user);

    health_check
        .or(world_stream)
        .or(myself)
        .or(schema)
        .or(terrain_rooms)
        .or(terrain)
        .or(save_script)
        .or(compile)
        .or(register)
        .or(put_user)
}

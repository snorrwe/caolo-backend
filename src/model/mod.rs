pub mod world;

use crate::config::Config;
use crate::PgPool;
pub use biscuit::jwk::JWK;
use biscuit::{
    jwa::SignatureAlgorithm,
    jwk::{AlgorithmParameters, JWKSet},
    JWT,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use slog::{debug, info, trace, warn, Logger};
use sqlx::FromRow;
use std::convert::Infallible;
use std::sync::{Arc, Once, RwLock};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct JWKAddition {}

pub type JWKS = JWKSet<JWKAddition>;

#[derive(Debug, FromRow, Serialize)]
pub struct User {
    pub id: Uuid,
    pub auth0_id: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

#[derive(Debug)]
pub struct Identity {
    pub user_id: String,
}
#[derive(Debug, Deserialize, Serialize)]
struct PrivateClaims {}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptMetadata {
    pub name: String,
    pub script_id: Uuid,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptEntity {
    pub name: String,
    pub user_id: Uuid,
    pub script_id: Uuid,
    pub payload: Value,
}

impl Identity {
    /// Returns None on error and logs it.
    pub fn validated_id(
        logger: &Logger,
        _config: &Config,
        token: &str,
        jwks: &JWKS,
    ) -> Option<Self> {
        trace!(logger, "deseralizing Identity: {:?}", token);
        let token = JWT::<_, biscuit::Empty>::new_encoded(&token);
        let kid = token
            .unverified_header()
            .map_err(|err| {
                warn!(logger, "failed to deserialize header : {:?}", err);
            })
            .ok()?
            .registered
            .key_id?;
        trace!(logger, "found kid: {:?}", kid);
        let jwk = jwks.find(kid.as_str())?;
        trace!(logger, "found jwk: {:?}", jwk);
        let secret = match jwk.algorithm {
            AlgorithmParameters::RSA(ref alg) => alg.jws_public_key_secret(),
            _ => panic!("Given jwk algorithm not implemented {:?}", jwk),
        };
        let token = token
            .into_decoded(&secret, SignatureAlgorithm::RS256)
            .map_err(|err| warn!(logger, "failed to validate token {:?}", err))
            .ok()?;
        let (_, res) = token.unwrap_decoded();
        let _: &PrivateClaims = &res.private; // provide type hint
        trace!(logger, "found identity: {:?}", res);
        let id = Identity {
            user_id: res
                .registered
                .subject
                .ok_or_else(|| {
                    debug!(logger, "JWT did not contain subject field");
                })
                .ok()?,
        };
        Some(id)
    }
}

pub async fn current_user(
    logger: Logger,
    id: Option<Identity>,
    pool: PgPool,
) -> Result<Option<User>, Infallible> {
    trace!(logger, "current_user by id {:?}", id);

    let id = match id {
        Some(id) => id,
        None => return Ok(None),
    };
    let res = sqlx::query_as!(
        User,
        "
        SELECT ua.id, ua.auth0_id, ua.display_name, ua.email, ua.created, ua.updated
        FROM user_account AS ua
        WHERE ua.auth0_id=$1
        ",
        id.user_id
    )
    .fetch_optional(&pool)
    .await
    .expect("failed to query database");
    if res.is_none() {
        warn!(logger, "Failed to find user by the given identity {:?}", id);
    }
    Ok(res)
}

static JWKS_LOAD: Once = Once::new();

pub async fn load_jwks<'a>(
    logger: Logger,
    cache: Arc<RwLock<std::mem::MaybeUninit<JWKS>>>,
) -> Result<&'a JWKS, Infallible> {
    {
        let cache = Arc::clone(&cache);
        tokio::task::spawn_blocking(move || {
            JWKS_LOAD.call_once(|| {
                info!(logger, "performing initial JWK load");
                let cc = Arc::clone(&cache);
                let cache = cc;
                let uri = std::env::var("JWKS_URI")
                    .expect("Can not perform authorization without JWKS_URI");
                let payload = reqwest::blocking::get(&uri);
                let payload = payload.map(|pl| pl.json::<serde_json::Value>());
                trace!(logger, "Got payload: {:#?}", payload);
                let payload = payload
                    .unwrap()
                    .expect("Failed to deserialize payload to json value");
                let payload: JWKS = serde_json::from_value(payload)
                    .expect("failed to deserialize payload value to jwks");

                let mut cache = cache.write().unwrap();
                *cache = std::mem::MaybeUninit::new(payload);
                info!(logger, "JWK load finished");
                debug!(logger, "JWKs loaded: {:#?}", *cache);
            });
        })
        .await
        .expect("Failed to load JWKS");
    }

    let cache = cache.read().unwrap();
    let cache = cache.as_ptr();
    unsafe { Ok(&*cache) }
}

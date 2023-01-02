use std::sync::Arc;

use uuid::Uuid;
use serde::{Deserialize, Serialize};
use sea_orm::{ActiveValue, EntityTrait, sea_query::OnConflict};
use reqwest::StatusCode;
use axum::{extract::State, Json};

use crate::{AppState, routers::ErrOr};
use yur_paintboard::entities::{prelude::*, auth};

#[derive(Deserialize)]
pub struct AuthPayload {
  uid: i32,
}

#[derive(Serialize)]
pub struct AuthResp {
  session: Uuid,
  token: Uuid,
}

pub async fn auth(
  State(state): State<Arc<AppState>>,
  Json(payload): Json<AuthPayload>,
) -> (StatusCode, Json<ErrOr<AuthResp>>) {
  let session = Uuid::new_v4();
  let token = Uuid::new_v4();

  let new_auth = auth::ActiveModel {
    uid: ActiveValue::set(payload.uid),
    session: ActiveValue::set(session),
    luogu_token: ActiveValue::set(token),
  };

  let res = Auth::insert(new_auth)
    .on_conflict(
      OnConflict::column(auth::Column::Uid)
        .update_columns([auth::Column::Session, auth::Column::LuoguToken])
        .to_owned()
    )
    .exec(&state.db).await;

  if res.is_err() {
    (
      StatusCode::INTERNAL_SERVER_ERROR,
      Json(ErrOr::Err("Error accessing database!".into())),
    )
  } else {
    (
      StatusCode::OK,
      Json(ErrOr::Ok(AuthResp { session, token }.into())),
    )
  }
}

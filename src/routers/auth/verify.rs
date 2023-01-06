use std::sync::Arc;

use uuid::Uuid;
use serde::{Deserialize, Serialize};
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, ActiveValue, sea_query::OnConflict};
use reqwest::StatusCode;
use axum::{extract::State, Json};

use yur_paintboard::entities::{prelude::*, auth, session};
use crate::{AppState, routers::ErrOr};
use super::check;

#[derive(Deserialize)]
pub struct VerifyPayload {
  session: Uuid,
}

#[derive(Serialize)]
pub struct VerifyResp {
  token: Uuid,
}

pub async fn verify(
  State(state): State<Arc<AppState>>,
  Json(payload): Json<VerifyPayload>,
) -> (StatusCode, Json<ErrOr<VerifyResp>>) {
  let auth = Auth::find()
    .filter(auth::Column::Session.eq(payload.session))
    .one(&state.db).await;

  if auth.is_err() {
    return (
      StatusCode::INTERNAL_SERVER_ERROR,
      Json(ErrOr::Err("Error accessing database!".into())),
    );
  }

  let auth = auth.unwrap();

  if auth.is_none() {
    return (
      StatusCode::NOT_FOUND,
      Json(ErrOr::Err("Session not found!".into())),
    );
  }

  let auth = auth.unwrap();

  if !check::check_user(auth.uid, auth.luogu_token).await {
    return (
      StatusCode::BAD_REQUEST,
      Json(ErrOr::Err("Authentication failed!".into())),
    );
  }
  
  let token = Uuid::new_v4();

  let new_session = session::ActiveModel {
    uid: ActiveValue::Set(auth.uid),
    paint_token: ActiveValue::Set(token),
  };

  let res = Session::insert(new_session)
    .on_conflict(
      OnConflict::column(session::Column::Uid)
        .update_column(session::Column::PaintToken)
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
      Json(ErrOr::Ok(VerifyResp { token }.into())),
    )
  }
}

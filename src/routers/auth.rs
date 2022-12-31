use std::sync::Arc;

use uuid::Uuid;
use serde::{Deserialize, Serialize};
use sea_orm::{EntityTrait, ActiveValue, sea_query::OnConflict, QueryFilter, ColumnTrait};
use axum::{extract::State, http::StatusCode, Json};

use crate::AppState;
use yur_paintboard::entities::{prelude::*, auth, session};
use super::ErrOr;

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

  if !check_user(auth.uid, auth.luogu_token).await {
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

// Other fields omitted
#[derive(Deserialize)]
struct UserDetails {
  introduction: Option<String>,
}

#[derive(Deserialize)]
struct LuoguUserData {
  user: UserDetails,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct LuoguResp<T> {
  currentData: T,
}

async fn check_user(
  uid: i32,
  token: Uuid,
) -> bool {
  let client = reqwest::Client::new();

  let resp = client.get(format!("https://www.luogu.com.cn/user/{uid}"))
    .header("x-luogu-type", "content-only")
    .send().await;

  if resp.is_err() {
    return false;
  }

  let resp = resp.unwrap()
    .bytes().await;

  if resp.is_err() {
    return false;
  }

  let resp = resp.unwrap();
  let res = serde_json::from_slice(&resp);

  if res.is_err() {
    return false;
  }

  let res: LuoguResp<LuoguUserData> = res.unwrap();
  let intro = res.currentData.user.introduction;

  if intro.is_none() {
    return false;
  }

  let intro = intro.unwrap();

  intro.starts_with(&token.to_string())
}

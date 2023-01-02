use uuid::Uuid;
use serde::Deserialize;

// Other fields omitted
#[derive(Deserialize)]
struct LuoguUserDetails {
  introduction: Option<String>,
}

#[derive(Deserialize)]
struct LuoguUserData {
  user: LuoguUserDetails,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct LuoguResp<T> {
  currentData: T,
}

pub async fn check_user(
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

  intro.trim()
    .starts_with(&token.to_string())
}

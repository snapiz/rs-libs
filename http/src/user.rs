use actix_web::{HttpRequest, Result};
use std::convert::TryFrom;
use timada_util::env;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Root,
    Admin,
    Staff,
    User,
}

impl AsRef<UserRole> for UserRole {
    fn as_ref(&self) -> &UserRole {
        self
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum UserState {
    Enabled,
    Disabled,
    ReadOnly,
}

impl AsRef<UserState> for UserState {
    fn as_ref(&self) -> &UserState {
        self
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: Uuid,
    pub email: Option<String>,
    pub username: Option<String>,
    pub role: UserRole,
    pub state: UserState,
}

const GATEWAY_SECRET_KEY_VAR: &str = "GATEWAY_SECRET_KEY";
const GATEWAY_SECRET_KEY_HEADER: &str = "x-gateway-key";
const GATEWAY_USER_HEADER: &str = "x-user";

impl TryFrom<&HttpRequest> for User {
    type Error = String;

    fn try_from(req: &HttpRequest) -> Result<Self, Self::Error> {
        let key = env::var(GATEWAY_SECRET_KEY_VAR);

        req.headers()
            .get(GATEWAY_SECRET_KEY_HEADER)
            .and_then(|gateway_key| gateway_key.to_str().ok())
            .and_then(|gateway_key| {
                if gateway_key == key {
                    Some(gateway_key)
                } else {
                    None
                }
            })
            .ok_or("Invalid gateway key")?;

        req.headers()
            .get(GATEWAY_USER_HEADER)
            .ok_or_else(|| "Missing user".to_owned())
            .and_then(|user| user.to_str().map_err(|e| e.to_string()))
            .and_then(|user| serde_json::from_str(user).map_err(|e| e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use actix_web::test::TestRequest;
    use std::convert::TryFrom;
    use std::env;

    use super::{
        User, UserRole, UserState, GATEWAY_SECRET_KEY_HEADER, GATEWAY_SECRET_KEY_VAR,
        GATEWAY_USER_HEADER,
    };

    #[test]
    fn try_from_request_key() {
        env::set_var(GATEWAY_SECRET_KEY_VAR, "timada");

        let req = TestRequest::default().to_http_request();

        assert_eq!(User::try_from(&req), Err("Invalid gateway key".to_owned()));

        let req = TestRequest::default()
            .header(GATEWAY_SECRET_KEY_HEADER, "wrong_key")
            .to_http_request();

        assert_eq!(User::try_from(&req), Err("Invalid gateway key".to_owned()));
    }

    #[test]
    fn try_from_request_missing_user() {
        env::set_var(GATEWAY_SECRET_KEY_VAR, "timada");

        let req = TestRequest::default()
            .header(GATEWAY_SECRET_KEY_HEADER, "timada")
            .to_http_request();

        assert_eq!(User::try_from(&req), Err("Missing user".to_owned()));
    }

    #[test]
    fn try_from_request_success() {
        env::set_var(GATEWAY_SECRET_KEY_VAR, "timada");
        let user = User {
            id: Default::default(),
            email: None,
            username: None,
            role: UserRole::User,
            state: UserState::ReadOnly,
        };
        let user_json = serde_json::to_string(&user).unwrap();
        let req = TestRequest::default()
            .header(GATEWAY_SECRET_KEY_HEADER, "timada")
            .header(GATEWAY_USER_HEADER, user_json)
            .to_http_request();

        assert_eq!(User::try_from(&req), Ok(user));
    }
}

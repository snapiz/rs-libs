use actix_web::dev::Payload;
use actix_web::{Error, FromRequest, HttpRequest, Result};
use futures::future::{ok, Ready};
use std::convert::TryFrom;

pub use super::user::{User, UserRole, UserState};

#[derive(Debug, PartialEq)]
pub enum ContextError<'a> {
    Anonymous,
    UserState(&'a UserState),
    Forbidden,
}

pub type ContextResult<'a, T> = Result<T, ContextError<'a>>;

#[derive(Debug, Default)]
pub struct Context {
    pub user: Option<User>,
}

impl Context {
    pub fn ensure_is_authorized(&self, roles: Option<Vec<UserRole>>) -> ContextResult<&User> {
        let user = self.user.as_ref().ok_or(ContextError::Anonymous)?;

        let authorized = roles
            .map(|roles| roles.iter().any(|role| &user.role == role))
            .unwrap_or(true);

        if !authorized {
            return Err(ContextError::Forbidden);
        }

        match user.state {
            UserState::Enabled => Ok(user),
            _ => Err(ContextError::UserState(&user.state)),
        }
    }
}

impl FromRequest for Context {
    type Future = Ready<Result<Context>>;
    type Error = Error;
    type Config = ();

    fn from_request(req: &HttpRequest, _pl: &mut Payload) -> Self::Future {
        let user = User::try_from(req).ok();

        ok(Self { user })
    }
}

#[cfg(test)]
mod tests {
    use super::{Context, ContextError};
    use super::{User, UserRole, UserState};

    #[test]
    fn ensure_is_authorized_anonymous() {
        let context = Context::default();

        assert_eq!(
            context.ensure_is_authorized(None),
            Err(ContextError::Anonymous)
        );
    }

    #[test]
    fn ensure_is_authorized_disabled() {
        let context = Context {
            user: Some(User {
                id: Default::default(),
                email: None,
                username: None,
                role: UserRole::User,
                state: UserState::Disabled,
            }),
        };

        assert_eq!(
            context.ensure_is_authorized(None),
            Err(ContextError::UserState(
                &context.user.as_ref().unwrap().state
            ))
        );
    }
    #[test]
    fn ensure_is_authorized_disabled_with_role() {
        let context = Context {
            user: Some(User {
                id: Default::default(),
                email: None,
                username: None,
                role: UserRole::User,
                state: UserState::Disabled,
            }),
        };

        assert_eq!(
            context.ensure_is_authorized(Some(vec![UserRole::User])),
            Err(ContextError::UserState(
                &context.user.as_ref().unwrap().state
            ))
        );
    }

    #[test]
    fn ensure_is_authorized_read_only() {
        let context = Context {
            user: Some(User {
                id: Default::default(),
                email: None,
                username: None,
                role: UserRole::User,
                state: UserState::ReadOnly,
            }),
        };

        assert_eq!(
            context.ensure_is_authorized(None),
            Err(ContextError::UserState(
                &context.user.as_ref().unwrap().state
            ))
        );
    }

    #[test]
    fn ensure_is_authorized_read_only_with_role() {
        let context = Context {
            user: Some(User {
                id: Default::default(),
                email: None,
                username: None,
                role: UserRole::User,
                state: UserState::ReadOnly,
            }),
        };

        assert_eq!(
            context.ensure_is_authorized(Some(vec![UserRole::User])),
            Err(ContextError::UserState(
                &context.user.as_ref().unwrap().state
            ))
        );
    }

    #[test]
    fn ensure_is_authorized_forbidden() {
        let context = Context {
            user: Some(User {
                id: Default::default(),
                email: None,
                username: None,
                role: UserRole::User,
                state: UserState::Enabled,
            }),
        };

        assert_eq!(
            context.ensure_is_authorized(Some(vec![UserRole::Root, UserRole::Admin])),
            Err(ContextError::Forbidden)
        );
    }

    #[test]
    fn ensure_is_authorized_success() {
        let context = Context {
            user: Some(User {
                id: Default::default(),
                email: None,
                username: None,
                role: UserRole::User,
                state: UserState::Enabled,
            }),
        };

        assert_eq!(
            context.ensure_is_authorized(None),
            Ok(context.user.as_ref().unwrap())
        );
    }

    #[test]
    fn ensure_is_authorized_success_with_role() {
        let context = Context {
            user: Some(User {
                id: Default::default(),
                email: None,
                username: None,
                role: UserRole::Admin,
                state: UserState::Enabled,
            }),
        };

        assert_eq!(
            context.ensure_is_authorized(Some(vec![UserRole::Admin])),
            Ok(context.user.as_ref().unwrap())
        );
    }
}

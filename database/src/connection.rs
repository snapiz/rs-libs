use diesel::PgConnection;
use diesel::prelude::*;
use diesel::ConnectionError;
use std::convert::From;
use std::fmt;
use timada_util::env;
use diesel::r2d2;
use diesel::r2d2::ConnectionManager;

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;
pub type PooledConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

pub struct DatabaseConnection {
    pub host: String,
    pub user: String,
    pub password: String,
    pub name: Option<String>,
}

impl DatabaseConnection {
    pub fn without_name(&self) -> Self {
        Self {
            host: self.host.to_owned(),
            user: self.user.to_owned(),
            password: self.password.to_owned(),
            name: None,
        }
    }

    pub fn establish(&self) -> Result<PgConnection, ConnectionError> {
        PgConnection::establish(&self.to_string())
    }
}

impl<'a> From<(&str, &str, &str)> for DatabaseConnection {
    fn from(value: (&str, &str, &str)) -> DatabaseConnection {
        let host = env::var(value.0);
        let user = env::var(value.1);
        let password = env::var(value.2);

        DatabaseConnection {
            host,
            user,
            password,
            name: None,
        }
    }
}

impl<'a> From<(&str, &str, &str, &str)> for DatabaseConnection {
    fn from(value: (&str, &str, &str, &str)) -> DatabaseConnection {
        let host = env::var(value.0);
        let user = env::var(value.1);
        let password = env::var(value.2);
        let name = env::var(value.3);

        DatabaseConnection {
            host,
            user,
            password,
            name: Some(name),
        }
    }
}

impl fmt::Display for DatabaseConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.name {
            Some(name) => write!(
                f,
                "postgres://{}:{}@{}/{}",
                self.user, self.password, self.host, name
            ),
            _ => write!(
                f,
                "postgres://{}:{}@{}",
                self.user, self.password, self.host,
            ),
        }
    }
}

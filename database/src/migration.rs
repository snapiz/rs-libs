use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::{ConnectionError, PgConnection};
use diesel_migrations as migrations;
use diesel_migrations::RunMigrationsError;
use std::convert::From;
use std::env;
use std::io::stdout;

use super::connection::DatabaseConnection;

#[derive(Debug, PartialEq)]
pub enum MigrationError {
    Diesel(DieselError),
    DieselConnection(ConnectionError),
    RunMigrations(RunMigrationsError),
    FixtureDenied(String),
    MissingDatabaseName,
}

impl From<DieselError> for MigrationError {
    fn from(e: DieselError) -> MigrationError {
        MigrationError::Diesel(e)
    }
}

impl From<RunMigrationsError> for MigrationError {
    fn from(e: RunMigrationsError) -> MigrationError {
        MigrationError::RunMigrations(e)
    }
}

impl From<ConnectionError> for MigrationError {
    fn from(e: ConnectionError) -> MigrationError {
        MigrationError::DieselConnection(e)
    }
}

pub type MigrationResult<T> = Result<T, MigrationError>;

table! {
    pg_database (datname) {
        datname -> Text,
        datistemplate -> Bool,
    }
}

pub fn pg_database_exists(conn: &PgConnection, database_name: &str) -> QueryResult<bool> {
    use self::pg_database::dsl::*;

    pg_database
        .select(datname)
        .filter(datname.eq(database_name))
        .filter(datistemplate.eq(false))
        .get_result::<String>(conn)
        .optional()
        .map(|x| x.is_some())
}

pub fn create_database(connection: &PgConnection, name: &str) -> QueryResult<usize> {
    connection.execute(&format!("CREATE DATABASE {}", name))
}

pub fn drop_database(connection: &PgConnection, name: &str) -> QueryResult<usize> {
    connection.execute(&format!("DROP DATABASE {}", name))
}

pub fn kill_database_connections(connection: &PgConnection, name: &str) -> QueryResult<usize> {
    connection.execute(&format!(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE pid <> pg_backend_pid() AND datname = '{}'",
        name
    ))
}

pub fn create_database_if_not_exists(connection: &PgConnection, name: &str) -> QueryResult<usize> {
    pg_database_exists(connection, name).and_then(|exists| {
        if exists {
            Ok(0)
        } else {
            create_database(connection, name)
        }
    })
}

pub fn drop_database_if_exists(connection: &PgConnection, name: &str) -> QueryResult<usize> {
    pg_database_exists(connection, name).and_then(|exists| {
        if exists {
            drop_database(connection, name)
        } else {
            Ok(0)
        }
    })
}

pub fn migrate(connection: &PgConnection, directory: &str) -> Result<(), RunMigrationsError> {
    let migration_dir = env::current_dir()
        .expect("Failed to get current dir")
        .join(directory);

    migrations::run_pending_migrations_in_directory(connection, &migration_dir, &mut stdout())
}

pub fn setup(config: &DatabaseConnection) -> MigrationResult<()> {
    let connection = config.without_name().establish()?;
    let db_name = config
        .name
        .as_ref()
        .ok_or(MigrationError::MissingDatabaseName)?;
    create_database_if_not_exists(&connection, db_name)?;
    let connection = config.establish()?;
    Ok(migrate(&connection, "migrations")?)
}

pub fn reset(config: &DatabaseConnection) -> MigrationResult<()> {
    let db_name = config
        .name
        .as_ref()
        .ok_or(MigrationError::MissingDatabaseName)?;
    if !db_name.ends_with("_dev") {
        return Err(MigrationError::FixtureDenied(db_name.to_owned()));
    }

    {
        let connection = config.establish()?;
        kill_database_connections(&connection, &db_name)?;
    }

    let connection = config.without_name().establish()?;
    drop_database_if_exists(&connection, &db_name)?;
    create_database(&connection, &db_name)?;

    let connection = config.establish()?;
    Ok(migrate(&connection, "migrations")?)
}

pub fn fixture(config: &DatabaseConnection) -> MigrationResult<()> {
    let connection = config.establish()?;
    Ok(migrate(&connection, "fixtures")?)
}

#[cfg(test)]
mod tests {
    use diesel::prelude::*;
    use std::env;
    use uuid::Uuid;

    use super::{DatabaseConnection, MigrationError};

    table! {
        todos (id) {
            id -> Uuid,
            text -> Varchar,
            is_done -> Bool,
        }
    }

    #[derive(Debug, Queryable, PartialEq)]
    pub struct Todo {
        pub id: Uuid,
        pub text: String,
        pub is_done: bool,
    }

    #[test]
    fn migratation() {
        use self::todos::dsl::{id, todos};

        let host = env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_owned());
        let user = env::var("DB_USER").unwrap_or_else(|_| "root".to_owned());
        let password = env::var("DB_PASSWORD").unwrap_or_else(|_| "root".to_owned());

        let config = &DatabaseConnection {
            host,
            user,
            password,
            name: Some("timada_database_dev".to_owned()),
        };

        assert_eq!(super::setup(&config), Ok(()));
        assert_eq!(super::reset(&config), Ok(()));
        assert_eq!(super::fixture(&config), Ok(()));

        let connection = config.establish().unwrap();
        let todo = todos.first::<Todo>(&connection).unwrap();
        let todo1 = Todo {
            id: Uuid::parse_str("fb1de7a6-996f-48c6-9973-f434852ad843").unwrap(),
            text: "Todo 1".to_owned(),
            is_done: true,
        };

        assert_eq!(&todo, &todo1);

        diesel::delete(todos.filter(id.eq(todo.id)))
            .execute(&connection)
            .unwrap();

        let todo = todos.first::<Todo>(&connection).unwrap();

        assert_eq!(
            todo,
            Todo {
                id: Uuid::parse_str("29eab018-54bc-4edb-9f0e-c63c975b1b36").unwrap(),
                text: "Todo 2".to_owned(),
                is_done: true
            }
        );

        assert_eq!(super::reset(&config), Ok(()));
        assert_eq!(super::fixture(&config), Ok(()));

        let connection = config.establish().unwrap();
        let todo = todos.first::<Todo>(&connection).unwrap();
        assert_eq!(&todo, &todo1);
    }

    #[test]
    fn reset_bad_db_name() {
        let host = env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_owned());
        let user = env::var("DB_USER").unwrap_or_else(|_| "root".to_owned());
        let password = env::var("DB_PASSWORD").unwrap_or_else(|_| "root".to_owned());

        let config = &DatabaseConnection {
            host,
            user,
            password,
            name: Some("timada".to_owned()),
        };

        assert_eq!(
            super::reset(&config),
            Err(MigrationError::FixtureDenied("timada".to_owned()))
        );
    }
}

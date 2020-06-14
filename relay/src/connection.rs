use diesel::result::Error as DieselError;
use std::convert::From;

use super::cursor::CursorError;

#[derive(Debug, PartialEq)]
pub enum ConnectionError {
    Cursor(CursorError),
    Diesel(DieselError),
    Custom(String),
}

impl From<CursorError> for ConnectionError {
    fn from(e: CursorError) -> ConnectionError {
        ConnectionError::Cursor(e)
    }
}

impl From<DieselError> for ConnectionError {
    fn from(e: DieselError) -> ConnectionError {
        ConnectionError::Diesel(e)
    }
}

pub type ConnectionResult<T> = Result<T, ConnectionError>;

#[macro_export]
macro_rules! resolve_connection {
    ($model:ident, $conn:ident, $table:ident, $first:ident, $after:ident, $last:ident, $before:ident, $key_field:ident, $order_field:ident, $to_cursor:ident, $from_cursor:ident) => {{
        use async_graphql::{Connection, Cursor, EmptyEdgeFields, PageInfo};

        let backward =
            ($last.is_some() || $before.is_some()) && $first.is_none() && $after.is_none();

        let (limit, cursor) = if backward {
            ($last.unwrap_or(40), $before.as_ref())
        } else {
            ($first.unwrap_or(40), $after.as_ref())
        };

        let mut table = $table.limit((limit + 1) as i64);

        if let Some(cursor) = cursor {
            let (key_value, order_value) = $crate::from_cursor(&cursor)?;
            let (key_value, order_value) = $from_cursor(&key_value, &order_value)?;

            table = if backward {
                table
                    .filter($order_field.lt(order_value))
                    .or_filter($order_field.eq(order_value).and($key_field.lt(key_value)))
            } else {
                table
                    .filter($order_field.gt(order_value))
                    .or_filter($order_field.eq(order_value).and($key_field.gt(key_value)))
            };
        }

        table = if backward {
            table.order(($order_field.desc(), $key_field.desc()))
        } else {
            table.order(($order_field.asc(), $key_field.asc()))
        };

        let rows = table.load::<$model>($conn)?.into_iter().map(|row| {
            let (key_value, order_value) = $to_cursor(&row);
            let cursor = $crate::to_cursor(&key_value, &order_value);

            (Cursor::from(cursor), EmptyEdgeFields {}, row)
        });

        let mut nodes: Vec<(Cursor, EmptyEdgeFields, $model)> = if backward {
            rows.rev().collect()
        } else {
            rows.collect()
        };

        let len = nodes.len();
        let has_more = len > limit as usize;
        let remove_index = if backward { 0 } else { len - 1 };

        if has_more {
            nodes.remove(remove_index);
        };

        let page_info = if backward {
            let start_cursor = nodes.first().map(|(cursor, _, _)| cursor.clone());

            PageInfo {
                has_previous_page: has_more,
                has_next_page: false,
                start_cursor,
                end_cursor: None,
            }
        } else {
            let end_cursor = nodes.last().map(|(cursor, _, _)| cursor.clone());

            PageInfo {
                has_previous_page: false,
                has_next_page: has_more,
                start_cursor: None,
                end_cursor,
            }
        };

        Ok(Connection {
            total_count: None,
            page_info,
            nodes,
        })
    }};
}

#[cfg(test)]
mod tests {
    use async_graphql::{Connection, Cursor, ID};
    use chrono::{DateTime, Utc};
    use diesel::prelude::*;
    use futures_await_test::async_test;
    use std::env;
    use timada_database::DatabaseConnection;
    use uuid::Uuid;

    use super::{ConnectionError, ConnectionResult};
    use crate::uuid::to_id;

    table! {
        todos (id) {
            id -> Uuid,
            text -> Varchar,
            is_done -> Bool,
            created_at -> Timestamptz,
        }
    }

    #[derive(Debug, Queryable, PartialEq, Clone)]
    pub struct Todo {
        pub id: Uuid,
        pub text: String,
        pub is_done: bool,
        pub created_at: DateTime<Utc>,
    }

    #[async_graphql::Object]
    impl Todo {
        #[field]
        async fn id(&self) -> ID {
            to_id("User", &self.id)
        }

        #[field]
        async fn text(&self) -> &str {
            self.text.as_str()
        }

        #[field]
        async fn is_done(&self) -> bool {
            self.is_done
        }
    }

    lazy_static::lazy_static! {
        pub static ref TODO_1: Todo = Todo {
            id: Uuid::parse_str("fb1de7a6-996f-48c6-9973-f434852ad843").unwrap(),
            text: "Todo 1".to_owned(),
            is_done: true,
            created_at: DateTime::parse_from_rfc3339("2020-01-01T00:00:00.010Z").map(DateTime::<Utc>::from).unwrap()
        };
        pub static ref TODO_2: Todo = Todo {
            id: Uuid::parse_str("29eab018-54bc-4edb-9f0e-c63c975b1b36").unwrap(),
            text: "Todo 2".to_owned(),
            is_done: true,
            created_at: DateTime::parse_from_rfc3339("2020-01-01T00:00:00.010Z").map(DateTime::<Utc>::from).unwrap()
        };
        pub static ref TODO_3: Todo = Todo {
            id: Uuid::parse_str("6a45fd71-cc32-4eeb-823e-e8ef08ecd004").unwrap(),
            text: "Todo 3".to_owned(),
            is_done: false,
            created_at: DateTime::parse_from_rfc3339("2020-01-01T00:00:00.010Z").map(DateTime::<Utc>::from).unwrap()
        };
        pub static ref TODO_4: Todo = Todo {
            id: Uuid::parse_str("7f2a35d7-6e20-40bf-9f35-91cb7ca7e8d6").unwrap(),
            text: "Todo 4".to_owned(),
            is_done: false,
            created_at: DateTime::parse_from_rfc3339("2020-01-01T00:00:00.020Z").map(DateTime::<Utc>::from).unwrap()
        };
        pub static ref TODO_5: Todo = Todo {
            id: Uuid::parse_str("0035b208-34fb-4548-ba20-cd9dcbe717fa").unwrap(),
            text: "Todo 5".to_owned(),
            is_done: false,
            created_at: DateTime::parse_from_rfc3339("2020-01-07T00:00:00.000Z").map(DateTime::<Utc>::from).unwrap()
        };
    }

    fn connection() -> diesel::PgConnection {
        let host = env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_owned());
        let user = env::var("DB_USER").unwrap_or_else(|_| "root".to_owned());
        let password = env::var("DB_PASSWORD").unwrap_or_else(|_| "root".to_owned());

        let config = DatabaseConnection {
            host,
            user,
            password,
            name: Some("timada_relay_dev".to_owned()),
        };

        timada_database::setup(&config).unwrap();
        timada_database::fixture(&config).unwrap();

        config.establish().unwrap()
    }

    fn to_todo_cursor(todo: &Todo) -> (String, String) {
        (todo.id.to_string(), todo.created_at.to_rfc3339())
    }

    fn from_todo_cursor(
        key_value: &str,
        order_value: &str,
    ) -> ConnectionResult<(Uuid, DateTime<Utc>)> {
        let key_value =
            Uuid::parse_str(key_value).map_err(|e| ConnectionError::Custom(e.to_string()))?;
        let order_value = DateTime::parse_from_rfc3339(order_value)
            .map(DateTime::<Utc>::from)
            .map_err(|e| ConnectionError::Custom(e.to_string()))?;

        Ok((key_value, order_value))
    }

    fn resolve_connection(
        first: Option<usize>,
        after: Option<String>,
        last: Option<usize>,
        before: Option<String>,
    ) -> ConnectionResult<Connection<Todo>> {
        use self::todos::dsl::{created_at, id, todos};

        let conn = &connection();
        let table = todos.into_boxed();

        crate::resolve_connection!(
            Todo,
            conn,
            table,
            first,
            after,
            last,
            before,
            id,
            created_at,
            to_todo_cursor,
            from_todo_cursor
        )
    }

    #[async_test]
    async fn resolve_connection_no_args() {
        let res = resolve_connection(None, None, None, None).unwrap();
        let page_info = res.page_info().await;

        assert_eq!(page_info.has_previous_page, false);
        assert_eq!(page_info.has_next_page, false);
        assert_eq!(page_info.start_cursor, None);
        assert_eq!(page_info.end_cursor, Some(Cursor::from("MDAzNWIyMDgtMzRmYi00NTQ4LWJhMjAtY2Q5ZGNiZTcxN2ZhOjIwMjAtMDEtMDdUMDA6MDA6MDArMDA6MDA=")));

        let mut nodes = Vec::new();
        let edges = res.edges().await.unwrap();

        for edge in edges.iter() {
            let edge = edge.as_ref().unwrap();
            nodes.push(edge.node().await);
        }

        assert_eq!(
            nodes,
            vec![
                &TODO_2.clone(),
                &TODO_3.clone(),
                &TODO_1.clone(),
                &TODO_4.clone(),
                &TODO_5.clone()
            ]
        );
    }

    #[async_test]
    async fn resolve_connection_first() {
        let mut nodes = Vec::new();
        let res = resolve_connection(Some(2), None, None, None).unwrap();
        let page_info = res.page_info().await;

        assert_eq!(page_info.has_previous_page, false);
        assert_eq!(page_info.has_next_page, true);
        assert_eq!(page_info.start_cursor, None);
        assert_eq!(page_info.end_cursor, Some(Cursor::from("NmE0NWZkNzEtY2MzMi00ZWViLTgyM2UtZThlZjA4ZWNkMDA0OjIwMjAtMDEtMDFUMDA6MDA6MDAuMDEwKzAwOjAw")));

        let edges = res.edges().await.unwrap();

        for edge in edges.iter() {
            let edge = edge.as_ref().unwrap();
            nodes.push(edge.node().await);
        }

        assert_eq!(nodes, vec![&TODO_2.clone(), &TODO_3.clone()]);
    }

    #[async_test]
    async fn resolve_connection_first_after() {
        let mut nodes = Vec::new();
        let res = resolve_connection(Some(2), Some("NmE0NWZkNzEtY2MzMi00ZWViLTgyM2UtZThlZjA4ZWNkMDA0OjIwMjAtMDEtMDFUMDA6MDA6MDAuMDEwKzAwOjAw".to_owned()), None, None).unwrap();
        let page_info = res.page_info().await;

        assert_eq!(page_info.has_previous_page, false);
        assert_eq!(page_info.has_next_page, true);
        assert_eq!(page_info.start_cursor, None);
        assert_eq!(page_info.end_cursor, Some(Cursor::from("N2YyYTM1ZDctNmUyMC00MGJmLTlmMzUtOTFjYjdjYTdlOGQ2OjIwMjAtMDEtMDFUMDA6MDA6MDAuMDIwKzAwOjAw")));

        let edges = res.edges().await.unwrap();

        for edge in edges.iter() {
            let edge = edge.as_ref().unwrap();
            nodes.push(edge.node().await);
        }

        assert_eq!(nodes, vec![&TODO_1.clone(), &TODO_4.clone()]);
    }

    #[async_test]
    async fn resolve_connection_last() {
        let mut nodes = Vec::new();
        let res = resolve_connection(None, None, Some(2), None).unwrap();
        let page_info = res.page_info().await;

        assert_eq!(page_info.has_previous_page, true);
        assert_eq!(page_info.has_next_page, false);
        assert_eq!(page_info.start_cursor, Some(Cursor::from("N2YyYTM1ZDctNmUyMC00MGJmLTlmMzUtOTFjYjdjYTdlOGQ2OjIwMjAtMDEtMDFUMDA6MDA6MDAuMDIwKzAwOjAw")));
        assert_eq!(page_info.end_cursor, None);

        let edges = res.edges().await.unwrap();

        for edge in edges.iter() {
            let edge = edge.as_ref().unwrap();
            nodes.push(edge.node().await);
        }

        assert_eq!(nodes, vec![&TODO_4.clone(), &TODO_5.clone()]);
    }

    #[async_test]
    async fn resolve_connection_last_before() {
        let mut nodes = Vec::new();
        let res = resolve_connection(None, None, Some(2), Some("N2YyYTM1ZDctNmUyMC00MGJmLTlmMzUtOTFjYjdjYTdlOGQ2OjIwMjAtMDEtMDFUMDA6MDA6MDAuMDIwKzAwOjAw".to_owned())).unwrap();
        let page_info = res.page_info().await;

        assert_eq!(page_info.has_previous_page, true);
        assert_eq!(page_info.has_next_page, false);
        assert_eq!(page_info.start_cursor, Some(Cursor::from("NmE0NWZkNzEtY2MzMi00ZWViLTgyM2UtZThlZjA4ZWNkMDA0OjIwMjAtMDEtMDFUMDA6MDA6MDAuMDEwKzAwOjAw")));
        assert_eq!(page_info.end_cursor, None);

        let edges = res.edges().await.unwrap();

        for edge in edges.iter() {
            let edge = edge.as_ref().unwrap();
            nodes.push(edge.node().await);
        }

        assert_eq!(nodes, vec![&TODO_3.clone(), &TODO_1.clone()]);
    }
}

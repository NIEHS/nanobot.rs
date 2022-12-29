use serde::{Deserialize, Serialize};
use serde_json::{from_str, Map, Value};
use sqlx::sqlite::{SqlitePool, SqliteRow};
use sqlx::Row;

pub const LIMIT_MAX: usize = 100;
pub const LIMIT_DEFAULT: usize = 10; // TODO: 100?

#[derive(Clone, Debug, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]
pub enum Operator {
    EQUALS,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]
pub enum Direction {
    ASC,
    DESC,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Select {
    pub table: String,
    pub select: Vec<String>,
    pub filter: Vec<(String, Operator, Value)>,
    pub order: Vec<(String, Direction)>,
    pub limit: usize,
    pub offset: usize,
}

impl Select {
    pub fn new() -> Select {
        Default::default()
    }

    pub fn clone(select: &Select) -> Select {
        Select { ..select.clone() }
    }

    pub fn table<'a, S: Into<String>>(&'a mut self, table: S) -> &'a mut Select {
        self.table = table.into();
        self
    }

    pub fn select<'a, S: Into<String>>(&'a mut self, select: Vec<S>) -> &'a mut Select {
        for s in select {
            self.select.push(s.into());
        }
        self
    }

    pub fn filter<'a, S: Into<String>>(
        &'a mut self,
        filter: Vec<(S, Operator, Value)>,
    ) -> &'a mut Select {
        for (s, o, v) in filter {
            self.filter.push((s.into(), o, v));
        }
        self
    }

    pub fn order<'a, S: Into<String>>(&'a mut self, order: Vec<(S, Direction)>) -> &'a mut Select {
        for (s, d) in order {
            self.order.push((s.into(), d));
        }
        self
    }

    pub fn limit<'a>(&'a mut self, limit: usize) -> &'a mut Select {
        self.limit = limit;
        self
    }

    pub fn offset<'a>(&'a mut self, offset: usize) -> &'a mut Select {
        self.offset = offset;
        self
    }
}

/// Convert a Select struct to a SQL string.
///
/// ```sql
/// SELECT json_object(
///     'table', "table",
///     'path', "path",
///     'type', "type",
///     'description', "description"
/// ) AS json_result
/// FROM "table";
/// ```
///
/// # Examples
///
/// ```
/// assert_eq!("foo", "foo");
/// ```
pub fn select_to_sql(s: &Select) -> String {
    let mut lines: Vec<String> = vec!["SELECT json_object(".to_string()];
    let parts: Vec<String> = s
        .select
        .iter()
        .map(|c| format!(r#"'{}', "{}""#, c, c))
        .collect();
    lines.push(format!("  {}", parts.join(",\n  ")));
    lines.push(") AS json_result".to_string());
    lines.push(format!(r#"FROM "{}""#, s.table));
    let mut filters: Vec<String> = vec![];
    if s.filter.len() > 0 {
        for filter in &s.filter {
            filters.push(format!(
                r#""{}" = '{}'"#,
                filter.0,
                filter.2.as_str().unwrap().to_string()
            ));
        }
        lines.push(format!("WHERE {}", filters.join("\n  AND ")));
    }
    if s.order.len() > 0 {
        let parts: Vec<String> = s
            .order
            .iter()
            .map(|(c, d)| format!(r#""{}" {:?}"#, c, d))
            .collect();
        lines.push(format!("ORDER BY {}", parts.join(", ")));
    }
    if s.limit > 0 {
        lines.push(format!("LIMIT {}", s.limit));
    }
    if s.offset > 0 {
        lines.push(format!("OFFSET {}", s.offset));
    }
    lines.join("\n")
}

// TODO: remove duplicate code
pub fn select_to_sql_count(s: &Select) -> String {
    let mut lines: Vec<String> = vec!["SELECT COUNT(*) AS count".to_string()];
    lines.push(format!(r#"FROM "{}""#, s.table));
    let mut filters: Vec<String> = vec![];
    if s.filter.len() > 0 {
        for filter in &s.filter {
            filters.push(format!(
                r#""{}" = '{}'"#,
                filter.0,
                filter.2.as_str().unwrap().to_string()
            ));
        }
        lines.push(format!("WHERE {}", filters.join("\n  AND ")));
    }
    lines.join("\n")
}

pub fn select_to_url(s: &Select) -> String {
    let mut params: Vec<String> = vec![];
    if s.filter.len() > 0 {
        for filter in &s.filter {
            params.push(format!(
                r#"{}=eq.{}"#,
                filter.0,
                filter.2.as_str().unwrap().to_string()
            ));
        }
    }
    if s.order.len() > 0 {
        let parts: Vec<String> = s
            .order
            .iter()
            .map(|(c, d)| format!(r#"{}.{}"#, c, format!("{:?}", d).to_lowercase()))
            .collect();
        params.push(format!("order={}", parts.join(", ")));
    }
    if s.limit > 0 && s.limit != LIMIT_DEFAULT {
        params.push(format!("limit={}", s.limit));
    }
    if s.offset > 0 {
        params.push(format!("offset={}", s.offset));
    }
    if params.len() > 0 {
        format!("{}?{}", s.table, params.join("&"))
    } else {
        s.table.clone()
    }
}

pub async fn get_table_from_pool(
    pool: &SqlitePool,
    select: &Select,
) -> Result<Vec<Map<String, Value>>, sqlx::Error> {
    let sql = select_to_sql(select);
    let rows: Vec<SqliteRow> = sqlx::query(&sql).fetch_all(pool).await?;
    Ok(rows
        .iter()
        .map(|row| {
            let result: &str = row.get("json_result");
            from_str::<Map<String, Value>>(&result).unwrap()
        })
        .collect())
}

pub async fn get_count_from_pool(pool: &SqlitePool, select: &Select) -> Result<usize, sqlx::Error> {
    let sql = select_to_sql_count(select);
    let row: SqliteRow = sqlx::query(&sql).fetch_one(pool).await?;
    let count: usize = usize::try_from(row.get::<i64, &str>("count")).unwrap();
    Ok(count)
}

pub fn rows_to_map(rows: Vec<Map<String, Value>>, column: &str) -> Map<String, Value> {
    let mut map = Map::new();
    for row in rows.iter() {
        // we want to drop one key (column), but remove does not preserve order
        // https://github.com/serde-rs/json/issues/807
        let mut r = Map::new();
        let mut key = String::from("");
        for (k, v) in row.iter() {
            if k == column {
                key = v.as_str().unwrap().to_string();
            } else {
                r.insert(k.to_string(), v.clone());
            }
        }
        map.insert(key, Value::Object(r));
    }
    map
}

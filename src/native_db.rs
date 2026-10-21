// Databases for built programs, on SQLite. It goes into programs that use a database, as
// `rt::db`, next to the rest of the runtime. lust uses the same code to give LustScript its
// databases, so both say the same things.
//
// A table is used like a cMap: the first column is the key. Queries are built up step by step
// (:where, :orderBy, :reverse, :first) and only run when their rows are needed.

use rusqlite::types::{Value, ValueRef};

#[derive(Clone, Debug)]
pub enum SqlVal {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
}

impl SqlVal {
    fn value(&self) -> Value {
        match self {
            SqlVal::Null => Value::Null,
            SqlVal::Int(i) => Value::Integer(*i),
            SqlVal::Real(f) => Value::Real(*f),
            SqlVal::Text(s) => Value::Text(s.clone()),
        }
    }
}

pub struct Db {
    conn: rusqlite::Connection,
    path: String,
}

fn sql_type(kind: &str, key: bool) -> String {
    let t = match kind {
        "int" | "truth" => "INTEGER",
        "float" => "REAL",
        _ => "TEXT",
    };
    if key && t == "INTEGER" && kind == "int" {
        // New keys count on from the biggest there's ever been, as in a cMap: a removed row's
        // key isn't handed out again.
        format!("{t} PRIMARY KEY AUTOINCREMENT")
    } else if key {
        format!("{t} PRIMARY KEY")
    } else {
        format!("{t} NOT NULL")
    }
}

/// Opens the database file, making it and its tables if they aren't there yet. A table that's
/// there already must have the same columns.
pub fn open_db(path: &str, tables: &[(&str, &[(&str, &str)])]) -> Result<Db, String> {
    let conn = rusqlite::Connection::open(path).map_err(|e| format!("can't open the database {path}: {e}."))?;
    let db = Db { conn, path: path.to_string() };
    for (name, cols) in tables {
        let have: Vec<(String, String)> = {
            let mut st = db.conn.prepare(&format!("PRAGMA table_info(\"{name}\")")).map_err(|e| e.to_string())?;
            let rows = st
                .query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, String>(2)?)))
                .map_err(|e| e.to_string())?;
            rows.filter_map(|r| r.ok()).collect()
        };
        if have.is_empty() {
            let defs: Vec<String> = cols.iter().enumerate().map(|(i, (c, k))| format!("\"{c}\" {}", sql_type(k, i == 0))).collect();
            db.conn
                .execute(&format!("CREATE TABLE \"{name}\" ({})", defs.join(", ")), [])
                .map_err(|e| format!("can't make the table {name} in {path}: {e}."))?;
        } else {
            let want: Vec<(String, String)> = cols
                .iter()
                .enumerate()
                .map(|(i, (c, k))| (c.to_string(), sql_type(k, i == 0).split(' ').next().unwrap().to_string()))
                .collect();
            let same = have.len() == want.len() && have.iter().zip(&want).all(|(h, w)| h.0 == w.0 && h.1.eq_ignore_ascii_case(&w.1));
            if !same {
                let shown: Vec<String> = have.iter().map(|(c, _)| c.clone()).collect();
                return Err(format!(
                    "{path} already has a table called {name}, with other columns ({}). Use another file, or another table name.",
                    shown.join(", ")
                ));
            }
        }
    }
    Ok(db)
}

pub fn exec(db: &Db, sql: &str, params: &[SqlVal]) -> Result<usize, String> {
    let vals: Vec<Value> = params.iter().map(|p| p.value()).collect();
    db.conn
        .execute(sql, rusqlite::params_from_iter(vals))
        .map_err(|e| format!("the database {} said: {e}.", db.path))
}

pub fn rows(db: &Db, sql: &str, params: &[SqlVal]) -> Result<Vec<Vec<SqlVal>>, String> {
    let vals: Vec<Value> = params.iter().map(|p| p.value()).collect();
    let mut st = db.conn.prepare(sql).map_err(|e| format!("the database {} said: {e}.", db.path))?;
    let n = st.column_count();
    let mut out = Vec::new();
    let mut q = st.query(rusqlite::params_from_iter(vals)).map_err(|e| format!("the database {} said: {e}.", db.path))?;
    while let Some(r) = q.next().map_err(|e| format!("the database {} said: {e}.", db.path))? {
        let mut row = Vec::with_capacity(n);
        for i in 0..n {
            row.push(match r.get_ref(i).map_err(|e| e.to_string())? {
                ValueRef::Null => SqlVal::Null,
                ValueRef::Integer(x) => SqlVal::Int(x),
                ValueRef::Real(x) => SqlVal::Real(x),
                ValueRef::Text(t) => SqlVal::Text(String::from_utf8_lossy(t).to_string()),
                ValueRef::Blob(b) => SqlVal::Text(String::from_utf8_lossy(b).to_string()),
            });
        }
        out.push(row);
    }
    Ok(out)
}

// ----- Lust's side: tables and queries -----

/// A table, or some of its rows. A table is a query that picks every row.
#[derive(Clone)]
pub struct Q {
    db: Rc<Db>,
    table: &'static str,
    shown: &'static str,
    cols: &'static [&'static str],
    textkey: bool,
    wheres: Vec<String>,
    params: Vec<SqlVal>,
    order: Option<&'static str>,
    desc: bool,
    limit: Option<i64>,
}

pub fn table(db: Rc<Db>, table: &'static str, shown: &'static str, cols: &'static [&'static str], textkey: bool) -> Q {
    Q { db, table, shown, cols, textkey, wheres: vec![], params: vec![], order: None, desc: false, limit: None }
}

pub fn open(path: &str, tables: &[(&str, &[(&str, &str)])]) -> Rc<Db> {
    match open_db(path, tables) {
        Ok(d) => Rc::new(d),
        Err(e) => fail(e),
    }
}

fn key(q: &Q) -> &'static str {
    q.cols[0]
}

fn keytext(_q: &Q, k: &SqlVal) -> String {
    match k {
        SqlVal::Text(s) => qtext(s),
        SqlVal::Int(i) => itext(*i),
        SqlVal::Real(f) => ftext(*f),
        SqlVal::Null => "nothing".into(),
    }
}

fn ok<T>(r: Result<T, String>) -> T {
    match r {
        Ok(v) => v,
        Err(e) => fail(e),
    }
}

/// SELECT what FROM the table, with the query's filters, order and limit.
fn select(q: &Q, what: &str) -> String {
    let mut s = format!("SELECT {what} FROM \"{}\"", q.table);
    if !q.wheres.is_empty() {
        s.push_str(&format!(" WHERE {}", q.wheres.iter().map(|w| format!("({w})")).collect::<Vec<_>>().join(" AND ")));
    }
    let dir = if q.desc { " DESC" } else { "" };
    match q.order {
        Some(c) if c != key(q) => s.push_str(&format!(" ORDER BY \"{c}\"{dir}, \"{}\"{dir}", key(q))),
        _ => s.push_str(&format!(" ORDER BY \"{}\"{dir}", key(q))),
    }
    if let Some(n) = q.limit {
        s.push_str(&format!(" LIMIT {n}"));
    }
    s
}

pub fn q_where(q: &Q, sql: &str, params: Vec<SqlVal>) -> Q {
    let mut n = q.clone();
    n.wheres.push(sql.to_string());
    n.params.extend(params);
    n
}

pub fn q_order(q: &Q, col: &'static str) -> Q {
    let mut n = q.clone();
    n.order = Some(col);
    n.desc = false;
    n
}

pub fn q_reverse(q: &Q) -> Q {
    let mut n = q.clone();
    n.desc = !n.desc;
    n
}

pub fn q_first(q: &Q, count: i64) -> Q {
    if count < 0 {
        fail(format!("first needs 0 or more, not {count}."));
    }
    let mut n = q.clone();
    n.limit = Some(count);
    n
}

pub fn q_rows(q: &Q) -> Vec<Vec<SqlVal>> {
    let cols: Vec<String> = q.cols.iter().map(|c| format!("\"{c}\"")).collect();
    ok(rows(&q.db, &select(q, &cols.join(", ")), &q.params))
}

pub fn q_count(q: &Q) -> i64 {
    let inner = select(q, &format!("\"{}\"", key(q)));
    match ok(rows(&q.db, &format!("SELECT COUNT(*) FROM ({inner})"), &q.params)).first().and_then(|r| r.first()) {
        Some(SqlVal::Int(n)) => *n,
        _ => 0,
    }
}

pub fn q_delete(q: &Q) {
    let inner = select(q, &format!("\"{}\"", key(q)));
    ok(exec(&q.db, &format!("DELETE FROM \"{}\" WHERE \"{}\" IN ({inner})", q.table, key(q)), &q.params));
}

pub fn t_has(q: &Q, k: SqlVal) -> bool {
    let sql = format!("SELECT 1 FROM \"{}\" WHERE \"{}\" = ?", q.table, key(q));
    !ok(rows(&q.db, &sql, &[k])).is_empty()
}

pub fn t_inject(q: &Q, mut values: Vec<SqlVal>) {
    let with_key = values.len() == q.cols.len();
    if with_key && t_has(q, values[0].clone()) {
        fail(format!("there's already a row with {} {} in {}.", key(q), keytext(q, &values[0]), q.shown));
    }
    let cols: Vec<&str> = if with_key { q.cols.to_vec() } else { q.cols[1..].to_vec() };
    if !with_key && q.textkey {
        fail(format!("{}'s key ({}) is text, so each new row needs it first.", q.shown, key(q)));
    }
    let names: Vec<String> = cols.iter().map(|c| format!("\"{c}\"")).collect();
    let marks: Vec<&str> = cols.iter().map(|_| "?").collect();
    let sql = format!("INSERT INTO \"{}\" ({}) VALUES ({})", q.table, names.join(", "), marks.join(", "));
    values.truncate(cols.len());
    ok(exec(&q.db, &sql, &values));
}

pub fn t_remove(q: &Q, k: SqlVal) {
    let sql = format!("DELETE FROM \"{}\" WHERE \"{}\" = ?", q.table, key(q));
    if ok(exec(&q.db, &sql, std::slice::from_ref(&k))) == 0 {
        fail(format!("there's no row with {} {} in {} to remove.", key(q), keytext(q, &k), q.shown));
    }
}

/// One row, found by its key. Reading and changing it goes to the database each time.
#[derive(Clone)]
pub struct RowRef {
    q: Q,
    k: SqlVal,
}

pub fn t_get(q: &Q, k: SqlVal) -> RowRef {
    if !t_has(q, k.clone()) {
        fail(format!("there's no row with {} {} in {}.", key(q), keytext(q, &k), q.shown));
    }
    RowRef { q: q.clone(), k }
}

fn gone(r: &RowRef) -> ! {
    fail(format!("this row of {} ({} {}) has been removed.", r.q.shown, key(&r.q), keytext(&r.q, &r.k)))
}

pub fn r_get(r: &RowRef, col: &str) -> SqlVal {
    let sql = format!("SELECT \"{col}\" FROM \"{}\" WHERE \"{}\" = ?", r.q.table, key(&r.q));
    match ok(rows(&r.q.db, &sql, std::slice::from_ref(&r.k))).into_iter().next() {
        Some(mut row) => row.remove(0),
        None => gone(r),
    }
}

pub fn r_set(r: &RowRef, col: &str, v: SqlVal) {
    let sql = format!("UPDATE \"{}\" SET \"{col}\" = ? WHERE \"{}\" = ?", r.q.table, key(&r.q));
    if ok(exec(&r.q.db, &sql, &[v, r.k.clone()])) == 0 {
        gone(r);
    }
}

/// The rows as a cMap of their own: a copy, so changing it doesn't change the database.
pub fn q_cmap<R: Row>(q: &Q, decode: &dyn Fn(&[SqlVal]) -> (R::K, R)) -> CMap<R> {
    let rows: Vec<(Option<R::K>, R)> = q_rows(q)
        .iter()
        .map(|r| {
            let (k, row) = decode(r);
            (Some(k), row)
        })
        .collect();
    carr(key(q), None, rows)
}

// Values going in and coming out.
pub fn sv_int(v: i64) -> SqlVal {
    SqlVal::Int(v)
}
pub fn sv_float(v: f64) -> SqlVal {
    SqlVal::Real(v)
}
pub fn sv_string(v: String) -> SqlVal {
    SqlVal::Text(v)
}
pub fn sv_truth(v: bool) -> SqlVal {
    SqlVal::Int(v as i64)
}
pub fn sv_state(v: &'static str) -> SqlVal {
    SqlVal::Text(v.to_string())
}
pub fn v_int(v: &SqlVal) -> i64 {
    match v {
        SqlVal::Int(i) => *i,
        SqlVal::Real(f) => toint(*f),
        _ => 0,
    }
}
pub fn v_float(v: &SqlVal) -> f64 {
    match v {
        SqlVal::Int(i) => *i as f64,
        SqlVal::Real(f) => *f,
        _ => 0.0,
    }
}
pub fn v_string(v: &SqlVal) -> String {
    match v {
        SqlVal::Text(s) => s.clone(),
        SqlVal::Int(i) => itext(*i),
        SqlVal::Real(f) => ftext(*f),
        SqlVal::Null => String::new(),
    }
}
pub fn v_truth(v: &SqlVal) -> bool {
    v_int(v) != 0
}
pub fn v_state(v: &SqlVal, opts: &[&'static str]) -> &'static str {
    let s = v_string(v);
    opts.iter().find(|o| **o == s).copied().unwrap_or(opts[0])
}

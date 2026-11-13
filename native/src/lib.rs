use jni::{
    objects::{JClass, JString},
    sys::jstring,
    JNIEnv,
};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::io::{BufWriter, Write};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const MAX: i64 = 100_000_000;

fn integer(v: &Value, key: &str, min: i64) -> Result<i64> {
    let n = v[key]
        .as_i64()
        .ok_or("Enter a valid whole quantity or peso amount.")?;
    if n < min || n > MAX {
        return Err("Number is outside the allowed range.".into());
    }
    Ok(n)
}
fn name(v: &Value, key: &str) -> Result<String> {
    let n = v[key].as_str().unwrap_or("").trim();
    if n.is_empty() || n.len() > 200 {
        return Err("Enter a name or reason (up to 200 bytes).".into());
    }
    Ok(n.to_owned())
}
fn init(c: &mut Connection) -> Result<()> {
    c.busy_timeout(std::time::Duration::from_secs(5))?;
    c.execute_batch("PRAGMA foreign_keys=ON; PRAGMA cache_size=-256; PRAGMA synchronous=FULL;")?;
    let version: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version >= 2 {
        return Ok(());
    }
    let tx = c.transaction()?;
    tx.execute_batch("CREATE TABLE IF NOT EXISTS items(id TEXT PRIMARY KEY, name TEXT NOT NULL COLLATE NOCASE UNIQUE, qty INTEGER NOT NULL CHECK(qty>=0), cost INTEGER NOT NULL CHECK(cost>=0), price INTEGER NOT NULL CHECK(price>=0));
        CREATE TABLE IF NOT EXISTS events(id INTEGER PRIMARY KEY, date TEXT NOT NULL, kind TEXT NOT NULL, item_id TEXT, name TEXT NOT NULL, qty INTEGER NOT NULL DEFAULT 0, amount INTEGER NOT NULL CHECK(amount>=0), outside INTEGER NOT NULL DEFAULT 0, store INTEGER NOT NULL DEFAULT 0);
        CREATE INDEX IF NOT EXISTS event_kind ON events(kind);")?;
    let old: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='ledger' AND type='table')",
        [],
        |r| r.get(0),
    )?;
    if old {
        let data: Option<String> = tx
            .query_row("SELECT data FROM ledger WHERE id=1", [], |r| r.get(0))
            .optional()?;
        if let Some(data) = data {
            let d: Value = serde_json::from_str(&data)?;
            for i in d["items"].as_array().ok_or("Invalid old inventory")? {
                tx.execute(
                    "INSERT INTO items VALUES(?1,?2,?3,?4,?5)",
                    params![
                        i["id"].as_str().ok_or("Invalid old item id")?,
                        name(i, "name")?,
                        integer(i, "qty", 0)?,
                        integer(i, "cost", 0)?,
                        integer(i, "price", 0)?
                    ],
                )?;
            }
            for (array, kind) in [
                ("sales", "sale"),
                ("purchases", "purchase"),
                ("adjustments", "out"),
            ] {
                for e in d[array].as_array().ok_or("Invalid old transactions")? {
                    tx.execute("INSERT INTO events(date,kind,item_id,name,qty,amount,outside,store) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![e["date"].as_str().ok_or("Invalid old date")?,kind,e["id"].as_str(),name(e,if kind=="out" {"reason"} else {"name"})?,if kind=="out" {0} else {integer(e,"qty",0)?},integer(e,"amount",0)?,if kind=="purchase" {integer(e,"outside",0)?} else {0},if kind=="purchase" {integer(e,"store",0)?} else {0}])?;
                }
            }
        }
    }
    tx.execute_batch("PRAGMA user_version=2;")?;
    tx.commit()?;
    Ok(())
}
use rusqlite::OptionalExtension;
fn totals(c: &Connection) -> Result<Value> {
    let (sales,purchases,outside,out): (i64,i64,i64,i64) = c.query_row("SELECT COALESCE(SUM(CASE WHEN kind='sale' THEN amount ELSE 0 END),0), COALESCE(SUM(CASE WHEN kind='purchase' THEN amount ELSE 0 END),0),COALESCE(SUM(outside),0),COALESCE(SUM(CASE WHEN kind='out' THEN amount ELSE 0 END),0) FROM events",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
    Ok(
        json!({"sales":sales,"purchases":purchases,"outside":outside,"out":out,"cash":sales+outside-purchases-out}),
    )
}
fn item(c: &Connection, id: &str) -> Result<Value> {
    Ok(c.query_row(
        "SELECT id,name,qty,cost,price FROM items WHERE id=?1",
        [id],
        row_item,
    )?)
}
fn row_item(r: &rusqlite::Row) -> rusqlite::Result<Value> {
    Ok(
        json!({"id":r.get::<_,String>(0)?,"name":r.get::<_,String>(1)?,"qty":r.get::<_,i64>(2)?,"cost":r.get::<_,i64>(3)?,"price":r.get::<_,i64>(4)?}),
    )
}
fn field(v: &Value, key: &str) -> Result<String> {
    Ok(v[key].as_str().ok_or("Missing field")?.to_owned())
}
fn money(n: i64) -> String {
    format!("{}.{:02}", n / 100, n.abs() % 100)
}
fn csv_row(w: &mut impl Write, fields: &[String]) -> Result<()> {
    for (i, s) in fields.iter().enumerate() {
        if i > 0 {
            write!(w, ",")?;
        }
        let prefix = if s.starts_with(['=', '+', '-', '@', '\t', '\r']) {
            "'"
        } else {
            ""
        };
        write!(w, "\"{}{}\"", prefix, s.replace('"', "\"\""))?;
    }
    writeln!(w, "\r")?;
    Ok(())
}
fn export(c: &Connection, v: &Value) -> Result<Value> {
    let mut w = BufWriter::with_capacity(8192, std::fs::File::create(field(v, "path")?)?);
    write!(w, "\u{feff}")?;
    if v["type"] == "inventory" {
        csv_row(
            &mut w,
            &[
                "Item",
                "Quantity",
                "Cost PHP per piece",
                "Selling price PHP",
            ]
            .map(str::to_owned),
        )?;
        let mut s = c.prepare("SELECT id,name,qty,cost,price FROM items ORDER BY name")?;
        let mut rows = s.query([])?;
        while let Some(r) = rows.next()? {
            csv_row(
                &mut w,
                &[
                    r.get(1)?,
                    r.get::<_, i64>(2)?.to_string(),
                    money(r.get(3)?),
                    money(r.get(4)?),
                ],
            )?;
        }
    } else {
        csv_row(
            &mut w,
            &[
                "Date UTC",
                "Type",
                "Item or reason",
                "Quantity",
                "Amount PHP",
                "Outside capital PHP",
                "Store cash used PHP",
            ]
            .map(str::to_owned),
        )?;
        let mut s =
            c.prepare("SELECT date,kind,name,qty,amount,outside,store FROM events ORDER BY id")?;
        let mut rows = s.query([])?;
        while let Some(r) = rows.next()? {
            csv_row(
                &mut w,
                &[
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get::<_, i64>(3)?.to_string(),
                    money(r.get(4)?),
                    money(r.get(5)?),
                    money(r.get(6)?),
                ],
            )?;
        }
    }
    w.flush()?;
    Ok(json!({}))
}
fn request(c: &mut Connection, v: &Value) -> Result<Value> {
    match v["op"].as_str().unwrap_or("") {
        "list" => {
            let offset = integer(v, "offset", 0)?;
            let search = field(v, "search")?;
            let mut s=c.prepare("SELECT id,name,qty,cost,price FROM items WHERE instr(lower(name),lower(?1))>0 ORDER BY name LIMIT 51 OFFSET ?2")?;
            let items = s
                .query_map(params![search, offset], row_item)?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            Ok(json!({"totals":totals(c)?,"items":items}))
        }
        "export" => export(c, v),
        "purchase" | "sale" | "edit" | "out" => {
            let tx = c.transaction()?;
            let op = field(v, "op")?;
            let cash = totals(&tx)?["cash"].as_i64().unwrap();
            if op == "purchase" {
                let n = name(v, "name")?;
                let q = integer(v, "qty", 1)?;
                let a = integer(v, "amount", 0)?;
                let p = integer(v, "price", 0)?;
                let mode = field(v, "mode")?;
                let outside = match mode.as_str() {
                    "outside" => a,
                    "split" => (a - cash.max(0)).max(0),
                    "cash" => {
                        if a > cash {
                            return Err("Not enough store cash. Select outside money.".into());
                        }
                        0
                    }
                    _ => return Err("Invalid funding source".into()),
                };
                let existing: Option<(String, i64)> = tx
                    .query_row("SELECT id,qty FROM items WHERE name=?1", [&n], |r| {
                        Ok((r.get(0)?, r.get(1)?))
                    })
                    .optional()?;
                let id = if let Some((id, qty)) = existing {
                    if qty + q > MAX {
                        return Err("Stock is too large".into());
                    }
                    tx.execute(
                        "UPDATE items SET qty=qty+?1,cost=?2,price=?3 WHERE id=?4",
                        params![q, (a + q / 2) / q, p, id],
                    )?;
                    id
                } else {
                    let id: String =
                        tx.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))?;
                    tx.execute(
                        "INSERT INTO items VALUES(?1,?2,?3,?4,?5)",
                        params![id, n, q, (a + q / 2) / q, p],
                    )?;
                    id
                };
                tx.execute("INSERT INTO events(date,kind,item_id,name,qty,amount,outside,store) VALUES(strftime('%Y-%m-%dT%H:%M:%fZ','now'),'purchase',?1,?2,?3,?4,?5,?6)",params![id,n,q,a,outside,a-outside])?;
            } else if op == "sale" {
                let id = field(v, "id")?;
                let q = integer(v, "qty", 1)?;
                let i = item(&tx, &id)?;
                if q > i["qty"].as_i64().unwrap() {
                    return Err("Not enough stock".into());
                }
                let amount = q
                    .checked_mul(i["price"].as_i64().unwrap())
                    .filter(|x| *x <= MAX)
                    .ok_or("Sale is too large")?;
                tx.execute("UPDATE items SET qty=qty-?1 WHERE id=?2", params![q, id])?;
                tx.execute("INSERT INTO events(date,kind,item_id,name,qty,amount) VALUES(strftime('%Y-%m-%dT%H:%M:%fZ','now'),'sale',?1,?2,?3,?4)",params![id,i["name"].as_str(),q,amount])?;
            } else if op == "out" {
                let amount = integer(v, "amount", 1)?;
                let reason = name(v, "reason")?;
                if amount > cash {
                    return Err("Amount exceeds available store cash".into());
                }
                tx.execute("INSERT INTO events(date,kind,name,amount) VALUES(strftime('%Y-%m-%dT%H:%M:%fZ','now'),'out',?1,?2)",params![reason,amount])?;
            } else {
                let id = field(v, "id")?;
                item(&tx, &id)?;
                tx.execute(
                    "UPDATE items SET name=?1,qty=?2,cost=?3,price=?4 WHERE id=?5",
                    params![
                        name(v, "name")?,
                        integer(v, "qty", 0)?,
                        integer(v, "cost", 0)?,
                        integer(v, "price", 0)?,
                        id
                    ],
                )?;
            }
            tx.commit()?;
            Ok(json!({}))
        }
        _ => Err("Unknown operation".into()),
    }
}
fn execute(path: &str, input: &str) -> Result<Value> {
    let mut c = Connection::open(path)?;
    init(&mut c)?;
    request(&mut c, &serde_json::from_str(input)?)
}

#[no_mangle]
pub extern "system" fn Java_com_seraphene_inventory_MainActivity_nativeCall(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
    input: JString,
) -> jstring {
    let result = (|| -> Result<Value> {
        let path: String = env.get_string(&path)?.into();
        let input: String = env.get_string(&input)?.into();
        execute(&path, &input)
    })();
    let response = match result {
        Ok(data) => json!({"ok":true,"data":data}),
        Err(e) => json!({"ok":false,"error":e.to_string()}),
    };
    match env.new_string(response.to_string()) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn durable_storage_and_csv() {
        let base = std::env::temp_dir().join(format!(
            "tindahan-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = base.with_extension("db");
        let csv = base.with_extension("csv");
        let p = path.to_str().unwrap();
        execute(p,&json!({"op":"purchase","name":"=Rice,\"premium\"","qty":2,"amount":101,"price":75,"mode":"outside"}).to_string()).unwrap();
        let result = execute(p, &json!({"op":"list","offset":0,"search":""}).to_string()).unwrap();
        assert_eq!(result["items"][0]["qty"], 2);
        assert_eq!(result["totals"]["outside"], 101);
        execute(
            p,
            &json!({"op":"export","type":"inventory","path":csv}).to_string(),
        )
        .unwrap();
        let contents = std::fs::read_to_string(&csv).unwrap();
        assert!(contents.contains("'=Rice,\"\"premium\"\""));
        assert!(contents.contains("0.75"));
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(csv).unwrap();
    }
    #[test]
    fn bounded_pages() {
        let mut c = db();
        let tx = c.transaction().unwrap();
        for i in 0..1000 {
            tx.execute(
                "INSERT INTO items VALUES(?1,?2,1,10,20)",
                params![i.to_string(), format!("Item {i:04}")],
            )
            .unwrap();
        }
        tx.commit().unwrap();
        let r = request(&mut c, &json!({"op":"list","offset":0,"search":""})).unwrap();
        assert_eq!(r["items"].as_array().unwrap().len(), 51);
        let r = request(&mut c, &json!({"op":"list","offset":950,"search":""})).unwrap();
        assert_eq!(r["items"].as_array().unwrap().len(), 50);
    }
    #[test]
    fn bad_migration_preserves_old_data() {
        let mut c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE ledger(id INTEGER PRIMARY KEY,data TEXT); INSERT INTO ledger VALUES(1,'invalid json')").unwrap();
        assert!(init(&mut c).is_err());
        assert_eq!(
            c.query_row("SELECT data FROM ledger", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "invalid json"
        );
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    fn db() -> Connection {
        let mut c = Connection::open_in_memory().unwrap();
        init(&mut c).unwrap();
        c
    }
    fn seed(c: &Connection) {
        c.execute("INSERT INTO events(date,kind,name,qty,amount) VALUES('2026-01-01','sale','Opening test sale',1,100000)",[]).unwrap();
    }
    #[test]
    fn funding_and_history() {
        for (mode, outside, cash) in [("split", 100000, 0), ("outside", 200000, 100000)] {
            let mut c = db();
            seed(&c);
            request(&mut c,&json!({"op":"purchase","name":"Rice","qty":20,"amount":200000,"price":12000,"mode":mode})).unwrap();
            let t = totals(&c).unwrap();
            assert_eq!(t["sales"], 100000);
            assert_eq!(t["outside"], outside);
            assert_eq!(t["cash"], cash);
            let id: String = c
                .query_row("SELECT id FROM items", [], |r| r.get(0))
                .unwrap();
            request(&mut c, &json!({"op":"sale","id":id,"qty":2})).unwrap();
            request(&mut c,&json!({"op":"edit","id":id,"name":"Rice edited","qty":18,"cost":10000,"price":13000})).unwrap();
            assert_eq!(totals(&c).unwrap()["sales"], 124000);
            request(
                &mut c,
                &json!({"op":"out","amount":24000,"reason":"Transport"}),
            )
            .unwrap();
            assert_eq!(totals(&c).unwrap()["cash"], cash);
        }
    }
    #[test]
    fn invalid_purchase_rolls_back() {
        let mut c = db();
        let v = json!({"op":"purchase","name":"Rice","qty":20,"amount":200000,"price":12000,"mode":"cash"});
        assert!(request(&mut c, &v).is_err());
        assert_eq!(
            c.query_row("SELECT count(*) FROM items", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn migration_once() {
        let mut c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE ledger(id INTEGER PRIMARY KEY,data TEXT)")
            .unwrap();
        let data = json!({"items":[{"id":"a","name":"Rice","qty":1,"cost":100,"price":200}],"sales":[{"date":"2026-01-01","id":"a","name":"Rice","qty":1,"amount":200}],"purchases":[{"date":"2026-01-01","id":"a","name":"Rice","qty":2,"amount":200,"outside":200,"store":0}],"adjustments":[]});
        c.execute("INSERT INTO ledger VALUES(1,?1)", [data.to_string()])
            .unwrap();
        init(&mut c).unwrap();
        init(&mut c).unwrap();
        assert_eq!(totals(&c).unwrap()["cash"], 200);
        assert_eq!(item(&c, "a").unwrap()["qty"], 1);
    }
    #[test]
    fn stock_and_validation() {
        let mut c = db();
        request(&mut c,&json!({"op":"purchase","name":"<Rice>","qty":2,"amount":100,"price":70,"mode":"outside"})).unwrap();
        let id: String = c
            .query_row("SELECT id FROM items", [], |r| r.get(0))
            .unwrap();
        assert!(request(&mut c, &json!({"op":"sale","id":id,"qty":3})).is_err());
        assert!(request(&mut c, &json!({"op":"sale","id":id,"qty":-1})).is_err());
        assert_eq!(item(&c, &id).unwrap()["qty"], 2);
        assert!(request(&mut c, &json!({"op":"out","amount":1,"reason":"Test"})).is_err());
    }
}

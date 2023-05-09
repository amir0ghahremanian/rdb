mod rdb;

use crate::rdb::db::Db;

fn main() {
    let mut mydb = Db::open("mydb").unwrap();
    mydb.append_entry(vec!["Amerius".to_string(), "Magnus".to_string()]);
    mydb.close().unwrap();
}

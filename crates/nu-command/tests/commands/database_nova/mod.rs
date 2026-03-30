use std::time::Duration;

use nu_experimental::DATABASE_NOVA;
use nu_protocol::{test_record, Value};
use nu_test_support::prelude::*;

use chrono::Local;

#[test]
#[exp(DATABASE_NOVA)]
fn save_and_open() -> Result {
    Playground::setup("database_nova_save_and_open", |dirs, _| {
        let sample = Value::test_list(vec![test_record! {
            "binary" => Value::test_binary([1, 2, 3]),
            "bool" => Value::test_bool(true),
            // "cell_path" => Value::test_cell_path(CellPath { members: Vec::new() }),
            "date" => Value::test_date(Local::now().fixed_offset()),
            "duration" => Value::test_duration(Duration::from_secs(4).as_nanos() as i64),
            "filesize" => Value::test_filesize(1024),
            "float" => Value::test_float(3.14),
            "glob" => Value::test_glob("*.nu"),
            "int" => Value::test_int(42),
            "list" => Value::test_list(vec![Value::test_int(1), Value::test_string("two")]),
            "nothing" => Value::test_nothing(),
            "record" => test_record! { "nested" => Value::test_int(1) },
            "string" => Value::test_string("sample"),
        }]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run_with_data("$in | save sample.sqlite", sample.clone())?;
        tester.run("open sample.sqlite").expect_value_eq(sample)
    })
}

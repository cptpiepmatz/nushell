use std::sync::Arc;

use nu_engine::command_prelude::*;
use nu_protocol::FromValue;
use parking_lot::Mutex;

use crate::database_nova::{
    plumbing::{connection::DatabaseConnection, name::DatabaseName, table::DatabaseTableName},
    value::{DatabaseSystemValue, DatabaseTableValue, DatabaseValue},
};

pub const TO_SQLITE: ToSqlite = ToSqlite { name: "to sqlite" };
pub const TO_DB: ToSqlite = ToSqlite { name: "to db" };

#[derive(Debug, Clone)]
pub struct ToSqlite {
    name: &'static str,
}

impl Command for ToSqlite {
    fn name(&self) -> &str {
        self.name
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .description(self.description())
            .search_terms(
                self.search_terms()
                    .into_iter()
                    .map(ToOwned::to_owned)
                    .collect(),
            )
            .category(Category::Database)
            .input_output_type(Type::Any, DatabaseSystemValue::expected_type())
    }

    fn description(&self) -> &str {
        "Serialize data into an SQLite table."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["sqlite", "db"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let input = input.into_value(call.head)?;
        if DatabaseSystemValue::is(&input) {
            return Ok(PipelineData::value(input, None));
        }

        let table_name = DatabaseTableName::new_internal("main");
        let conn = DatabaseConnection::new_from_value(input, table_name.clone(), call.head)?;

        let value = DatabaseValue::new(Arc::new(Mutex::new(conn)), DatabaseName::MAIN, call.head)?;
        let value = DatabaseTableValue::from_database(value, table_name, call.head)?;
        Ok(PipelineData::value(value.into_value(call.head), None))
    }
}

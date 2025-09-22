use nu_engine::command_prelude::*;
use nu_protocol::FromValue;

use crate::database_next::{
    plumbing::{connection::DatabaseConnection, name::DatabaseName},
    value::{DatabaseSystemValue, DatabaseValue},
};

#[derive(Debug, Clone)]
pub struct FromSqlite;

impl Command for FromSqlite {
    fn name(&self) -> &str {
        "from sqlite"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .description(self.description())
            .extra_description(self.extra_description())
            .search_terms(
                self.search_terms()
                    .into_iter()
                    .map(ToOwned::to_owned)
                    .collect(),
            )
            .category(Category::Database)
            .input_output_types(vec![
                (Type::Binary, Type::custom(DatabaseValue::TYPE_NAME)),
                (Type::Binary, DatabaseSystemValue::expected_type()), // if `--all` is used
            ])
            .switch("all", "Include all attached databases", None)
            .switch("promote", "Immediately promote database into memory", None)
    }

    fn description(&self) -> &str {
        "Deserialize an SQLite table."
    }

    fn extra_description(&self) -> &str {
        "This tries to conserve memory by opening a connection from the file path if it's available in the metadata."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["connection", "sqlite", "db"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let conn = DatabaseConnection::open_from_pipeline(input, call.head)?;
        let conn = match call.has_flag(engine_state, stack, "promote")? {
            true => conn.promote()?,
            false => conn,
        };
        let value = DatabaseSystemValue::new(conn);
        let value = match call.has_flag(engine_state, stack, "all")? {
            true => value.into_value(call.head),
            false => value
                .database(DatabaseName::MAIN, call.head)?
                .into_value(call.head),
        };
        Ok(PipelineData::value(value, None))
    }
}

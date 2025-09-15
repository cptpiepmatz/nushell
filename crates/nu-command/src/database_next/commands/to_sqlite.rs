
use nu_engine::command_prelude::*;
use nu_protocol::FromValue;

use crate::database_next::{plumbing::connection::DatabaseConnection, value::DatabaseValue};

#[derive(Debug, Clone)]
pub struct ToSqlite;

impl Command for ToSqlite {
    fn name(&self) -> &str {
        "to sqlite"
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
            .input_output_type(Type::Any, DatabaseValue::expected_type())
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
        if DatabaseValue::is(&input) { return Ok(PipelineData::value(input, None)) }
        todo!()
    }
}

use nu_engine::command_prelude::*;
use nu_protocol::FromValue;

use crate::database_next::value::DatabaseValue;

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
            .search_terms(self.search_terms().into_iter().map(ToOwned::to_owned).collect())
            .category(Category::Database)
            .input_output_type(Type::Binary, DatabaseValue::expected_type())
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
        todo!()
    }
}

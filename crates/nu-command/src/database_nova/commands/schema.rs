use nu_engine::command_prelude::*;
use nu_protocol::FromValue;

use crate::database_nova::{
    error::DatabaseError,
    plumbing::{connection::DatabaseConnection, name::DatabaseName},
    value::{DatabaseSystemValue, DatabaseValue},
};

#[derive(Debug, Clone)]
pub struct Schema;

impl Command for Schema {
    fn name(&self) -> &str {
        "schema"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).description(self.description())
    }

    fn description(&self) -> &str {
        "Show the schema of an SQLite database or table."
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
        Err(ShellError::from(DatabaseError::Todo {
            msg: "implement schema command".into(),
            span: call.head,
        }))
    }
}

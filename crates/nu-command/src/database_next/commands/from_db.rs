use crate::database_next::commands::FromSqlite;
use nu_engine::command_prelude::*;

#[derive(Debug, Clone)]
pub struct FromDb;

impl Command for FromDb {
    fn name(&self) -> &str {
        "from db"
    }

    fn signature(&self) -> Signature {
        Signature {
            name: self.name().into(),
            ..FromSqlite.signature()
        }
    }

    fn description(&self) -> &str {
        FromSqlite.description()
    }

    fn extra_description(&self) -> &str {
        FromSqlite.extra_description()
    }

    fn search_terms(&self) -> Vec<&str> {
        FromSqlite.search_terms()
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        FromSqlite.run(engine_state, stack, call, input)
    }
}

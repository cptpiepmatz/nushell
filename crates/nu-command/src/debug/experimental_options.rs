use nu_engine::command_prelude::*;
use nu_experimental::Status;

#[derive(Clone)]
pub struct DebugExperimentalOptions;

impl Command for DebugExperimentalOptions {
    fn name(&self) -> &str {
        "debug experimental-options"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(
                Type::Nothing,
                Type::Table(Box::from([
                    (String::from("identifier"), Type::String),
                    (String::from("enabled"), Type::Bool),
                    (String::from("status"), Type::String),
                    (String::from("description"), Type::String),
                    (String::from("since"), Type::String),
                    (String::from("issue"), Type::String),
                ])),
            )
            .category(Category::Debug)
    }

    fn description(&self) -> &str {
        "Show all experimental options."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::value(
            experimental_options_value(call.head),
            None,
        ))
    }

    fn run_const(
        &self,
        _working_set: &StateWorkingSet,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::value(
            experimental_options_value(call.head),
            None,
        ))
    }

    fn is_const(&self) -> bool {
        true
    }
}

fn experimental_options_value(span: Span) -> Value {
    Value::list(
        nu_experimental::ALL
            .iter()
            .map(|option| {
                Value::record(
                    nu_protocol::record! {
                        "identifier" => Value::string(option.identifier(), span),
                        "enabled" => Value::bool(option.get(), span),
                        "status" => Value::string(match option.status() {
                            Status::OptIn => "opt-in",
                            Status::OptOut => "opt-out",
                            Status::DeprecatedDiscard => "deprecated-discard",
                            Status::DeprecatedDefault => "deprecated-default"
                        }, span),
                        "description" => Value::string(option.description(), span),
                        "since" => Value::string({
                            let (major, minor, patch) = option.since();
                            format!("{major}.{minor}.{patch}")
                        }, span),
                        "issue" => Value::string(option.issue_url(), span)
                    },
                    span,
                )
            })
            .collect(),
        span,
    )
}

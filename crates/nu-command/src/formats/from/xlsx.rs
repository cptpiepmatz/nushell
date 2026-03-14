use calamine::*;
use chrono::{FixedOffset, Local, LocalResult, Offset, TimeZone, Utc};
use indexmap::IndexMap;
use nu_engine::command_prelude::*;

use std::io::Cursor;

#[derive(Clone)]
pub struct FromXlsx;

impl Command for FromXlsx {
    fn name(&self) -> &str {
        "from xlsx"
    }

    fn signature(&self) -> Signature {
        Signature::build("from xlsx")
            .input_output_types(vec![(Type::Binary, Type::table())])
            .allow_variants_without_examples(true)
            .switch("no-infer", "no field type inferencing", None)
            .named(
                "sheets",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Only convert specified sheets.",
                Some('s'),
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Parse binary Excel(.xlsx) data and create table."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        let sel_sheets = if let Some(Value::List { vals: columns, .. }) =
            call.get_flag(engine_state, stack, "sheets")?
        {
            convert_columns(columns.as_slice())?
        } else {
            vec![]
        };

        let no_infer = call.has_flag(engine_state, stack, "no-infer")?;

        let metadata = input.metadata().map(|md| md.with_content_type(None));
        from_xlsx(input, head, sel_sheets, no_infer).map(|pd| pd.set_metadata(metadata))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Convert binary .xlsx data to a table.",
                example: "open --raw test.xlsx | from xlsx",
                result: None,
            },
            Example {
                description: "Convert binary .xlsx data to a table, specifying the tables.",
                example: "open --raw test.xlsx | from xlsx --sheets [Spreadsheet1]",
                result: None,
            },
        ]
    }
}

fn convert_columns(columns: &[Value]) -> Result<Vec<String>, ShellError> {
    let res = columns
        .iter()
        .map(|value| match &value {
            Value::String { val: s, .. } => Ok(s.clone()),
            _ => Err(ShellError::IncompatibleParametersSingle {
                msg: "Incorrect column format, Only string as column name".to_string(),
                span: value.span(),
            }),
        })
        .collect::<Result<Vec<String>, _>>()?;

    Ok(res)
}

fn collect_binary(input: PipelineData, span: Span) -> Result<Vec<u8>, ShellError> {
    if let PipelineData::ByteStream(stream, ..) = input {
        stream.into_bytes()
    } else {
        let mut bytes = vec![];
        let mut values = input.into_iter();

        loop {
            match values.next() {
                Some(Value::Binary { val: b, .. }) => {
                    bytes.extend_from_slice(&b);
                }
                Some(x) => {
                    return Err(ShellError::UnsupportedInput {
                        msg: "Expected binary from pipeline".to_string(),
                        input: "value originates from here".into(),
                        msg_span: span,
                        input_span: x.span(),
                    });
                }
                None => break,
            }
        }

        Ok(bytes)
    }
}

fn from_xlsx(
    input: PipelineData,
    head: Span,
    sel_sheets: Vec<String>,
    no_infer: bool,
) -> Result<PipelineData, ShellError> {
    let span = input.span();
    let bytes = collect_binary(input, head)?;
    let buf: Cursor<Vec<u8>> = Cursor::new(bytes);
    let mut xlsx = Xlsx::<_>::new(buf).map_err(|_| ShellError::UnsupportedInput {
        msg: "Could not load XLSX file".to_string(),
        input: "value originates from here".into(),
        msg_span: head,
        input_span: span.unwrap_or(head),
    })?;

    let mut dict = IndexMap::new();

    let mut sheet_names = xlsx.sheet_names();
    if !sel_sheets.is_empty() {
        sheet_names.retain(|e| sel_sheets.contains(e));
    }

    let tz = match Local.timestamp_opt(0, 0) {
        LocalResult::Single(tz) => *tz.offset(),
        _ => Utc.fix(),
    };

    // welp, cool idea but formula is actually only formula
    // we cannot read v values from calamine yet
    for sheet_name in sheet_names {
        let sheet_output = match no_infer {
            false => convert_worksheet(
                || xlsx.worksheet_range(&sheet_name).map(|range| dbg!(range)),
                data_cell_to_value,
                tz,
                head,
            ),
            true => convert_worksheet(
                || xlsx.worksheet_formula(&sheet_name).map(|range| dbg!(range)),
                formula_cell_to_value,
                tz,
                head,
            ),
        }
        .map_err(|_| ShellError::UnsupportedInput {
            msg: format!("Could not load sheet {sheet_name:?}"),
            input: "value originates from here".into(),
            msg_span: head,
            input_span: span.unwrap_or(head),
        })?;
        dict.insert(sheet_name, sheet_output);
    }

    Ok(PipelineData::value(
        Value::record(dict.into_iter().collect(), head),
        None,
    ))
}

fn convert_worksheet<C: CellType>(
    mut current_sheet: impl FnMut() -> Result<calamine::Range<C>, XlsxError>,
    cell_to_value: impl Fn(&C, FixedOffset, Span) -> Value,
    tz: FixedOffset,
    span: Span,
) -> Result<Value, XlsxError> {
    let current_sheet = match current_sheet() {
        Ok(current_sheet) => current_sheet,
        Err(err) => return Err(err),
    };

    let rows = current_sheet
        .rows()
        .map(|row| {
            let record: Record = row
                .iter()
                .enumerate()
                .map(|(i, cell)| {
                    let value = cell_to_value(cell, tz, span);
                    (format!("column{i}"), value)
                })
                .collect();
            Value::record(record, span)
        })
        .collect();

    Ok(Value::list(rows, span))
}

fn data_cell_to_value(cell: &calamine::Data, tz: FixedOffset, span: Span) -> Value {
    match cell {
        Data::Empty => Value::nothing(span),
        Data::String(s) => Value::string(s, span),
        Data::Float(f) => Value::float(*f, span),
        Data::Int(i) => Value::int(*i, span),
        Data::Bool(b) => Value::bool(*b, span),
        Data::DateTime(d) => d
            .as_datetime()
            .and_then(|d| match tz.from_local_datetime(&d) {
                LocalResult::Single(d) => Some(d),
                _ => None,
            })
            .map(|d| Value::date(d, span))
            .unwrap_or(Value::nothing(span)),
        _ => Value::nothing(span),
    }
}

fn formula_cell_to_value(cell: &String, _: FixedOffset, span: Span) -> Value {
    Value::string(cell, span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(FromXlsx)
    }
}

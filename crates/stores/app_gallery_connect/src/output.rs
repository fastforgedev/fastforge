use anyhow::{Result, anyhow};
use serde::Serialize;
use serde_json::{Map, Value};
use unicode_width::UnicodeWidthStr;

pub fn print_json<T: Serialize>(value: &T, fields: Option<&str>) -> Result<()> {
    let value = select_fields(serde_json::to_value(value)?, fields)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

fn select_fields(value: Value, fields: Option<&str>) -> Result<Value> {
    let Some(fields) = fields else {
        return Ok(value);
    };
    let fields = fields
        .split(',')
        .map(str::trim)
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    if fields.is_empty() {
        return Ok(value);
    }
    match value {
        Value::Array(items) => items
            .into_iter()
            .map(|item| select_object_fields(item, &fields))
            .collect::<Result<Vec<_>>>()
            .map(Value::Array),
        item => select_object_fields(item, &fields),
    }
}

fn select_object_fields(value: Value, fields: &[&str]) -> Result<Value> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("--json field selection requires object output"))?;
    let mut selected = Map::new();
    for field in fields {
        let value = object.get(*field).ok_or_else(|| {
            let available = object.keys().cloned().collect::<Vec<_>>().join(", ");
            anyhow!("unknown JSON field `{field}`; available fields: {available}")
        })?;
        selected.insert((*field).to_string(), value.clone());
    }
    Ok(Value::Object(selected))
}

pub fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    let mut widths = headers
        .iter()
        .map(|header| header.width())
        .collect::<Vec<_>>();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            if let Some(width) = widths.get_mut(index) {
                *width = (*width).max(cell.width());
            }
        }
    }
    print_row(
        headers.iter().map(|value| (*value).to_string()).collect(),
        &widths,
    );
    for row in rows {
        print_row(row.clone(), &widths);
    }
}

fn print_row(row: Vec<String>, widths: &[usize]) {
    for (index, cell) in row.iter().enumerate() {
        let width = widths.get(index).copied().unwrap_or(cell.width());
        if index + 1 == row.len() {
            print!("{cell}");
        } else {
            print!("{cell}{}  ", " ".repeat(width.saturating_sub(cell.width())));
        }
    }
    println!();
}

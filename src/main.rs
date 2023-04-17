use anyhow::{self, Error, Result};

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

use serde_yaml;
use std::io;

fn main() -> Result<()> {
    let input: serde_json::Value = serde_yaml::from_reader(io::stdin())?;
    let output = parse_input(input);
    let output = serde_yaml::to_string(&output)?;
    // Load these once at the start of your program
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ps.find_syntax_by_extension("yaml").unwrap();
    let mut theme = ts.themes.get("base16-ocean.dark").unwrap().to_owned();
    let mut h = HighlightLines::new(syntax, &theme);
    for line in LinesWithEndings::from(output.as_str()) {
        // LinesWithEndings enables use of newlines mode
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
        print!("{}", escaped);
    }
    return Ok(());
}

fn parse_input(mut input: serde_json::Value) -> serde_json::Value {
    if let serde_json::Value::Object(ref mut map) = input {
        if let Some(kind) = map.get("kind") {
            if kind == "Secret" {
                if let Some(data) = map.remove("data") {
                    let mut string_data = serde_json::Map::new();

                    if let Some(existing_string_data) = map.remove("stringData") {
                        for (key, value) in existing_string_data.as_object().unwrap() {
                            string_data.insert(key.to_string(), value.to_owned());
                        }
                    }

                    for (key, value) in data.as_object().unwrap() {
                        let decoded = base64::decode(value.as_str().unwrap()).unwrap();
                        let decoded_string = String::from_utf8(decoded).unwrap();
                        string_data
                            .insert(key.to_string(), serde_json::Value::String(decoded_string));
                    }
                    map.insert(
                        "stringData".to_string(),
                        serde_json::Value::Object(string_data.to_owned()),
                    );
                }
            } else if kind == "List" {
                if let Some(items) = map.get("items") {
                    let mut newItems = Vec::new();
                    for item in items.as_array().unwrap() {
                        newItems.push(parse_input(item.clone()));
                    }
                    map.insert("items".to_string(), serde_json::Value::Array(newItems));
                }
            }
        }
    }
    return input;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_input() {
        let input = serde_json::json!({
            "foo": "bar"
        });

        let output = parse_input(input);
        let outputValue = output.get("foo").unwrap().as_str().unwrap();

        assert_eq!(outputValue, "bar");
    }

    #[test]
    fn test_decode_secret() {
        let input = serde_json::json!({
            "kind": "Secret",
            "apiVersion": "v1",
            "metadata": {
                "name": "example",
                "creationTimestamp": null
            },
            "data": {
                "key": "dmFsdWU="
            }
        });

        let output = parse_input(input);
        let outputValue = output
            .get("stringData")
            .unwrap()
            .get("key")
            .unwrap()
            .as_str()
            .unwrap();

        assert_eq!(outputValue, "value");
    }

    #[test]
    fn test_string_data_already_exists() {
        let input = serde_json::json!({
            "kind": "Secret",
            "apiVersion": "v1",
            "metadata": {
                "name": "example",
                "creationTimestamp": null
            },
            "data": {
                "key": "dmFsdWU="
            },
            "stringData": {
                "hello": "world"
            }
        });

        let output = parse_input(input);
        let outputValue = output
            .get("stringData")
            .unwrap()
            .get("hello")
            .unwrap()
            .as_str()
            .unwrap();

        assert_eq!(outputValue, "world");
    }

    #[test]
    fn test_decode_secret_list() {
        let input = serde_json::json!({
          "kind": "List",
          "metadata": {},
          "items": [
            {
              "apiVersion": "v1",
              "data": {
                "key": "dmFsdWU="
              },
              "kind": "Secret",
              "metadata": {
                "creationTimestamp": null,
                "name": "example"
              }
            },
            {
              "apiVersion": "v1",
              "data": {
                "key": "dmFsdWU="
              },
              "kind": "Pod",
              "metadata": {
                "name": "example"
              }
            }
          ]
        }
        );

        let output = parse_input(input);
        let outputValue = output.get("items").unwrap().as_array().unwrap()[0]
            .get("stringData")
            .unwrap()
            .get("key")
            .unwrap()
            .as_str()
            .unwrap();

        assert_eq!(outputValue, "value");
    }
}

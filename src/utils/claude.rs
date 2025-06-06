use std::io;

use serde_json::Value;

pub fn parse_claude_response(stdout: &str) -> Result<Value, io::Error> {
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    println!("Result: {:?}\n", result);
    let result_json = result.get("result").unwrap();
    println!("Result JSON: {:?}\n", result_json);

    let result_json_str = result_json.as_str().unwrap();
    println!("Result JSON: {:?}\n", result_json);
    let mut start_bytes = result_json_str.find("```json\n").unwrap();
    start_bytes += 7;
    let end_bytes = result_json_str.rfind("```").unwrap();

    let result_sjon = &result_json_str[start_bytes..end_bytes];
    let result_sjon_replace = result_sjon.replace("\n", "");
    let final_json: serde_json::Value = serde_json::from_str(&result_sjon_replace).unwrap();
    println!(
        "search:{:?}, {:?}, \n{:?}",
        result_sjon_replace,
        final_json,
        final_json.get("pr_description")
    );
    Ok(final_json)
}

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use serde_json::json;

pub fn generate_cairo1_inputs(
    commitment_hash: u64,
    proof_array: Vec<u64>,
    new_root: u64,
    output_dir: &Path,
) -> io::Result<()> {
    let mut full_input = vec![commitment_hash];
    full_input.extend(&proof_array);
    full_input.push(new_root);

    // Generate input.cairo1.json
    let json_path = output_dir.join("input.cairo1.json");
    let json_data = json!([full_input]);
    let mut json_file = File::create(json_path)?;
    write!(json_file, "{}", json_data.to_string())?;

    // Generate input.cairo1.txt
    let txt_path = output_dir.join("input.cairo1.txt");
    let txt_data: String = full_input.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ");
    let mut txt_file = File::create(txt_path)?;
    write!(txt_file, "{}", txt_data)?;

    Ok(())
}

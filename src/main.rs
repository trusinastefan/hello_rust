use slug::slugify;
use std::io;
use std::env;
use std::process;
use std::error::Error;


fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        process::exit(1);
    }

    let output = match args[1].as_str() {
        "lowercase" => transform_lowercase(),
        "uppercase" => transform_uppercase(),
        "no-spaces" => transform_no_spaces(),
        "slugify" => transform_slugify(),
        "czechify" => transform_czechify(),
        "emphasize" => transform_emphasize(),
        "csv" => transform_csv(),
        _ => Err("Invalid command line argument!".into())
    };
    
    match output {
        Ok(transformed_str) => println!("{}", transformed_str),
        Err(e) => println!("ERROR: {}", e)
    }
}


fn get_line_from_user() -> Result<String, Box<dyn Error>> {
    println!("Type your string!");
    let mut input_str = String::new();
    io::stdin().read_line(&mut input_str)?;
    Ok(input_str)
}


fn validate_input(input_str: &String) -> Result<(), Box<dyn Error>> {
    if input_str.trim().is_empty() {
        return Err("Not a valid string!".into());
    }
    return Ok(());
}


fn transform_lowercase() -> Result<String, Box<dyn Error>> {
    let input_str = get_line_from_user()?;
    validate_input(&input_str)?;
    let output_str = input_str.to_lowercase();
    Ok(output_str)
}


fn transform_uppercase() -> Result<String, Box<dyn Error>> {
    let input_str = get_line_from_user()?;
    validate_input(&input_str)?;
    let output_str = input_str.to_uppercase();
    Ok(output_str)
}


fn transform_no_spaces() -> Result<String, Box<dyn Error>> {
    let input_str = get_line_from_user()?;
    validate_input(&input_str)?;
    let output_str = input_str.replace(" ", "");
    Ok(output_str)
}


fn transform_slugify() -> Result<String, Box<dyn Error>> {
    let input_str = get_line_from_user()?;
    validate_input(&input_str)?;
    let output_str = slugify(input_str);
    Ok(output_str)
}


fn transform_czechify() -> Result<String, Box<dyn Error>> {
    let input_str = get_line_from_user()?;
    validate_input(&input_str)?;
    let output_str = input_str.replace("r", "ř").replace("R", "Ř").replace("e", "ě").replace("E", "Ě");
    Ok(output_str)
}


fn transform_emphasize() -> Result<String, Box<dyn Error>> {
    let input_str = get_line_from_user()?;
    validate_input(&input_str)?;
    let output_str = format!("{}{}{}", "!!!", input_str, "!!!");
    Ok(output_str)
}


fn transform_csv() -> Result<String, Box<dyn Error>> {
    println!("Enter csv!");
    let mut input_csv_table = Vec::new();
    
    let mut header_line = String::new();
    io::stdin().read_line(&mut header_line)?;
    let header_line_vector: Vec<String> = header_line.trim().split(",").map(|s| s.to_string()).collect();
    let columns_n = header_line_vector.len();
    input_csv_table.push(header_line_vector);

    
    loop {
        let mut record_line = String::new();
        io::stdin().read_line(&mut record_line)?;
        if record_line.trim().is_empty() {
            break;
        }
        let record_line_vector: Vec<String> = record_line.trim().split(",").map(|s| s.to_string()).collect();
        if record_line_vector.len() != columns_n {
            return Err("Incorrect csv structure!".into());
        }
        input_csv_table.push(record_line_vector);
    }

    let mut output_str = String::new();
    for record in input_csv_table {
        output_str = format!("{}{}\n", output_str, "-".repeat(columns_n*17 + 1));
        for item in record {
            output_str = format!("{}|{:>16}", output_str, item);
        }
        output_str = format!("{}|\n", output_str);
    }
    output_str = format!("{}{}\n", output_str, "-".repeat(columns_n*17 + 1));
    Ok(output_str)

}
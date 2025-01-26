use slug::slugify;
use std::io;
use std::env;
use std::process;


fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        process::exit(1);
    }

    println!("Type your string!");
    let mut input_str = String::new();
    io::stdin().read_line(&mut input_str).expect("Failed to read a string!");
    input_str = input_str.trim_end().to_string();
    let mut output_str;
    if args[1] == "lowercase" {
        output_str = input_str.to_lowercase();
    } else if args[1] == "uppercase" {
        output_str = input_str.to_uppercase();
    } else if args[1] == "no-spaces" {
        output_str = input_str.replace(" ", "");
    } else if args[1] == "slugify" {
        output_str = slugify(input_str);
    } else if args[1] == "czechify" {
        output_str = input_str.replace("r", "ř").replace("R", "Ř").replace("e", "ě").replace("E", "Ě");
    } else if args[1] == "emphasize" {
        output_str = String::new();
        output_str.push_str("!!!");
        output_str.push_str(&input_str);
        output_str.push_str("!!!");
    } else {
        output_str = input_str
    }
    println!("{}", output_str);
}

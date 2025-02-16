use slug::slugify;
use std::io;
use std::error::Error;
use std::thread;
use std::sync::mpsc;
use std::str::FromStr;


enum Command {
    LowerCase(String),
    UpperCase(String),
    NoSpaces(String),
    Slugify(String),
    Czechify(String),
    Emphasize(String),
    Csv(String),
    Quit
}


impl FromStr for Command {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let raw_command = String::from(s);
        if raw_command.trim() == "quit" {
            return Ok(Command::Quit);
        }
        let (left, right) = match raw_command.split_once(" ") {
            Some((l, r)) => (l, r),
            None => return Err("Cannot parse into command and argument!".to_string())
        };
        let command = match left.trim() {
            "lowercase" => Command::LowerCase(right.trim().to_string()),
            "uppercase" => Command::UpperCase(right.trim().to_string()),
            "no-spaces" => Command::NoSpaces(right.trim().to_string()),
            "slugify" => Command::Slugify(right.trim().to_string()),
            "czechify" => Command::Czechify(right.trim().to_string()),
            "emphasize" => Command::Emphasize(right.trim().to_string()),
            "csv" => Command::Csv(right.trim().to_string()),
            _ => return Err("Not a valid command!".to_string())
        };
        Ok(command)
    }
}


fn main() {
    let (sender, receiver) = mpsc::channel::<Command>();
    let handle = thread::spawn(move || {
        if let Err(e) = reader_loop(sender) {
            eprintln!("ERROR: {}", e);
        };
    });

    //writer loop
    match writer_loop(receiver) {
        Ok(()) => {
            println!("Quitting...");
        },
        Err(e) => {
            eprintln!("ERROR: {}", e);
        }
    }
    
    handle.join().unwrap();
}


fn reader_loop(sender: mpsc::Sender<Command>) -> Result<(), Box<dyn Error>> {
    loop {
        let input = get_line_from_user()?;
        let command = Command::from_str(&input)?;
        let stop = match command {
            Command::Quit => true,
            _ => false
        };
        sender.send(command)?;
        if stop {
            break;
        }
    }
    Ok(())
}


fn writer_loop(receiver: mpsc::Receiver<Command>) -> Result<(), Box<dyn Error>> {
    loop {
        match receiver.recv()? {
            Command::LowerCase(arg) => {
                let result = transform_lowercase(&arg)?;
                println!("{}", result);
            },
            Command::UpperCase(arg) => {
                let result = transform_uppercase(&arg)?;
                println!("{}", result);
            },
            Command::NoSpaces(arg) => {
                let result = transform_no_spaces(&arg)?;
                println!("{}", result);
            },
            Command::Slugify(arg) => {
                let result = transform_slugify(&arg)?;
                println!("{}", result);
            },
            Command::Czechify(arg) => {
                let result = transform_czechify(&arg)?;
                println!("{}", result);
            },
            Command::Emphasize(arg) => {
                let result = transform_emphasize(&arg)?;
                println!("{}", result);
            },
            Command::Csv(arg) => {
                let result = transform_csv(&arg)?;
                println!("{}", result);
            },
            Command::Quit => {
                break;
            }
        }
    }
    Ok(())
}


fn get_line_from_user() -> Result<String, Box<dyn Error>> {
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


fn transform_lowercase(input_str: &String) -> Result<String, Box<dyn Error>> {
    validate_input(&input_str)?;
    let output_str = input_str.to_lowercase();
    Ok(output_str)
}


fn transform_uppercase(input_str: &String) -> Result<String, Box<dyn Error>> {
    validate_input(&input_str)?;
    let output_str = input_str.to_uppercase();
    Ok(output_str)
}


fn transform_no_spaces(input_str: &String) -> Result<String, Box<dyn Error>> {
    validate_input(&input_str)?;
    let output_str = input_str.replace(" ", "");
    Ok(output_str)
}


fn transform_slugify(input_str: &String) -> Result<String, Box<dyn Error>> {
    validate_input(&input_str)?;
    let output_str = slugify(input_str);
    Ok(output_str)
}


fn transform_czechify(input_str: &String) -> Result<String, Box<dyn Error>> {
    validate_input(&input_str)?;
    let output_str = input_str.replace("r", "ř").replace("R", "Ř").replace("e", "ě").replace("E", "Ě");
    Ok(output_str)
}


fn transform_emphasize(input_str: &String) -> Result<String, Box<dyn Error>> {
    validate_input(&input_str)?;
    let output_str = format!("{}{}{}", "!!!", input_str, "!!!");
    Ok(output_str)
}


fn transform_csv(input_str: &String) -> Result<String, Box<dyn Error>> {
    Ok(String::from(input_str))
}

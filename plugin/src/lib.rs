use std::io;
use std::io::{Read, Write};
use std::string::String;

#[no_mangle]
pub fn add_emoji() {
    let pattern = ":-)";
    let mut input_vec: Vec<u8> = Vec::new();

    // read vector and convert it to a string
    io::stdin()
        .read_to_end(&mut input_vec)
        .expect("Unable to read input");
    let mut input = String::from_utf8(input_vec).expect("Unable to convert input to a string");

    // replace :-) by 😀
    while let Some(idx) = input.find(pattern) {
        input.replace_range(idx..idx + pattern.len(), "😀");
    }
    print!("{}", input);

    io::stdout().flush().unwrap();
}

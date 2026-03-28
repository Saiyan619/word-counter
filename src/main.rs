use std::fs::File;
use std::io::{self, prelude::*};
use std::process::exit;
// use std::env;

fn main() {

    println!("Find File Here:");

    let mut buffer = String::new();

    let read_path = match io::stdin().read_line(&mut buffer){
        Ok(_) => buffer.trim(),
        Err(err) => {println!("{err}"); exit(1)}
    };

    println!("cli read path: {:?}", read_path);
  
    let mut opened_path = match File::open(&read_path) {
        Ok(ok) => ok,
        Err(err) => {println!("something went wrong: {}", err); exit(1)}
    };

    // // open an empty heap string to save your contents in
    let mut content = String::new();

    // // read the content into the empty heap string
    match opened_path.read_to_string(&mut content){
        Err(err) => {println!("{err}"); exit(1)},
        Ok(ok) => ok
    };

    println!("path:{:?}, content:{}, content_length:{}", read_path, content, content.len());

    // // remove the whitespace and count the words only
    let word_counter= content.split_whitespace().count();

    // // log result
    println!("Words Counted: {:?}", word_counter);

}

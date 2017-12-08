use ::Executable;
use ::job_spec::JobSpec2;
use ::nom::IResult;
use std::os::unix::process::CommandExt;
use std::process::Command;

fn token_char(ch: char) -> bool {
    match ch {
        ch if ch as u8 <= 32 || 127 <= ch as u8 => false,
        '"' | '\'' | '>' | '<' | '|' | ';' | '{' | '}' => false,
        _ => true,
    }
}

named!(bare_token<&str, &str>, take_while1_s!(token_char));
named!(quoted_token<&str, &str>, delimited!(tag_s!("\""),
                                            take_until_s!("\""),
                                            tag_s!("\"")));
named!(place_holder<&str, &str>, tag_s!("{}"));
named!(command_token<&str, &str>,
       alt!(bare_token | quoted_token | place_holder));

named!(command< &str, Vec<&str> >,
       terminated!(ws!(many1!(command_token)), eof!()));

impl Executable for Command {
    fn exec(&mut self) -> ! {
        panic!("Failed to execute command {:?}", CommandExt::exec(self));
    }
}

fn parse_cmd<'a>(format: &'a str, args: &'a [&str]) -> Vec<&'a str> {
    let tokens = match command(format) {
        IResult::Done(_, result) => result,
        IResult::Error(error) => panic!("Error {:?}", error),
        IResult::Incomplete(needed) => panic!("Needed {:?}", needed)
    };
    let mut new_args: Vec<&str> = Vec::new();
    let mut i = 0;
    for arg in &tokens {
        if *arg == "{}" {
            new_args.push(args[i]);
            i += 1;
        } else {
            new_args.push(arg);
        }
    }
    new_args
}

pub fn new_command(format: &str, args: &[&str]) -> JobSpec2 {
    let vec = parse_cmd(format, args);
    let mut command = Command::new(vec[0]);
    if vec.len() > 1 {
        command.args(&vec[1..]);
    }
    JobSpec2::new(command)
}

#[macro_export]
macro_rules! cmd {
    ($format:expr) => ($crate::command::new_command($format, &[]));
    ($format:expr, $($arg:expr),+) => 
        ($crate::command::new_command($format, &[$($arg),+]));
}

#[test]
fn test_parse_cmd() {
    let tokens = parse_cmd(r#"cmd 1 2 
                              3 "
  4" {}"#, &["5"]);
    assert_eq!("cmd", tokens[0]);
    assert_eq!("1", tokens[1]);
    assert_eq!("2", tokens[2]);
    assert_eq!("3", tokens[3]);
    assert_eq!("\n  4", tokens[4]);
    assert_eq!("5", tokens[5]);
}

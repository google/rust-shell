use ::JobSpec;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std;
use ::nom::IResult;

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

pub fn parse_cmd(text: &str) -> Vec<&str> {
    match command(text) {
        IResult::Done(_, result) => result,
        IResult::Error(error) => panic!("Error {:?}", error),
        IResult::Incomplete(needed) => panic!("Needed {:?}", needed)
    }
}

/// Single Command
pub struct ShellCommand {
    command: Command,
    setpgid: bool
}

impl ShellCommand {
    pub fn new(format: &str, args: &[&str]) -> ShellCommand {
        let vec = parse_cmd(format);
        let mut command = Command::new(vec[0]);
        command.args(&vec.as_slice()[1..]);
        ShellCommand {
            command: command,
            setpgid: false
        }
    }
}

impl JobSpec for ShellCommand {
    fn exec(mut self) -> ! {
        self.command.exec();
        std::process::exit(1);
    }

    fn setpgid(mut self) -> Self {
        self.setpgid = true;
        self
    }

    fn getpgid(&self) -> bool {
        return self.setpgid;
    }
}

#[macro_export]
macro_rules! cmd {
    ($format:expr) => ($crate::command::ShellCommand::new($format, &[]));
    ($format:expr, $($arg:expr),+) => 
        ($crate::command::ShellCommand::new($format, &[$($arg),+]));
}

#[test]
fn test_parse_cmd() {
    let tokens = parse_cmd(r#"cmd 1 2 
                              3 "
  4""#);
    assert_eq!("cmd", tokens[0]);
    assert_eq!("1", tokens[1]);
    assert_eq!("2", tokens[2]);
    assert_eq!("3", tokens[3]);
    assert_eq!("\n  4", tokens[4]);
}

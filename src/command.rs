use shell_command::ShellCommand;
use nom::IResult;
use std::process::Command;
use std::env;
use std::env::VarError;

fn token_char(ch: char) -> bool {
    match ch {
        '\x00' ... '\x20' => false,
        '\x7f' | '"' | '\'' | '>' | '<' | '|' | ';' | '{' | '}' | '$' => false,
        _ => true,
    }
}

fn var_char(ch: char) -> bool {
    match ch {
        'a' ... 'z' => true,
        'A' ... 'Z' => true,
        '0' ... '9' => true,
        '_' => true,
        _ => false,
    }
}

enum TokenPart {
    Bare(String),
    Placeholder,
    EnvVariable(String),
}

struct Token(Vec<TokenPart>);

impl Token {
    fn into_string(self, args: &mut Iterator<Item = &str>)
            -> Result<String, VarError> {
        let mut token = String::from("");
        for part in self.0 {
            match part {
                TokenPart::Bare(s) => token += &s,
                TokenPart::Placeholder =>
                    token += args.next().expect("Too many placeholders"),
                TokenPart::EnvVariable(name) => {
                    debug!("Environment variable {}", name);
                    token += &env::var(name)?
                }
            }
        }
        Ok(token)
    }
}

named!(bare_token<&str, TokenPart>,
       map!(take_while1_s!(token_char), |s| TokenPart::Bare(String::from(s))));
named!(quoted_token<&str, TokenPart>,
       map!(delimited!(tag_s!("\""), take_until_s!("\""), tag_s!("\"")),
            |s| TokenPart::Bare(String::from(s))));
named!(place_holder<&str, TokenPart>,
       map!(tag_s!("{}"), |_| TokenPart::Placeholder));
named!(env_var<&str, TokenPart>,
       map!(preceded!(tag!("$"), take_while1_s!(var_char)),
            |name| TokenPart::EnvVariable(String::from(name))));
named!(command_token<&str, Token>,
       map!(many1!(alt!(bare_token | quoted_token | place_holder | env_var)),
            |vec| Token(vec)));

named!(command< &str, Vec<Token> >,
       terminated!(ws!(many1!(command_token)), eof!()));

#[macro_export]
macro_rules! cmd {
    ($format:expr) => ($crate::command::new_command($format, &[]).unwrap());
    ($format:expr, $($arg:expr),+) =>
        ($crate::command::new_command($format, &[$($arg),+]).unwrap());
}

fn parse_cmd<'a>(format: &'a str, args: &'a [&str])
        -> Result<Vec<String>, VarError> {
    let tokens = match command(format) {
        IResult::Done(_, result) => result,
        IResult::Error(error) => panic!("Error {:?}", error),
        IResult::Incomplete(needed) => panic!("Needed {:?}", needed)
    };
    let args = args.iter().map(|a| *a).collect::<Vec<_>>();
    let mut args = args.into_iter();
    tokens.into_iter().map(|token| token.into_string(&mut args))
        .collect::<Result<Vec<_>, _>>()
}

pub fn new_command(format: &str, args: &[&str])
        -> Result<ShellCommand, VarError> {
    let vec = parse_cmd(format, args)?;
    let mut command = Command::new(&vec[0]);
    if vec.len() > 1 {
        command.args(&vec[1..]);
    }
    let line = vec.join(" ");
    Ok(ShellCommand::new(line, command))
}


#[test]
fn test_parse_cmd() {
    let tokens = parse_cmd(r#"cmd 1 2
                              3 "
  4" {}"#, &["5"]).unwrap();
    assert_eq!("cmd", tokens[0]);
    assert_eq!("1", tokens[1]);
    assert_eq!("2", tokens[2]);
    assert_eq!("3", tokens[3]);
    assert_eq!("\n  4", tokens[4]);
    assert_eq!("5", tokens[5]);
}

#[test]
fn test_parse_cmd_env() {
    use env_logger;
    env_logger::init().unwrap();
    env::set_var("MY_VAR", "VALUE");
    let tokens = parse_cmd("echo $MY_VAR/dir", &[]).unwrap();
    assert_eq!("VALUE/dir", tokens[1]);
}

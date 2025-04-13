/// Enum representing CLI commands
#[derive(Debug, PartialEq)]
pub enum Command {
    Login,
    Whoami,
    Help,
    Unknown(String),
}

/// Parse command line arguments and return a Command
///
/// # Arguments
/// * `args` - Command line arguments (including program name)
///
/// # Returns
/// * `Command` - The parsed command
pub fn parse_args(args: &[String]) -> Command {
    if args.len() <= 1 {
        return Command::Help;
    }

    match args[1].as_str() {
        "login" => Command::Login,
        "whoami" => Command::Whoami,
        "help" => Command::Help,
        cmd => Command::Unknown(cmd.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_login_command() {
        let args = vec!["program".to_string(), "login".to_string()];
        assert_eq!(parse_args(&args), Command::Login);
    }

    #[test]
    fn test_parse_whoami_command() {
        let args = vec!["program".to_string(), "whoami".to_string()];
        assert_eq!(parse_args(&args), Command::Whoami);
    }

    #[test]
    fn test_parse_help_command() {
        let args = vec!["program".to_string(), "help".to_string()];
        assert_eq!(parse_args(&args), Command::Help);
    }

    #[test]
    fn test_parse_no_command() {
        let args = vec!["program".to_string()];
        assert_eq!(parse_args(&args), Command::Help);
    }

    #[test]
    fn test_parse_unknown_command() {
        let args = vec!["program".to_string(), "unknown".to_string()];
        assert_eq!(parse_args(&args), Command::Unknown("unknown".to_string()));
    }
}

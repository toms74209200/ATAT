/// Enum representing CLI commands
#[derive(Debug, PartialEq)]
pub enum Command {
    Login,
    Whoami,
    RemoteList,
    RemoteAdd { repo: String },
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
    match args.len() {
        0 | 1 => Command::Help,
        2 => match args[1].as_str() {
            "login" => Command::Login,
            "whoami" => Command::Whoami,
            "remote" => Command::RemoteList,
            "help" => Command::Help,
            cmd => Command::Unknown(cmd.to_string()),
        },
        3 => match (args[1].as_str(), args[2].as_str()) {
            ("remote", "add") => Command::Unknown("remote add <repository>".to_string()),
            ("remote", sub_cmd) => Command::Unknown(format!("remote {}", sub_cmd)),
            (cmd, _) => Command::Unknown(cmd.to_string()),
        },
        _ => match (args[1].as_str(), args[2].as_str()) {
            ("remote", "add") => {
                if args.len() >= 4 {
                    Command::RemoteAdd {
                        repo: args[3].clone(),
                    }
                } else {
                    Command::Unknown("remote add <repository>".to_string())
                }
            }
            (cmd1, cmd2) => Command::Unknown(format!("{} {}", cmd1, cmd2)),
        },
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
    fn test_parse_remote_list_command() {
        let args = vec!["program".to_string(), "remote".to_string()];
        assert_eq!(parse_args(&args), Command::RemoteList);
    }

    #[test]
    fn test_parse_remote_add_command() {
        let args = vec![
            "program".to_string(),
            "remote".to_string(),
            "add".to_string(),
            "owner/repo".to_string(),
        ];
        assert_eq!(
            parse_args(&args),
            Command::RemoteAdd {
                repo: "owner/repo".to_string()
            }
        );
    }

    #[test]
    fn test_parse_remote_add_missing_repo() {
        let args = vec![
            "program".to_string(),
            "remote".to_string(),
            "add".to_string(),
        ];
        assert_eq!(
            parse_args(&args),
            Command::Unknown("remote add <repository>".to_string())
        );
    }

    #[test]
    fn test_parse_remote_unknown_subcommand_with_two_args() {
        let args = vec![
            "program".to_string(),
            "remote".to_string(),
            "unknown_sub".to_string(),
        ];
        assert_eq!(
            parse_args(&args),
            Command::Unknown("remote unknown_sub".to_string())
        );
    }

    #[test]
    fn test_parse_too_many_args_for_known_command() {
        let args = vec![
            "program".to_string(),
            "login".to_string(),
            "extra_arg".to_string(),
        ];
        assert_eq!(parse_args(&args), Command::Unknown("login".to_string()));
    }

    #[test]
    fn test_parse_remote_add_with_extra_args() {
        let args = vec![
            "program".to_string(),
            "remote".to_string(),
            "add".to_string(),
            "owner/repo".to_string(),
            "extra".to_string(),
        ];
        assert_eq!(
            parse_args(&args),
            Command::RemoteAdd {
                repo: "owner/repo".to_string()
            }
        );
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

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
            ("remote", "add") => Command::Unknown(
                "Missing repository argument. Usage: atat remote add <owner>/<repo>".to_string(),
            ),
            ("remote", sub_cmd) => Command::Unknown(format!("remote {}", sub_cmd)),
            (cmd, _) => Command::Unknown(cmd.to_string()),
        },
        _ => match (args[1].as_str(), args[2].as_str()) {
            ("remote", "add") => {
                if args.len() >= 4 {
                    let repo_arg = &args[3];
                    let parts: Vec<&str> = repo_arg.split('/').collect();
                    if parts.len() == 2
                        && !parts[0].is_empty()
                        && !parts[1].is_empty()
                        && !parts[0].contains('/')
                        && !parts[1].contains('/')
                    {
                        Command::RemoteAdd {
                            repo: repo_arg.clone(),
                        }
                    } else {
                        Command::Unknown(
                            "Invalid repository format. Please use <owner>/<repo>.".to_string(),
                        )
                    }
                } else {
                    Command::Unknown(
                        "Missing repository argument. Usage: atat remote add <owner>/<repo>"
                            .to_string(),
                    )
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
            Command::Unknown(
                "Missing repository argument. Usage: atat remote add <owner>/<repo>".to_string()
            )
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

    #[test]
    fn test_parse_remote_add_invalid_format_no_slash() {
        let args = vec![
            "program".to_string(),
            "remote".to_string(),
            "add".to_string(),
            "ownerrepo".to_string(),
        ];
        assert_eq!(
            parse_args(&args),
            Command::Unknown("Invalid repository format. Please use <owner>/<repo>.".to_string())
        );
    }

    #[test]
    fn test_parse_remote_add_invalid_format_empty_owner() {
        let args = vec![
            "program".to_string(),
            "remote".to_string(),
            "add".to_string(),
            "/repo".to_string(),
        ];
        assert_eq!(
            parse_args(&args),
            Command::Unknown("Invalid repository format. Please use <owner>/<repo>.".to_string())
        );
    }

    #[test]
    fn test_parse_remote_add_invalid_format_empty_repo() {
        let args = vec![
            "program".to_string(),
            "remote".to_string(),
            "add".to_string(),
            "owner/".to_string(),
        ];
        assert_eq!(
            parse_args(&args),
            Command::Unknown("Invalid repository format. Please use <owner>/<repo>.".to_string())
        );
    }

    #[test]
    fn test_parse_remote_add_invalid_format_too_many_slashes() {
        let args = vec![
            "program".to_string(),
            "remote".to_string(),
            "add".to_string(),
            "owner/repo/extra".to_string(),
        ];
        assert_eq!(
            parse_args(&args),
            Command::Unknown("Invalid repository format. Please use <owner>/<repo>.".to_string())
        );
    }

    #[test]
    fn test_parse_remote_add_invalid_format_owner_contains_slash() {
        let args = vec![
            "program".to_string(),
            "remote".to_string(),
            "add".to_string(),
            "ow/ner/repo".to_string(),
        ];
        assert_eq!(
            parse_args(&args),
            Command::Unknown("Invalid repository format. Please use <owner>/<repo>.".to_string())
        );
    }
}

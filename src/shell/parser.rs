/// Command parsing module

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCommand {
    pub command: String,
    pub args: Vec<String>,
    pub raw_input: String,  // Store original input with spaces preserved
}

impl ParsedCommand {
    /// Parse a command line string into command and arguments
    pub fn parse(input: &str) -> Option<Self> {
        let input = input.trim();

        if input.is_empty() {
            return None;
        }

        let mut parts = input.split_whitespace();
        let command = parts.next()?.to_string();
        let args: Vec<String> = parts.map(|s| s.to_string()).collect();

        Some(ParsedCommand {
            command,
            args,
            raw_input: input.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_command() {
        let cmd = ParsedCommand::parse("ls").unwrap();
        assert_eq!(cmd.command, "ls");
        assert_eq!(cmd.args.len(), 0);
    }

    #[test]
    fn test_parse_with_args() {
        let cmd = ParsedCommand::parse("ls -la /home").unwrap();
        assert_eq!(cmd.command, "ls");
        assert_eq!(cmd.args, vec!["-la", "/home"]);
    }

    #[test]
    fn test_parse_empty() {
        assert!(ParsedCommand::parse("").is_none());
        assert!(ParsedCommand::parse("   ").is_none());
    }
}

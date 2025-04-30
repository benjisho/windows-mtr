#[cfg(test)]
mod tests {
    use clap::Parser;
    use std::time::Duration;
    use trippy::config::{OutputFormat, Protocol};

    // We need to import the Cli struct and map_cli_to_config function from our main.rs
    // This requires exposing these items or creating a lib.rs with shared functionality
    // For now, we'll mock the structures to test the logic

    #[derive(Parser, Debug)]
    struct MockCli {
        host: String,
        #[arg(short = 'T')]
        tcp: bool,
        #[arg(short = 'U')]
        udp: bool,
        #[arg(short = 'P')]
        port: Option<u16>,
        #[arg(short = 'r')]
        report: bool,
        #[arg(short = 'c')]
        count: Option<usize>,
        #[arg(short = 'i')]
        interval: Option<f32>,
        #[arg(short = 'w')]
        timeout: Option<f32>,
    }

    fn parse_args(args: Vec<&str>) -> MockCli {
        use clap::CommandFactory;
        let mut cmd = MockCli::command();
        MockCli::from_arg_matches(&cmd.get_matches_from(args)).unwrap()
    }

    #[test]
    fn test_icmp_config() {
        let args = parse_args(vec!["mtr", "8.8.8.8"]);
        assert_eq!(args.host, "8.8.8.8");
        assert!(!args.tcp);
        assert!(!args.udp);
    }

    #[test]
    fn test_tcp_config() {
        let args = parse_args(vec!["mtr", "8.8.8.8", "-T", "-P", "443"]);
        assert_eq!(args.host, "8.8.8.8");
        assert!(args.tcp);
        assert!(!args.udp);
        assert_eq!(args.port, Some(443));
    }

    #[test]
    fn test_udp_config() {
        let args = parse_args(vec!["mtr", "8.8.8.8", "-U", "-P", "53"]);
        assert_eq!(args.host, "8.8.8.8");
        assert!(!args.tcp);
        assert!(args.udp);
        assert_eq!(args.port, Some(53));
    }

    #[test]
    fn test_report_mode() {
        let args = parse_args(vec!["mtr", "8.8.8.8", "-r", "-c", "10"]);
        assert_eq!(args.host, "8.8.8.8");
        assert!(args.report);
        assert_eq!(args.count, Some(10));
    }
}
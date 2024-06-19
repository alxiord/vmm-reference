// Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause
//! CLI for the Reference VMM.

#![deny(missing_docs)]
use std::result;

use clap::{Arg, ArgAction, Command};
use vmm::VMMConfig;

/// Command line parser.
pub struct Cli;

impl Cli {
    /// Parses the command line options into VMM configurations.
    ///
    /// # Arguments
    ///
    /// * `cmdline_args` - command line arguments passed to the application.
    pub fn launch(cmdline_args: &Vec<String>) -> result::Result<VMMConfig, String> {
        let cn: &'static str = cmdline_args[0].clone().leak();
        let mut cmd = Command::new(cn).arg(
            Arg::new("memory").long("memory")
            .num_args(1)
            .action(ArgAction::Set)
            .help("Guest memory configuration.\n\tFormat: \"size_mib=<u32>\"")
        ,)

            .arg(
                Arg::new("vcpu")
                    .long("vcpu")
                    .num_args(1)
                    .help("vCPU configuration.\n\tFormat: \"num=<u8>\""),
            )
            .arg(
                Arg::new("kernel")
                    .long("kernel")
                    .required(true)
                    .num_args(1)
                    .help("Kernel configuration.\n\tFormat: \"path=<string>[,cmdline=<string>,kernel_load_addr=<u64>]\""),
            )
            .arg(
                Arg::new("net")
                    .long("net")
                    .num_args(1)
                    .help("Network device configuration. \n\tFormat: \"tap=<string>\"")
            )
            .arg(
                Arg::new("block")
                    .long("block")
                    .required(false)
                    .num_args(1)
                    .help("Block device configuration. \n\tFormat: \"path=<string>\"")
            );

        let matches = cmd.try_get_matches_from_mut(cmdline_args).map_err(|e| {
            if let Err(ioe) = cmd.print_help() {
                return format!("IO error: {:?}", ioe);
            }
            format!("Invalid command line arguments: {}", e)
        })?;

        VMMConfig::builder()
            .memory_config(matches.get_one::<String>("memory").map(|s| s.as_str()))
            .kernel_config(matches.get_one::<String>("kernel").map(|s| s.as_str()))
            .vcpu_config(matches.get_one::<String>("vcpu").map(|s| s.as_str()))
            .net_config(matches.get_one::<String>("net").map(|s| s.as_str()))
            .block_config(matches.get_one::<String>("block").map(|s| s.as_str()))
            .build()
            .map_err(|e| format!("{:?}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    use linux_loader::cmdline::Cmdline;

    use vmm::{KernelConfig, MemoryConfig, VcpuConfig, DEFAULT_KERNEL_LOAD_ADDR};

    #[test]
    fn test_launch() {
        // Missing command line arguments.
        assert!(Cli::launch(&vec!["foobar".to_string()]).is_err());

        // Invalid extra command line parameter.
        assert!(Cli::launch(
            &[
                "foobar",
                "--memory",
                "size_mib=128",
                "--vcpu",
                "num=1",
                "--kernel",
                "path=/foo/bar,cmdline=\"foo=bar\",kernel_load_addr=42",
                "foobar",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
        )
        .is_err());

        // Invalid memory config: invalid value for `size_mib`.
        assert!(Cli::launch(
            &[
                "foobar",
                "--memory",
                "size_mib=foobar",
                "--vcpu",
                "num=1",
                "--kernel",
                "path=/foo/bar,cmdline=\"foo=bar\",kernel_load_addr=42",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
        )
        .is_err());

        // Memory config: missing value for `size_mib`, use the default
        assert!(Cli::launch(
            &[
                "foobar",
                "--memory",
                "size_mib=",
                "--vcpu",
                "num=1",
                "--kernel",
                "path=/foo/bar,cmdline=\"foo=bar\",kernel_load_addr=42",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
        )
        .is_ok());

        // Invalid memory config: unexpected parameter `foobar`.
        assert!(Cli::launch(
            &[
                "foobar",
                "--memory",
                "foobar=1024",
                "--vcpu",
                "num=1",
                "--kernel",
                "path=/foo/bar,cmdline=\"foo=bar\",kernel_load_addr=42",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
        )
        .is_err());

        // Invalid kernel config: invalid value for `kernel_load_addr`.
        // TODO: harden cmdline check.
        assert!(Cli::launch(
            &[
                "foobar",
                "--memory",
                "size_mib=128",
                "--vcpu",
                "num=1",
                "--kernel",
                "path=/foo/bar,cmdline=\"foo=bar\",kernel_load_addr=foobar",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
        )
        .is_err());

        // Kernel config: missing value for `kernel_load_addr`, use default
        assert!(Cli::launch(
            &[
                "foobar",
                "--memory",
                "size_mib=128",
                "--vcpu",
                "num=1",
                "--kernel",
                "path=/foo/bar,cmdline=\"foo=bar\",kernel_load_addr=",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
        )
        .is_ok());

        // Invalid kernel config: unexpected parameter `foobar`.
        assert!(Cli::launch(
            &[
                "foobar",
                "--memory",
                "size_mib=128",
                "--vcpu",
                "num=1",
                "--kernel",
                "path=/foo/bar,cmdline=\"foo=bar\",kernel_load_addr=42,foobar=42",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
        )
        .is_err());

        // Invalid vCPU config: invalid value for `num_vcpus`.
        assert!(Cli::launch(
            &[
                "foobar",
                "--memory",
                "size_mib=128",
                "--vcpu",
                "num=foobar",
                "--kernel",
                "path=/foo/bar,cmdline=\"foo=bar\",kernel_load_addr=42",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
        )
        .is_err());

        // vCPU config: missing value for `num_vcpus`, use default
        assert!(Cli::launch(
            &[
                "foobar",
                "--memory",
                "size_mib=128",
                "--vcpu",
                "num=",
                "--kernel",
                "path=/foo/bar,cmdline=\"foo=bar\",kernel_load_addr=42",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
        )
        .is_ok());

        // Invalid vCPU config: unexpected parameter `foobar`.
        assert!(Cli::launch(
            &[
                "foobar",
                "--memory",
                "size_mib=128",
                "--vcpu",
                "foobar=1",
                "--kernel",
                "path=/foo/bar,cmdline=\"foo=bar\",kernel_load_addr=42",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
        )
        .is_err());

        let mut foo_cmdline = Cmdline::new(4096);
        foo_cmdline.insert_str("\"foo=bar bar=foo\"").unwrap();

        // OK.
        assert_eq!(
            Cli::launch(
                &[
                    "foobar",
                    "--memory",
                    "size_mib=128",
                    "--vcpu",
                    "num=1",
                    "--kernel",
                    "path=/foo/bar,cmdline=\"foo=bar bar=foo\",kernel_load_addr=42",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
            )
            .unwrap(),
            VMMConfig {
                kernel_config: KernelConfig {
                    path: PathBuf::from("/foo/bar"),
                    cmdline: foo_cmdline,
                    load_addr: 42,
                },
                memory_config: MemoryConfig { size_mib: 128 },
                vcpu_config: VcpuConfig { num: 1 },
                block_config: None,
                net_config: None,
            }
        );

        // Test default values.
        assert_eq!(
            Cli::launch(
                &["foobar", "--kernel", "path=/foo/bar",]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            )
            .unwrap(),
            VMMConfig {
                kernel_config: KernelConfig {
                    path: PathBuf::from("/foo/bar"),
                    cmdline: KernelConfig::default_cmdline(),
                    load_addr: DEFAULT_KERNEL_LOAD_ADDR,
                },
                memory_config: MemoryConfig { size_mib: 256 },
                vcpu_config: VcpuConfig { num: 1 },
                block_config: None,
                net_config: None,
            }
        );
    }
}

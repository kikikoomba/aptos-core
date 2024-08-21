// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::types::{CliCommand, CliError, CliTypedResult},
    update::get_movefmt_path,
};
use async_trait::async_trait;
use clap::{Args, Parser};
use std::{collections::BTreeMap, path::PathBuf, process::Command};

/// Format the Move source code.
#[derive(Debug, Parser)]
pub struct Fmt {
    #[clap(flatten)]
    pub command: FmtCommand,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
pub enum EmitMode {
    Overwrite,
    NewFile,
    StdOut,
    Diff,
}

#[derive(Debug, Args)]
#[clap(group(clap::ArgGroup::new("input")
.required(true)
.multiple(false)
.args(&["file_path", "dir_path"]),
))]
pub struct FmtCommand {
    /// How to generate and show the result after reformatting
    #[clap(long, value_enum, default_value = "overwrite")]
    emit_mode: EmitMode,

    /// Path to the file to be formatted
    #[clap(long, group = "input")]
    file_path: Option<PathBuf>,

    /// Path to the directory to be formatted
    /// if neither file_path or dir_path is set, all move files in the current folder will be formatted
    #[clap(long, group = "input")]
    dir_path: Option<PathBuf>,

    /// Path for the configuration file
    /// Recursively searches the given path for the
    /// movefmt.toml config file
    #[clap(long, value_parser)]
    pub config_path: Option<PathBuf>,

    /// Set options from command line. These settings take
    /// priority over movefmt.toml.
    /// Config options can be found at https://github.com/movebit/movefmt/blob/develop/doc/how_to_use.md
    #[clap(long, value_parser = crate::common::utils::parse_map::<String, String>, default_value = "")]
    pub(crate) config: BTreeMap<String, String>,

    #[clap(long, short)]
    /// Print verbose output
    pub verbose: bool,

    #[clap(long, short)]
    /// Print less output
    pub quiet: bool,
}

#[async_trait]
impl CliCommand<String> for Fmt {
    fn command_name(&self) -> &'static str {
        "Fmt"
    }

    async fn execute(mut self) -> CliTypedResult<String> {
        self.command.execute().await
    }
}

impl FmtCommand {
    async fn execute(self) -> CliTypedResult<String> {
        let exe = get_movefmt_path()?;
        let mut cmd = Command::new(exe.as_path());
        let input_opt = self.file_path;
        let dir_opt = self.dir_path;
        let config_path_opt = self.config_path;
        let config_map = self.config;
        let verbose_flag = self.verbose;
        let quiet_flag = self.quiet;
        let emit_mode = match self.emit_mode {
            EmitMode::Overwrite => "overwrite",
            EmitMode::NewFile => "new_file",
            EmitMode::StdOut => "stdout",
            EmitMode::Diff => "diff",
        };
        cmd.arg(format!("--emit={}", emit_mode));
        if let Some(config_path) = config_path_opt {
            cmd.arg(format!("--config-path={}", config_path.as_path().display()));
        }
        if verbose_flag {
            cmd.arg("-v");
        } else if quiet_flag {
            cmd.arg("-q");
        }
        if !config_map.is_empty() {
            let mut config_map_str_vec = vec![];
            for (key, value) in config_map {
                config_map_str_vec.push(format!("{}={}", key, value));
            }
            cmd.arg(format!("--config={}", config_map_str_vec.join(",")));
        }
        if let Some(file_path) = input_opt {
            cmd.arg(format!("--file-path={}", file_path.as_path().display()));
        } else {
            let dir_path = if let Some(dir_path) = dir_opt {
                dir_path.as_path().display().to_string()
            } else {
                "./".to_string()
            };
            cmd.arg(format!("--dir-path={}", dir_path));
        }
        let to_cli_error = |e| CliError::IO(exe.display().to_string(), e);
        let out = cmd.output().map_err(to_cli_error)?;
        if out.status.success() {
            // let string_res = String::from_utf8(out.stdout);
            match String::from_utf8(out.stdout) {
                Ok(output) => {
                    eprint!("{}", output);
                    Ok("ok".to_string())
                },
                Err(err) => Err(CliError::UnexpectedError(format!(
                    "output generated by formatter is not valid utf8: {}",
                    err
                ))),
            }
        } else {
            Err(CliError::UnexpectedError(format!(
                "formatter exited with status {}: {}",
                out.status,
                String::from_utf8(out.stderr).unwrap_or_default()
            )))
        }
    }
}
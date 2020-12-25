//! Rust tools

#[macro_use]
mod macros;

mod action_option;
mod actions;
mod cfg;
mod cfg_option;
mod error;
mod params;
mod parse_cfg;
mod transforming_params;

use action_option::ActionOption;
use actions::Actions;
use cfg_option::CfgOption;
use error::Error;
use params::Params;
use parse_cfg::parse_cfg;
use std::{
  env::{args, Args},
  fs::File,
  io::{stderr, stdout, BufRead, BufReader, Write},
  process::{Command, Stdio},
};
use transforming_params::TransformingParams;

type Result<T> = core::result::Result<T, Error>;

fn main() -> Result<()> {
  let mut args = args();
  let _ = req(&mut args)?;
  let mut maybe_action = req(&mut args)?;

  let mut param = |name: &str| {
    Ok::<_, Error>(if maybe_action == name {
      let rslt = req(&mut args)?;
      maybe_action = req(&mut args)?;
      rslt
    } else {
      Default::default()
    })
  };

  let file = param("--file")?;
  let (mut params, mut tp) =
    if !file.is_empty() { parse_cfg(File::open(file)?)? } else { Default::default() };

  let template = param("--template")?;
  if !template.is_empty() {
    params = template.parse::<CfgOption>()?.into_params();
  }

  let toolchain = param("--toolchain")?;
  if !toolchain.is_empty() {
    tp.toolchain = toolchain;
  }

  parse_action(&mut args, maybe_action, params, tp)?;

  Ok(())
}

fn handle_cmd_output(mut cmd: &mut Command) -> Result<()> {
  cmd = cmd.stdin(Stdio::piped()).stdout(Stdio::piped());
  let child = cmd.spawn()?;
  let mut buffer = String::new();
  macro_rules! write {
    ($inner:expr, $output:expr) => {
      let mut br = BufReader::new($inner);
      while br.read_line(&mut buffer)? != 0 {
        $output.write_all(buffer.as_bytes())?;
        buffer.clear();
      }
    };
  };
  if let Some(child_stderr) = child.stderr {
    write!(child_stderr, stderr());
  }
  if let Some(child_stdout) = child.stdout {
    write!(child_stdout, stdout());
  }
  Ok(())
}

fn opt(args: &mut Args) -> String {
  args.next().unwrap_or_default()
}

fn parse_action(
  args: &mut Args,
  action_string: String,
  params: Params,
  mut tp: TransformingParams,
) -> crate::Result<()> {
  let mut actions = Actions::new(params);
  match action_string.parse()? {
    ActionOption::BuildGeneric => {
      actions.params.modify(&tp);
      actions.build_generic(req(args)?)?;
    }
    ActionOption::BuildWithFeatures => {
      actions.params.modify(&tp);
      actions.build_with_features(req(args)?, opt(args))?;
    }
    ActionOption::CheckGeneric => {
      actions.params.modify(&tp);
      actions.check_generic(req(args)?)?;
    }
    ActionOption::CheckWithFeatures => {
      actions.params.modify(&tp);
      actions.check_with_features(req(args)?, opt(args))?;
    }
    ActionOption::Clippy => {
      tp.add_clippy_flags.extend(opt(args).split(',').map(|e| e.into()));
      tp.rm_clippy_flags.extend(opt(args).split(',').map(|e| e.into()));
      actions.params.modify(&tp);
      actions.clippy()?;
    }
    ActionOption::RustFlags => {
      tp.add_rust_flags.extend(opt(args).split(',').map(|e| e.into()));
      tp.rm_rust_flags.extend(opt(args).split(',').map(|e| e.into()));
      actions.params.modify(&tp);
      actions.rust_flags()?;
    }
    ActionOption::Rustfmt => {
      actions.params.modify(&tp);
      actions.rustfmt()?;
    }
    ActionOption::SetUp => {
      actions.params.modify(&tp);
      actions.set_up()?;
    }
    ActionOption::TestGeneric => {
      actions.params.modify(&tp);
      actions.test_generic(req(args)?)?;
    }
    ActionOption::TestWithFeatures => {
      actions.params.modify(&tp);
      actions.test_with_features(req(args)?, opt(args))?;
    }
  };
  Ok(())
}

fn req(args: &mut Args) -> Result<String> {
  args.next().ok_or(Error::WrongNumberOfArgs { expected: 1, received: 0 })
}

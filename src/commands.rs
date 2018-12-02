use std::fmt;
use std::process::{Command, Output};

use failure::Error;

#[derive(Fail)]
pub struct FailedExecution {
    output: Output,
}

impl FailedExecution {
    pub fn from_output(output: Output) -> FailedExecution {
        FailedExecution { output }
    }

    pub fn exit_code(&self) -> Option<i32> {
        self.output.status.code()
    }

    pub fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.output.stdout).to_string()
    }

    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).to_string()
    }
}

impl fmt::Debug for FailedExecution {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FailedExecution {{ exit_code: {:?} }}", self.exit_code())
    }
}

impl fmt::Display for FailedExecution {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let exit_code = match self.exit_code() {
            Some(code) => format!(" with exit code {}", code),
            None => "".to_string(),
        };

        let stdout = self.stdout();
        let stdout = match stdout.is_empty() {
            false => format!("\n\nstdout: {}", stdout.trim()),
            true => stdout,
        };

        let stderr = self.stderr();
        let stderr = match stderr.is_empty() {
            false => format!("\n\nstderr: {}", stderr.trim()),
            true => stderr,
        };

        write!(f, "The command failed{}.{}{}", exit_code, stdout, stderr)
    }
}

pub trait SuccessOutput {
    fn success_output(&mut self) -> Result<Output, Error>;
}

impl SuccessOutput for Command {
    fn success_output(&mut self) -> Result<Output, Error> {
        let output = self.output()?;
        match output.status.success() {
            true => Ok(output),
            false => Err(FailedExecution::from_output(output).into()),
        }
    }
}

//! A whole table job: how the data is read, the steps, and how the result
//! is written. The table tab edits one; a recipe's `table` step holds one.

use serde::{Deserialize, Serialize};

use super::export::{Export, Output};
use super::import::{Input, Source};
use super::{Table, TableStep};

fn yes() -> bool {
    true
}
fn is_true(b: &bool) -> bool {
    *b
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JobStep {
    #[serde(flatten)]
    pub op: TableStep,
    /// A switched-off step is kept but skipped.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub enabled: bool,
}

impl From<TableStep> for JobStep {
    fn from(op: TableStep) -> Self {
        Self { op, enabled: true }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TableJob {
    #[serde(default)]
    pub read: Source,
    #[serde(default)]
    pub steps: Vec<JobStep>,
    #[serde(default)]
    pub write: Export,
}

/// Where a job failed: reading (`step` is `None`), a step, or writing.
#[derive(Clone, Debug, PartialEq)]
pub struct JobError {
    pub step: Option<usize>,
    pub message: String,
}

impl std::fmt::Display for JobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.step {
            Some(s) => write!(
                f,
                "{}",
                t!(
                    "table-step-failed",
                    step = s + 1,
                    message = self.message.clone()
                )
            ),
            None => f.write_str(&self.message),
        }
    }
}

impl TableJob {
    /// Runs the switched-on steps from `table`.
    pub fn apply_steps(&self, mut table: Table) -> Result<Table, JobError> {
        for (i, s) in self.steps.iter().enumerate().filter(|(_, s)| s.enabled) {
            table = s.op.apply(&table).map_err(|message| JobError {
                step: Some(i),
                message,
            })?;
        }
        Ok(table)
    }

    /// Reads, changes and writes.
    pub fn run(&self, input: Input) -> Result<Output, JobError> {
        let table = self.read.read(input).map_err(|message| JobError {
            step: None,
            message,
        })?;
        let table = self.apply_steps(table)?;
        self.write.write(&table).map_err(|message| JobError {
            step: None,
            message,
        })
    }

    /// One line per step, for recipes.
    pub fn summary(&self) -> Vec<String> {
        let mut out = vec![self.read.summary()];
        out.extend(
            self.steps
                .iter()
                .filter(|s| s.enabled)
                .map(|s| s.op.summary()),
        );
        out.push(t!("table-export-to", format = self.write.summary()));
        out
    }

    /// Steps that pick rows by their number.
    pub fn has_row_number_steps(&self) -> bool {
        self.steps.iter().any(|s| s.enabled && s.op.by_row_number())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::export::QuoteMode;
    use crate::table::steps::{Condition, SortKey};

    #[test]
    fn a_job_replays_on_new_data() {
        let job = TableJob {
            read: Source::Csv {
                delimiter: ',',
                quote: Some('"'),
                header: true,
                detect_types: true,
            },
            steps: vec![
                TableStep::RenameColumn {
                    column: "n".into(),
                    to: "Count".into(),
                }
                .into(),
                TableStep::FilterRows {
                    column: "Count".into(),
                    condition: Condition::GreaterOrEqual,
                    value: "2".into(),
                    values: vec![],
                    match_case: false,
                }
                .into(),
                TableStep::SortRows {
                    keys: vec![SortKey {
                        column: "name".into(),
                        descending: false,
                    }],
                }
                .into(),
            ],
            write: Export::Csv {
                delimiter: '|',
                quote: QuoteMode::Needed,
                header: true,
                crlf: false,
            },
        };
        let out = job.run(Input::Text("name,n\nb,3\na,1\nc,2\n")).unwrap();
        assert_eq!(out, Output::Text("name|Count\nb|3\nc|2\n".into()));
        // Next week's file has its columns the other way round.
        let out = job.run(Input::Text("n,name\n5,z\n9,y\n")).unwrap();
        assert_eq!(out, Output::Text("Count|name\n9|y\n5|z\n".into()));
        let err = job.run(Input::Text("x\n1\n")).unwrap_err();
        assert_eq!(err.step, Some(0));
        let json = serde_json::to_string(&job).unwrap();
        assert_eq!(serde_json::from_str::<TableJob>(&json).unwrap(), job);
    }
}

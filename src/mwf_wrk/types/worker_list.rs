use super::{WorkerInfo, WorkerOutputFormat, terminal_worker_column};

/// List of worker information returned by MWFWrk.
#[derive(Debug, Clone)]
pub struct WorkerList {
    /// Worker information entries contained in the list.
    pub worker_info_list: Vec<WorkerInfo>,
}

impl std::fmt::Display for WorkerList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        writeln!(f, "{}", Self::get_header(WorkerOutputFormat::Terminal))?;

        for line in self.to_string_list(WorkerOutputFormat::Terminal) {
            writeln!(f, "{line}")?;
        }
        Ok(())
    }
}

impl WorkerList {
    /// Returns the column header corresponding to the requested format.
    pub fn get_header(format: WorkerOutputFormat) -> String {
        match format {
            WorkerOutputFormat::Terminal => {
                let header = format!("{:<width_worker$} | {:<width_father$} | {:<width_state$}", "WorkerID", "ParentID", "State", width_worker = terminal_worker_column::WORKER_ID_LEN, width_father = terminal_worker_column::FATHER_ID_LEN, width_state = terminal_worker_column::STATE_LEN,);

                let separator = "-".repeat(header.len());

                format!("{header}\n{separator}")
            }

            WorkerOutputFormat::CSV => String::from("WorkerID,ParentID,State"),
        }
    }

    /// Converts a worker information list into strings.
    pub fn to_string_list(&self, format: WorkerOutputFormat) -> Vec<String> {
        let mut worker_info_list = self.worker_info_list.clone();
        worker_info_list.sort_by(|a, b| a.worker_id.cmp(&b.worker_id));

        match format {
            WorkerOutputFormat::Terminal => worker_info_list.iter().map(|worker_info| worker_info.to_terminal_string()).collect(),

            WorkerOutputFormat::CSV => worker_info_list.iter().map(|worker_info| worker_info.to_csv_string()).collect(),
        }
    }
}

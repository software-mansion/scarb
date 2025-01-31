use anyhow::Context;
use anyhow::Result;
use cairo_lang_runner::casm_run::format_for_panic;
use cairo_lang_runner::CairoHintProcessor;
use cairo_vm::vm::runners::cairo_runner::CairoRunner;
use scarb_ui::Message;
use serde::{Serialize, Serializer};

#[derive(Serialize)]
pub struct ExecutionSummary {
    pub output: Option<ExecutionOutput>,
    pub resources: Option<ExecutionResources>,
}

impl Message for ExecutionSummary {
    fn print_text(self)
    where
        Self: Sized,
    {
        if let Some(output) = self.output {
            output.print_text();
        }
        if let Some(resources) = self.resources {
            resources.print_text();
        }
    }

    fn structured<S: Serializer>(self, ser: S) -> std::result::Result<S::Ok, S::Error>
    where
        Self: Sized,
    {
        self.serialize(ser)
    }
}

#[derive(Serialize)]
pub struct ExecutionOutput(String);

impl ExecutionOutput {
    pub fn try_new(runner: &mut CairoRunner) -> Result<Self> {
        let mut output_buffer = "Program output:\n".to_string();
        runner.vm.write_output(&mut output_buffer)?;
        let output = output_buffer.trim_end().to_string();
        Ok(Self(output))
    }
}

impl Message for ExecutionOutput {
    fn print_text(self)
    where
        Self: Sized,
    {
        println!("{}", self.0);
    }

    fn structured<S: Serializer>(self, ser: S) -> std::result::Result<S::Ok, S::Error>
    where
        Self: Sized,
    {
        self.serialize(ser)
    }
}

#[derive(Serialize)]
pub struct ExecutionResources {
    n_steps: usize,
    n_memory_holes: usize,
    builtin_instance_counter: Vec<(String, usize)>,
    syscalls: Vec<(String, usize)>,
}

impl ExecutionResources {
    pub fn try_new(runner: &CairoRunner, hint_processor: CairoHintProcessor) -> Result<Self> {
        let used_resources = runner
            .get_execution_resources()
            .context("failed to get execution resources, but the run was successful")?;

        let mut all_used_resources = hint_processor.syscalls_used_resources;
        all_used_resources.basic_resources += &used_resources;

        let resources = all_used_resources.basic_resources;
        let builtin_instance_counter = sort_by_value(&resources.builtin_instance_counter)
            .into_iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect::<Vec<_>>();
        let syscalls = sort_by_value(&all_used_resources.syscalls)
            .into_iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect::<Vec<_>>();

        Ok(Self {
            n_steps: resources.n_steps,
            n_memory_holes: resources.n_memory_holes,
            builtin_instance_counter,
            syscalls,
        })
    }
}

impl Message for ExecutionResources {
    fn print_text(self)
    where
        Self: Sized,
    {
        println!("Resources:");
        println!("\tsteps: {}", self.n_steps);
        println!("\tmemory holes: {}", self.n_memory_holes);
        println!(
            "\tbuiltins: ({})",
            format_items(&self.builtin_instance_counter)
        );
        println!("\tsyscalls: ({})", format_items(&self.syscalls));
    }

    fn structured<S: Serializer>(self, ser: S) -> anyhow::Result<S::Ok, S::Error>
    where
        Self: Sized,
    {
        self.serialize(ser)
    }
}

fn sort_by_value<'a, K, V, M>(map: M) -> Vec<(&'a K, &'a V)>
where
    M: IntoIterator<Item = (&'a K, &'a V)>,
    V: Ord,
{
    let mut sorted: Vec<_> = map.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    sorted
}

fn format_items<K, V>(items: &[(K, V)]) -> String
where
    K: std::fmt::Debug,
    V: std::fmt::Display,
{
    items
        .iter()
        .map(|(key, value)| format!("{key:?}: {value}"))
        .collect::<Vec<String>>()
        .join(", ")
}

#[derive(Serialize)]
pub struct PanicReason(Option<String>);

impl PanicReason {
    pub fn try_new(runner: &CairoRunner, hint_processor: &CairoHintProcessor) -> Result<Self> {
        let panic_reason = if let [.., start_marker, end_marker] = &hint_processor.markers[..] {
            let size = (*end_marker - *start_marker).with_context(|| {
                format!("panic data markers mismatch: start={start_marker}, end={end_marker}")
            })?;
            let panic_data = runner
                .vm
                .get_integer_range(*start_marker, size)
                .with_context(|| "failed reading panic data")?;
            Some(format_for_panic(panic_data.into_iter().map(|value| *value)))
        } else {
            None
        };
        Ok(Self(panic_reason))
    }

    pub fn into_result(self) -> Result<()> {
        self.into()
    }
}

impl Message for PanicReason {
    fn print_text(self)
    where
        Self: Sized,
    {
        if let Some(reason) = self.0 {
            println!("{reason}");
        }
    }

    fn structured<S: Serializer>(self, ser: S) -> std::result::Result<S::Ok, S::Error>
    where
        Self: Sized,
    {
        self.serialize(ser)
    }
}

impl From<PanicReason> for Result<()> {
    fn from(panic_reason: PanicReason) -> Self {
        if let Some(reason) = panic_reason.0 {
            Err(anyhow::anyhow!(reason))
        } else {
            Ok(())
        }
    }
}

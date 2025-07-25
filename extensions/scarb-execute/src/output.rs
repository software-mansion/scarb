use anyhow::Context;
use anyhow::Result;
use cairo_lang_runner::CairoHintProcessor;
use cairo_vm::vm::errors::trace_errors::TraceError;
use cairo_vm::vm::runners::cairo_runner::CairoRunner;
use console::Style;
use scarb_ui::Message;
use serde::{Serialize, Serializer};
use thousands::Separable;

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
    memory_segment_sizes: Vec<(String, usize)>,
    max_memory_address: usize,
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
            .filter(|(_, v)| *v > 0)
            .collect::<Vec<_>>();
        let syscalls = sort_by_value(&all_used_resources.syscalls)
            .into_iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect::<Vec<_>>();

        let mut memory_segment_addresses = runner.get_memory_segment_addresses()?;

        let (trace_first, trace_last) = runner
            .relocated_trace
            .as_ref()
            .map(|trace| (trace.first().unwrap(), trace.last().unwrap()))
            .ok_or(TraceError::TraceNotRelocated)?;
        memory_segment_addresses.insert("execution", (trace_first.ap, trace_last.ap));

        let max_memory_address = memory_segment_addresses
            .values()
            .map(|(_, stop_ptr)| *stop_ptr)
            .max()
            .unwrap_or(0);

        let mut memory_segment_sizes: Vec<(String, usize)> = memory_segment_addresses
            .into_iter()
            .map(|(segment, (start_ptr, stop_ptr))| (segment.to_string(), stop_ptr - start_ptr))
            .filter(|(_, size)| *size > 0)
            .collect();
        memory_segment_sizes.sort_by(|(_, a), (_, b)| b.cmp(a));

        Ok(Self {
            n_steps: resources.n_steps,
            n_memory_holes: resources.n_memory_holes,
            builtin_instance_counter,
            syscalls,
            memory_segment_sizes,
            max_memory_address,
        })
    }
}

impl Message for ExecutionResources {
    fn print_text(self)
    where
        Self: Sized,
    {
        println!("Resources:");
        println!("\t{}", format_property("steps", self.n_steps));
        println!(
            "\t{}",
            format_property("max memory address", self.max_memory_address)
        );
        println!("\t{}", format_property("memory holes", self.n_memory_holes));
        println!(
            "\tbuiltins:\n{}",
            format_items(&self.builtin_instance_counter)
        );
        println!(
            "\tmemory segments:\n{}",
            format_items(&self.memory_segment_sizes)
        );
        if !self.syscalls.is_empty() {
            println!("\tsyscalls:\n{}", format_items(&self.syscalls));
        }
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
    K: std::fmt::Display,
    V: std::fmt::Display + Separable,
{
    items
        .iter()
        .map(|(key, value)| format!("\t\t{}", format_property(key, value)))
        .collect::<Vec<String>>()
        .join("\n")
}

fn format_property<K, V>(key: K, value: V) -> String
where
    K: std::fmt::Display,
    V: std::fmt::Display + Separable,
{
    format!(
        "{key}: {}",
        Style::new()
            .yellow()
            .bold()
            .apply_to(value.separate_with_commas())
    )
}

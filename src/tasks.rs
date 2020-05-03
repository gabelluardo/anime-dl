use anyhow::Result;
use colored::Colorize;

use std::thread::JoinHandle;

type Task = JoinHandle<Result<()>>;

pub struct Tasks {
    tasks: Vec<Task>,
}

impl Tasks {
    pub fn new() -> Self {
        Self { tasks: vec![] }
    }

    pub fn add(&mut self, task: Task) {
        self.tasks.push(task)
    }

    pub fn unpark_and_join(&mut self, max_threads: usize) {
        let mut actived: Vec<Task> = vec![];

        for _ in 0..self.tasks.len() {
            let t = self.tasks.remove(0);

            t.thread().unpark();
            actived.push(t);

            if actived.len() >= max_threads {
                self.print_result(actived.remove(0))
            }
        }

        for t in actived {
            self.print_result(t)
        }
    }

    fn print_result(&self, t: Task) {
        match t.join().unwrap() {
            Ok(_) => (),
            Err(e) => eprintln!("{}", format!("[ERROR] {}", e).red()),
        }
    }
}

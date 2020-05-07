use anyhow::Result;

use std::thread::JoinHandle;

type Task = JoinHandle<Result<()>>;

pub struct TaskPool {
    tasks: Vec<Task>,
    max_tasks: usize,
}

impl TaskPool {
    pub fn new(max_tasks: usize) -> Self {
        Self {
            tasks: vec![],
            max_tasks,
        }
    }

    pub fn add(&mut self, task: Task) {
        self.tasks.push(task)
    }

    pub fn unpark_and_join(&mut self) {
        let mut actived: Vec<Task> = vec![];

        for _ in 0..self.tasks.len() {
            let t = self.tasks.remove(0);

            t.thread().unpark();
            actived.push(t);

            if actived.len() >= self.max_tasks {
                print_result!(actived.remove(0))
            }
        }

        for t in actived {
            print_result!(t)
        }
    }
}

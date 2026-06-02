use spin::Mutex;
use crate::scheduler::task::TaskId;
use crate::process::ProcessId;

struct CurrentMap {
    entries: [(u32, u32); 32],
    count: usize,
}

impl CurrentMap {
    const fn new() -> Self {
        Self { entries: [(0, 0); 32], count: 0 }
    }

    fn associate(&mut self, task: TaskId, proc: ProcessId) {
        for e in self.entries.iter_mut() {
            if e.0 == 0 {
                *e = (task.0, proc.0);
                self.count += 1;
                return;
            }
        }
    }

    fn get_process(&self, task: TaskId) -> Option<ProcessId> {
        self.entries.iter()
            .find(|e| e.0 == task.0)
            .map(|e| ProcessId(e.1))
    }

    fn remove(&mut self, task: TaskId) {
        for e in self.entries.iter_mut() {
            if e.0 == task.0 {
                *e = (0, 0);
                self.count -= 1;
                return;
            }
        }
    }
}

static MAP: Mutex<CurrentMap> = Mutex::new(CurrentMap::new());

pub fn associate(task: TaskId, proc: ProcessId) {
    MAP.lock().associate(task, proc);
}

pub fn remove(task: TaskId) {
    MAP.lock().remove(task);
}

pub fn get_process(task: TaskId) -> Option<ProcessId> {
    MAP.lock().get_process(task)
}

pub fn current_process() -> Option<ProcessId> {
    let task_id = crate::scheduler::current_task_id()?;
    get_process(task_id)
}
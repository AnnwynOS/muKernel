pub mod context;
pub mod task;
pub mod switch;
pub mod current;

use spin::Mutex;
use task::{Task, TaskId, TaskState, PriorityClass, TaskStack};
use context::TaskContext;
use crate::debug::log::Logger;

const MAX_TASKS: usize = 16;

use core::cell::UnsafeCell;

struct StackPool(UnsafeCell<[TaskStack; MAX_TASKS]>);
unsafe impl Sync for StackPool {}

static TASK_STACKS: StackPool = StackPool(UnsafeCell::new({
    const EMPTY: TaskStack = TaskStack::new();
    [
        EMPTY, EMPTY, EMPTY, EMPTY,
        EMPTY, EMPTY, EMPTY, EMPTY,
        EMPTY, EMPTY, EMPTY, EMPTY,
        EMPTY, EMPTY, EMPTY, EMPTY,
    ]
}));

struct Sched {
    tasks: [Option<Task>; MAX_TASKS],
    current: usize,
    next_id: u32,
    tick_count: u64,
    started: bool,
}

impl Sched {
    const fn new() -> Self {
        Self {
            tasks: [
                None, None, None, None,
                None, None, None, None,
                None, None, None, None,
                None, None, None, None,
            ],
            current: 0,
            next_id: 1,
            tick_count: 0,
            started: false,
        }
    }

    fn pick_next(&self) -> usize {
        for class in [
            PriorityClass::RealTime,
            PriorityClass::Interactive,
            PriorityClass::Productive,
            PriorityClass::Background,
        ] {
            let start = (self.current + 1) % MAX_TASKS;
            let mut i = start;
            loop {
                if let Some(t) = &self.tasks[i] {
                    if t.priority == class && t.state == TaskState::Ready {
                        return i;
                    }
                }
                i = (i + 1) % MAX_TASKS;
                if i == start { break; }
            }
        }
        self.current
    }

    fn stack_top(slot: usize) -> u64 {
        unsafe {
            let stacks = &*TASK_STACKS.0.get();
            stacks[slot].top_addr()
        }
    }
}

static SCHED: Mutex<Sched> = Mutex::new(Sched::new());

pub fn spawn(name: &'static str, priority: PriorityClass, entry: fn() -> !) -> Option<TaskId> {
    let mut s = SCHED.lock();
    for i in 0..MAX_TASKS {
        if s.tasks[i].is_none() {
            let stack_top = Sched::stack_top(i);
            let id = TaskId(s.next_id);
            s.next_id += 1;
            s.tasks[i] = Some(Task::new(id, name, priority, entry as u64, stack_top));
            return Some(id);
        }
    }
    None
}

pub fn start() {
    let mut s = SCHED.lock();
    if let Some(t) = &mut s.tasks[0] {
        t.state = TaskState::Running;
    }
    s.started = true;
    Logger::log("≺SCHED≻ Started");
}

pub fn tick_count() -> u64 {
    SCHED.lock().tick_count
}

pub fn block_current() {
    let mut s = SCHED.lock();
    let cur = s.current;
    if let Some(t) = &mut s.tasks[cur] {
        t.state = TaskState::Blocked;
    }
}

pub fn unblock(id: TaskId) {
    let mut s = SCHED.lock();
    for slot in s.tasks.iter_mut().flatten() {
        if slot.id == id && slot.state == TaskState::Blocked {
            slot.state = TaskState::Ready;
            return;
        }
    }
}

/// # Safety : appelé depuis handler d'interruption, interruptions désactivées.
pub unsafe fn on_tick() -> Option<(*mut TaskContext, *const TaskContext)> {
    let mut s = SCHED.try_lock()?;
    s.tick_count += 1;
    if !s.started { return None; }

    let cur = s.current;
    if let Some(t) = &mut s.tasks[cur] {
        t.ticks += 1;
    }

    let next = s.pick_next();
    if next == cur { return None; }

    if let Some(t) = &mut s.tasks[cur] {
        if t.state == TaskState::Running { t.state = TaskState::Ready; }
    }
    if let Some(t) = &mut s.tasks[next] {
        t.state = TaskState::Running;
    }
    s.current = next;

    let old_ctx = &mut s.tasks[cur].as_mut()?.context as *mut TaskContext;
    let new_ctx = & s.tasks[next].as_ref()?.context as *const TaskContext;

    drop(s);
    Some((old_ctx, new_ctx))
}

pub fn current_task_id() -> Option<task::TaskId> {
    let s = SCHED.try_lock()?;
    if !s.started { return None; }
    s.tasks[s.current].as_ref().map(|t| t.id)
}
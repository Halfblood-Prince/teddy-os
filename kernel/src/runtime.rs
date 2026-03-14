use spin::Mutex;
use teddy_boot_proto::BootInfo;

use crate::{input, shell, timer};

const MAX_TASKS: usize = 3;

#[derive(Clone, Copy)]
pub struct TaskContext {
    pub tick_count: u64,
}

#[derive(Clone, Copy)]
struct Task {
    callback: fn(TaskContext),
}

struct Scheduler {
    tasks: [Option<Task>; MAX_TASKS],
    task_count: usize,
    next_index: usize,
}

impl Scheduler {
    const fn new() -> Self {
        Self {
            tasks: [None, None, None],
            task_count: 0,
            next_index: 0,
        }
    }

    fn register(&mut self, task: Task) {
        if self.task_count < MAX_TASKS {
            self.tasks[self.task_count] = Some(task);
            self.task_count += 1;
        }
    }

    fn run_next(&mut self, tick_count: u64) {
        if self.task_count == 0 {
            return;
        }

        let task = self.tasks[self.next_index % self.task_count]
            .expect("scheduler slot should be populated");
        self.next_index = (self.next_index + 1) % self.task_count;
        (task.callback)(TaskContext { tick_count });
    }
}

static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

pub fn init(boot_info: &BootInfo) {
    shell::init(boot_info.framebuffer);

    let mut scheduler = SCHEDULER.lock();
    scheduler.register(Task {
        callback: pump_keyboard_task,
    });
    scheduler.register(Task {
        callback: pump_mouse_task,
    });
    scheduler.register(Task {
        callback: render_task,
    });
}

pub fn run_next_task() {
    let tick_count = timer::ticks();
    SCHEDULER.lock().run_next(tick_count);
}

fn pump_keyboard_task(_context: TaskContext) {
    while let Some(event) = input::pop_keyboard_event() {
        shell::handle_keyboard_event(event);
    }
}

fn pump_mouse_task(_context: TaskContext) {
    while let Some(event) = input::pop_mouse_event() {
        shell::handle_mouse_event(event);
    }
}

fn render_task(context: TaskContext) {
    shell::render(context.tick_count);
}

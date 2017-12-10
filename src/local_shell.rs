use job_handle::ChildProcess;
use process_manager::PROCESS_MANAGER;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

/// Thread local shell.
pub struct LocalShell {
    processes: Vec<Arc<RwLock<ChildProcess>>>
}

impl LocalShell {
    fn new() -> LocalShell {
        LocalShell {
            processes: Vec::new()
        }
    }

    pub fn add_process(&mut self, process: &Arc<RwLock<ChildProcess>>) {
    }

    pub fn remove_process(&mut self, process: &Arc<RwLock<ChildProcess>>) {
    }
}

pub struct LocalShellHolder(Arc<Mutex<LocalShell>>);

impl LocalShellHolder {
    fn new() -> LocalShellHolder {
        let mut lock = PROCESS_MANAGER.lock().unwrap();
        let arc = Arc::new(Mutex::new(LocalShell::new()));
        lock.add_local_shell(&arc);
        LocalShellHolder(arc)
    }
}

impl Deref for LocalShellHolder {
    type Target = Mutex<LocalShell>;
    fn deref(&self) -> &Mutex<LocalShell> {
        self.0.deref()
    }
}

impl Drop for LocalShellHolder {
    fn drop(&mut self) {
        let mut lock = PROCESS_MANAGER.lock().unwrap();
        lock.remove_local_shell(&self.0);
    }
}

thread_local! {
    pub static LOCAL_SHELL: LocalShellHolder = LocalShellHolder::new();
}

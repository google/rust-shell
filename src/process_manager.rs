use libc::c_int;
use libc;
use result::ShellResultExt;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::thread::ThreadId;
use local_shell::LocalShell;
use job_handle::ChildProcess;


struct ProcessEntry {
    thread_id: ThreadId,
    process: Arc<RwLock<ChildProcess>>
}

/// Managing global child process state.
pub struct ProcessManager {
    children: Vec<ProcessEntry>
}

impl ProcessManager {
    extern fn handle_signal(signal: c_int) {
        ::std::thread::spawn(move || {
            let mut lock = PROCESS_MANAGER.lock().unwrap();
            for entry in lock.children.drain(..) {
                entry.process.read().unwrap().signal(signal).print_error();
            }
            ::std::process::exit(128 + signal);
        });
    }

    fn new() -> ProcessManager {
        let result = unsafe {
            libc::signal(libc::SIGINT, ProcessManager::handle_signal as usize)
        };
        if result == ::libc::SIG_ERR {
            panic!("signal failed");
        }
        let result = unsafe {
            libc::signal(libc::SIGTERM, ProcessManager::handle_signal as usize)
        };
        if result == ::libc::SIG_ERR {
            panic!("signal failed");
        }
        ProcessManager {
            children: Vec::new()
        }
    }

    pub fn add_local_shell(&mut self, shell: &Arc<Mutex<LocalShell>>) {
    }

    pub fn remove_local_shell(&mut self, shell: &Arc<Mutex<LocalShell>>) {
    }

    pub fn remove_job(&mut self, job: &Arc<RwLock<ChildProcess>>) {
        self.children.retain(|entry| !Arc::ptr_eq(&entry.process, job));
    }

    pub fn signal_thread_jobs(&mut self, id: &ThreadId, signal: c_int) {
        for entry in &self.children {
            if entry.thread_id != *id {
                continue;
            }
            entry.process.read().unwrap().signal(signal).print_error();
        }
    }
}

lazy_static! {
    pub static ref PROCESS_MANAGER: Mutex<ProcessManager> =
        Mutex::new(ProcessManager::new());
}

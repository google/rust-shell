use libc::c_int;
use libc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::ThreadId;
use local_shell::LocalShell;
use std::collections::HashMap;

/// Managing global child process state.
pub struct ProcessManager {
    children: HashMap<ThreadId, Arc<Mutex<LocalShell>>>
}

impl ProcessManager {
    extern fn handle_signal(signal: c_int) {
        ::std::thread::spawn(move || {
            let mut lock = PROCESS_MANAGER.lock().unwrap();
            for (_, entry) in lock.children.drain() {
                let mut lock = entry.lock().unwrap();
                lock.signal(signal);
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
            children: HashMap::new()
        }
    }

    pub fn add_local_shell(&mut self, id: &ThreadId, 
                           shell: &Arc<Mutex<LocalShell>>) {
        self.children.insert(id.clone(), shell.clone());
    }

    pub fn remove_local_shell(&mut self, id: &ThreadId) {
        self.children.remove(id);
    }

    pub fn signal_thread_jobs(&mut self, id: &ThreadId, signal: c_int) {
        for (_, local_shell) in &self.children {
            let mut lock = local_shell.lock().unwrap();
            if lock.thread_id() == id {
                lock.signal(signal);
            }
        }
    }
}

lazy_static! {
    pub static ref PROCESS_MANAGER: Mutex<ProcessManager> =
        Mutex::new(ProcessManager::new());
}

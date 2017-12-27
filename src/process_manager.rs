use libc::c_int;
use libc;
use libc::sigset_t;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::ThreadId;
use local_shell::LocalShell;
use std::collections::HashMap;
use result::check_errno;
use result::ShellError;
use std::mem;
use errno::Errno;
use std::thread;

/// Managing global child process state.
pub struct ProcessManager {
    children: HashMap<ThreadId, Arc<Mutex<LocalShell>>>
}

impl ProcessManager {
    fn new() -> ProcessManager {
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
}

/// Traps SIGINT and SIGTERM, waits for child process completion, and exits
/// the current process.
///
/// It must be invoked before any thread is launched, because it internally
/// uses pthread_sigmask.
#[allow(dead_code)]
pub fn trap_signal_and_wait_children() -> Result<(), ShellError> {
    unsafe {
        let mut sigset = mem::uninitialized::<sigset_t>();
        check_errno("sigemptyset",
                    libc::sigemptyset(&mut sigset as *mut sigset_t))?;
        check_errno("sigaddset", libc::sigaddset(
                &mut sigset as *mut sigset_t, libc::SIGINT))?;
        check_errno("sigaddset", libc::sigaddset(
                &mut sigset as *mut sigset_t, libc::SIGTERM))?;

        let mut oldset = mem::uninitialized::<sigset_t>();
        let result = libc::pthread_sigmask(
            libc::SIG_BLOCK, &mut sigset as *mut sigset_t,
            &mut oldset as *mut sigset_t);
        if result != 0 {
            return Err(ShellError::Errno("pthread_sigmask", Errno(result)));
        }

        thread::spawn(move || {
            info!("Start waitinig signal");
            let mut signal: c_int = 0;
            let result = libc::sigwait(
                &sigset as *const sigset_t, &mut signal as *mut c_int);
            if result != 0 {
                eprintln!("sigwait failed {}", result);
                return;
            }
            info!("Signal {} is received", signal);
            let mut lock = PROCESS_MANAGER.lock().unwrap();
            let mut children = lock.children.drain().collect::<Vec<_>>();
            info!("Wait for {} child processes exiting", children.len());
            for &mut (_, ref entry) in &mut children {
                let mut lock = entry.lock().unwrap();
                lock.wait();
            }
            ::std::process::exit(128 + signal);
        });
        Ok(())
    }
}

lazy_static! {
    pub static ref PROCESS_MANAGER: Mutex<ProcessManager> =
        Mutex::new(ProcessManager::new());
}

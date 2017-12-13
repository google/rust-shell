use job_handle::ChildProcess;
use libc::c_int;
use libc;
use process_manager::PROCESS_MANAGER;
use result::ShellResultExt;
use std::any::Any;
use std::cell::RefCell;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::thread::JoinHandle;
use std::thread::ThreadId;
use std::thread;

/// Thread local shell.
pub struct LocalShell {
    thread_id: ThreadId,
    processes: Vec<Arc<RwLock<ChildProcess>>>,
    signaled: bool
}

impl LocalShell {
    fn new() -> LocalShell {
        LocalShell {
            thread_id: thread::current().id(),
            processes: Vec::new(),
            signaled: false
        }
    }

    pub fn add_process(&mut self, process: &Arc<RwLock<ChildProcess>>) {
        self.processes.push(process.clone());
    }

    pub fn remove_process(&mut self, process: &Arc<RwLock<ChildProcess>>) {
        self.processes.retain(|p| !Arc::ptr_eq(p, process));
    }

    pub fn signal(&mut self, signal: c_int) {
        self.signaled = true;
        for process in &self.processes {
            let lock = process.read().unwrap();
            lock.signal(signal).print_error();
        }
    }

    pub fn signaled(&self) -> bool {
        self.signaled
    }

    pub fn thread_id(&self) -> &ThreadId {
        &self.thread_id
    }
}

struct LocalShellScope(ThreadId, Arc<Mutex<LocalShell>>);

impl LocalShellScope {
    fn new(arc: &Arc<Mutex<LocalShell>>) -> LocalShellScope {
        let mut lock = PROCESS_MANAGER.lock().unwrap();
        let id = thread::current().id();
        lock.add_local_shell(&id, arc);

        LocalShellScope(id, arc.clone())
    }
}

impl Default for LocalShellScope {
    fn default() -> LocalShellScope {
        let id = thread::current().id();
        LocalShellScope(id, Arc::new(Mutex::new(LocalShell::new())))
    }
}

impl Drop for LocalShellScope {
    fn drop(&mut self) {
        let mut lock = PROCESS_MANAGER.lock().unwrap();
        lock.remove_local_shell(&self.0);
    }
}

pub struct ShellHandle<T> {
    join_handle: JoinHandle<T>,
    shell: Arc<Mutex<LocalShell>>
}

impl <T> ShellHandle<T> {
    pub fn signal(&self, signal: c_int) {
        let mut lock = self.shell.lock().unwrap();
        lock.signal(signal);
    }

    pub fn terminate(self) -> Result<T, Box<Any + Send + 'static>> {
        self.signal(libc::SIGTERM);
        self.join_handle.join()
    }

    pub fn join(self) -> Result<T, Box<Any + Send + 'static>> {
        self.join_handle.join()
    }
}

impl <T> Deref for ShellHandle<T> {
    type Target = JoinHandle<T>;
    fn deref(&self) -> &Self::Target {
        &self.join_handle
    }
}

pub fn spawn<F, T>(f: F) -> ShellHandle<T> where
        F: FnOnce() -> T, F: Send + 'static, T: Send + 'static {
    let arc = Arc::new(Mutex::new(LocalShell::new()));
    let arc_clone = arc.clone();
    let join_handle = thread::spawn(move || -> T {
        LOCAL_SHELL_SCOPE.with(|shell| {
            let mut shell = shell.borrow_mut();
            if shell.is_some() {
                panic!("Shell has already registered");
            }
            *shell = Some(LocalShellScope::new(&arc_clone));
        });
        f()
    });
    ShellHandle {
        join_handle: join_handle,
        shell: arc
    }
}

pub fn current_shell() -> Arc<Mutex<LocalShell>> {
    LOCAL_SHELL_SCOPE.with(|shell| {
        shell.borrow_mut()
            .get_or_insert(LocalShellScope::default()).1.clone()
    })
}

thread_local! {
    static LOCAL_SHELL_SCOPE: RefCell<Option<LocalShellScope>> =
        RefCell::new(None);
}


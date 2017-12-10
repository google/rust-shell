use std::cell::RefCell;
use std::sync::Arc;
use process_manager::PROCESS_MANAGER;

/// Thread local shell.
pub struct LocalShell {

}

impl LocalShell {
    fn new() -> LocalShell {
        LocalShell {}
    }
}

struct LocalShellHolder(Arc<RefCell<LocalShell>>);

impl LocalShellHolder {
    fn new() -> LocalShellHolder {
        let arc = Arc::new(RefCell::new(LocalShell {}));
        PROCESS_MANAGER.lock().unwrap().add_local_shell(&arc);
        LocalShellHolder(arc)
    }
}

impl Drop for LocalShellHolder {
    fn drop(&mut self) {
        PROCESS_MANAGER.lock().unwrap().remove_local_shell(&self.0);
    }
}

thread_local! {
  static LOCAL_SHELL: LocalShellHolder = LocalShellHolder::new();
}

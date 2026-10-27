use sysinfo::{System, ProcessesToUpdate};
use std::time::Duration;
use std::thread;

fn main() {
    let mut system = System::new_all();
    
    // Spawn a dummy process
    let mut child = std::process::Command::new("ping")
        .args(&["127.0.0.1", "-n", "10"])
        .spawn()
        .unwrap();

    let pid = child.id();
    
    // Immediately check sysinfo
    system.refresh_processes(ProcessesToUpdate::All, true);
    if let Some(process) = system.process(sysinfo::Pid::from_u32(pid)) {
        println!("Immediate poll: name='{}', exe='{:?}'", process.name().to_string_lossy(), process.exe());
    } else {
        println!("Immediate poll: process not found");
    }

    // Wait a bit
    thread::sleep(Duration::from_millis(500));
    system.refresh_processes(ProcessesToUpdate::All, true);
    if let Some(process) = system.process(sysinfo::Pid::from_u32(pid)) {
        println!("Later poll: name='{}', exe='{:?}'", process.name().to_string_lossy(), process.exe());
    }

    let _ = child.kill();
}
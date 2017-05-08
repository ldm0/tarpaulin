use nix::libc::{pid_t, c_long};
use nix::Result;
use ptrace_control::*;
use nix::{Error, Errno};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const INT: u64 = 0xCC;


/// Breakpoint construct used to monitor program execution. As tarpaulin is an
/// automated process, this will likely have less functionality than most 
/// breakpoint implementations.
#[derive(Debug)]
pub struct Breakpoint { 
    /// Current process id
    pub pid: pid_t,
    /// Program counter
    pub pc: u64,
    /// Bottom byte of address data. 
    /// This is replaced to enable the interrupt. Rest of data is never changed.
    data: i64,
    /// Reading from memory with ptrace gives addresses aligned to bytes. 
    /// We therefore need to know the shift to place the breakpoint in the right place
    shift: u64,
    /// When execution pauses. 
    is_running: bool
}

impl Breakpoint {
    /// Creates a new breakpoint for the given process and program counter. 
    pub fn new(pid:pid_t, pc:u64) ->Result<Breakpoint> {
        let aligned = pc & !0x7u64;
        let data = read_address(pid, aligned)?;

        let mut b = Breakpoint{ 
            pid: pid,
            pc: pc,
            data: data,
            shift: 8 * (pc - aligned),
            is_running: true,
        };
        match b.enable() {
            Ok(_) => Ok(b),
            Err(e) => Err(e)
        }
    }

    /// Attaches the current breakpoint.
    fn enable(&mut self) -> Result<c_long> {
        let mut intdata = self.data & (!(0xFFu64 << self.shift) as i64);
        intdata |= (INT << self.shift) as i64;
        write_to_address(self.pid, self.aligned_address(), intdata)
    }
    
    fn disable(&self) -> Result<c_long> {
        write_to_address(self.pid, self.aligned_address(), self.data)
    }

    /// Processes the breakpoint. This steps over the breakpoint
    pub fn process(&mut self) -> Result<bool> {
        println!("Processing");
        if self.is_running {
            self.step()?;
            self.is_running = false;
            Ok(true)
        } else {
            self.enable()?;
            continue_exec(self.pid)?;
            self.is_running = true;
            Ok(false)
        }
    }

    /// Steps past the current breakpoint.
    /// For more advanced coverage may interrogate the variables of a branch.
    fn step(&mut self) -> Result<c_long> {
        // Remove the breakpoint, reset the program counter to step before it
        // hit the breakpoint then step to execute the original instruction.
        self.disable()?;
        // Need to set the program counter back one.
        set_instruction_pointer(self.pid, self.pc)?;
        single_step(self.pid)
    }

    fn aligned_address(&self) -> u64 {
        self.pc & !0x7u64
    }
}
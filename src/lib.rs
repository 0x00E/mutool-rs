#![recursion_limit="128"]
#![feature(
    const_fn,
    drop_types_in_const,
    conservative_impl_trait,
    abi_thiscall,
)]

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate detour;
#[macro_use] extern crate matches;

#[macro_use] extern crate pest_derive;
extern crate pest;

#[macro_use] extern crate serde_json;
#[macro_use] extern crate serde_derive;
extern crate serde;

extern crate musdk as mu;
extern crate muonline_packet;
extern crate strsim;
extern crate knock;
extern crate toml;
extern crate tap;
extern crate hsl;
extern crate num_traits;

#[cfg(windows)] extern crate boolinator;
#[cfg(windows)] extern crate kernel32;
#[cfg(windows)] extern crate user32;
#[cfg(windows)] extern crate winapi;

macro_rules! try_opt {
    ($expr:expr) => (match $expr {
        ::std::option::Option::Some(val) => val,
        ::std::option::Option::None => return,
    })
}

use main::MuTool;
use filter::ItemFilter;

mod ext;
mod filter;
mod main;
mod util;

static mut TOOL: Option<MuTool> = None;

#[no_mangle]
#[allow(non_snake_case)]
#[cfg(windows)]
pub unsafe extern "system" fn DllMain(
    _module: winapi::HINSTANCE,
    reason: winapi::DWORD,
    _reserved: winapi::LPVOID) -> winapi::BOOL {
  use winapi::INVALID_HANDLE_VALUE;

  const DLL_PROCESS_ATTACH: winapi::DWORD = 1;
  const DLL_PROCESS_DETACH: winapi::DWORD = 0;

  static mut LOG_FILE: winapi::HANDLE = INVALID_HANDLE_VALUE;

  match reason {
    DLL_PROCESS_ATTACH => {
      match setup_stdio("mutool.log") {
        Ok(handle) => LOG_FILE = handle,
        Err(error) => {
          let code = kernel32::GetLastError();
          assert!(kernel32::AllocConsole() != 0, "creating console");
          eprintln!("[Main:Error] Failed to setup log file({}): {}", code, error);
        },
      }

      println!("[Main] Initializing MuTool...");
      match MuTool::new() {
        Err(error) => eprintln!("[Main:Error] Failed to initialize: {}", error),
        Ok(tool) => {
          println!("[Main] Initialized MuTool");
          TOOL = Some(tool);
        },
      }
    },
    DLL_PROCESS_DETACH if LOG_FILE != INVALID_HANDLE_VALUE => {
      assert!(kernel32::CloseHandle(LOG_FILE) != 0, "failed to close log file");
    },
    _ => ()
  }

  return winapi::TRUE;
}

#[cfg(windows)]
pub unsafe fn setup_stdio(log_path: &str) -> std::io::Result<winapi::HANDLE> {
  use std::io;
  use boolinator::Boolinator;
  use winapi::{STD_OUTPUT_HANDLE, STD_ERROR_HANDLE, INVALID_HANDLE_VALUE};

  let stdout = kernel32::GetStdHandle(STD_OUTPUT_HANDLE);
  let stderr = kernel32::GetStdHandle(STD_ERROR_HANDLE);

  (stdout != INVALID_HANDLE_VALUE)
    .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "could not retrieve standard output"))?;
  (stderr != INVALID_HANDLE_VALUE)
    .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "could not retrieve standard error"))?;

  let mut name = log_path.encode_utf16().collect::<Vec<_>>();
  name.push(0);

  let output = kernel32::CreateFileW(
    name.as_ptr() as *const _,
    winapi::FILE_GENERIC_WRITE,
    0,
    ::std::ptr::null_mut(),
    winapi::OPEN_ALWAYS,
    winapi::FILE_ATTRIBUTE_NORMAL,
    ::std::ptr::null_mut());

  (output != INVALID_HANDLE_VALUE)
    .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "could not create log file"))?;

  let offset = kernel32::SetFilePointer(output, 0, ::std::ptr::null_mut(), winapi::FILE_END);
  (offset != winapi::INVALID_SET_FILE_POINTER)
    .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "could not set log file offset"))?;

  (kernel32::SetStdHandle(STD_OUTPUT_HANDLE, output) != 0)
    .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "could not redirect standard output"))?;
  //(!kernel32::GetStdHandle(STD_OUTPUT_HANDLE).is_null())
  //  .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "standard output was null"))?;
  (kernel32::SetStdHandle(STD_ERROR_HANDLE, output) != 0)
    .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "could not redirect standard error"))?;
  (!kernel32::GetStdHandle(STD_ERROR_HANDLE).is_null())
    .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "standard error was null"))?;

  if offset > 0 {
    // Append a new line between each run
    println!("");
  }

  //let fd = libc::open_osfhandle(output as _, libc::O_WRONLY | libc::O_TEXT);
  //libc::dup2(fd, 1);
  //libc::dup2(fd, 2);
  //libc::close(fd);

  Ok(output)
}

#[cfg(unix)]
pub unsafe extern "C" fn dll_main() {
  MuTool::new().unwrap();
}
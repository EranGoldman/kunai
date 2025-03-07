use core::mem::{size_of, MaybeUninit};
use ip_network::IpNetwork;
use libc::{clock_gettime, rlimit, timespec, CLOCK_MONOTONIC};
use md5::{Digest, Md5};
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use std::{fs, io, net::IpAddr};
use thiserror::Error;

pub mod account;
pub mod bpf;
pub mod elf;
pub mod namespace;
pub mod uname;
pub mod uptime;

#[inline]
pub fn is_public_ip(ip: IpAddr) -> bool {
    let ip_network: IpNetwork = ip.into();

    match ip_network {
        IpNetwork::V4(v4) => v4.is_global(),
        IpNetwork::V6(v6) => v6.is_global(),
    }
}

/// Function getting time since boot. Does not include
/// suspended time.
pub fn ktime_get_ns() -> Result<u64, io::Error> {
    let mut ts: timespec = unsafe { std::mem::zeroed() };

    // Call clock_gettime with CLOCK_BOOTTIME
    let result = unsafe { clock_gettime(CLOCK_MONOTONIC, &mut ts) };

    if result == 0 {
        // Convert seconds and nanoseconds to total nanoseconds
        Ok(ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64)
    } else {
        Err(io::Error::last_os_error())
    }
}

fn sysconf<T: From<i64>>(var: libc::c_int) -> Result<T, io::Error> {
    let v = unsafe { libc::sysconf(var) };
    if v == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(v.into())
}

#[inline]
pub fn get_clk_tck() -> Result<i64, io::Error> {
    sysconf(libc::_SC_CLK_TCK)
}

#[inline]
pub fn page_size() -> Result<i64, io::Error> {
    sysconf(libc::_SC_PAGESIZE)
}

#[inline]
pub fn page_shift() -> Result<u64, io::Error> {
    let page_size = page_size()?;
    let mut page_shift = 0u64;

    while (1 << page_shift) < page_size {
        page_shift += 1
    }
    Ok(page_shift)
}

#[derive(Debug, Error)]
pub enum RandError {
    #[error("getrandom call failure")]
    CallFailure,
    #[error("getrandom partially randomized")]
    PartiallyRandomized,
}

pub fn get_current_uid() -> libc::uid_t {
    unsafe { libc::getuid() }
}

pub fn getrandom<T: Sized>() -> Result<T, RandError> {
    let mut t = MaybeUninit::<T>::uninit();
    let buflen = size_of::<T>();
    let rc = unsafe { libc::getrandom(t.as_mut_ptr() as *mut _, buflen, 0) };
    if rc == -1 {
        return Err(RandError::CallFailure);
    }
    if rc as usize != buflen {
        return Err(RandError::PartiallyRandomized);
    }
    Ok(unsafe { t.assume_init() })
}

pub fn kill(pid: i32, sig: i32) -> Result<(), io::Error> {
    if unsafe { libc::kill(pid, sig) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[inline(always)]
pub fn getrlimit(resource: u32) -> Result<rlimit, io::Error> {
    let mut rlim: rlimit = rlimit {
        rlim_cur: 0, // Set the soft limit to 0 initially
        rlim_max: 0, // Set the hard limit to 0 initially
    };

    // Get the current limit
    if unsafe { libc::getrlimit(resource, &mut rlim) } != 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(rlim)
}

#[inline(always)]
pub fn setrlimit(resource: u32, rlimit: rlimit) -> Result<(), io::Error> {
    // Set the new limit
    if unsafe { libc::setrlimit(resource, &rlimit) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[inline]
pub fn md5_data<T: AsRef<[u8]>>(data: T) -> String {
    let mut h = Md5::new();
    h.update(data.as_ref());
    hex::encode(h.finalize())
}

#[inline]
pub fn sha1_data<T: AsRef<[u8]>>(data: T) -> String {
    let mut h = Sha1::new();
    h.update(data.as_ref());
    hex::encode(h.finalize())
}

#[inline]
pub fn sha256_data<T: AsRef<[u8]>>(data: T) -> String {
    let mut h = Sha256::new();
    h.update(data.as_ref());
    hex::encode(h.finalize())
}

#[inline]
pub fn sha512_data<T: AsRef<[u8]>>(data: T) -> String {
    let mut h = Sha512::new();
    h.update(data.as_ref());
    hex::encode(h.finalize())
}

#[inline]
pub fn is_bpf_lsm_enabled() -> Result<bool, io::Error> {
    Ok(fs::read_to_string("/sys/kernel/security/lsm")?
        .split(',')
        .any(|s| s == "bpf"))
}

#[cfg(test)]
mod test {
    use crate::util::*;

    #[test]
    fn test_page_size() {
        println!("PAGE_SIZE: {}", page_size().unwrap());
        println!("PAGE_SHIFT: {}", page_shift().unwrap());
    }
}

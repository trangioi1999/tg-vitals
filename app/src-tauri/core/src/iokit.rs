//! Minimal IOKit / IOHID FFI with RAII wrappers. Every pointer is null-checked.
#![allow(non_upper_case_globals, non_camel_case_types)]
use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
use core_foundation_sys::base::{kCFAllocatorDefault, CFAllocatorRef, CFGetTypeID, CFRelease, CFTypeRef};
use core_foundation_sys::dictionary::{CFDictionaryGetValue, CFDictionaryGetTypeID, CFDictionaryRef, CFMutableDictionaryRef};
use core_foundation_sys::number::{
    kCFNumberSInt64Type, CFBooleanGetTypeID, CFBooleanGetValue, CFBooleanRef, CFNumberGetTypeID,
    CFNumberGetValue, CFNumberRef,
};
use core_foundation_sys::string::{CFStringGetTypeID, CFStringRef};
use std::ffi::{c_char, c_void, CString};

type io_object_t = u32;
type kern_return_t = i32;
type mach_port_t = u32;
/// `kIOMainPortDefault` is MACH_PORT_NULL.
const MAIN_PORT: mach_port_t = 0;

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOServiceMatching(name: *const c_char) -> CFMutableDictionaryRef;
    fn IOServiceGetMatchingServices(main: mach_port_t, matching: CFDictionaryRef, iter: *mut io_object_t) -> kern_return_t;
    fn IOIteratorNext(iter: io_object_t) -> io_object_t;
    fn IOObjectRelease(obj: io_object_t) -> kern_return_t;
    fn IORegistryEntryCreateCFProperties(
        entry: io_object_t,
        props: *mut CFMutableDictionaryRef,
        allocator: CFAllocatorRef,
        options: u32,
    ) -> kern_return_t;
    fn IORegistryEntryCreateCFProperty(entry: io_object_t, key: CFStringRef, allocator: CFAllocatorRef, options: u32) -> CFTypeRef;
    fn IOServiceOpen(service: io_object_t, owning_task: mach_port_t, kind: u32, connect: *mut io_object_t) -> kern_return_t;
    fn IOServiceClose(connect: io_object_t) -> kern_return_t;
    fn IOConnectCallStructMethod(
        connect: io_object_t,
        selector: u32,
        input: *const c_void,
        input_size: usize,
        output: *mut c_void,
        output_size: *mut usize,
    ) -> kern_return_t;

    // IOHID event system (private but stable since 2020; used by Stats, macmon…).
    fn IOHIDEventSystemClientCreate(allocator: CFAllocatorRef) -> *mut c_void;
    fn IOHIDEventSystemClientSetMatching(client: *mut c_void, matching: CFDictionaryRef) -> i32;
    fn IOHIDEventSystemClientCopyServices(client: *mut c_void) -> CFArrayRef;
    fn IOHIDServiceClientCopyProperty(service: *const c_void, key: CFStringRef) -> CFTypeRef;
    fn IOHIDServiceClientCopyEvent(service: *const c_void, kind: i64, options: i32, timestamp: i64) -> *mut c_void;
    fn IOHIDEventGetFloatValue(event: *mut c_void, field: i32) -> f64;
}

/// An owned CF object, released on drop.
pub struct Owned(CFTypeRef);

impl Owned {
    pub fn new(ptr: CFTypeRef) -> Option<Self> {
        // Not `then_some(Self(ptr))`: that builds the value eagerly and, when ptr is NULL,
        // drops it — i.e. calls CFRelease(NULL), which traps.
        if ptr.is_null() {
            None
        } else {
            Some(Self(ptr))
        }
    }
    pub fn get(&self) -> CFTypeRef {
        self.0
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: we own one reference.
        unsafe { CFRelease(self.0) }
    }
}

pub struct Service(io_object_t);

impl Drop for Service {
    fn drop(&mut self) {
        // SAFETY: we own the io_object.
        unsafe { IOObjectRelease(self.0) };
    }
}

impl Service {
    /// All registry entries of an IOKit class (e.g. "IOBlockStorageDriver").
    pub fn all(class: &str) -> Vec<Service> {
        let Ok(name) = CString::new(class) else { return Vec::new() };
        let mut iter: io_object_t = 0;
        let mut out = Vec::new();
        // SAFETY: IOServiceGetMatchingServices consumes `matching`.
        unsafe {
            let matching = IOServiceMatching(name.as_ptr());
            if matching.is_null() || IOServiceGetMatchingServices(MAIN_PORT, matching, &mut iter) != 0 {
                return out;
            }
            loop {
                let obj = IOIteratorNext(iter);
                if obj == 0 {
                    break;
                }
                out.push(Service(obj));
            }
            IOObjectRelease(iter);
        }
        out
    }

    /// All properties as an owned CFDictionary.
    pub fn properties(&self) -> Option<Owned> {
        let mut props: CFMutableDictionaryRef = std::ptr::null_mut();
        // SAFETY: valid out-pointer; result follows the create rule.
        let r = unsafe { IORegistryEntryCreateCFProperties(self.0, &mut props, kCFAllocatorDefault, 0) };
        if r != 0 {
            return None;
        }
        Owned::new(props as CFTypeRef)
    }

    pub fn property(&self, key: &str) -> Option<Owned> {
        let key = CFString::new(key);
        // SAFETY: create rule.
        Owned::new(unsafe { IORegistryEntryCreateCFProperty(self.0, key.as_concrete_TypeRef(), kCFAllocatorDefault, 0) })
    }
}

/// Borrowed value from a dictionary (valid while the dictionary lives).
pub fn dict_get(dict: CFTypeRef, key: &str) -> Option<CFTypeRef> {
    // SAFETY: type-checked before use.
    unsafe {
        if dict.is_null() || CFGetTypeID(dict) != CFDictionaryGetTypeID() {
            return None;
        }
        let key = CFString::new(key);
        let v = CFDictionaryGetValue(dict as CFDictionaryRef, key.as_concrete_TypeRef() as *const c_void);
        (!v.is_null()).then_some(v as CFTypeRef)
    }
}

pub fn as_i64(v: CFTypeRef) -> Option<i64> {
    // SAFETY: type-checked.
    unsafe {
        if v.is_null() || CFGetTypeID(v) != CFNumberGetTypeID() {
            return None;
        }
        let mut out: i64 = 0;
        CFNumberGetValue(v as CFNumberRef, kCFNumberSInt64Type, (&mut out as *mut i64).cast()).then_some(out)
    }
}

pub fn as_bool(v: CFTypeRef) -> Option<bool> {
    // SAFETY: type-checked.
    unsafe {
        if v.is_null() || CFGetTypeID(v) != CFBooleanGetTypeID() {
            return None;
        }
        Some(CFBooleanGetValue(v as CFBooleanRef))
    }
}

pub fn as_string(v: CFTypeRef) -> Option<String> {
    // SAFETY: type-checked; wrap_under_get_rule retains, so no double free.
    unsafe {
        if v.is_null() || CFGetTypeID(v) != CFStringGetTypeID() {
            return None;
        }
        Some(CFString::wrap_under_get_rule(v as CFStringRef).to_string())
    }
}

pub fn get_i64(dict: CFTypeRef, key: &str) -> Option<i64> {
    dict_get(dict, key).and_then(as_i64)
}

pub fn get_bool(dict: CFTypeRef, key: &str) -> Option<bool> {
    dict_get(dict, key).and_then(as_bool)
}

/// Apple silicon temperature sensors through the IOHID event system.
pub struct HidThermal {
    client: Owned,
    /// (name, service) — services are borrowed from `services`.
    services: Owned,
}

const HID_EVENT_TYPE_TEMPERATURE: i64 = 15;

impl HidThermal {
    pub fn new() -> Option<Self> {
        use core_foundation::dictionary::CFDictionary;
        use core_foundation::number::CFNumber;
        // SAFETY: create rule; matching dict outlives the call.
        unsafe {
            let client = Owned::new(IOHIDEventSystemClientCreate(kCFAllocatorDefault) as CFTypeRef)?;
            let matching = CFDictionary::from_CFType_pairs(&[
                (CFString::new("PrimaryUsagePage"), CFNumber::from(0xff00i32)),
                (CFString::new("PrimaryUsage"), CFNumber::from(5i32)),
            ]);
            IOHIDEventSystemClientSetMatching(client.get() as *mut c_void, matching.as_concrete_TypeRef());
            let services = Owned::new(IOHIDEventSystemClientCopyServices(client.get() as *mut c_void) as CFTypeRef)?;
            Some(Self { client, services })
        }
    }

    /// (sensor name, °C) for every readable sensor.
    pub fn read(&self) -> Vec<(String, f64)> {
        let _keep = &self.client;
        let mut out = Vec::new();
        let key = CFString::new("Product");
        // SAFETY: services array is owned by self; events follow the create rule.
        unsafe {
            let array = self.services.get() as CFArrayRef;
            for i in 0..CFArrayGetCount(array) {
                let service = CFArrayGetValueAtIndex(array, i);
                if service.is_null() {
                    continue;
                }
                let Some(name) = Owned::new(IOHIDServiceClientCopyProperty(service, key.as_concrete_TypeRef())) else {
                    continue;
                };
                let Some(name) = as_string(name.get()) else { continue };
                let Some(event) = Owned::new(IOHIDServiceClientCopyEvent(service, HID_EVENT_TYPE_TEMPERATURE, 0, 0) as CFTypeRef) else {
                    continue;
                };
                let value = IOHIDEventGetFloatValue(event.get() as *mut c_void, (HID_EVENT_TYPE_TEMPERATURE as i32) << 16);
                out.push((name, value));
            }
        }
        out
    }
}

/// The System Management Controller (`AppleSMC`). Intel Macs keep their temperatures here.
pub struct Smc(io_object_t);

/// `SMCKeyData_t` from Apple's (unpublished) SMC user client; 80 bytes.
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct SmcKeyData {
    key: u32,
    vers: [u8; 6],
    p_limit: SmcPLimit,
    key_info: SmcKeyInfo,
    result: u8,
    status: u8,
    data8: u8,
    data32: u32,
    bytes: [u8; 32],
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct SmcPLimit {
    version: u16,
    length: u16,
    cpu: u32,
    gpu: u32,
    mem: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct SmcKeyInfo {
    data_size: u32,
    data_type: u32,
    attributes: u8,
}

const SMC_HANDLE_YPC_EVENT: u32 = 2;
const SMC_CMD_READ_KEY: u8 = 5;
const SMC_CMD_KEY_INFO: u8 = 9;

impl Smc {
    pub fn open() -> Option<Self> {
        let service = Service::all("AppleSMC").into_iter().next()?;
        let mut conn: io_object_t = 0;
        // SAFETY: valid service and out-pointer; we close `conn` on drop.
        #[allow(deprecated)]
        let r = unsafe { IOServiceOpen(service.0, libc::mach_task_self(), 0, &mut conn) };
        (r == 0 && conn != 0).then_some(Self(conn))
    }

    fn call(&self, input: &SmcKeyData) -> Option<SmcKeyData> {
        let mut out = SmcKeyData::default();
        let mut out_size = std::mem::size_of::<SmcKeyData>();
        // SAFETY: both buffers are SmcKeyData-sized and live for the call.
        let r = unsafe {
            IOConnectCallStructMethod(
                self.0,
                SMC_HANDLE_YPC_EVENT,
                (input as *const SmcKeyData).cast(),
                std::mem::size_of::<SmcKeyData>(),
                (&mut out as *mut SmcKeyData).cast(),
                &mut out_size,
            )
        };
        (r == 0 && out.result == 0).then_some(out)
    }

    /// Raw value of a four-char key: (type, bytes). None when the key does not exist.
    pub fn read(&self, key: &str) -> Option<([u8; 4], Vec<u8>)> {
        let key: [u8; 4] = key.as_bytes().try_into().ok()?;
        let mut input = SmcKeyData { key: u32::from_be_bytes(key), data8: SMC_CMD_KEY_INFO, ..Default::default() };
        let info = self.call(&input)?.key_info;
        let size = (info.data_size as usize).min(32);
        if size == 0 {
            return None;
        }
        input.key_info.data_size = info.data_size;
        input.data8 = SMC_CMD_READ_KEY;
        let out = self.call(&input)?;
        Some((info.data_type.to_be_bytes(), out.bytes[..size].to_vec()))
    }

    /// (key, °C) for every key in `keys` that exists and decodes.
    pub fn temperatures(&self, keys: &[&str]) -> Vec<(String, f64)> {
        keys.iter()
            .filter_map(|k| {
                let (kind, bytes) = self.read(k)?;
                Some((k.to_string(), crate::sensors::smc_decode(&kind, &bytes)?))
            })
            .collect()
    }
}

impl Drop for Smc {
    fn drop(&mut self) {
        // SAFETY: we own the connection.
        unsafe { IOServiceClose(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smc_struct_matches_c_layout() {
        assert_eq!(std::mem::size_of::<SmcKeyData>(), 80);
    }

    /// Real Macs (Intel and Apple silicon) have an SMC; `#KEY` is the key count.
    /// CI virtual machines have no AppleSMC, so the test skips there.
    #[test]
    #[ignore = "needs a real Mac"]
    fn smc_reads_key_count() {
        let Some(smc) = Smc::open() else {
            eprintln!("no AppleSMC (virtual machine?) — skipping");
            return;
        };
        let (kind, bytes) = smc.read("#KEY").expect("#KEY");
        eprintln!("#KEY {:?} {:?}", std::str::from_utf8(&kind), bytes);
        assert_eq!(&kind, b"ui32");
        assert!(smc.read("ZZZZ").is_none());
        // Apple silicon `flt ` / Intel `sp78` temperature keys, whichever this Mac has.
        eprintln!("temps {:?}", smc.temperatures(&["Tp01", "Tp05", "Tp09", "TC0P", "TC1C", "TB0T"]));
    }

    /// Regression: a NULL result must never reach CFRelease (crashed the app on launch).
    #[test]
    fn null_is_not_owned_or_released() {
        assert!(Owned::new(std::ptr::null()).is_none());
    }

    /// Runs on every Mac, including CI VMs where the HID service list comes back NULL.
    #[test]
    fn thermal_client_never_crashes() {
        if let Some(t) = HidThermal::new() {
            let _ = t.read();
        }
    }
}
